//! Internal shared utilities: cross-platform path handling, lexical path
//! normalization and bounded text reads.
//!
//! Everything in this module is optimised for small, allocation-lean
//! operations because it sits on the hot path of repository analysis.

use std::fs;
use std::path::{Component, Path, PathBuf};

/// Maximum number of bytes considered "a regular stream analysis case".
/// Files larger than this limit are skipped by the import extractor.
pub const DEFAULT_MAX_SOURCE_BYTES: u64 = 512 * 1024;

/// Convert a path to a forward-slash string, hiding OS-specific separators
/// so that analysis output is byte-for-byte identical across platforms.
pub fn forward_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

/// Lexically normalise `path` by removing `.` segments and collapsing `..`
/// segments. This never touches the filesystem, so it is safe to apply to
/// both relative and absolute paths produced during resolution.
pub fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Read `path` as UTF-8 text if it is smaller than `max_bytes`.
///
/// Returns `Ok(None)` when the file is too large or not valid UTF-8 so that
/// callers can transparently skip files they are not interested in. Hard I/O
/// failures (missing file, permission errors) are surfaced as `Err`.
pub fn read_text_limited(path: &Path, max_bytes: u64) -> std::io::Result<Option<String>> {
    let metadata = fs::metadata(path)?;
    if metadata.len() > max_bytes {
        return Ok(None);
    }
    let bytes = fs::read(path)?;
    match String::from_utf8(bytes) {
        Ok(text) => Ok(Some(text)),
        Err(_) => Ok(None),
    }
}

/// Cheap trick to check whether a path exists on disk, collapsing the
/// result into a boolean the way callers in this crate want it.
pub fn path_exists(path: &Path) -> bool {
    path.try_exists().unwrap_or(false)
}
