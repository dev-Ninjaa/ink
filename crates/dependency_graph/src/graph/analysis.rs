//! Analysis orchestration helpers.

use crate::models::GraphStats;

/// Inputs used to build aggregate graph statistics.
#[derive(Debug, Clone, Copy)]
pub struct GraphStatsInput {
    /// Number of file nodes.
    pub node_count: usize,
    /// Number of file edges.
    pub edge_count: usize,
    /// Number of module nodes.
    pub module_count: usize,
    /// Number of module edges.
    pub module_edge_count: usize,
    /// Number of entry points.
    pub entrypoint_count: usize,
    /// Number of file cycles.
    pub file_cycle_count: usize,
    /// Number of module cycles.
    pub module_cycle_count: usize,
    /// Largest weakly connected component.
    pub largest_connected_component: usize,
    /// Directed graph density.
    pub graph_density: f64,
    /// Maximum dependency depth.
    pub maximum_depth: usize,
    /// Average dependency depth.
    pub average_depth: f64,
    /// Estimated graph memory use.
    pub memory_estimate_bytes: usize,
}

/// Build aggregate graph statistics.
pub fn graph_stats(input: GraphStatsInput) -> GraphStats {
    GraphStats {
        node_count: input.node_count,
        edge_count: input.edge_count,
        module_count: input.module_count,
        module_edge_count: input.module_edge_count,
        entrypoint_count: input.entrypoint_count,
        file_cycle_count: input.file_cycle_count,
        module_cycle_count: input.module_cycle_count,
        largest_connected_component: input.largest_connected_component,
        graph_density: input.graph_density,
        maximum_depth: input.maximum_depth,
        average_depth: input.average_depth,
        memory_estimate_bytes: input.memory_estimate_bytes,
    }
}
