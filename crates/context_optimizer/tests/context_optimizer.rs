//! End-to-end integration tests for the context optimizer against temporary
//! repositories analyzed with the full Repository Intelligence + Dependency
//! Graph pipeline.

use std::path::Path;

use context_optimizer::models::DroppedReason;
use context_optimizer::{optimize_context, ContextOptimizer, ContextRequest, OptimizerConfig};
use dependency_graph::analyze_dependencies;
use repository_intelligence::analyze_repository;

fn write(root: &Path, rel: &str, content: &str) {
    let path = root.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(path, content).unwrap();
}

struct Fixture {
    _guard: tempfile::TempDir,
    analysis: repository_intelligence::RepositoryAnalysis,
    graph: dependency_graph::models::AnalysisResult,
}

fn fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // Entry point + auth module.
    write(
        root,
        "src/main.rs",
        "mod auth;\nmod db;\nmod ui;\nfn main() { auth::login(); }",
    );
    write(
        root,
        "src/auth.rs",
        "pub fn login() {}\npub fn register() {}",
    );
    write(root, "src/db.rs", "pub fn connect() {}");
    write(root, "src/ui.rs", "pub fn render() {}");

    // A doc file and a test file.
    write(root, "README.md", "# Demo\n\nAuth flow documentation.\n");
    write(root, "tests/auth_test.rs", "fn test_login() {}");

    // A duplicate pair (identical content).
    write(
        root,
        "src/auth_mirror.rs",
        "pub fn login() {}\npub fn register() {}",
    );

    let analysis = analyze_repository(root).unwrap();
    let graph = analyze_dependencies(&analysis);
    Fixture {
        _guard: dir,
        analysis,
        graph,
    }
}

#[test]
fn selects_relevant_files_first() {
    let fixture = fixture();
    let request = ContextRequest {
        query: "auth".to_owned(),
        ..Default::default()
    };
    let context = optimize_context(&fixture.analysis, Some(&fixture.graph), &request).unwrap();

    assert_eq!(context.selected[0].path, "src/auth.rs");
    assert!(context
        .selected
        .iter()
        .any(|file| file.path == "src/main.rs"));
    assert!(
        context
            .selected
            .iter()
            .position(|file| file.path == "src/auth.rs")
            < context
                .selected
                .iter()
                .position(|file| file.path == "src/ui.rs")
    );
    assert!(context.selected[0]
        .reasons
        .iter()
        .any(|r| r.contains("auth")));
}

#[test]
fn collapses_near_duplicate_files() {
    let fixture = fixture();
    let request = ContextRequest {
        query: String::new(),
        ..Default::default()
    };
    let context = optimize_context(&fixture.analysis, Some(&fixture.graph), &request).unwrap();

    let kept_auth = context
        .selected
        .iter()
        .any(|file| file.path == "src/auth.rs");
    let kept_mirror = context
        .selected
        .iter()
        .any(|file| file.path == "src/auth_mirror.rs");
    assert!(
        kept_auth || kept_mirror,
        "one copy of the duplicate must be kept"
    );
    assert!(
        !(kept_auth && kept_mirror),
        "duplicates must not both be kept"
    );

    let dropped_duplicate = context
        .dropped
        .iter()
        .find(|entry| entry.reason == DroppedReason::Duplicate);
    assert!(dropped_duplicate.is_some());
    assert_eq!(context.metrics.files_dropped_duplicates, 1);
    assert_eq!(context.dedup.files_collapsed, 1);
    assert_eq!(context.dedup.groups.len(), 1);
}

#[test]
fn enforces_token_budget() {
    let fixture = fixture();
    let request = ContextRequest {
        query: String::new(),
        max_tokens: Some(5),
        ..Default::default()
    };
    let context = optimize_context(&fixture.analysis, Some(&fixture.graph), &request).unwrap();

    assert!(context.tokens.within_budget);
    assert!(context.tokens.tokens_after <= 5);
    assert!(!context.selected.is_empty());
    assert!(context.metrics.files_dropped_budget > 0);
    assert!(!context.dropped.is_empty());
}

#[test]
fn respects_include_and_exclude_filters() {
    let fixture = fixture();
    let request = ContextRequest {
        query: String::new(),
        include_paths: vec!["src".to_owned()],
        exclude_paths: vec!["src/ui.rs".to_owned()],
        ..Default::default()
    };
    let context = optimize_context(&fixture.analysis, Some(&fixture.graph), &request).unwrap();

    for file in &context.selected {
        assert!(file.path.starts_with("src/"));
        assert_ne!(file.path, "src/ui.rs");
    }
    assert_eq!(context.metrics.files_excluded, 3); // README + tests + ui
}

#[test]
fn empty_repository_yields_empty_context() {
    let dir = tempfile::tempdir().unwrap();
    let analysis = analyze_repository(dir.path()).unwrap();
    let graph = analyze_dependencies(&analysis);
    let request = ContextRequest::default();
    let context = optimize_context(&analysis, Some(&graph), &request).unwrap();

    assert!(context.selected.is_empty());
    assert_eq!(context.metrics.files_considered, 0);
}

