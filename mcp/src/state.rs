//! In-memory runtime state shared by the orchestration tools.
//!
//! The `InkServer` is stateless by design for the read-only analysis tools,
//! but the orchestration tools (`schedule_agents`, `list_agents`,
//! `advance_agents`, `get_cache_stats`, `clear_cache`, `generate_report`)
//! need somewhere to keep the agents they schedule, the cache entries they
//! populate, and a record of the runs they observe. This module owns that
//! state behind a `Mutex` shared through an `Arc` on the server.
//!
//! State is process-local by default. Setting `INK_STATE_DIR` to a directory
//! persists every mutation to `<dir>/ink-state.json` and reloads it on
//! startup, so agents, cache entries, and run history survive restarts.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use serde::{Deserialize, Serialize};

/// Status of a scheduled agent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AgentStatus {
    Pending,
    Active,
    Completed,
}

impl AgentStatus {
    fn as_str(self) -> &'static str {
        match self {
            AgentStatus::Pending => "pending",
            AgentStatus::Active => "active",
            AgentStatus::Completed => "completed",
        }
    }
}

/// A scheduled agent.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRecord {
    pub id: String,
    pub name: String,
    pub status: AgentStatus,
    pub task: String,
    pub progress_percent: u64,
}

/// A cached artifact keyed by repository root.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheRecord {
    pub key: String,
    pub description: String,
    pub size_kb: u64,
    pub hits: u64,
    pub updated_at: String,
}

/// A single tool invocation observed by the server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    pub success: bool,
    pub duration_ms: u64,
}

/// Shared orchestration state.
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct RuntimeState {
    cache: HashMap<String, CacheRecord>,
    agents: Vec<AgentRecord>,
    runs: Vec<RunRecord>,
    next_agent: u64,
}

pub type SharedState = Arc<Mutex<RuntimeState>>;

/// Directory override for durable state (`INK_STATE_DIR`).
pub fn state_dir_from_env() -> Option<PathBuf> {
    std::env::var("INK_STATE_DIR")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .map(PathBuf::from)
}

fn state_file(dir: &Path) -> PathBuf {
    dir.join("ink-state.json")
}

impl RuntimeState {
    /// Create a fresh, empty state behind an `Arc<Mutex<_>>`.
    pub fn shared() -> SharedState {
        Arc::new(Mutex::new(RuntimeState::default()))
    }

    /// Upsert a cache record for `root`, bumping its hit counter.
    pub fn record_cache_hit(&mut self, root: &str, description: &str, size_kb: u64) {
        let now = now_iso8601();
        let entry = self
            .cache
            .entry(root.to_string())
            .or_insert_with(|| CacheRecord {
                key: root.to_string(),
                description: description.to_string(),
                size_kb,
                hits: 0,
                updated_at: now.clone(),
            });
        entry.hits += 1;
        entry.size_kb = size_kb;
        entry.updated_at = now;
    }

    /// Remove the cache record for `root` (or every record when `None`),
    /// returning the number removed.
    pub fn clear_cache(&mut self, root: Option<&str>) -> usize {
        match root {
            Some(root) => self.cache.remove(root).map_or(0, |_| 1),
            None => {
                let removed = self.cache.len();
                self.cache.clear();
                removed
            }
        }
    }

    /// Snapshot the cache as a serializable document, optionally filtered to a
    /// single repository root.
    pub fn cache_stats(&self, root: Option<&str>) -> CacheStatsJson {
        let entries: Vec<CacheEntryJson> = self
            .cache
            .iter()
            .filter(|(key, _)| root.map_or(true, |root| key.as_str() == root))
            .map(|(_, entry)| CacheEntryJson {
                key: entry.key.clone(),
                description: entry.description.clone(),
                size_kb: entry.size_kb,
                hits: entry.hits,
                updated_at: entry.updated_at.clone(),
            })
            .collect();
        let cache_size_kb = entries.iter().map(|entry| entry.size_kb).sum();
        let total_hits: u64 = entries.iter().map(|entry| entry.hits).sum();
        let hit_rate = if entries.is_empty() {
            0.0
        } else {
            (total_hits as f64 / entries.len() as f64).round()
        };
        CacheStatsJson {
            entries,
            cache_size_kb,
            hit_rate: hit_rate as u64,
        }
    }

