//! Entrypoint reachability analysis.

use std::collections::BTreeSet;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;

use crate::models::Reachability;

/// Determine reachable and unreachable files from entry points.
pub fn analyze_reachability(
    graph: &DiGraph<String, crate::models::EdgeKind>,
    entrypoints: &[String],
    all_nodes: &[String],
    index_by_path: &std::collections::BTreeMap<String, NodeIndex>,
) -> Reachability {
    let mut reachable = BTreeSet::new();
    for entrypoint in entrypoints {
        if let Some(index) = index_by_path.get(entrypoint).copied() {
            visit(graph, index, &mut reachable);
        }
    }

    let all = all_nodes.iter().cloned().collect::<BTreeSet<_>>();
    let unreachable = all
        .difference(&reachable)
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();

    Reachability {
        entrypoints: entrypoints.to_vec(),
        reachable_nodes: reachable.into_iter().collect(),
        unreachable_nodes: unreachable,
    }
}

fn visit(
    graph: &DiGraph<String, crate::models::EdgeKind>,
    index: NodeIndex,
    reachable: &mut BTreeSet<String>,
) {
    let mut stack = vec![index];
    while let Some(current) = stack.pop() {
        let id = graph[current].clone();
        if !reachable.insert(id) {
            continue;
        }

        let mut targets = graph
            .edges(current)
            .map(|edge| edge.target())
            .collect::<Vec<_>>();
        targets.sort_by_key(|target| std::cmp::Reverse(graph[*target].clone()));
        stack.extend(targets);
    }
}
