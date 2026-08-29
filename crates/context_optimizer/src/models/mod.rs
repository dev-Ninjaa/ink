//! Serializable context optimizer models.

pub mod context;
pub mod dedup;
pub mod metrics;
pub mod request;
pub mod tokens;

pub use context::{DroppedFile, DroppedReason, FileContext, OptimizedContext};
pub use dedup::{DedupGroup, DedupSummary};
pub use metrics::OptimizationMetrics;
pub use request::ContextRequest;
pub use tokens::TokenSummary;