#[test]
fn output_is_deterministic() {
    let fixture = fixture();
    let request = ContextRequest {
        query: "auth login".to_owned(),
        max_tokens: Some(20),
        ..Default::default()
    };

    let mut first = optimize_context(&fixture.analysis, Some(&fixture.graph), &request).unwrap();
    let second = optimize_context(&fixture.analysis, Some(&fixture.graph), &request).unwrap();

    // Timing metrics legitimately differ between runs; pin them so the
    // structural output can be compared byte-for-byte.
    first.metrics.duration_ms = second.metrics.duration_ms;

    let first_json = context_optimizer::output::json::to_json(&first).unwrap();
    let second_json = context_optimizer::output::json::to_json(&second).unwrap();
    assert_eq!(first_json, second_json);
}

#[test]
fn token_estimates_are_reported_per_file() {
    let fixture = fixture();
    let request = ContextRequest {
        query: String::new(),
        ..Default::default()
    };
    let context = optimize_context(&fixture.analysis, Some(&fixture.graph), &request).unwrap();

    assert!(context.selected.iter().all(|file| file.tokens >= 1));
    assert_eq!(
        context.tokens.tokens_after,
        context
            .selected
            .iter()
            .map(|file| file.tokens)
            .sum::<usize>()
    );
}

#[test]
fn works_without_dependency_graph() {
    let fixture = fixture();
    let request = ContextRequest {
        query: "db".to_owned(),
        ..Default::default()
    };
    let context = optimize_context(&fixture.analysis, None, &request).unwrap();
    assert!(context.selected[0].path == "src/db.rs");
    assert!(context.metrics.files_selected >= 1);
}

fn gate_fixture() -> Fixture {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    write(root, "src/main.rs", "fn main() {}");
    write(root, "src/app.rs", "pub fn run() {}");
    write(root, "README.md", "# Demo\n");
    write(root, ".gitignore", "node_modules/\n");
    write(
        root,
        "package-lock.json",
        "{\"lockfileVersion\":3,\"packages\":{}}",
    );
    write(
        root,
        "Cargo.lock",
        "# This file is automatically @generated by Cargo.\nversion = 4\n",
    );

    // A genuinely binary file (non-UTF-8 bytes).
    let png = root.join("assets/logo.png");
    std::fs::create_dir_all(png.parent().unwrap()).unwrap();
    std::fs::write(
        png,
        [
            0x89u8, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a, 0x00, 0x01, 0x02, 0x03,
        ],
    )
    .unwrap();

    let analysis = analyze_repository(root).unwrap();
    let graph = analyze_dependencies(&analysis);
    Fixture {
        _guard: dir,
        analysis,
        graph,
    }
}

#[test]
fn excludes_binary_and_generated_files() {
    let fixture = gate_fixture();
    let request = ContextRequest::default();
    let context = optimize_context(&fixture.analysis, Some(&fixture.graph), &request).unwrap();

    let selected: Vec<&str> = context
        .selected
        .iter()
        .map(|file| file.path.as_str())
        .collect();
    assert!(!selected.contains(&"assets/logo.png"));
    assert!(!selected.contains(&"package-lock.json"));
    assert!(!selected.contains(&"Cargo.lock"));
    assert!(selected.contains(&"src/app.rs"));

    assert_eq!(context.metrics.files_dropped_non_text, 1);
    assert_eq!(context.metrics.files_dropped_generated, 2);
    assert!(context
        .dropped
        .iter()
        .any(|entry| entry.reason == DroppedReason::NonText));
    assert!(context
        .dropped
        .iter()
        .any(|entry| entry.reason == DroppedReason::Generated));
}

#[test]
fn min_relevance_filters_low_relevance_files() {
    let fixture = gate_fixture();
    let request = ContextRequest {
        query: "app".to_owned(),
        min_relevance: Some(0.5),
        ..Default::default()
    };
    let context = optimize_context(&fixture.analysis, Some(&fixture.graph), &request).unwrap();

    assert!(context.selected.iter().all(|file| file.relevance >= 0.5));
    assert!(context.metrics.files_dropped_low_relevance >= 2); // README + .gitignore score 0
    assert!(context
        .dropped
        .iter()
        .any(|entry| entry.reason == DroppedReason::LowRelevance));
}

#[test]
fn content_gate_can_be_disabled() {
    let fixture = gate_fixture();
    let optimizer = ContextOptimizer::new(OptimizerConfig {
        content_gate_enabled: false,
        ..Default::default()
    });
    let request = ContextRequest::default();
    let context = optimizer
        .optimize(&fixture.analysis, Some(&fixture.graph), &request)
        .unwrap();

    // Without the gate the binary file is still a candidate (its content is
    // unreadable, so it is kept with a size-based token estimate + warning).
    assert!(context
        .selected
        .iter()
        .any(|file| file.path == "assets/logo.png"));
    assert!(context
        .selected
        .iter()
        .any(|file| file.path == "package-lock.json"));
    assert_eq!(context.metrics.files_dropped_non_text, 0);
    assert_eq!(context.metrics.files_dropped_generated, 0);
}
