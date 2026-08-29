//! Tooling metadata detection: package managers, build systems, lockfiles,
//! configuration files, CI pipelines and container setup.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use crate::models::ProjectMetadata;
use crate::util;

use super::scanner::ScanResult;

/// Filenames that mark a directory as a project root.
pub const MANIFEST_NAMES: &[&str] = &[
    "package.json",
    "Cargo.toml",
    "pyproject.toml",
    "requirements.txt",
    "requirements-dev.txt",
    "setup.py",
    "Pipfile",
    "go.mod",
    "Gopkg.toml",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "settings.gradle",
    "settings.gradle.kts",
    "sbt.build",
    "mix.exs",
    "composer.json",
    "pubspec.yaml",
    "Package.swift",
    "Gemfile",
    "meson.build",
    "Cargo.lock",
];

/// Lockfile filename → package manager.
const LOCKFILE_PACKAGE_MANAGERS: &[(&str, &str)] = &[
    ("package-lock.json", "npm"),
    ("pnpm-lock.yaml", "pnpm"),
    ("pnpm-lock.yml", "pnpm"),
    ("yarn.lock", "yarn"),
    ("bun.lockb", "bun"),
    ("bun.lock", "bun"),
    ("Cargo.lock", "cargo"),
    ("poetry.lock", "poetry"),
    ("Pipfile.lock", "pipenv"),
    ("uv.lock", "uv"),
    ("go.sum", "go"),
    ("composer.lock", "composer"),
    ("Gemfile.lock", "bundler"),
];

/// Build system definition filename → build system name.
const BUILD_SYSTEMS: &[(&str, &str)] = &[
    ("Makefile", "make"),
    ("makefile", "make"),
    ("GNUmakefile", "make"),
    ("CMakeLists.txt", "cmake"),
    ("meson.build", "meson"),
    ("build.gradle", "gradle"),
    ("build.gradle.kts", "gradle"),
    ("settings.gradle", "gradle"),
    ("settings.gradle.kts", "gradle"),
    ("pom.xml", "maven"),
    ("BUILD", "bazel"),
    ("BUILD.bazel", "bazel"),
    ("WORKSPACE", "bazel"),
    ("justfile", "just"),
    ("Taskfile.yml", "task"),
    ("Taskfile.yaml", "task"),
    ("Rakefile", "rake"),
    ("BUCK", "buck"),
    ("build.ninja", "ninja"),
];

/// Notable configuration filenames we report on.
const CONFIG_FILES: &[&str] = &[
    "package.json",
    "tsconfig.json",
    "jsconfig.json",
    "Cargo.toml",
    "pyproject.toml",
    "requirements.txt",
    "Pipfile",
    "go.mod",
    ".gitignore",
    ".gitattributes",
    ".editorconfig",
    ".prettierrc",
    ".prettierrc.json",
    ".eslintrc",
    ".eslintrc.json",
    ".eslintrc.js",
    ".eslintrc.yml",
    "eslint.config.js",
    "eslint.config.mjs",
    ".eslintignore",
    ".npmrc",
    ".nvmrc",
    ".node-version",
    "pnpm-workspace.yaml",
    ".yarnrc.yml",
    "biome.json",
    "biome.jsonc",
    ".env.example",
    ".env.development",
    ".env.production",
    "jest.config.js",
    "jest.config.ts",
    "vitest.config.ts",
    "vite.config.ts",
    "vite.config.js",
    "next.config.js",
    "next.config.mjs",
    "tailwind.config.js",
    "tailwind.config.ts",
    "webpack.config.js",
    "rollup.config.js",
    "vue.config.js",
    "nuxt.config.ts",
    "postcss.config.js",
    "babel.config.js",
    "esbuild.config.js",
    "deno.json",
    "deno.jsonc",
    "Cargo.lock",
    "rust-toolchain.toml",
    "rustfmt.toml",
    "clippy.toml",
    "mypy.ini",
    ".flake8",
    "ruff.toml",
    ".pre-commit-config.yaml",
    "docker-compose.yml",
    "docker-compose.yaml",
    "compose.yaml",
    "compose.yml",
    ".dockerignore",
];

