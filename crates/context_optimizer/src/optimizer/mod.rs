//! The context optimizer orchestrator: request → content gate → candidate
//! scoring → near-duplicate removal → budget pruning → optimized context
//! bundle.

use std::collections::{HashMap, HashSet};
use std::path::Path;
use std::time::Instant;

use dependency_graph::models::AnalysisResult;
use repository_intelligence::models::{Language, RepositoryAnalysis};
use repository_intelligence::util::{read_text_limited, DEFAULT_MAX_SOURCE_BYTES};

use crate::dedup::{self, DedupConfig};
use crate::error::Result;
use crate::gate::{self, ContentClass};
use crate::models::{
    ContextRequest, DedupGroup, DedupSummary, DroppedFile, DroppedReason, FileContext,
    OptimizationMetrics, OptimizedContext, TokenSummary,
};
use crate::pruner::{PruneCandidate, PruneOutcome, Pruner};
use crate::selector::{score as score_file, FileSignals, QueryTokens};
use crate::tokens;

/// Tunable knobs for context optimization.
#[derive(Debug, Clone)]
pub struct OptimizerConfig {
    /// Maximum source file size (bytes) whose content is read and optimized.
    pub max_source_bytes: u64,
    /// Whether near-duplicate detection is enabled.
    pub dedup_enabled: bool,
    /// Knobs for the near-duplicate pipeline.
    pub dedup_config: DedupConfig,
    /// Hard cap on how many top-ranked files are checked for duplicates.
    pub max_dedup_candidates: usize,
    /// Whether entry point / centrality / reachability boosts are applied.
    pub structural_boost: bool,
    /// Whether the content eligibility gate (binary/media and generated
    /// lockfile exclusion) is applied before scoring.
    pub content_gate_enabled: bool,
}

impl Default for OptimizerConfig {
    fn default() -> Self {
        OptimizerConfig {
            max_source_bytes: DEFAULT_MAX_SOURCE_BYTES,
            dedup_enabled: true,
            dedup_config: DedupConfig::default(),
            max_dedup_candidates: 4096,
            structural_boost: true,
            content_gate_enabled: true,
        }
    }
}

/// A ranked candidate file before final selection.
struct CandidateMeta {
    path: String,
    size: u64,
    language: Option<String>,
    score: f64,
    relevance: f64,
    reasons: Vec<String>,
}

/// The context optimizer engine entry point.
#[derive(Debug, Clone)]
pub struct ContextOptimizer {
    config: OptimizerConfig,
}

impl ContextOptimizer {
    /// Construct an optimizer with custom configuration.
    pub fn new(config: OptimizerConfig) -> Self {
        ContextOptimizer { config }
    }

    /// Construct an optimizer with default configuration.
    pub fn with_defaults() -> Self {
        ContextOptimizer {
            config: OptimizerConfig::default(),
        }
    }

    /// Access the effective configuration.
    pub fn config(&self) -> &OptimizerConfig {
        &self.config
    }

