//! Framework detection from manifests and configuration files.
//!
//! Detection is driven by a project's own metadata (not code heuristics):
//!
//! * JavaScript/TypeScript: `package.json` dependencies
//!   (`next`, `react`, `express`, `@nestjs/core`, `vite`) plus a
//!   `next.config.*` fallback.
//! * Python: `requirements*.txt`, `pyproject.toml`, `Pipfile`, `setup.py`.
//! * Rust: `Cargo.toml` dependency sections.

use std::collections::BTreeSet;

use regex::Regex;

use crate::models::Framework;
use crate::util;

use super::scanner::ScanResult;

/// Frameworks as declared in a `package.json` dependency map.
const PACKAGE_JSON_FRAMEWORKS: &[(&str, Framework)] = &[
    ("next", Framework::NextJs),
    ("react", Framework::React),
    ("express", Framework::Express),
    ("@nestjs/core", Framework::NestJs),
    ("vite", Framework::Vite),
];

/// Rust crate names recognised as frameworks.
const CARGO_FRAMEWORKS: &[(&str, Framework)] = &[
    ("axum", Framework::Axum),
    ("actix-web", Framework::Actix),
    ("rocket", Framework::Rocket),
];

/// Python dependency names recognised as frameworks.
const PYTHON_FRAMEWORKS: &[(&str, Framework)] = &[
    ("fastapi", Framework::FastApi),
    ("flask", Framework::Flask),
    ("django", Framework::Django),
];

/// Filenames that can contribute framework signals; everything else is
/// skipped without a filesystem read.
const RELEVANT_NAMES: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "requirements.txt",
    "requirements-dev.txt",
    "requirements-prod.txt",
    "requirements-dev.in",
    "Pipfile",
    "setup.py",
    "pyproject.toml",
];

/// Detects frameworks present in a repository.
#[derive(Debug, Default, Clone, Copy)]
pub struct FrameworkDetector;

impl FrameworkDetector {
    /// Detect frameworks. `max_config_bytes` bounds how much of each manifest
    /// is read (they are far smaller in practice).
    pub fn detect(&self, scan: &ScanResult, max_config_bytes: u64) -> Vec<Framework> {
        let mut found: BTreeSet<Framework> = BTreeSet::new();
        let cargo_re = Regex::new(r"(?m)^\s*(?P<name>axum|actix-web|rocket)\s*[=:]").unwrap();
        let python_re = Regex::new(r"(?im)^\s*(?P<name>fastapi|flask|django)\b").unwrap();

        for file in &scan.files {
            let Some(name) = file.rel.file_name().and_then(|n| n.to_str()) else {
                continue;
            };

            if !RELEVANT_NAMES.contains(&name) && !name.starts_with("next.config") {
                continue;
            }

            let content = match util::read_text_limited(&file.path, max_config_bytes) {
                Ok(Some(text)) => text,
                _ => continue,
            };

            match name {
                "package.json" => {
                    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
                        collect_from_package_json(&value, &mut found);
                    }
                }
                "Cargo.toml" => {
                    for captures in cargo_re.captures_iter(&content) {
                        if let Some(crate_name) = captures.name("name") {
                            if let Some((_, framework)) = CARGO_FRAMEWORKS
                                .iter()
                                .find(|(name, _)| *name == crate_name.as_str())
                            {
                                found.insert(*framework);
                            }
                        }
                    }
                }
                "requirements.txt"
                | "requirements-dev.txt"
                | "requirements-prod.txt"
                | "requirements-dev.in"
                | "Pipfile"
                | "setup.py" => {
                    collect_from_python(&content, &python_re, &mut found);
                }
                "pyproject.toml" => {
                    // pyproject is TOML but is also a dependency manifest; scan
                    // lines that look like dependency entries.
                    for line in content.lines() {
                        for (dep, framework) in PYTHON_FRAMEWORKS {
                            if line.trim_start().starts_with(dep)
                                || line.contains(&format!("\"{dep}"))
                                || line.contains(&format!("'{dep}"))
                            {
                                found.insert(*framework);
                            }
                        }
                    }
                }
                _ => {
                    if name.starts_with("next.config") {
                        found.insert(Framework::NextJs);
                    }
                }
            }
        }

