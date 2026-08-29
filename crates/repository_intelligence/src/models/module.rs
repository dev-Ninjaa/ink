//! Logical module model produced by the module detector.

use serde::{Deserialize, Serialize};

/// The kind of a discovered logical module.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ModuleKind {
    /// A feature module (e.g. the `auth` or `cart` folder) that groups
    /// behaviour around a business capability.
    Feature,
    /// A layered module following an N-tier architecture (e.g. `controllers`,
    /// `models`, `repositories`).
    Layer,
    /// A standalone package (e.g. a workspace member/monorepo package).
    Package,
}

/// A logical module inside a repository.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Module {
    /// Short module name derived from the folder name.
    pub name: String,
    /// Classification of the module.
    pub kind: ModuleKind,
    /// Repository-relative path of the module root folder.
    pub root: String,
    /// Repository-relative paths of every file owned by the module.
    pub files: Vec<String>,
}