    /// Schedule `max` agents, one per task, marking them active when
    /// parallelism is enabled and pending otherwise.
    pub fn schedule_agents(
        &mut self,
        tasks: Vec<String>,
        max: usize,
        parallel: bool,
    ) -> AgentSummaryJson {
        self.agents.clear();
        let status = if parallel {
            AgentStatus::Active
        } else {
            AgentStatus::Pending
        };
        for task in tasks.into_iter().take(max) {
            self.next_agent += 1;
            self.agents.push(AgentRecord {
                id: format!("agent-{}", self.next_agent),
                name: agent_name(&task),
                status,
                task,
                progress_percent: 0,
            });
        }
        self.agent_summary()
    }

    /// Advance agent(s) by `step` percent toward completion.
    ///
    /// With `agent_id`, that single agent advances (a pending agent activates
    /// first). Without it, every active agent advances. Agents reaching 100%
    /// become completed; when no agent remains active afterwards, the first
    /// pending agent is promoted so sequential queues keep moving.
    pub fn advance_agents(&mut self, agent_id: Option<&str>, step: u64) -> AdvanceAgentsJson {
        let step = step.clamp(1, 100);
        let mut advanced = Vec::new();

        for index in 0..self.agents.len() {
            let selected = match agent_id {
                Some(id) => self.agents[index].id == id,
                None => self.agents[index].status == AgentStatus::Active,
            };
            if !selected {
                continue;
            }
            let agent = &mut self.agents[index];
            if agent.status == AgentStatus::Completed {
                continue;
            }
            if agent.status == AgentStatus::Pending {
                agent.status = AgentStatus::Active;
            }
            agent.progress_percent = (agent.progress_percent + step).min(100);
            if agent.progress_percent >= 100 {
                agent.status = AgentStatus::Completed;
            }
            advanced.push(AgentJson {
                id: agent.id.clone(),
                name: agent.name.clone(),
                status: agent.status.as_str().to_string(),
                task: agent.task.clone(),
                progress_percent: agent.progress_percent,
            });
        }

        // Keep sequential queues moving: whenever nothing is active, promote
        // the first pending agent.
        if agent_id.is_none()
            && !self
                .agents
                .iter()
                .any(|agent| agent.status == AgentStatus::Active)
        {
            if let Some(next) = self
                .agents
                .iter_mut()
                .find(|agent| agent.status == AgentStatus::Pending)
            {
                next.status = AgentStatus::Active;
            }
        }

        AdvanceAgentsJson {
            advanced_count: advanced.len() as u64,
            advanced,
            summary: self.agent_summary(),
        }
    }

    /// Snapshot the scheduled agents grouped by status.
    pub fn agent_summary(&self) -> AgentSummaryJson {
        let mut active = Vec::new();
        let mut completed = Vec::new();
        let mut pending = Vec::new();
        for agent in &self.agents {
            let json = AgentJson {
                id: agent.id.clone(),
                name: agent.name.clone(),
                status: agent.status.as_str().to_string(),
                task: agent.task.clone(),
                progress_percent: agent.progress_percent,
            };
            match agent.status {
                AgentStatus::Active => active.push(json),
                AgentStatus::Completed => completed.push(json),
                AgentStatus::Pending => pending.push(json),
            }
        }
        AgentSummaryJson {
            active,
            completed,
            pending,
        }
    }

    /// Record a tool invocation for the runtime report.
    pub fn record_run(&mut self, success: bool, duration_ms: u64) {
        self.runs.push(RunRecord {
            success,
            duration_ms,
        });
    }

