//! Cycle detection through strongly connected components.

use petgraph::algo::kosaraju_scc;
use petgraph::graph::DiGraph;
use petgraph::visit::EdgeRef;

use crate::models::Cycle;

/// Detect file-level cycles.
pub fn detect_file_cycles(graph: &DiGraph<String, crate::models::EdgeKind>) -> Vec<Cycle> {
    detect_cycles(graph, true)
}

/// Detect module-level cycles.
pub fn detect_module_cycles(graph: &DiGraph<String, crate::models::EdgeKind>) -> Vec<Cycle> {
    detect_cycles(graph, false)
}

fn detect_cycles(graph: &DiGraph<String, crate::models::EdgeKind>, file_cycle: bool) -> Vec<Cycle> {
    let mut cycles = kosaraju_scc(graph)
        .into_iter()
        .filter_map(|component| {
            let is_self_cycle = component.len() == 1
                && graph
                    .edges(component[0])
                    .any(|edge| edge.target() == component[0]);
            if component.len() <= 1 && !is_self_cycle {
                return None;
            }

            let mut nodes = component
                .into_iter()
                .map(|index| graph[index].clone())
                .collect::<Vec<_>>();
            nodes.sort();

            let id = format!("cycle:{}", nodes.join("|"));
            Some(if file_cycle {
                Cycle {
                    id,
                    size: nodes.len(),
                    files: nodes,
                    modules: Vec::new(),
                }
            } else {
                Cycle {
                    id,
                    size: nodes.len(),
                    files: Vec::new(),
                    modules: nodes,
                }
            })
        })
        .collect::<Vec<_>>();

    cycles.sort_by(|left, right| left.id.cmp(&right.id));
    cycles
}
