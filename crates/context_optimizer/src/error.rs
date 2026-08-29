//! Error type and `Result` alias for the context optimizer.

use std::io;
use std::path::PathBuf;

/// Errors produced while optimizing context.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// A filesystem operation failed for a specific path.
    #[error("I/O error while accessing `{path}`: {source}")]
    Io {
        /// The path that caused the failure.
        path: PathBuf,
        /// The underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// A file we had to read was not valid UTF-8 text.
    #[error("file `{0}` is not valid UTF-8 text")]
    NonUtf8(PathBuf),

    /// JSON serialization failed.
    #[error("failed to serialize JSON output: {0}")]
    Json(#[from] serde_json::Error),

    /// An unexpected or user-facing error with no dedicated variant yet.
    #[error("{0}")]
    Message(String),
}

impl Error {
    /// Construct an [`Error::Io`] for the given path.
    pub fn io(path: impl Into<PathBuf>, source: io::Error) -> Self {
        Error::Io {
            path: path.into(),
            source,
        }
    }
}

/// Convenience alias used throughout the crate.
pub type Result<T> = std::result::Result<T, Error>;
