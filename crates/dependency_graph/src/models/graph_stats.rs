//! Aggregate graph statistics.

use serde::{Deserialize, Serialize};

/// Summary statistics for dependency graph analysis.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GraphStats {
    /// Number of file nodes.
    pub node_count: usize,
    /// Number of file edges.
    pub edge_count: usize,
    /// Number of module nodes.
    pub module_count: usize,
    /// Number of module edges.
    pub module_edge_count: usize,
    /// Number of Repository Intelligence entry points present in the graph.
    pub entrypoint_count: usize,
    /// Number of detected file cycles.
    pub file_cycle_count: usize,
    /// Number of detected module cycles.
    pub module_cycle_count: usize,
    /// Size of the largest weakly connected file component.
    pub largest_connected_component: usize,
    /// Directed file graph density.
    pub graph_density: f64,
    /// Maximum dependency depth from entry points.
    pub maximum_depth: usize,
    /// Average shortest dependency depth from entry points.
    pub average_depth: f64,
    /// Estimated heap footprint of graph data in bytes.
    pub memory_estimate_bytes: usize,
}
