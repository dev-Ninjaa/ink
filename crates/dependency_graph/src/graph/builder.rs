//! Dependency graph builder.

use repository_intelligence::RepositoryAnalysis;

use crate::graph::{analysis, cycle_detector, file_graph, metrics, module_graph, reachability};
use crate::models::{AnalysisResult, FileGraph, ModuleGraph};

/// Builds and analyses dependency graphs from Repository Intelligence output.
pub struct DependencyGraphBuilder<'a> {
    analysis: &'a RepositoryAnalysis,
}

impl<'a> DependencyGraphBuilder<'a> {
    /// Create a builder for a Repository Intelligence result.
    pub fn new(analysis: &'a RepositoryAnalysis) -> Self {
        Self { analysis }
    }

    /// Build the dependency graph and run all core analysis passes.
    pub fn build(&self) -> AnalysisResult {
        let module_by_file = module_graph::module_ownership(self.analysis);
        let file_data = file_graph::build_file_graph(self.analysis, &module_by_file);
        let module_data =
            module_graph::build_module_graph(self.analysis, &file_data.edges, &module_by_file);

        let all_file_ids = file_data
            .nodes
            .iter()
            .map(|node| node.id.clone())
            .collect::<Vec<_>>();
        let reachability = reachability::analyze_reachability(
            &file_data.graph,
            &file_data.entrypoints,
            &all_file_ids,
            &file_data.index_by_path,
        );

        let file_cycles = cycle_detector::detect_file_cycles(&file_data.graph);
        let module_cycles = cycle_detector::detect_module_cycles(&module_data.graph);
        let mut cycles = file_cycles.clone();
        cycles.extend(module_cycles.clone());
        cycles.sort_by(|left, right| left.id.cmp(&right.id));

        let file_metrics = metrics::degree_metrics(&file_data.graph);
        let module_metrics = metrics::degree_metrics(&module_data.graph);
        let central_files = metrics::central_nodes(&file_metrics, 10);
        let central_modules = metrics::central_nodes(&module_metrics, 10);
        let (maximum_depth, average_depth, dependency_chains) = metrics::depth_analysis(
            &file_data.graph,
            &file_data.entrypoints,
            &file_data.index_by_path,
        );

        let string_bytes = file_data
            .nodes
            .iter()
            .map(|node| node.id.len() + node.language.as_ref().map_or(0, String::len))
            .sum::<usize>()
            + file_data
                .edges
                .iter()
                .map(|edge| edge.source.len() + edge.target.len() + edge.id.len())
                .sum::<usize>()
            + module_data
                .nodes
                .iter()
                .map(|node| node.id.len() + node.name.len() + node.root.len())
                .sum::<usize>()
            + module_data
                .edges
                .iter()
                .map(|edge| edge.source.len() + edge.target.len() + edge.id.len())
                .sum::<usize>();

        let statistics = analysis::graph_stats(analysis::GraphStatsInput {
            node_count: file_data.nodes.len(),
            edge_count: file_data.edges.len(),
            module_count: module_data.nodes.len(),
            module_edge_count: module_data.edges.len(),
            entrypoint_count: file_data.entrypoints.len(),
            file_cycle_count: file_cycles.len(),
            module_cycle_count: module_cycles.len(),
            largest_connected_component: metrics::largest_weak_component(&file_data.graph),
            graph_density: metrics::density(file_data.nodes.len(), file_data.edges.len()),
            maximum_depth,
            average_depth,
            memory_estimate_bytes: metrics::memory_estimate(
                file_data.nodes.len(),
                file_data.edges.len(),
                module_data.nodes.len(),
                module_data.edges.len(),
                string_bytes,
            ),
        });

        let file_graph = FileGraph {
            nodes: file_data.nodes,
            edges: file_data.edges,
        };
        let module_graph = ModuleGraph {
            nodes: module_data.nodes,
            edges: module_data.edges,
        };

        AnalysisResult {
            nodes: file_graph.nodes.clone(),
            edges: file_graph.edges.clone(),
            modules: module_graph.nodes.clone(),
            file_graph,
            module_graph,
            file_cycles,
            module_cycles,
            cycles,
            reachability,
            file_metrics,
            module_metrics,
            central_files,
            central_modules,
            dependency_chains,
            statistics,
            warnings: file_data.warnings,
        }
    }
}
