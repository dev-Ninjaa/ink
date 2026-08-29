use std::collections::BTreeMap;
use std::path::Path;
use std::time::Duration;

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use dependency_graph::analyze_dependencies;
use repository_intelligence::{
    analyze_repository, AnalysisSummary, FileEntry, Language, Module, ModuleKind,
    PerformanceMetrics, ProjectMetadata, Relationship, RelationshipKind, RepositoryAnalysis,
};

fn synthetic_repository(node_count: usize, module_count: usize) -> RepositoryAnalysis {
    let files = (0..node_count)
        .map(|index| FileEntry {
            path: format!("src/module{}/file_{index}.rs", index % module_count),
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
    let modules = (0..module_count)
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
            directories: module_count as u64,
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
        entry_points: vec![repository_intelligence::EntryPoint {
            path: files
                .first()
                .map(|file| file.path.clone())
                .unwrap_or_default(),
            confidence: 1.0,
            heuristic: "bench".to_string(),
        }],
        modules,
        relationships,
        files,
        directories: Vec::new(),
    }
}

fn bench_synthetic(c: &mut Criterion) {
    let cases = [
        ("small", synthetic_repository(100, 5)),
        ("medium", synthetic_repository(1_000, 20)),
        ("large", synthetic_repository(10_000, 60)),
    ];

    let mut group = c.benchmark_group("dependency_graph_synthetic");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    for (name, analysis) in cases {
        group.bench_with_input(
            BenchmarkId::new("full_analysis", name),
            &analysis,
            |b, input| {
                b.iter(|| analyze_dependencies(black_box(input)));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("cycle_detection", name),
            &analysis,
            |b, input| {
                b.iter(|| black_box(analyze_dependencies(black_box(input)).file_cycles));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("reachability", name),
            &analysis,
            |b, input| {
                b.iter(|| black_box(analyze_dependencies(black_box(input)).reachability));
            },
        );
        group.bench_with_input(
            BenchmarkId::new("statistics", name),
            &analysis,
            |b, input| {
                b.iter(|| black_box(analyze_dependencies(black_box(input)).statistics));
            },
        );
    }
    group.finish();
}

fn bench_real_repositories(c: &mut Criterion) {
    let candidates = [
        ("ink_extension", r"C:\Users\hp\Documents\ink.extension"),
        ("maple", r"C:\Users\hp\Documents\maple\maple"),
    ];
    let analyses = candidates
        .into_iter()
        .filter_map(|(name, path)| {
            Path::new(path)
                .exists()
                .then(|| {
                    analyze_repository(path)
                        .ok()
                        .map(|analysis| (name, analysis))
                })
                .flatten()
        })
        .collect::<Vec<_>>();

    if analyses.is_empty() {
        return;
    }

    let mut group = c.benchmark_group("dependency_graph_real_repositories");
    group
        .sample_size(10)
        .warm_up_time(Duration::from_millis(500))
        .measurement_time(Duration::from_secs(2));
    for (name, analysis) in analyses {
        group.bench_with_input(
            BenchmarkId::new("full_analysis", name),
            &analysis,
            |b, input| {
                b.iter(|| analyze_dependencies(black_box(input)));
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_synthetic, bench_real_repositories);
criterion_main!(benches);
