//! Entry point detection.
//!
//! Probable application entry points are derived from well-known file
//! layouts inside each detected project root, plus the explicit `main`/`bin`
//! fields of `package.json`. Every candidate carries a confidence score.

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};

use crate::models::EntryPoint;
use crate::util;

use super::scanner::ScanResult;

/// Rooted layout rules: (path relative to the project root, confidence,
/// heuristic label). Applied to every project root.
const ROOTED_RULES: &[(&str, f64, &str)] = &[
    // Rust
    ("src/main.rs", 0.98, "rust_binary_main"),
    ("src/lib.rs", 0.95, "rust_library_crate"),
    ("main.rs", 0.85, "rust_main"),
    // Node.js / TypeScript
    ("src/main.ts", 0.9, "node_main"),
    ("src/main.tsx", 0.9, "node_main"),
    ("src/main.js", 0.9, "node_main"),
    ("src/main.mjs", 0.9, "node_main"),
    ("main.ts", 0.85, "node_root_main"),
    ("main.js", 0.85, "node_root_main"),
    ("main.mjs", 0.85, "node_root_main"),
    ("src/server.ts", 0.88, "node_server"),
    ("src/server.js", 0.88, "node_server"),
    ("server.ts", 0.85, "node_root_server"),
    ("server.js", 0.85, "node_root_server"),
    ("src/index.ts", 0.85, "node_src_index"),
    ("src/index.js", 0.85, "node_src_index"),
    ("index.ts", 0.8, "node_index"),
    ("index.js", 0.8, "node_index"),
    ("index.mjs", 0.8, "node_index"),
    ("app.ts", 0.75, "node_app"),
    ("app.js", 0.75, "node_app"),
    ("cli.ts", 0.7, "node_cli"),
    ("cli.js", 0.7, "node_cli"),
    ("src/app/page.tsx", 0.85, "next_app_page"),
    ("src/app/page.jsx", 0.85, "next_app_page"),
    ("src/extension.ts", 0.85, "vscode_extension_main"),
    // Python
    ("main.py", 0.92, "python_main"),
    ("src/main.py", 0.9, "python_src_main"),
    ("app/main.py", 0.88, "python_app_main"),
    ("manage.py", 0.9, "django_manage"),
    ("app.py", 0.85, "python_app"),
    ("__main__.py", 0.9, "python_package_main"),
    ("cli.py", 0.7, "python_cli"),
    ("wsgi.py", 0.8, "python_wsgi"),
    ("asgi.py", 0.8, "python_asgi"),
    // Go
    ("main.go", 0.8, "go_main"),
];

/// Detects probable entry points from a scan.
#[derive(Debug, Default, Clone, Copy)]
pub struct EntryPointDetector;

impl EntryPointDetector {
    /// Detect entry points. Results are sorted by confidence (desc) and then
    /// by path (asc) so the output is deterministic.
    pub fn detect(&self, scan: &ScanResult) -> Vec<EntryPoint> {
        let file_set: HashSet<PathBuf> = scan.files.iter().map(|f| f.rel.clone()).collect();
        let mut results: BTreeSet<(String, i64, String)> = BTreeSet::new();

        let roots = &scan.project_roots;
        for root in roots {
            // Cargo binary targets inside `src/bin/`.
            for file in &scan.files {
                if file.rel.starts_with(root.join("src/bin"))
                    && file.rel.extension().and_then(|e| e.to_str()) == Some("rs")
                {
                    results.insert((
                        util::forward_slashes(&file.rel),
                        confidence_to_score(0.85),
                        "rust_bin_target".to_string(),
                    ));
                }
            }
            // Go command entry points: `cmd/<name>/main.go`.
            for file in &scan.files {
                if let Ok(stripped) = file.rel.strip_prefix(root.join("cmd")) {
                    if file.rel.extension().and_then(|e| e.to_str()) == Some("go")
                        && stripped.file_name().and_then(|n| n.to_str()) == Some("main.go")
                    {
                        results.insert((
                            util::forward_slashes(&file.rel),
                            confidence_to_score(0.85),
                            "go_command_main".to_string(),
                        ));
                    }
                }
            }

            for (relative, confidence, heuristic) in ROOTED_RULES {
                let candidate = root.join(relative);
                if file_set.contains(&candidate) {
                    results.insert((
                        util::forward_slashes(&candidate),
                        confidence_to_score(*confidence),
                        (*heuristic).to_string(),
                    ));
                }
            }
        }

        // Explicit package.json `main`/`bin` declarations.
        for file in &scan.files {
            let Some(name) = file.rel.file_name().and_then(|n| n.to_str()) else {
                continue;
            };
            if name != "package.json" {
                continue;
            }
            let Some(content) = util::read_text_limited(&file.path, 1_000_000)
                .ok()
                .flatten()
            else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
                continue;
            };
            let dir = file.rel.parent().unwrap_or(Path::new(""));
            if let Some(main) = value.get("main").and_then(|v| v.as_str()) {
                for target in expand_module_target(main, &file_set, dir) {
                    results.insert((
                        util::forward_slashes(&target),
                        confidence_to_score(0.92),
                        "package_json_main".to_string(),
                    ));
                }
            }
            if let Some(bin) = value.get("bin") {
                for target in extract_bin_targets(bin, &file_set, dir) {
                    results.insert((
                        util::forward_slashes(&target),
                        confidence_to_score(0.9),
                        "package_json_bin".to_string(),
                    ));
                }
            }
        }

