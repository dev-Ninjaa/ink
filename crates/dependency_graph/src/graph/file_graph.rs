//! File graph construction.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::{DiGraph, NodeIndex};
use repository_intelligence::RepositoryAnalysis;

use crate::models::{EdgeKind, FileNode, GraphEdge, GraphWarning, Severity};

/// Internal petgraph file graph bundle.
#[derive(Debug, Clone)]
pub struct FileGraphData {
    /// Directed graph where `A -> B` means `A` depends on `B`.
    pub graph: DiGraph<String, EdgeKind>,
    /// Stable file nodes.
    pub nodes: Vec<FileNode>,
    /// Stable serialized edges.
    pub edges: Vec<GraphEdge>,
    /// File path to node index.
    pub index_by_path: BTreeMap<String, NodeIndex>,
    /// Entry point paths present in the graph.
    pub entrypoints: Vec<String>,
    /// Build warnings.
    pub warnings: Vec<GraphWarning>,
}

/// Build the file dependency graph from Repository Intelligence output.
pub fn build_file_graph(
    analysis: &RepositoryAnalysis,
    module_by_file: &BTreeMap<String, String>,
) -> FileGraphData {
    let entrypoint_set = analysis
        .entry_points
        .iter()
        .map(|entry| entry.path.clone())
        .collect::<BTreeSet<_>>();

    let mut files = analysis.files.clone();
    files.sort_by(|left, right| left.path.cmp(&right.path));

    let mut graph = DiGraph::<String, EdgeKind>::new();
    let mut index_by_path = BTreeMap::new();
    let mut nodes = Vec::with_capacity(files.len());

    for file in files {
        let id = file.path.clone();
        let index = graph.add_node(id.clone());
        index_by_path.insert(id.clone(), index);
        nodes.push(FileNode {
            id: id.clone(),
            path: id.clone(),
            language: file.language.map(|language| language.to_string()),
            size: file.size,
            module_id: module_by_file.get(&id).cloned(),
            is_entrypoint: entrypoint_set.contains(&id),
        });
    }

    let mut edge_counts = BTreeMap::<(String, String, EdgeKind), usize>::new();
    let mut skipped_unresolved = 0usize;
    let mut skipped_missing = 0usize;

    let mut relationships = analysis.relationships.clone();
    relationships.sort_by(|left, right| {
        (&left.source, &left.target, left.kind).cmp(&(&right.source, &right.target, right.kind))
    });

    for relationship in relationships {
        if !relationship.resolved {
            skipped_unresolved += 1;
            continue;
        }

        let Some(source_index) = index_by_path.get(&relationship.source).copied() else {
            skipped_missing += 1;
            continue;
        };
        let Some(target_index) = index_by_path.get(&relationship.target).copied() else {
            skipped_missing += 1;
            continue;
        };

        let kind = EdgeKind::from(relationship.kind);
        graph.add_edge(source_index, target_index, kind.clone());
        *edge_counts
            .entry((relationship.source, relationship.target, kind))
            .or_default() += 1;
    }

    let edges = edge_counts
        .into_iter()
        .map(|((source, target, kind), weight)| GraphEdge {
            id: format!("{source}->{target}:{kind:?}").to_ascii_lowercase(),
            source,
            target,
            kind,
            weight,
        })
        .collect();

    let mut entrypoints = analysis
        .entry_points
        .iter()
        .filter(|entry| index_by_path.contains_key(&entry.path))
        .map(|entry| entry.path.clone())
        .collect::<Vec<_>>();
    entrypoints.sort();
    entrypoints.dedup();

    let mut warnings = Vec::new();
    if skipped_unresolved > 0 {
        warnings.push(GraphWarning {
            severity: Severity::Warning,
            code: "unresolved_relationships_skipped".to_string(),
            message: format!("{skipped_unresolved} unresolved relationships were skipped"),
        });
    }
    if skipped_missing > 0 {
        warnings.push(GraphWarning {
            severity: Severity::Warning,
            code: "relationships_with_missing_files_skipped".to_string(),
            message: format!(
                "{skipped_missing} relationships referenced files outside file entries"
            ),
        });
    }

    FileGraphData {
        graph,
        nodes,
        edges,
        index_by_path,
        entrypoints,
        warnings,
    }
}
