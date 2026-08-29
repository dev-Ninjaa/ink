use std::collections::BTreeMap;
use std::time::Duration;

use context_optimizer::{optimize_context, ContextRequest};
use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dependency_graph::analyze_dependencies;
use repository_intelligence::{
    AnalysisSummary, EntryPoint, FileEntry, Language, Module, ModuleKind, PerformanceMetrics,
    ProjectMetadata, Relationship, RelationshipKind, RepositoryAnalysis,
};

fn synthetic_repository(node_count: usize) -> RepositoryAnalysis {
    let files = (0..node_count)
        .map(|index| FileEntry {
            path: format!("src/module{}/file_{index}.rs", index % 10),
            size: 128,
            language: Some(Language::Rust),
        })
        .collect::<Vec<_>>();
    let relationships = (0..node_count.saturating_sub(1))
        .map(|index| Relationship {
            source: files[index].path.clone(),
            target: files[index + 1].path.clone(),
            kind: RelationshipKind::Import,
            resolved: true,
        })
        .collect::<Vec<_>>();
    let modules = (0..10)
        .map(|module_index| Module {
            name: format!("module{module_index}"),
            kind: ModuleKind::Feature,
            root: format!("src/module{module_index}"),
            files: files
                .iter()
                .filter(|file| file.path.starts_with(&format!("src/module{module_index}/")))
                .map(|file| file.path.clone())
                .collect(),
        })
        .collect::<Vec<_>>();

    RepositoryAnalysis {
        root: "synthetic".to_string(),
        analyzer_version: "bench".to_string(),
        summary: AnalysisSummary {
            files: node_count as u64,
            directories: 0,
            project_roots: 1,
            bytes: (node_count * 128) as u64,
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
        entry_points: vec![EntryPoint {
            path: "src/module0/file_0.rs".to_string(),
            confidence: 1.0,
            heuristic: "bench".to_string(),
        }],
        modules,
        relationships,
        files,
        directories: Vec::new(),
    }
}

fn bench_optimization(c: &mut Criterion) {
    let cases = [
        ("small", synthetic_repository(200)),
        ("medium", synthetic_repository(1_000)),
        ("large", synthetic_repository(5_000)),
    ];

    let mut group = c.benchmark_group("context_optimizer_synthetic");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));

    for (name, analysis) in cases {
        let graph = analyze_dependencies(&analysis);
        let request = ContextRequest {
            query: "file_3".to_string(),
            max_tokens: Some(4_000),
            ..Default::default()
        };
        group.bench_with_input(
            BenchmarkId::new("optimize", name),
            &(analysis, graph, request),
            |b, input| {
                b.iter(|| {
                    let (analysis, graph, request) = input;
                    black_box(
                        optimize_context(
                            black_box(analysis),
                            black_box(Some(graph)),
                            black_box(request),
                        )
                        .unwrap(),
                    );
                });
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_optimization);
criterion_main!(benches);
