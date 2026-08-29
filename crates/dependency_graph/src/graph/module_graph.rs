//! Module graph construction.

use std::collections::{BTreeMap, BTreeSet};

use petgraph::graph::{DiGraph, NodeIndex};
use repository_intelligence::RepositoryAnalysis;

use crate::models::{EdgeKind, GraphEdge, ModuleNode};

/// Internal petgraph module graph bundle.
#[derive(Debug, Clone)]
pub struct ModuleGraphData {
    /// Directed module graph.
    pub graph: DiGraph<String, EdgeKind>,
    /// Stable module nodes.
    pub nodes: Vec<ModuleNode>,
    /// Stable module edges.
    pub edges: Vec<GraphEdge>,
    /// Module identifier to node index.
    pub index_by_id: BTreeMap<String, NodeIndex>,
    /// File path to owning module identifier.
    pub module_by_file: BTreeMap<String, String>,
}

/// Build a deterministic file-to-module ownership map.
pub fn module_ownership(analysis: &RepositoryAnalysis) -> BTreeMap<String, String> {
    let mut module_by_file = BTreeMap::new();
    let mut modules = analysis.modules.clone();
    modules.sort_by(|left, right| {
        (&left.root, &left.name, left.kind).cmp(&(&right.root, &right.name, right.kind))
    });

    for module in modules {
        let module_id = module_id(&module.root, &module.name);
        for file in module.files {
            module_by_file
                .entry(file)
                .or_insert_with(|| module_id.clone());
        }
    }

    module_by_file
}

/// Build the module dependency graph from file edges.
pub fn build_module_graph(
    analysis: &RepositoryAnalysis,
    file_edges: &[GraphEdge],
    module_by_file: &BTreeMap<String, String>,
) -> ModuleGraphData {
    let mut modules = analysis.modules.clone();
    modules.sort_by(|left, right| {
        (&left.root, &left.name, left.kind).cmp(&(&right.root, &right.name, right.kind))
    });

    let mut graph = DiGraph::<String, EdgeKind>::new();
    let mut index_by_id = BTreeMap::new();
    let mut nodes = Vec::with_capacity(modules.len());

    for module in modules {
        let id = module_id(&module.root, &module.name);
        let index = graph.add_node(id.clone());
        index_by_id.insert(id.clone(), index);
        let mut files = module.files;
        files.sort();
        files.dedup();
        nodes.push(ModuleNode {
            id,
            name: module.name,
            root: module.root,
            kind: format!("{:?}", module.kind).to_ascii_lowercase(),
            files,
        });
    }

    let mut edge_counts = BTreeMap::<(String, String), usize>::new();
    for edge in file_edges {
        let Some(source_module) = module_by_file.get(&edge.source) else {
            continue;
        };
        let Some(target_module) = module_by_file.get(&edge.target) else {
            continue;
        };
        if source_module == target_module {
            continue;
        }
        *edge_counts
            .entry((source_module.clone(), target_module.clone()))
            .or_default() += edge.weight;
    }

    let mut unique_edges = BTreeSet::new();
    let mut edges = Vec::with_capacity(edge_counts.len());
    for ((source, target), weight) in edge_counts {
        let Some(source_index) = index_by_id.get(&source).copied() else {
            continue;
        };
        let Some(target_index) = index_by_id.get(&target).copied() else {
            continue;
        };
        if unique_edges.insert((source.clone(), target.clone())) {
            graph.add_edge(source_index, target_index, EdgeKind::ModuleDependency);
        }
        edges.push(GraphEdge {
            id: format!("{source}->{target}:module_dependency"),
            source,
            target,
            kind: EdgeKind::ModuleDependency,
            weight,
        });
    }

    ModuleGraphData {
        graph,
        nodes,
        edges,
        index_by_id,
        module_by_file: module_by_file.clone(),
    }
}

fn module_id(root: &str, name: &str) -> String {
    if root.is_empty() {
        name.to_string()
    } else {
        format!("{root}:{name}")
    }
}