    /// Optimize repository context for `request`.
    ///
    /// `analysis` is the Repository Intelligence output for the repository;
    /// `graph` is the optional Dependency Graph output used for structural
    /// relevance signals.
    pub fn optimize(
        &self,
        analysis: &RepositoryAnalysis,
        graph: Option<&AnalysisResult>,
        request: &ContextRequest,
    ) -> Result<OptimizedContext> {
        let started = Instant::now();
        let query = QueryTokens::new(&request.query);

        let mut warnings = Vec::new();
        if query.is_empty() {
            warnings.push(
                "query produced no tokens; selection uses structural signals only".to_owned(),
            );
        }

        let (candidates, mut dropped, excluded) =
            self.rank_candidates(analysis, graph, &query, request);
        let tokens_before: usize = candidates
            .iter()
            .map(|candidate| tokens::estimate_tokens_from_bytes(candidate.size))
            .sum();

        // -- Near-duplicate detection over the top-ranked candidates --------
        let mut content_cache: HashMap<String, Option<String>> = HashMap::new();
        let mut duplicate_of: HashMap<String, String> = HashMap::new();
        let mut duplicate_similarity: HashMap<String, f64> = HashMap::new();
        let mut dedup_groups: Vec<DedupGroup> = Vec::new();
        let mut dedup_files = 0usize;
        let mut dedup_bytes = 0u64;

        let dedup_limit = self.dedup_limit(candidates.len(), request);
        if self.config.dedup_enabled && dedup_limit >= 2 {
            let mut entries: Vec<(String, String)> = Vec::new();
            for candidate in candidates.iter().take(dedup_limit) {
                let content = self.read_content(&analysis.root, &candidate.path);
                content_cache.insert(candidate.path.clone(), content.clone());
                if let Some(content) = content {
                    entries.push((candidate.path.clone(), content));
                }
            }
            for group in dedup::detect_near_duplicates(&entries, &self.config.dedup_config) {
                dedup_groups.push(DedupGroup {
                    representative: group.representative.clone(),
                    members: group
                        .members
                        .iter()
                        .map(|member| member.path.clone())
                        .collect(),
                    max_similarity: group
                        .members
                        .iter()
                        .map(|member| member.similarity)
                        .fold(0.0, f64::max),
                });
                for member in group.members {
                    duplicate_of.insert(member.path.clone(), group.representative.clone());
                    duplicate_similarity.insert(member.path.clone(), member.similarity);
                }
            }
        }
        dedup_groups.sort_by(|left, right| left.representative.cmp(&right.representative));

        // -- Budget pruning in relevance order ------------------------------
        let mut pruner = Pruner::new(request.max_files, request.max_tokens);
        let mut selected: Vec<FileContext> = Vec::new();

        for candidate in &candidates {
            if let Some(representative) = duplicate_of.get(&candidate.path) {
                let similarity = duplicate_similarity
                    .get(&candidate.path)
                    .copied()
                    .unwrap_or(0.0);
                dropped.push(DroppedFile {
                    path: candidate.path.clone(),
                    reason: DroppedReason::Duplicate,
                    detail: format!(
                        "near-duplicate of `{representative}` (similarity {similarity:.2})"
                    ),
                });
                dedup_files += 1;
                dedup_bytes += candidate.size;
                continue;
            }

            let content = self.cached_content(&mut content_cache, &analysis.root, candidate);
            let tokens = content.as_ref().map_or_else(
                || tokens::estimate_tokens_from_bytes(candidate.size),
                |text| tokens::estimate_tokens(text),
            );

            match pruner.offer(&PruneCandidate {
                path: candidate.path.clone(),
                tokens,
            }) {
                PruneOutcome::Keep => {
                    if content.is_none() {
                        warnings.push(format!(
                            "`{}` content not read (too large or not UTF-8); token estimate uses file size",
                            candidate.path
                        ));
                    }
                    selected.push(FileContext {
                        path: candidate.path.clone(),
                        language: candidate.language.clone(),
                        size_bytes: candidate.size,
                        tokens,
                        score: candidate.score,
                        relevance: candidate.relevance,
                        reasons: candidate.reasons.clone(),
                        content,
                    });
                }
                PruneOutcome::Drop { reason, detail } => {
                    dropped.push(DroppedFile {
                        path: candidate.path.clone(),
                        reason,
                        detail,
                    });
                }
            }
        }

        // -- Aggregate accounting ------------------------------------------
        let files_selected = selected.len();
        let files_dropped_duplicates = dedup_files;
        let files_dropped_budget = dropped
            .iter()
            .filter(|entry| entry.reason == DroppedReason::BudgetExceeded)
            .count();
        let tokens_after: usize = selected.iter().map(|file| file.tokens).sum();
        let bytes_before: u64 = candidates.iter().map(|candidate| candidate.size).sum();
        let bytes_after: u64 = selected.iter().map(|file| file.size_bytes).sum();
        let reduction = if tokens_before == 0 {
            0.0
        } else {
            (tokens_before - tokens_after) as f64 / tokens_before as f64
        };
        let within_budget = match request.max_tokens {
            Some(limit) => tokens_after <= limit,
            None => true,
        };
        let files_dropped_non_text = dropped
            .iter()
            .filter(|entry| entry.reason == DroppedReason::NonText)
            .count();
        let files_dropped_generated = dropped
            .iter()
            .filter(|entry| entry.reason == DroppedReason::Generated)
            .count();
        let files_dropped_low_relevance = dropped
            .iter()
            .filter(|entry| entry.reason == DroppedReason::LowRelevance)
            .count();

        Ok(OptimizedContext {
            root: analysis.root.clone(),
            optimizer_version: env!("CARGO_PKG_VERSION").to_owned(),
            query: request.query.clone(),
            selected,
            dropped,
            dedup: DedupSummary {
                groups: dedup_groups,
                files_collapsed: dedup_files,
                bytes_saved: dedup_bytes,
            },
            tokens: TokenSummary {
                tokens_before,
                tokens_after,
                budget: request.max_tokens,
                within_budget,
            },
            metrics: OptimizationMetrics {
                files_considered: candidates.len(),
                files_selected,
                files_dropped_budget,
                files_dropped_duplicates,
                files_dropped_non_text,
                files_dropped_generated,
                files_dropped_low_relevance,
                files_excluded: excluded,
                bytes_before,
                bytes_after,
                tokens_before,
                tokens_after,
                token_reduction_percent: reduction * 100.0,
                redundancy_ratio: reduction,
                duration_ms: started.elapsed().as_secs_f64() * 1000.0,
            },
            warnings,
        })
    }