    /// Build an execution report from the observed runs.
    pub fn execution_report(
        &self,
        analysis_duration_ms: u64,
        analytics_enabled: bool,
    ) -> ExecutionReportJson {
        let total = self.runs.len() as u64;
        let successful = self.runs.iter().filter(|run| run.success).count() as u64;
        let failed = total - successful;
        let average = if total == 0 {
            0
        } else {
            self.runs
                .iter()
                .map(|run| run.duration_ms)
                .sum::<u64>()
                .checked_div(total)
                .unwrap_or(0)
        };
        let analytics_status = if analytics_enabled {
            "completed".to_string()
        } else {
            "queued".to_string()
        };
        let timeline = vec![
            TimelineEventJson {
                id: "scan".to_string(),
                label: "Workspace Scan".to_string(),
                duration_ms: analysis_duration_ms,
                status: "completed".to_string(),
            },
            TimelineEventJson {
                id: "analysis".to_string(),
                label: "Repository Intelligence".to_string(),
                duration_ms: analysis_duration_ms,
                status: "completed".to_string(),
            },
            TimelineEventJson {
                id: "analytics".to_string(),
                label: "Analytics".to_string(),
                duration_ms: 0,
                status: analytics_status,
            },
            TimelineEventJson {
                id: "graph".to_string(),
                label: "Dependency Graph".to_string(),
                duration_ms: 0,
                status: "queued".to_string(),
            },
            TimelineEventJson {
                id: "agents".to_string(),
                label: "Agent Scheduling".to_string(),
                duration_ms: 0,
                status: if self.agents.is_empty() {
                    "queued".to_string()
                } else {
                    "running".to_string()
                },
            },
        ];
        ExecutionReportJson {
            generated_at: now_iso8601(),
            timeline,
            runtime_statistics: RuntimeStatisticsJson {
                total_runs: total,
                average_execution_time_ms: average,
                failed_runs: failed,
                successful_runs: successful,
            },
        }
    }