        found.into_iter().collect()
    }
}

fn collect_from_package_json(value: &serde_json::Value, found: &mut BTreeSet<Framework>) {
    let Some(object) = value.as_object() else {
        return;
    };
    for key in [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ] {
        let Some(deps) = object.get(key).and_then(|v| v.as_object()) else {
            continue;
        };
        for dependency in deps.keys() {
            if let Some((_, framework)) = PACKAGE_JSON_FRAMEWORKS
                .iter()
                .find(|(name, _)| *name == dependency.as_str())
            {
                found.insert(*framework);
            }
        }
    }
}

fn collect_from_python(content: &str, regex: &Regex, found: &mut BTreeSet<Framework>) {
    for captures in regex.captures_iter(content) {
        if let Some(dep) = captures.name("name") {
            if let Some((_, framework)) = PYTHON_FRAMEWORKS
                .iter()
                .find(|(name, _)| *name == dep.as_str())
            {
                found.insert(*framework);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzer::scanner::RepositoryScanner;
    use std::path::Path;
    use tempfile::tempdir;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    fn scan(root: &Path) -> ScanResult {
        RepositoryScanner::with_defaults().scan(root).unwrap()
    }

    #[test]
    fn detects_next_react_from_package_json() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "package.json",
            r#"{"dependencies":{"next":"14.0.0","react":"18.2.0","express":"4.19.0","vite":"5.0.0"}}"#,
        );
        let frameworks = FrameworkDetector.detect(&scan(dir.path()), 1_000_000);
        assert_eq!(
            frameworks,
            vec![
                Framework::NextJs,
                Framework::React,
                Framework::Express,
                Framework::Vite
            ]
        );
    }

    #[test]
    fn detects_nestjs_from_dev_dependencies() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "package.json",
            r#"{"devDependencies":{"@nestjs/core":"^10.0.0"}}"#,
        );
        let frameworks = FrameworkDetector.detect(&scan(dir.path()), 1_000_000);
        assert_eq!(frameworks, vec![Framework::NestJs]);
    }

    #[test]
    fn detects_axum_from_cargo() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "Cargo.toml",
            "[dependencies]\naxum = \"0.7\"\ntokio = { version = \"1\", features = [\"full\"] }\n",
        );
        let frameworks = FrameworkDetector.detect(&scan(dir.path()), 1_000_000);
        assert_eq!(frameworks, vec![Framework::Axum]);
    }

    #[test]
    fn detects_fastapi_flask_django() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "requirements.txt",
            "fastapi==0.111.0\nuvicorn[standard]\n",
        );
        write(
            dir.path(),
            "server/pyproject.toml",
            "[project]\ndependencies = [\"django>=4.2\"]\n",
        );
        write(dir.path(), "legacy/app.py", "# flask app\nimport flask\n");
        let frameworks = FrameworkDetector.detect(&scan(dir.path()), 1_000_000);
        assert_eq!(frameworks, vec![Framework::FastApi, Framework::Django]);
    }

    #[test]
    fn next_config_fallback() {
        let dir = tempdir().unwrap();
        write(dir.path(), "next.config.mjs", "export default {}");
        let frameworks = FrameworkDetector.detect(&scan(dir.path()), 1_000_000);
        assert_eq!(frameworks, vec![Framework::NextJs]);
    }

    #[test]
    fn empty_repo_detects_nothing() {
        let dir = tempdir().unwrap();
        write(dir.path(), "filler.txt", "nothing");
        let frameworks = FrameworkDetector.detect(&scan(dir.path()), 1_000_000);
        assert!(frameworks.is_empty());
    }

    #[test]
    fn scan_options_used_for_bounds() {
        let dir = tempdir().unwrap();
        write(
            dir.path(),
            "package.json",
            r#"{"dependencies":{"next":"1.0.0"}}"#,
        );
        // max_config_bytes far below file size => nothing detected.
        let frameworks = FrameworkDetector.detect(&scan(dir.path()), 8);
        assert!(frameworks.is_empty());
    }
}