/// CI configuration lookup. Values are (filename or nested path, ci name).
const CI_SYSTEMS: &[(&str, &str)] = &[
    (".github/workflows", "github_actions"),
    (".gitlab-ci.yml", "gitlab_ci"),
    ("Jenkinsfile", "jenkins"),
    ("buildkite.yml", "buildkite"),
    (".buildkite", "buildkite"),
    (".circleci/config.yml", "circleci"),
    ("azure-pipelines.yml", "azure_pipelines"),
    ("appveyor.yml", "appveyor"),
    ("bitbucket-pipelines.yml", "bitbucket_pipelines"),
    ("travis.yml", "travis_ci"),
    ("drone.yml", "drone"),
];

/// Detects project tooling metadata from a completed scan.
#[derive(Debug, Default, Clone, Copy)]
pub struct MetadataDetector;

impl MetadataDetector {
    /// Analyse tooling metadata for `scan`.
    pub fn detect(&self, scan: &ScanResult) -> ProjectMetadata {
        let mut package_managers = BTreeSet::new();
        let mut build_systems = BTreeSet::new();
        let mut lockfiles = Vec::new();
        let mut config_files = Vec::new();
        let mut manifests = Vec::new();
        let mut ci = BTreeSet::new();
        let mut has_package_json = false;
        let mut has_js_lockfile = false;
        let mut has_cargo_toml = false;
        let mut has_go_mod = false;
        let mut has_python_manifest = false;
        let mut has_docker = false;

        for file in &scan.files {
            let name = file.rel.file_name().and_then(|n| n.to_str());
            let Some(name) = name else {
                continue;
            };

            // Lockfiles & package managers.
            if let Some((_, manager)) = LOCKFILE_PACKAGE_MANAGERS
                .iter()
                .find(|(lock, _)| *lock == name)
            {
                package_managers.insert((*manager).to_string());
                let is_js = name.ends_with("lock.json")
                    || name == "pnpm-lock.yaml"
                    || name == "pnpm-lock.yml"
                    || name == "yarn.lock"
                    || name == "bun.lockb"
                    || name == "bun.lock";
                if is_js {
                    has_js_lockfile = true;
                }
                lockfiles.push(util::forward_slashes(&file.rel));
            }

            if name == "package.json" {
                has_package_json = true;
            }
            if name == "Cargo.toml" || name == "Cargo.lock" {
                has_cargo_toml = true;
            }
            if name == "go.mod" {
                has_go_mod = true;
            }
            if name == "requirements.txt"
                || name == "setup.py"
                || name == "pyproject.toml"
                || name == "Pipfile"
            {
                has_python_manifest = true;
            }

            // Build systems.
            if let Some((_, system)) = BUILD_SYSTEMS.iter().find(|(b, _)| *b == name) {
                build_systems.insert((*system).to_string());
            }

            // .NET projects expose build info through extensions.
            if name.ends_with(".csproj") {
                build_systems.insert("dotnet".to_string());
            }
            if name.ends_with(".sln") {
                build_systems.insert("dotnet".to_string());
            }

            // Configuration files.
            if CONFIG_FILES.contains(&name) {
                config_files.push(util::forward_slashes(&file.rel));
            }

            // Manifests.
            if MANIFEST_NAMES.contains(&name) {
                manifests.push(util::forward_slashes(&file.rel));
            }

            // Containers.
            if name.starts_with("Dockerfile")
                || name.starts_with("docker-compose")
                || name == "compose.yml"
                || name == "compose.yaml"
            {
                has_docker = true;
            }

            // CI.
            if name == ".gitlab-ci.yml"
                || name == "Jenkinsfile"
                || name == "azure-pipelines.yml"
                || name == "appveyor.yml"
                || name == "bitbucket-pipelines.yml"
                || name == "travis.yml"
                || name == "drone.yml"
            {
                if let Some((_, system)) = CI_SYSTEMS.iter().find(|(c, _)| *c == name) {
                    ci.insert((*system).to_string());
                }
            }
        }

        // Nested CI paths covered by directory checks.
        let ci_dirs = [".github/workflows", ".buildkite", ".circleci"];
        for dir in &scan.directories {
            let rel = util::normalize_lexical(dir);
            if ci_dirs.iter().any(|d| rel.starts_with(d)) {
                let system = if rel.starts_with(".github") {
                    "github_actions"
                } else if rel.starts_with(".circleci") {
                    "circleci"
                } else {
                    "buildkite"
                };
                ci.insert(system.to_string());
            }
        }

        // Infer npm when a package.json is present but no JS lockfile was found.
        if has_package_json && !has_js_lockfile {
            package_managers.insert("npm".to_string());
        }
        if has_cargo_toml {
            package_managers.insert("cargo".to_string());
        }
        if has_go_mod {
            package_managers.insert("go".to_string());
        }
        if has_python_manifest
            && !package_managers.contains("poetry")
            && !package_managers.contains("uv")
            && !package_managers.contains("pipenv")
        {
            package_managers.insert("pip".to_string());
        }

        ProjectMetadata {
            package_managers: package_managers.into_iter().collect(),
            build_systems: build_systems.into_iter().collect(),
            lockfiles,
            config_files,
            manifests,
            ci: ci.into_iter().collect(),
            has_docker,
        }
    }

