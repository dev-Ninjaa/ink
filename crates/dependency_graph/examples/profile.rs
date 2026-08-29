use std::env;

use dependency_graph::analyze_dependencies;
use repository_intelligence::analyze_repository;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: cargo run -p dependency_graph --example profile -- <repo-path> [--full]");
        std::process::exit(2);
    };
    let full = args.any(|arg| arg == "--full");

    let repository = analyze_repository(&path)?;
    let graph = analyze_dependencies(&repository);
    if full {
        println!("{}", serde_json::to_string_pretty(&graph)?);
    } else {
        println!("{}", serde_json::to_string_pretty(&graph.statistics)?);
    }
    Ok(())
}
