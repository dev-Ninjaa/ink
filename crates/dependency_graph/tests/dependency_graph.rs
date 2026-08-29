use std::collections::BTreeMap;

use dependency_graph::analyze_dependencies;
use repository_intelligence::{
    AnalysisSummary, EntryPoint, FileEntry, Language, Module, ModuleKind, PerformanceMetrics,
    ProjectMetadata, Relationship, RelationshipKind, RepositoryAnalysis,
};

fn analysis(
    files: &[&str],
    modules: Vec<Module>,
    relationships: Vec<Relationship>,
    entrypoints: &[&str],
) -> RepositoryAnalysis {
    RepositoryAnalysis {
        root: "fixture".to_string(),
        analyzer_version: "test".to_string(),
        summary: AnalysisSummary {
            files: files.len() as u64,
            directories: 0,
            project_roots: 1,
            bytes: 0,
        },
        performance: PerformanceMetrics {
            scan_duration_ms: 0.0,
            analysis_duration_ms: 0.0,
            total_duration_ms: 0.0,
            files_per_second: 0.0,
        },
        languages: BTreeMap::new(),
        frameworks: Vec::new(),
        metadata: ProjectMetadata::default(),
        project_roots: vec![".".to_string()],
        entry_points: entrypoints
            .iter()
            .map(|path| EntryPoint {
                path: (*path).to_string(),
                confidence: 1.0,
                heuristic: "test".to_string(),
            })
            .collect(),
        modules,
        relationships,
        files: files
            .iter()
            .map(|path| FileEntry {
                path: (*path).to_string(),
                size: 1,
                language: Some(Language::Rust),
            })
            .collect(),
        directories: Vec::new(),
    }
}

fn rel(source: &str, target: &str) -> Relationship {
    Relationship {
        source: source.to_string(),
        target: target.to_string(),
        kind: RelationshipKind::Import,
        resolved: true,
    }
}

fn module(name: &str, root: &str, files: &[&str]) -> Module {
    Module {
        name: name.to_string(),
        kind: ModuleKind::Feature,
        root: root.to_string(),
        files: files.iter().map(|file| (*file).to_string()).collect(),
    }
}

#[test]
fn file_graph_creation_uses_resolved_relationships() {
    let input = analysis(
        &["src/main.rs", "src/auth.rs", "src/db.rs"],
        Vec::new(),
        vec![
            rel("src/main.rs", "src/auth.rs"),
            Relationship {
                source: "src/auth.rs".to_string(),
                target: "sqlx".to_string(),
                kind: RelationshipKind::Import,
                resolved: false,
            },
        ],
        &["src/main.rs"],
    );

    let result = analyze_dependencies(&input);

    assert_eq!(result.nodes.len(), 3);
    assert_eq!(result.edges.len(), 1);
    assert_eq!(result.edges[0].source, "src/main.rs");
    assert_eq!(result.edges[0].target, "src/auth.rs");
    assert_eq!(result.warnings.len(), 1);
}

#[test]
fn module_graph_creation_aggregates_many_to_many_file_edges() {
    let input = analysis(
        &[
            "auth/login.rs",
            "auth/session.rs",
            "db/pool.rs",
            "db/user_repo.rs",
        ],
        vec![
            module("auth", "auth", &["auth/login.rs", "auth/session.rs"]),
            module("db", "db", &["db/pool.rs", "db/user_repo.rs"]),
        ],
        vec![
            rel("auth/login.rs", "db/pool.rs"),
            rel("auth/session.rs", "db/user_repo.rs"),
        ],
        &["auth/login.rs"],
    );

    let result = analyze_dependencies(&input);

    assert_eq!(result.modules.len(), 2);
    assert_eq!(result.module_graph.edges.len(), 1);
    assert_eq!(result.module_graph.edges[0].weight, 2);
    assert_eq!(result.module_graph.edges[0].source, "auth:auth");
    assert_eq!(result.module_graph.edges[0].target, "db:db");
}

#[test]
fn cycle_detection_reports_file_and_module_cycles() {
    let input = analysis(
        &["a/mod.rs", "b/mod.rs", "c/mod.rs"],
        vec![
            module("a", "a", &["a/mod.rs"]),
            module("b", "b", &["b/mod.rs"]),
            module("c", "c", &["c/mod.rs"]),
        ],
        vec![
            rel("a/mod.rs", "b/mod.rs"),
            rel("b/mod.rs", "c/mod.rs"),
            rel("c/mod.rs", "a/mod.rs"),
        ],
        &["a/mod.rs"],
    );

    let result = analyze_dependencies(&input);

    assert_eq!(result.file_cycles.len(), 1);
    assert_eq!(result.file_cycles[0].size, 3);
    assert_eq!(result.module_cycles.len(), 1);
    assert_eq!(result.statistics.file_cycle_count, 1);
    assert_eq!(result.statistics.module_cycle_count, 1);
}

