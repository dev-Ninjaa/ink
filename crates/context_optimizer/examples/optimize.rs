//! Profile a context optimization run against a real repository.
//!
//! Usage: `cargo run -p context_optimizer --example optimize -- <repo> [query]`

use context_optimizer::{optimize_context, output, ContextRequest};
use dependency_graph::analyze_dependencies;
use repository_intelligence::analyze_repository;

fn main() {
    let root = std::env::args().nth(1).unwrap_or_else(|| ".".to_owned());
    let query = std::env::args().nth(2).unwrap_or_default();

    let analysis = analyze_repository(&root).expect("repository analysis failed");
    let graph = analyze_dependencies(&analysis);
    let request = ContextRequest {
        query,
        max_tokens: Some(8_000),
        ..Default::default()
    };
    let context =
        optimize_context(&analysis, Some(&graph), &request).expect("context optimization failed");

    println!("{}", context_optimizer::render_report(&context));
    println!(
        "{}",
        output::json::to_json(&context).expect("json serialization failed")
    );
}
