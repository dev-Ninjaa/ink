//! # dependency_graph
//!
//! Petgraph-backed dependency graph analysis for Ink.
//!
//! This crate consumes [`repository_intelligence::RepositoryAnalysis`] and
//! builds deterministic file and module dependency graphs without rescanning
//! repositories or duplicating Repository Intelligence extraction logic.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![warn(rustdoc::broken_intra_doc_links)]

pub mod graph;
pub mod models;
pub mod output;

pub use graph::builder::DependencyGraphBuilder;
pub use models::{
    AnalysisResult, CentralNode, Cycle, DependencyChain, EdgeKind, EdgeMetrics, FileGraph,
    FileNode, GraphEdge, GraphStats, GraphWarning, ModuleGraph, ModuleNode, NodeKind, Reachability,
    Severity,
};

/// Build and analyse a dependency graph from Repository Intelligence output.
pub fn analyze_dependencies(
    analysis: &repository_intelligence::RepositoryAnalysis,
) -> AnalysisResult {
    DependencyGraphBuilder::new(analysis).build()
}
