//! Import/export relationship model.

use serde::{Deserialize, Serialize};

/// The nature of a relationship between two files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RelationshipKind {
    /// The source file imports from the target.
    Import,
    /// The source file re-exports the target.
    Export,
    /// The source file declares the target as a submodule (Rust `mod`).
    ModuleReference,
}

/// A directed edge between two repository files.
///
/// `source` and `target` are repository-relative paths. When an import
/// specifier could not be mapped to a real file, `target` holds the raw
/// specifier and `resolved` is `false`, so downstream consumers can always
/// distinguish a confirmed edge from a heuristic one.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Relationship {
    /// Repository-relative path of the file containing the reference.
    pub source: String,
    /// Repository-relative path of the referenced file, or the raw
    /// specifier when it could not be resolved to a file.
    pub target: String,
    /// What kind of reference this is.
    pub kind: RelationshipKind,
    /// Whether `target` points at a real file inside the repository.
    pub resolved: bool,
}
