//! End-to-end pipeline demo: run all three Ink engine features against a real
//! repository and print a summary of each stage.
//!
//! Features exercised:
//!   1. `AnalyzeRepository` — Repository Intelligence scanning/analysis.
//!   2. `BuildDependencyGraph` — Dependency Graph statistics + centrality.
//!   3. `OptimizeContext` — the Smart Context Optimizer report.
//!
//! Usage: `cargo run -p context_optimizer --example pipeline -- <repo> [query] [--full]`
//!
//! `--full` additionally dumps the full JSON documents of each stage.

use context_optimizer::{optimize_context, output, render_report, ContextRequest};
use dependency_graph::analyze_dependencies;
use repository_intelligence::{analyze_repository, output::json};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let root = args.next().unwrap_or_else(|| ".".to_owned());
    let query = args.next().unwrap_or_default();
    let full = args.any(|arg| arg == "--full");

    // ---------------------------------------------------------------- Feature 1
    println!("===== 1. AnalyzeRepository ===============================");
    let analysis = analyze_repository(&root)?;
    let scan_ms = analysis.performance.scan_duration_ms + analysis.performance.analysis_duration_ms;
    println!(
        "{} files, {} dirs, {} bytes in {:.2} ms ({:.0} files/s)",
        analysis.summary.files,
        analysis.summary.directories,
        analysis.summary.bytes,
        scan_ms,
        analysis.performance.files_per_second
    );
    println!("entry points: {}", analysis.entry_points.len());
    for entry in &analysis.entry_points {
        println!(
            "  - `{}` (confidence {:.2}, {})",
            entry.path, entry.confidence, entry.heuristic
        );
    }
    println!("modules: {}", analysis.modules.len());
    for module in &analysis.modules {
        println!(
            "  - {} [{:?}] ({} files)",
            module.name,
            module.kind,
            module.files.len()
        );
    }
    println!("relationships: {}", analysis.relationships.len());
    if !analysis.frameworks.is_empty() {
        println!(
            "frameworks: {}",
            analysis
                .frameworks
                .iter()
                .map(|framework| framework.display_name().to_owned())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    println!("languages:");
    for (language, count) in &analysis.languages {
        println!("  - {}: {}", language.as_str(), count);
    }
    println!(
        "took {:.2} ms total (scan {:.2} + analysis {:.2})",
        scan_ms, analysis.performance.scan_duration_ms, analysis.performance.analysis_duration_ms
    );
    if full {
        println!("{}", json::to_json(&analysis)?);
    }

    // ---------------------------------------------------------------- Feature 2
    println!("\n===== 2. BuildDependencyGraph ============================");
    let start = std::time::Instant::now();
    let graph = analyze_dependencies(&analysis);
    let stats = &graph.statistics;
    println!(
        "{} file nodes, {} edges | {} module nodes, {} module edges",
        stats.node_count, stats.edge_count, stats.module_count, stats.module_edge_count
    );
    println!(
        "entry points in graph: {} | file cycles: {} | module cycles: {}",
        stats.entrypoint_count, stats.file_cycle_count, stats.module_cycle_count
    );
    println!(
        "largest connected component: {} | graph density: {:.4}",
        stats.largest_connected_component, stats.graph_density
    );
    println!(
        "max dependency depth: {} | avg depth: {:.2}",
        stats.maximum_depth, stats.average_depth
    );
    println!(
        "reachable from entry points: {} | unreachable: {}",
        graph.reachability.reachable_nodes.len(),
        graph.reachability.unreachable_nodes.len()
    );
    println!("central files (top 10 by degree):");
    for node in graph.central_files.iter().take(10) {
        println!(
            "  - `{}` (in {} / out {} / total {})",
            node.id, node.in_degree, node.out_degree, node.total_degree
        );
    }
    println!("took {:.2} ms", start.elapsed().as_secs_f64() * 1000.0);
    if full {
        println!("{}", serde_json::to_string_pretty(&graph)?);
    }

    // ---------------------------------------------------------------- Feature 3
    println!("\n===== 3. OptimizeContext =================================");
    let request = ContextRequest {
        query,
        max_tokens: Some(8_000),
        ..Default::default()
    };
    let start = std::time::Instant::now();
    let context = optimize_context(&analysis, Some(&graph), &request)?;
    let report = render_report(&context);
    println!("{report}");
    println!("took {:.2} ms", start.elapsed().as_secs_f64() * 1000.0);
    if full {
        println!("{}", output::json::to_json(&context)?);
    }

    Ok(())
}
