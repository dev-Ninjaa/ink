//! Degree, depth, component, and memory metrics.

use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::mem::size_of;

use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use petgraph::Direction;

use crate::models::{CentralNode, DependencyChain, EdgeMetrics};

/// Calculate degree metrics for every node.
pub fn degree_metrics(graph: &DiGraph<String, crate::models::EdgeKind>) -> Vec<EdgeMetrics> {
    let mut metrics = graph
        .node_indices()
        .map(|index| {
            let in_degree = graph.edges_directed(index, Direction::Incoming).count();
            let out_degree = graph.edges_directed(index, Direction::Outgoing).count();
            EdgeMetrics {
                id: graph[index].clone(),
                in_degree,
                out_degree,
                total_degree: in_degree + out_degree,
            }
        })
        .collect::<Vec<_>>();
    metrics.sort_by(|left, right| left.id.cmp(&right.id));
    metrics
}

/// Rank the most central nodes by total degree.
pub fn central_nodes(metrics: &[EdgeMetrics], limit: usize) -> Vec<CentralNode> {
    let mut nodes = metrics
        .iter()
        .map(|metric| CentralNode {
            id: metric.id.clone(),
            in_degree: metric.in_degree,
            out_degree: metric.out_degree,
            total_degree: metric.total_degree,
        })
        .collect::<Vec<_>>();
    nodes.sort_by(|left, right| {
        right
            .total_degree
            .cmp(&left.total_degree)
            .then_with(|| right.in_degree.cmp(&left.in_degree))
            .then_with(|| left.id.cmp(&right.id))
    });
    nodes.truncate(limit);
    nodes
}

/// Size of the largest weakly connected component.
pub fn largest_weak_component(graph: &DiGraph<String, crate::models::EdgeKind>) -> usize {
    let mut seen = BTreeSet::new();
    let mut largest = 0usize;

    for start in graph.node_indices() {
        if seen.contains(&start.index()) {
            continue;
        }

        let mut queue = VecDeque::from([start]);
        let mut size = 0usize;
        seen.insert(start.index());

        while let Some(index) = queue.pop_front() {
            size += 1;
            for neighbor in graph.neighbors_undirected(index) {
                if seen.insert(neighbor.index()) {
                    queue.push_back(neighbor);
                }
            }
        }
        largest = largest.max(size);
    }

    largest
}

/// Directed graph density.
pub fn density(node_count: usize, edge_count: usize) -> f64 {
    if node_count <= 1 {
        0.0
    } else {
        edge_count as f64 / (node_count * (node_count - 1)) as f64
    }
}

/// Calculate shortest average depth and representative longest dependency chains.
pub fn depth_analysis(
    graph: &DiGraph<String, crate::models::EdgeKind>,
    entrypoints: &[String],
    index_by_path: &BTreeMap<String, NodeIndex>,
) -> (usize, f64, Vec<DependencyChain>) {
    let shortest_depths = shortest_depths(graph, entrypoints, index_by_path);
    let average_depth = if shortest_depths.is_empty() {
        0.0
    } else {
        shortest_depths.values().sum::<usize>() as f64 / shortest_depths.len() as f64
    };

    let mut chains = Vec::new();
    for entrypoint in entrypoints {
        if let Some(index) = index_by_path.get(entrypoint).copied() {
            let mut path = Vec::new();
            let mut seen = BTreeSet::new();
            let chain = longest_chain(graph, index, &mut seen, &mut path);
            if !chain.nodes.is_empty() {
                chains.push(chain);
            }
        }
    }

    chains.sort_by(|left, right| {
        right
            .depth
            .cmp(&left.depth)
            .then_with(|| left.nodes.cmp(&right.nodes))
    });
    chains.truncate(10);
    let maximum_depth = chains.first().map_or(0, |chain| chain.depth);

    (maximum_depth, average_depth, chains)
}

/// Estimate heap memory consumed by graph-facing collections.
pub fn memory_estimate(
    node_count: usize,
    edge_count: usize,
    module_count: usize,
    module_edge_count: usize,
    string_bytes: usize,
) -> usize {
    node_count * (size_of::<String>() + size_of::<NodeIndex>())
        + edge_count * (size_of::<crate::models::EdgeKind>() + size_of::<(usize, usize)>())
        + module_count * (size_of::<String>() + size_of::<NodeIndex>())
        + module_edge_count * (size_of::<crate::models::EdgeKind>() + size_of::<(usize, usize)>())
        + string_bytes
}

fn shortest_depths(
    graph: &DiGraph<String, crate::models::EdgeKind>,
    entrypoints: &[String],
    index_by_path: &BTreeMap<String, NodeIndex>,
) -> BTreeMap<String, usize> {
    let mut depths = BTreeMap::new();
    let mut queue = VecDeque::new();

    for entrypoint in entrypoints {
        if let Some(index) = index_by_path.get(entrypoint).copied() {
            depths.entry(graph[index].clone()).or_insert(1);
            queue.push_back((index, 1usize));
        }
    }

    while let Some((index, depth)) = queue.pop_front() {
        let mut targets = graph
            .edges(index)
            .map(|edge| edge.target())
            .collect::<Vec<_>>();
        targets.sort_by_key(|target| graph[*target].clone());

        for target in targets {
            let id = graph[target].clone();
            if depths.contains_key(&id) {
                continue;
            }
            depths.insert(id, depth + 1);
            queue.push_back((target, depth + 1));
        }
    }

    depths
}

fn longest_chain(
    graph: &DiGraph<String, crate::models::EdgeKind>,
    start: NodeIndex,
    _seen: &mut BTreeSet<usize>,
    _path: &mut Vec<String>,
) -> DependencyChain {
    let mut stack = vec![Frame {
        node: start,
        targets: sorted_targets(graph, start),
        next_target: 0,
    }];
    let mut seen = BTreeSet::from([start.index()]);
    let mut path = vec![graph[start].clone()];
    let mut best_nodes = path.clone();

    while let Some(frame) = stack.last_mut() {
        if frame.next_target < frame.targets.len() {
            let target = frame.targets[frame.next_target];
            frame.next_target += 1;
            if seen.contains(&target.index()) {
                continue;
            }

            seen.insert(target.index());
            path.push(graph[target].clone());
            stack.push(Frame {
                node: target,
                targets: sorted_targets(graph, target),
                next_target: 0,
            });
            continue;
        }

        let is_better =
            path.len() > best_nodes.len() || (path.len() == best_nodes.len() && path < best_nodes);
        if is_better {
            best_nodes = path.clone();
        }

        let finished = stack.pop().expect("stack is non-empty");
        seen.remove(&finished.node.index());
        path.pop();
    }

    DependencyChain {
        depth: best_nodes.len(),
        nodes: best_nodes,
    }
}

#[derive(Debug, Clone)]
struct Frame {
    node: NodeIndex,
    targets: Vec<NodeIndex>,
    next_target: usize,
}

fn sorted_targets(
    graph: &DiGraph<String, crate::models::EdgeKind>,
    index: NodeIndex,
) -> Vec<NodeIndex> {
    let mut targets = graph
        .edges(index)
        .map(|edge| edge.target())
        .collect::<Vec<_>>();
    targets.sort_by_key(|target| graph[*target].clone());
    targets
}
