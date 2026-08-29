//! Public strongly-typed models produced by the analysis engine.

pub mod framework;
pub mod language;
pub mod module;
pub mod relationship;
pub mod repository;

pub use framework::{Ecosystem, Framework};
pub use language::Language;
pub use module::{Module, ModuleKind};
pub use relationship::{Relationship, RelationshipKind};
pub use repository::{
    AnalysisSummary, EntryPoint, FileEntry, PerformanceMetrics, ProjectMetadata, RepositoryAnalysis,
};
