//! Criterion benchmarks for the repository intelligence engine.
//!
//! Cases:
//! * `small`  — 100 files
//! * `medium` — 1,000 files
//! * `large`  — 10,000 files
//!
//! Each case measures scan-only and full-analysis throughput. With the
//! `heap-profiling` feature enabled, a one-shot heap allocation report is
//! printed before running the timed benchmarks (`cargo bench --features
//! heap-profiling`).

use criterion::{Criterion, Throughput};
use repository_intelligence::analyzer::Analyzer;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

// ---------------------------------------------------------------------------
// Heap profiling support (optional)
// ---------------------------------------------------------------------------

#[cfg(feature = "heap-profiling")]
use dhat::{Alloc, HeapStats, Profiler};

#[cfg(feature = "heap-profiling")]
#[global_allocator]
static ALLOCATOR: Alloc = Alloc;

// ---------------------------------------------------------------------------
// Synthetic repository generation
// ---------------------------------------------------------------------------

struct RepoCase {
    name: &'static str,
    files: usize,
}

const CASES: &[RepoCase] = &[
    RepoCase {
        name: "small",
        files: 100,
    },
    RepoCase {
        name: "medium",
        files: 1_000,
    },
    RepoCase {
        name: "large",
        files: 10_000,
    },
];

const EXTENSIONS: &[&str] = &[
    ".rs", ".ts", ".tsx", ".py", ".go", ".java", ".cs", ".c", ".cpp", ".json", ".yaml", ".toml",
    ".md",
];

fn ensure_repo(dir: &Path, files: usize) {
    if dir.join("MARKER").exists() {
        return;
    }
    let _ = fs::remove_dir_all(dir);
    fs::create_dir_all(dir).unwrap();

    for index in 0..files {
        let ext = EXTENSIONS[index % EXTENSIONS.len()];
        let subdir = if index % 3 == 0 {
            format!("src/module{}", index % 20)
        } else {
            "src".to_string()
        };
        let path = dir.join(subdir).join(format!("file_{index}{ext}"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        let body = match ext {
            ".rs" => "use crate::helper;\npub fn f() {}\nmod inner;\n",
            ".ts" | ".tsx" => "import { helper } from './helper';\nexport const value = 1;\n",
            ".py" => "from .helper import thing\n\ndef f():\n    return 1\n",
            _ => "{}\n",
        };
        fs::write(path, body).unwrap();
    }

    // Framework manifests so detection has something to chew on.
    fs::write(
        dir.join("package.json"),
        r#"{"name":"bench","dependencies":{"next":"14.0.0","react":"18.2.0"}}"#,
    )
    .unwrap();
    fs::write(dir.join("Cargo.toml"), "[dependencies]\naxum = \"0.7\"\n").unwrap();
    fs::write(dir.join("MARKER"), "").unwrap();
}

// ---------------------------------------------------------------------------
// Benchmarks
// ---------------------------------------------------------------------------

fn benches(c: &mut Criterion) {
    let workdir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let repos = workdir.join("target/bench_repos");

    for case in CASES {
        let repo_dir = repos.join(case.name);
        ensure_repo(&repo_dir, case.files);

        let mut group = c.benchmark_group(format!("repository/{}", case.name));
        group.throughput(Throughput::Elements(case.files as u64));
        group.sample_size(10);
        group
            .measurement_time(Duration::from_secs(5))
            .warm_up_time(Duration::from_secs(1));
        group.bench_function("scan", |b| {
            let analyzer = Analyzer::with_defaults();
            b.iter(|| {
                let _ = analyzer.scan(&repo_dir).unwrap();
            });
        });
        group.bench_function("analyze", |b| {
            let analyzer = Analyzer::with_defaults();
            b.iter(|| {
                let _ = analyzer.analyze(&repo_dir).unwrap();
            });
        });
        group.finish();
    }
}

#[cfg(feature = "heap-profiling")]
fn heap_report() {
    let _profiler = Profiler::new_heap();
    let workdir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".into()));
    let repos = workdir.join("target/bench_repos");
    let analyzer = Analyzer::with_defaults();

    println!("\n### Heap allocation report\n");
    for case in CASES {
        let repo_dir = repos.join(case.name);
        ensure_repo(&repo_dir, case.files);
        let before = HeapStats::get();
        let analysis = analyzer.analyze(&repo_dir).unwrap();
        let after = HeapStats::get();
        let bytes = after.total_bytes.saturating_sub(before.total_bytes);
        let blocks = after.total_blocks.saturating_sub(before.total_blocks);
        println!(
            "{:<8} {} files  allocated={} bytes in {} blocks",
            case.name, case.files, bytes, blocks
        );
        let _ = analysis.summary.files;
    }
    println!("Output profiler file: `dhat-heap.json` / `dhat-heap.out`.\n");
}

fn main() {
    #[cfg(feature = "heap-profiling")]
    heap_report();

    let mut criterion = Criterion::default().configure_from_args();
    benches(&mut criterion);
    criterion.final_summary();
}