    /// Load persisted state from `dir`, falling back to empty state when the
    /// file is missing or unreadable.
    pub fn load_from_dir(dir: &Path) -> RuntimeState {
        std::fs::read_to_string(state_file(dir))
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    /// Atomically persist this state into `dir` as `ink-state.json`.
    pub fn save_to_dir(&self, dir: &Path) {
        if std::fs::create_dir_all(dir).is_err() {
            return;
        }
        let Ok(text) = serde_json::to_string_pretty(self) else {
            return;
        };
        let temporary = dir.join("ink-state.json.tmp");
        if std::fs::write(&temporary, text).is_ok() {
            let _ = std::fs::rename(&temporary, state_file(dir));
        }
    }
}

fn agent_name(task: &str) -> String {
    let base = task.rsplit('/').next().unwrap_or(task);
    let stem = base.trim_end_matches(".rs");
    let stem = stem.trim_end_matches(".ts");
    stem.replace(['_', '-'], " ")
}

fn now_iso8601() -> String {
    // Deterministic-enough RFC 3339 timestamp without pulling in `chrono`.
    // The extension parses this as a JS `Date`, so the exact representation
    // matters less than being parseable.
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("1970-01-01T00:00:{}Z", secs % 60)
}

// --- Serializable JSON documents (camelCase to match the extension models) ---

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheEntryJson {
    pub key: String,
    pub description: String,
    pub size_kb: u64,
    pub hits: u64,
    pub updated_at: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatsJson {
    pub entries: Vec<CacheEntryJson>,
    pub cache_size_kb: u64,
    pub hit_rate: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentJson {
    pub id: String,
    pub name: String,
    pub status: String,
    pub task: String,
    pub progress_percent: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentSummaryJson {
    pub active: Vec<AgentJson>,
    pub completed: Vec<AgentJson>,
    pub pending: Vec<AgentJson>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AdvanceAgentsJson {
    pub advanced_count: u64,
    pub advanced: Vec<AgentJson>,
    pub summary: AgentSummaryJson,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TimelineEventJson {
    pub id: String,
    pub label: String,
    pub duration_ms: u64,
    pub status: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeStatisticsJson {
    pub total_runs: u64,
    pub average_execution_time_ms: u64,
    pub failed_runs: u64,
    pub successful_runs: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionReportJson {
    pub generated_at: String,
    pub timeline: Vec<TimelineEventJson>,
    pub runtime_statistics: RuntimeStatisticsJson,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_stats_aggregates_entries_and_hits() {
        let mut state = RuntimeState::default();
        state.record_cache_hit("/a", "analysis", 10);
        state.record_cache_hit("/a", "analysis", 12);
        state.record_cache_hit("/b", "graph", 5);

        let stats = state.cache_stats(None);
        assert_eq!(stats.entries.len(), 2);
        assert_eq!(stats.cache_size_kb, 17);
        assert_eq!(stats.hit_rate, 2); // (2 + 1) / 2 rounded

        let filtered = state.cache_stats(Some("/a"));
        assert_eq!(filtered.entries.len(), 1);
        assert_eq!(filtered.entries[0].key, "/a");
    }

    #[test]
    fn clear_cache_removes_entries() {
        let mut state = RuntimeState::default();
        state.record_cache_hit("/a", "analysis", 10);
        assert_eq!(state.clear_cache(None), 1);
        assert!(state.cache_stats(None).entries.is_empty());
    }

    #[test]
    fn schedule_agents_creates_agents_capped_by_max() {
        let mut state = RuntimeState::default();
        let tasks = vec!["src/main.rs".to_string(), "src/routes.rs".to_string()];

        let pending = state.schedule_agents(tasks.clone(), 1, false);
        assert_eq!(pending.pending.len(), 1);
        assert_eq!(pending.pending[0].name, "main");
        assert!(pending.active.is_empty());

        let active = state.schedule_agents(tasks, 2, true);
        assert_eq!(active.active.len(), 2);
        assert!(active.pending.is_empty());
    }

    #[test]
    fn advance_agents_progresses_completes_and_promotes() {
        let mut state = RuntimeState::default();
        state.schedule_agents(
            vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            2,
            false,
        );

        // Nothing active yet: the first call only promotes the queue head.
        let promoted = state.advance_agents(None, 25);
        assert_eq!(promoted.advanced_count, 0);
        assert_eq!(promoted.summary.active.len(), 1);
        assert_eq!(promoted.summary.active[0].progress_percent, 0);

        // Four steps complete the first agent; its completion promotes the
        // second, so every call still advances exactly one agent.
        for _ in 0..4 {
            let result = state.advance_agents(None, 25);
            assert_eq!(result.advanced_count, 1);
        }
        let summary = state.agent_summary();
        assert_eq!(summary.completed.len(), 1);
        assert_eq!(summary.active.len(), 1);

        for _ in 0..4 {
            state.advance_agents(None, 25);
        }
        let summary = state.agent_summary();
        assert_eq!(summary.completed.len(), 2);
        assert!(summary.active.is_empty());
    }

    #[test]
    fn advance_specific_agent_by_id() {
        let mut state = RuntimeState::default();
        let summary = state.schedule_agents(
            vec!["src/main.rs".to_string(), "src/lib.rs".to_string()],
            2,
            true,
        );
        let target_id = summary.active[1].id.clone();

        let result = state.advance_agents(Some(&target_id), 50);
        assert_eq!(result.advanced_count, 1);
        assert_eq!(result.advanced[0].id, target_id);
        assert_eq!(result.advanced[0].progress_percent, 50);

        let other = result
            .summary
            .active
            .iter()
            .find(|agent| agent.id != target_id)
            .unwrap();
        assert_eq!(other.progress_percent, 0);
    }

    #[test]
    fn state_persists_and_reloads_from_dir() {
        let dir = tempfile::tempdir().unwrap();

        {
            let mut state = RuntimeState::default();
            state.record_cache_hit("/repo", "analysis", 42);
            state.record_run(true, 10);
            state.schedule_agents(vec!["src/main.rs".to_string()], 1, true);
            let advanced = state.advance_agents(None, 50);
            assert_eq!(advanced.advanced_count, 1);
            state.save_to_dir(dir.path());
        }

        let reloaded = RuntimeState::load_from_dir(dir.path());
        let cache = reloaded.cache_stats(Some("/repo"));
        assert_eq!(cache.entries.len(), 1);
        assert_eq!(cache.entries[0].hits, 1);
        let summary = reloaded.agent_summary();
        assert_eq!(summary.active.len(), 1);
        assert_eq!(summary.active[0].progress_percent, 50);
        assert_eq!(
            reloaded
                .execution_report(5, true)
                .runtime_statistics
                .total_runs,
            1
        );
    }

    #[test]
    fn load_missing_dir_falls_back_to_default() {
        let dir = tempfile::tempdir().unwrap();
        let state = RuntimeState::load_from_dir(&dir.path().join("does-not-exist"));
        assert!(state.agent_summary().active.is_empty());
    }

    #[test]
    fn execution_report_counts_runs() {
        let mut state = RuntimeState::default();
        state.record_run(true, 100);
        state.record_run(false, 50);
        let report = state.execution_report(80, true);
        assert_eq!(report.runtime_statistics.total_runs, 2);
        assert_eq!(report.runtime_statistics.successful_runs, 1);
        assert_eq!(report.runtime_statistics.failed_runs, 1);
        assert_eq!(report.runtime_statistics.average_execution_time_ms, 75);
        assert_eq!(report.timeline.len(), 5);

        let disabled = state.execution_report(80, false);
        assert_eq!(
            disabled
                .timeline
                .iter()
                .find(|e| e.id == "analytics")
                .unwrap()
                .status,
            "queued"
        );
    }
}