        let mut out: Vec<EntryPoint> = results
            .into_iter()
            .map(|(path, score, heuristic)| EntryPoint {
                path,
                confidence: score_to_confidence(score),
                heuristic,
            })
            .collect();
        out.sort_by(|a, b| {
            b.confidence
                .partial_cmp(&a.confidence)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| a.path.cmp(&b.path))
        });
        out
    }
}

/// Expand a `package.json` `main`/`bin` specifier into existing files.
/// TS entry points commonly drop the extension, so we probe common ones.
fn expand_module_target(spec: &str, file_set: &HashSet<PathBuf>, dir: &Path) -> Vec<PathBuf> {
    let base = util::normalize_lexical(&dir.join(spec));
    let ext_candidates = ["", ".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];
    let mut found = Vec::new();
    for ext in ext_candidates {
        let candidate = if ext.is_empty() {
            base.clone()
        } else {
            base.with_extension(ext.trim_start_matches('.'))
        };
        if file_set.contains(&candidate) {
            found.push(candidate);
        }
    }
    found
}

fn extract_bin_targets(
    bin: &serde_json::Value,
    file_set: &HashSet<PathBuf>,
    dir: &Path,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    match bin {
        serde_json::Value::String(spec) => out.extend(expand_module_target(spec, file_set, dir)),
        serde_json::Value::Object(map) => {
            for spec in map.values().filter_map(|v| v.as_str()) {
                out.extend(expand_module_target(spec, file_set, dir));
            }
        }
        _ => {}
    }
    out
}

/// Serialize confidence to a sortable integer to de-duplicate equal paths.
fn confidence_to_score(confidence: f64) -> i64 {
    (confidence * 1_000_000.0).round() as i64
}

fn score_to_confidence(score: i64) -> f64 {
    score as f64 / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::scanner::RepositoryScanner;
    use tempfile::tempdir;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    fn detect(root: &Path) -> Vec<EntryPoint> {
        let scan = RepositoryScanner::with_defaults().scan(root).unwrap();
        EntryPointDetector.detect(&scan)
    }

    #[test]
    fn multiple_entry_points_and_order() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/main.rs", "fn main() {}");
        write(root, "src/lib.rs", "pub fn f(){}");
        write(root, "server.js", "console.log(1)");
        write(root, "src/app/page.tsx", "export default function P(){}");

        let entries = detect(root);
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(
            paths,
            vec!["src/main.rs", "src/lib.rs", "server.js", "src/app/page.tsx"]
        );
        // Confidence strictly descending.
        for w in entries.windows(2) {
            assert!(w[0].confidence >= w[1].confidence);
        }
    }

    #[test]
    fn missing_entry_points_is_empty() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/util/helper.ts", "export const h = 1;");
        write(root, "README.md", "# x");
        let entries = detect(root);
        assert!(entries.is_empty());
    }

    #[test]
    fn monorepo_layout_detects_per_package() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", "{}");
        write(root, "apps/api/package.json", r#"{"main":"src/index.ts"}"#);
        write(root, "apps/api/src/index.ts", "console.log('api')");
        write(root, "apps/cli/package.json", "{}");
        write(root, "apps/cli/src/main.ts", "console.log('cli')");

        let entries = detect(root);
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert!(paths.contains(&"apps/api/src/index.ts"));
        assert!(paths.contains(&"apps/cli/src/main.ts"));
        for entry in &entries {
            assert!(
                entry.path.starts_with("apps/"),
                "unexpected: {}",
                entry.path
            );
        }
    }

    #[test]
    fn python_entry_points() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "main.py", "print('hi')");
        write(
            root,
            "manage.py",
            "from django.core.management import execute",
        );
        let entries = detect(root);
        let paths: Vec<&str> = entries.iter().map(|e| e.path.as_str()).collect();
        assert_eq!(paths, vec!["main.py", "manage.py"]);
    }

    #[test]
    fn rust_bin_targets() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/bin/tools.rs", "fn main(){}");
        let entries = detect(root);
        assert_eq!(entries[0].path, "src/bin/tools.rs");
        assert_eq!(entries[0].heuristic, "rust_bin_target");
    }

    #[test]
    fn vscode_extension_entry_point() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", r#"{"main":"./out/extension.js"}"#);
        write(
            root,
            "src/extension.ts",
            "import * as vscode from 'vscode';",
        );
        let entries = detect(root);
        assert!(entries.iter().any(|e| e.path == "src/extension.ts"));
    }
}
