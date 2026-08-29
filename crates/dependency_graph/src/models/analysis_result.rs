//! Complete analysis result models.

use serde::{Deserialize, Serialize};

use super::{FileGraph, GraphStats, ModuleGraph};

/// Warning severity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Severity {
    /// Informational warning.
    Info,
    /// Warning that may affect graph completeness.
    Warning,
}

/// A deterministic analysis warning.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GraphWarning {
    /// Warning severity.
    pub severity: Severity,
    /// Machine-readable warning code.
    pub code: String,
    /// Human-readable warning message.
    pub message: String,
}

/// A detected strongly connected dependency cycle.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Cycle {
    /// Stable cycle identifier.
    pub id: String,
    /// Number of involved nodes.
    pub size: usize,
    /// Involved file paths, when this is a file cycle.
    pub files: Vec<String>,
    /// Involved module identifiers, when this is a module cycle.
    pub modules: Vec<String>,
}

/// Reachability result from all entry points.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Reachability {
    /// Entry point files used as graph roots.
    pub entrypoints: Vec<String>,
    /// Files reachable through dependency edges from entry points.
    pub reachable_nodes: Vec<String>,
    /// Files not reachable from any entry point.
    pub unreachable_nodes: Vec<String>,
}

/// Degree metrics for a node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct EdgeMetrics {
    /// Node identifier.
    pub id: String,
    /// Incoming dependency count.
    pub in_degree: usize,
    /// Outgoing dependency count.
    pub out_degree: usize,
    /// Sum of incoming and outgoing dependency counts.
    pub total_degree: usize,
}

/// A central node ranked by degree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct CentralNode {
    /// Node identifier.
    pub id: String,
    /// Incoming dependency count.
    pub in_degree: usize,
    /// Outgoing dependency count.
    pub out_degree: usize,
    /// Sum of incoming and outgoing dependency counts.
    pub total_degree: usize,
}

/// A representative dependency chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct DependencyChain {
    /// Number of nodes in the chain.
    pub depth: usize,
    /// Ordered node identifiers.
    pub nodes: Vec<String>,
}

/// Complete graph engine output.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct AnalysisResult {
    /// File dependency graph.
    pub file_graph: FileGraph,
    /// Module dependency graph.
    pub module_graph: ModuleGraph,
    /// Alias for file graph nodes for downstream JSON consumers.
    pub nodes: Vec<super::FileNode>,
    /// Alias for file graph edges for downstream JSON consumers.
    pub edges: Vec<super::GraphEdge>,
    /// Alias for module graph nodes for downstream JSON consumers.
    pub modules: Vec<super::ModuleNode>,
    /// Detected file cycles.
    pub file_cycles: Vec<Cycle>,
    /// Detected module cycles.
    pub module_cycles: Vec<Cycle>,
    /// All detected cycles.
    pub cycles: Vec<Cycle>,
    /// Reachability from entry points.
    pub reachability: Reachability,
    /// Per-file degree metrics.
    pub file_metrics: Vec<EdgeMetrics>,
    /// Per-module degree metrics.
    pub module_metrics: Vec<EdgeMetrics>,
    /// Highest-degree files.
    pub central_files: Vec<CentralNode>,
    /// Highest-degree modules.
    pub central_modules: Vec<CentralNode>,
    /// Representative dependency chains.
    pub dependency_chains: Vec<DependencyChain>,
    /// Aggregate statistics.
    pub statistics: GraphStats,
    /// Analysis warnings.
    pub warnings: Vec<GraphWarning>,
}
