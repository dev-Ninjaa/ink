//! Graph node models.

use serde::{Deserialize, Serialize};

/// A broad node class for serialized output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// Repository file node.
    File,
    /// Repository module node.
    Module,
}

/// A file-level graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FileNode {
    /// Stable node identifier, equal to the repository-relative path.
    pub id: String,
    /// Repository-relative path.
    pub path: String,
    /// Detected language, when Repository Intelligence found one.
    pub language: Option<String>,
    /// File size in bytes.
    pub size: u64,
    /// Owning module identifier, when known.
    pub module_id: Option<String>,
    /// Whether Repository Intelligence marked this file as an entry point.
    pub is_entrypoint: bool,
}

/// A module-level graph node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModuleNode {
    /// Stable module identifier.
    pub id: String,
    /// Display name.
    pub name: String,
    /// Repository-relative module root.
    pub root: String,
    /// Repository Intelligence module kind.
    pub kind: String,
    /// Files owned by the module.
    pub files: Vec<String>,
}

/// Serialized file graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct FileGraph {
    /// File nodes sorted by identifier.
    pub nodes: Vec<FileNode>,
    /// File edges sorted by source, target, and kind.
    pub edges: Vec<crate::models::GraphEdge>,
}

/// Serialized module graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct ModuleGraph {
    /// Module nodes sorted by identifier.
    pub nodes: Vec<ModuleNode>,
    /// Module edges sorted by source, target, and kind.
    pub edges: Vec<crate::models::GraphEdge>,
}