#[test]
fn reachability_splits_reachable_and_unreachable_nodes() {
    let input = analysis(
        &["main.rs", "router.rs", "auth.rs", "unused.rs"],
        Vec::new(),
        vec![rel("main.rs", "router.rs"), rel("router.rs", "auth.rs")],
        &["main.rs"],
    );

    let result = analyze_dependencies(&input);

    assert_eq!(
        result.reachability.reachable_nodes,
        vec!["auth.rs", "main.rs", "router.rs"]
    );
    assert_eq!(result.reachability.unreachable_nodes, vec!["unused.rs"]);
}

#[test]
fn depth_analysis_finds_longest_dependency_chain() {
    let input = analysis(
        &["main.rs", "auth.rs", "database.rs", "storage.rs"],
        Vec::new(),
        vec![
            rel("main.rs", "auth.rs"),
            rel("auth.rs", "database.rs"),
            rel("database.rs", "storage.rs"),
        ],
        &["main.rs"],
    );

    let result = analyze_dependencies(&input);

    assert_eq!(result.statistics.maximum_depth, 4);
    assert_eq!(result.dependency_chains[0].nodes[0], "main.rs");
    assert_eq!(result.dependency_chains[0].nodes[3], "storage.rs");
}

#[test]
fn statistics_include_components_density_and_degrees() {
    let input = analysis(
        &["a.rs", "b.rs", "c.rs", "d.rs"],
        Vec::new(),
        vec![rel("a.rs", "b.rs"), rel("b.rs", "c.rs")],
        &["a.rs"],
    );

    let result = analyze_dependencies(&input);

    assert_eq!(result.statistics.node_count, 4);
    assert_eq!(result.statistics.edge_count, 2);
    assert_eq!(result.statistics.largest_connected_component, 3);
    assert!(result.statistics.graph_density > 0.0);
    let b = result
        .file_metrics
        .iter()
        .find(|metric| metric.id == "b.rs")
        .unwrap();
    assert_eq!(b.total_degree, 2);
}

#[test]
fn monorepo_layouts_keep_package_module_boundaries() {
    let input = analysis(
        &[
            "packages/web/src/page.tsx",
            "packages/ui/src/button.tsx",
            "packages/api/src/client.ts",
        ],
        vec![
            Module {
                name: "web".to_string(),
                kind: ModuleKind::Package,
                root: "packages/web".to_string(),
                files: vec!["packages/web/src/page.tsx".to_string()],
            },
            Module {
                name: "ui".to_string(),
                kind: ModuleKind::Package,
                root: "packages/ui".to_string(),
                files: vec!["packages/ui/src/button.tsx".to_string()],
            },
            Module {
                name: "api".to_string(),
                kind: ModuleKind::Package,
                root: "packages/api".to_string(),
                files: vec!["packages/api/src/client.ts".to_string()],
            },
        ],
        vec![
            rel("packages/web/src/page.tsx", "packages/ui/src/button.tsx"),
            rel("packages/web/src/page.tsx", "packages/api/src/client.ts"),
        ],
        &["packages/web/src/page.tsx"],
    );

    let result = analyze_dependencies(&input);

    assert_eq!(result.module_graph.edges.len(), 2);
    assert!(result
        .module_graph
        .edges
        .iter()
        .any(|edge| edge.target == "packages/ui:ui"));
    assert!(result
        .module_graph
        .edges
        .iter()
        .any(|edge| edge.target == "packages/api:api"));
}

#[test]
fn self_cycles_and_disconnected_components_are_detected() {
    let input = analysis(
        &["a.rs", "b.rs", "c.rs"],
        Vec::new(),
        vec![rel("a.rs", "a.rs"), rel("b.rs", "c.rs")],
        &["b.rs"],
    );

    let result = analyze_dependencies(&input);

    assert_eq!(result.file_cycles.len(), 1);
    assert_eq!(result.file_cycles[0].files, vec!["a.rs"]);
    assert_eq!(result.statistics.largest_connected_component, 2);
}

#[test]
fn large_graphs_remain_deterministic() {
    let files = (0..250)
        .map(|index| format!("src/file_{index}.rs"))
        .collect::<Vec<_>>();
    let file_refs = files.iter().map(String::as_str).collect::<Vec<_>>();
    let relationships = (0..249)
        .map(|index| rel(&files[index], &files[index + 1]))
        .collect::<Vec<_>>();
    let input = analysis(&file_refs, Vec::new(), relationships, &["src/file_0.rs"]);

    let first = analyze_dependencies(&input);
    let second = analyze_dependencies(&input);

    assert_eq!(first.edges, second.edges);
    assert_eq!(first.statistics.node_count, 250);
    assert_eq!(first.statistics.maximum_depth, 250);
}