    /// Score and sort every candidate file by relevance.
    ///
    /// Returns the ranked candidates, the files dropped by the content gate
    /// (binary/media, generated lockfiles) and the `min_relevance` threshold,
    /// and the number of files excluded by include/exclude filters.
    fn rank_candidates(
        &self,
        analysis: &RepositoryAnalysis,
        graph: Option<&AnalysisResult>,
        query: &QueryTokens,
        request: &ContextRequest,
    ) -> (Vec<CandidateMeta>, Vec<DroppedFile>, usize) {
        let min_relevance = request.min_relevance.map(|value| value.clamp(0.0, 1.0));
        let entry_points: HashSet<&str> = analysis
            .entry_points
            .iter()
            .map(|entry| entry.path.as_str())
            .collect();
        let mut module_by_file: HashMap<&str, &str> = HashMap::new();
        for module in &analysis.modules {
            for file in &module.files {
                module_by_file.insert(file.as_str(), module.name.as_str());
            }
        }
        let graph_sets: Option<(HashSet<&str>, HashSet<&str>)> = graph.map(|graph| {
            let central: HashSet<&str> = graph
                .central_files
                .iter()
                .map(|node| node.id.as_str())
                .collect();
            let reachable: HashSet<&str> = graph
                .reachability
                .reachable_nodes
                .iter()
                .map(String::as_str)
                .collect();
            (central, reachable)
        });

        let mut candidates: Vec<CandidateMeta> = Vec::with_capacity(analysis.files.len());
        let mut dropped: Vec<DroppedFile> = Vec::new();
        let mut excluded = 0usize;
        for file in &analysis.files {
            if !matches_filters(&file.path, &request.include_paths, &request.exclude_paths) {
                excluded += 1;
                continue;
            }
            if self.config.content_gate_enabled {
                match gate::classify(&file.path) {
                    ContentClass::Source => {}
                    ContentClass::NonText => {
                        dropped.push(DroppedFile {
                            path: file.path.clone(),
                            reason: DroppedReason::NonText,
                            detail: "binary or media asset (non-text)".to_owned(),
                        });
                        continue;
                    }
                    ContentClass::Generated => {
                        dropped.push(DroppedFile {
                            path: file.path.clone(),
                            reason: DroppedReason::Generated,
                            detail: "generated lockfile or bundle".to_owned(),
                        });
                        continue;
                    }
                }
            }
            let (score, reasons) = score_file(
                &FileSignals {
                    path: &file.path,
                    language: file.language.map(Language::as_str),
                    module: module_by_file.get(file.path.as_str()).copied(),
                    is_entrypoint: entry_points.contains(file.path.as_str()),
                    is_central: graph_sets
                        .as_ref()
                        .is_some_and(|(central, _)| central.contains(file.path.as_str())),
                    is_reachable: graph_sets
                        .as_ref()
                        .is_some_and(|(_, reachable)| reachable.contains(file.path.as_str())),
                },
                query,
                self.config.structural_boost,
            );
            candidates.push(CandidateMeta {
                path: file.path.clone(),
                size: file.size,
                language: file.language.map(|language| language.as_str().to_owned()),
                score,
                relevance: 0.0,
                reasons,
            });
        }

        let max_score = candidates
            .iter()
            .map(|candidate| candidate.score)
            .fold(0.0, f64::max);
        for candidate in &mut candidates {
            candidate.relevance = if max_score > 0.0 {
                candidate.score / max_score
            } else {
                0.0
            };
        }
        if let Some(threshold) = min_relevance {
            let mut kept: Vec<CandidateMeta> = Vec::with_capacity(candidates.len());
            for candidate in candidates {
                if candidate.relevance >= threshold {
                    kept.push(candidate);
                } else {
                    dropped.push(DroppedFile {
                        path: candidate.path.clone(),
                        reason: DroppedReason::LowRelevance,
                        detail: format!(
                            "relevance {:.2} below minimum {threshold:.2}",
                            candidate.relevance
                        ),
                    });
                }
            }
            candidates = kept;
        }
        candidates.sort_by(|left, right| {
            right
                .score
                .total_cmp(&left.score)
                .then_with(|| left.path.cmp(&right.path))
        });
        (candidates, dropped, excluded)
    }