    /// Whether `dir` (a repository-relative path) contains a manifest that
    /// marks it as a project root.
    pub fn contains_manifest(dir: &Path, file_set: &std::collections::HashSet<PathBuf>) -> bool {
        MANIFEST_NAMES
            .iter()
            .any(|name| file_set.contains(&dir.join(name)))
    }
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

    fn scan(root: &Path) -> ScanResult {
        RepositoryScanner::with_defaults().scan(root).unwrap()
    }

    #[test]
    fn detects_package_managers_and_build_systems() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", "{}");
        write(root, "pnpm-lock.yaml", "");
        write(root, "Cargo.toml", "");
        write(root, "Makefile", "all:");
        write(root, ".github/workflows/ci.yml", "name: ci");
        write(root, "Dockerfile", "FROM scratch");
        write(root, ".env.example", "FOO=bar");
        write(root, "tsconfig.json", "{}");

        let meta = MetadataDetector.detect(&scan(root));
        assert!(meta.package_managers.iter().any(|p| p == "pnpm"));
        assert!(meta.package_managers.iter().any(|p| p == "cargo"));
        assert!(!meta.package_managers.iter().any(|p| p == "npm"));
        assert!(meta.build_systems.iter().any(|b| b == "make"));
        assert!(meta.ci.iter().any(|c| c == "github_actions"));
        assert!(meta.has_docker);
        assert!(meta
            .config_files
            .iter()
            .any(|c| c.ends_with("tsconfig.json")));
    }

    #[test]
    fn infers_npm_only_without_js_lockfile() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", "{}");

        let meta = MetadataDetector.detect(&scan(root));
        assert!(meta.package_managers.iter().any(|p| p == "npm"));
        assert_eq!(meta.package_managers, vec!["npm".to_string()]);
    }

    #[test]
    fn detects_nested_ci_configured() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, ".circleci/config.yml", "version: 2");

        let meta = MetadataDetector.detect(&scan(root));
        assert!(meta.ci.iter().any(|c| c == "circleci"));
    }

    #[test]
    fn manifest_lookup_works() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "packages/ui/package.json", "{}");
        let result = scan(root);
        let file_set: std::collections::HashSet<PathBuf> =
            result.files.iter().map(|f| f.rel.clone()).collect();
        assert!(MetadataDetector::contains_manifest(
            Path::new("packages/ui"),
            &file_set
        ));
        assert!(!MetadataDetector::contains_manifest(
            Path::new("packages"),
            &file_set
        ));
    }
}
