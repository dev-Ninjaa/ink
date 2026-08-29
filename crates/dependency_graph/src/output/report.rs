//! Human-readable graph reports.

use crate::models::AnalysisResult;

/// Render a concise Markdown report for a graph analysis result.
pub fn render_report(result: &AnalysisResult) -> String {
    let stats = &result.statistics;
    format!(
        "# Dependency Graph Report\n\n\
         ## Summary\n\n\
         - File nodes: {}\n\
         - File edges: {}\n\
         - Module nodes: {}\n\
         - Module edges: {}\n\
         - Entry points: {}\n\
         - File cycles: {}\n\
         - Module cycles: {}\n\
         - Largest connected component: {}\n\
         - Graph density: {:.6}\n\
         - Maximum dependency depth: {}\n\
         - Average dependency depth: {:.2}\n\
         - Memory estimate: {} bytes\n",
        stats.node_count,
        stats.edge_count,
        stats.module_count,
        stats.module_edge_count,
        stats.entrypoint_count,
        stats.file_cycle_count,
        stats.module_cycle_count,
        stats.largest_connected_component,
        stats.graph_density,
        stats.maximum_depth,
        stats.average_depth,
        stats.memory_estimate_bytes
    )
}