    /// Read and cache file content, reusing an earlier read when available.
    fn cached_content(
        &self,
        cache: &mut HashMap<String, Option<String>>,
        root: &str,
        candidate: &CandidateMeta,
    ) -> Option<String> {
        if let Some(content) = cache.get(&candidate.path) {
            return content.clone();
        }
        let content = self.read_content(root, &candidate.path);
        cache.insert(candidate.path.clone(), content.clone());
        content
    }

    /// Read a repository-relative file's content, bounded by the configured
    /// maximum source size.
    fn read_content(&self, root: &str, rel: &str) -> Option<String> {
        let path = Path::new(root).join(rel);
        read_text_limited(&path, self.config.max_source_bytes)
            .ok()
            .flatten()
    }

    /// How many top-ranked files to check for duplicates.
    fn dedup_limit(&self, candidate_len: usize, request: &ContextRequest) -> usize {
        let mut limit = self.config.max_dedup_candidates;
        if let Some(max_files) = request.max_files {
            limit = limit.min(max_files.saturating_mul(4).max(128));
        }
        limit.min(candidate_len)
    }
}

/// Apply `include`/`exclude` filters to a candidate path.
///
/// A pattern matches when it equals the path or is a directory prefix of it.
fn matches_filters(path: &str, include: &[String], exclude: &[String]) -> bool {
    if exclude.iter().any(|pattern| path_matches(path, pattern)) {
        return false;
    }
    if !include.is_empty() && !include.iter().any(|pattern| path_matches(path, pattern)) {
        return false;
    }
    true
}

/// Normalize a filter pattern and check whether `path` falls under it.
fn path_matches(path: &str, pattern: &str) -> bool {
    let pattern = pattern
        .trim()
        .trim_start_matches("./")
        .trim_end_matches('/');
    if pattern.is_empty() {
        return true;
    }
    path == pattern || path.starts_with(&format!("{pattern}/"))
}

/// Optimize repository context with a default-configuration optimizer.
///
/// ```no_run
/// use context_optimizer::{optimize_context, ContextRequest};
/// use dependency_graph::analyze_dependencies;
/// use repository_intelligence::analyze_repository;
///
/// let analysis = analyze_repository("path/to/repo")?;
/// let graph = analyze_dependencies(&analysis);
/// let request = ContextRequest { query: "auth".into(), ..Default::default() };
/// let context = optimize_context(&analysis, Some(&graph), &request)?;
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
pub fn optimize_context(
    analysis: &RepositoryAnalysis,
    graph: Option<&AnalysisResult>,
    request: &ContextRequest,
) -> Result<OptimizedContext> {
    ContextOptimizer::with_defaults().optimize(analysis, graph, request)
}
