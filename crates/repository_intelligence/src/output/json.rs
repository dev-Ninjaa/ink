//! Serialize a [`RepositoryAnalysis`] to JSON.
//!
//! All collections inside the analysis are deterministically ordered, so the
//! produced JSON is byte-for-byte stable across runs on the same repository.

use std::path::Path;

use serde_json::Value;

use crate::error::{Error, Result};
use crate::models::RepositoryAnalysis;

/// Pretty-printed JSON document of an analysis.
pub fn to_json(analysis: &RepositoryAnalysis) -> Result<String> {
    serde_json::to_string_pretty(analysis).map_err(Error::from)
}

/// Compact (single-line) JSON document of an analysis.
pub fn to_json_compact(analysis: &RepositoryAnalysis) -> Result<String> {
    serde_json::to_string(analysis).map_err(Error::from)
}

/// Write the pretty JSON document to `path`, creating parent directories as
/// needed.
pub fn write_json_file(analysis: &RepositoryAnalysis, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|source| Error::io(parent.to_path_buf(), source))?;
    }
    let contents = to_json(analysis)?;
    std::fs::write(path, contents).map_err(|source| Error::io(path.to_path_buf(), source))
}

/// Convert an analysis to a generic [`Value`] for embedding in larger
/// documents (e.g. the MCP server or extension payloads).
pub fn to_value(analysis: &RepositoryAnalysis) -> Result<Value> {
    serde_json::to_value(analysis).map_err(Error::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::Analyzer;
    use std::collections::BTreeMap;
    use std::path::PathBuf;

    fn sample() -> RepositoryAnalysis {
        RepositoryAnalysis {
            root: "/tmp/repo".into(),
            analyzer_version: "test".into(),
            summary: crate::models::AnalysisSummary {
                files: 2,
                directories: 1,
                project_roots: 1,
                bytes: 40,
            },
            performance: crate::models::PerformanceMetrics {
                scan_duration_ms: 1.0,
                analysis_duration_ms: 2.0,
                total_duration_ms: 3.0,
                files_per_second: 1000.0,
            },
            languages: BTreeMap::from([(crate::models::Language::Rust, 1)]),
            frameworks: vec![],
            metadata: crate::models::ProjectMetadata::default(),
            project_roots: vec![PathBuf::new()]
                .into_iter()
                .map(|p| p.display().to_string())
                .collect(),
            entry_points: vec![],
            modules: vec![],
            relationships: vec![],
            files: vec![],
            directories: vec![],
        }
    }

    #[test]
    fn json_is_deterministic() {
        let a = sample();
        let first = to_json(&a).unwrap();
        let second = to_json(&a).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn compact_and_pretty_both_parse() {
        let a = sample();
        let compact = to_json_compact(&a).unwrap();
        let pretty = to_json(&a).unwrap();
        let v1: serde_json::Value = serde_json::from_str(&compact).unwrap();
        let v2: serde_json::Value = serde_json::from_str(&pretty).unwrap();
        assert_eq!(v1, v2);
        assert!(pretty.contains('\n'));
    }

    #[test]
    fn round_trips_through_deserialize() {
        let a = sample();
        let json_str = to_json(&a).unwrap();
        let parsed: RepositoryAnalysis = serde_json::from_str(&json_str).unwrap();
        assert_eq!(parsed.summary.files, 2);
        assert_eq!(parsed.languages[&crate::models::Language::Rust], 1);
    }

    #[test]
    fn writes_file_with_parents() {
        let dir = tempfile::tempdir().unwrap();
        let target = dir.path().join("nested/dir/report.json");
        write_json_file(&sample(), &target).unwrap();
        let content = std::fs::read_to_string(&target).unwrap();
        assert!(content.contains("analyzer_version"));
    }

    #[test]
    fn analyzer_drives_test_coverage() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("main.rs"), "fn main() {}").unwrap();
        let analysis = Analyzer::with_defaults().analyze(dir.path()).unwrap();
        assert_eq!(analysis.languages[&crate::models::Language::Rust], 1);
    }
}
