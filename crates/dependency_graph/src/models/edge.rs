//! Graph edge models.

use serde::{Deserialize, Serialize};

/// Dependency edge kind.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EdgeKind {
    /// Source imports target.
    Import,
    /// Source exports target.
    Export,
    /// Source declares target as a module.
    ModuleReference,
    /// Aggregated module dependency.
    ModuleDependency,
}

impl From<repository_intelligence::RelationshipKind> for EdgeKind {
    fn from(kind: repository_intelligence::RelationshipKind) -> Self {
        match kind {
            repository_intelligence::RelationshipKind::Import => Self::Import,
            repository_intelligence::RelationshipKind::Export => Self::Export,
            repository_intelligence::RelationshipKind::ModuleReference => Self::ModuleReference,
        }
    }
}

/// A deterministic directed graph edge.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct GraphEdge {
    /// Stable edge identifier.
    pub id: String,
    /// Source node identifier.
    pub source: String,
    /// Target node identifier.
    pub target: String,
    /// Dependency kind.
    pub kind: EdgeKind,
    /// Number of file edges represented by this edge.
    pub weight: usize,
}
