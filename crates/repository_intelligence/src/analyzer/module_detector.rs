//! Logical module discovery.
//!
//! Modules are detected from directory structure rather than code semantics:
//!
//! * **Feature folders** — meaningful sub-directories of a project root or of
//!   its `src/` folder (e.g. `auth`, `cart`, `database`).
//! * **Layered architecture** — N-tier directories such as `controllers`,
//!   `routes`, `models`, `services`.
//! * **Monorepo packages** — workspace members living under a nest
//!   (`apps/`, `packages/`, `crates/`, `libs/`) with their own manifest.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use crate::models::{Module, ModuleKind};
use crate::util;

use super::metadata_detector::MetadataDetector;
use super::scanner::ScanResult;

/// Directories that never represent logical modules.
const EXCLUDED_DIRS: &[&str] = &[
    "src",
    "srcs",
    "tests",
    "__tests__",
    "__pycache__",
    "assets",
    "static",
    "public",
    "media",
    "styles",
    "css",
    "images",
    "img",
    "fonts",
    ".github",
    "scripts",
    "docs",
    "config",
    "types",
    "generated",
    "migrations",
    "mocks",
    "fixtures",
    "dev",
    "playground",
    "examples",
];

/// Layered-architecture directory names classified as `Layer` modules.
const LAYER_DIRS: &[&str] = &[
    "controllers",
    "routes",
    "routers",
    "models",
    "views",
    "services",
    "handlers",
    "middlewares",
    "middleware",
    "dtos",
    "dto",
    "schemas",
    "repositories",
    "repos",
    "daos",
    "serializers",
    "serializer",
    "interfaces",
    "graphql",
    "resolvers",
    "queries",
    "mutations",
];

/// Directory names that group monorepo packages.
const NEST_DIRS: &[&str] = &["apps", "packages", "crates", "libs", "modules"];

/// Detects logical modules from a scan.
#[derive(Debug, Clone, Copy)]
pub struct ModuleDetector {
    /// Minimum number of files a directory must contain to be considered a
    /// module.
    pub min_files: usize,
}

impl Default for ModuleDetector {
    fn default() -> Self {
        ModuleDetector { min_files: 1 }
    }
}

impl ModuleDetector {
    /// Detect modules. Results are sorted by module root for determinism.
    pub fn detect(&self, scan: &ScanResult) -> Vec<Module> {
        let mut ctx = ModuleContext {
            scan,
            file_set: scan.files.iter().map(|f| f.rel.clone()).collect(),
            parent_map: build_parent_map(&scan.directories),
            min_files: self.min_files,
            modules: Vec::new(),
            visited: HashSet::new(),
        };

        let mut roots = scan.project_roots.clone();
        roots.sort();
        for root in &roots {
            let mut children = ctx.children_of(root);
            children.sort();
            for child in children {
                ctx.emit_candidate(&child);
            }
        }

        ctx.modules.sort_by(|a, b| a.root.cmp(&b.root));
        ctx.modules
    }
}

/// Mutable traversal state shared across module detection.
struct ModuleContext<'a> {
    scan: &'a ScanResult,
    file_set: HashSet<PathBuf>,
    parent_map: HashMap<PathBuf, Vec<PathBuf>>,
    min_files: usize,
    modules: Vec<Module>,
    visited: HashSet<PathBuf>,
}

impl ModuleContext<'_> {
    fn children_of(&self, dir: &Path) -> Vec<PathBuf> {
        self.parent_map.get(dir).cloned().unwrap_or_default()
    }

    fn emit_candidate(&mut self, dir: &Path) {
        let Some(name) = dir.file_name().and_then(|n| n.to_str()) else {
            return;
        };
        if name.starts_with('.') {
            return;
        }

        // `src`/`app` style roots: their children are the modules.
        if name == "src" || name == "app" {
            let mut inner = self.children_of(dir);
            inner.sort();
            for sub in inner {
                let Some(sub_name) = sub.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if sub_name.starts_with('.') || EXCLUDED_DIRS.contains(&sub_name) {
                    continue;
                }
                if !self.visited.insert(sub.clone()) {
                    continue;
                }
                let kind = self.classify_rooted(&sub);
                self.push_module(&sub, kind);
            }
            return;
        }

        if EXCLUDED_DIRS.contains(&name) {
            return;
        }

        // Monorepo nest: each child is a module.
        if NEST_DIRS.contains(&name) {
            let mut members = self.children_of(dir);
            members.sort();
            for member in members {
                if !self.visited.insert(member.clone()) {
                    continue;
                }
                let kind = if is_project_root(&member, &self.file_set) {
                    ModuleKind::Package
                } else {
                    ModuleKind::Feature
                };
                self.push_module(&member, kind);
            }
            return;
        }

        if !self.visited.insert(dir.to_path_buf()) {
            return;
        }
        let kind = self.classify_rooted(dir);
        self.push_module(dir, kind);

        // Recurse one level for grouped feature folders so that
        // `auth/login`, `auth/session` style layouts surface.
        if kind == ModuleKind::Feature {
            let mut inner = self.children_of(dir);
            inner.sort();
            for sub in inner {
                let Some(sub_name) = sub.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if sub_name.starts_with('.') || EXCLUDED_DIRS.contains(&sub_name) {
                    continue;
                }
                if self.visited.contains(&sub) {
                    continue;
                }
                let sub_kind = if is_project_root(&sub, &self.file_set) {
                    ModuleKind::Package
                } else if LAYER_DIRS.contains(&sub_name) {
                    ModuleKind::Layer
                } else {
                    ModuleKind::Feature
                };
                self.push_module(&sub, sub_kind);
            }
        }
    }

    fn classify_rooted(&self, dir: &Path) -> ModuleKind {
        let name = dir.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if is_project_root(dir, &self.file_set) {
            ModuleKind::Package
        } else if LAYER_DIRS.contains(&name) {
            ModuleKind::Layer
        } else {
            ModuleKind::Feature
        }
    }

    fn push_module(&mut self, dir: &Path, kind: ModuleKind) {
        let files: Vec<String> = self
            .scan
            .files
            .iter()
            .filter(|f| f.rel.starts_with(dir))
            .map(|f| util::forward_slashes(&f.rel))
            .collect();
        if files.len() < self.min_files {
            return;
        }
        self.modules.push(Module {
            name: dir
                .file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string(),
            kind,
            root: util::forward_slashes(dir),
            files,
        });
    }
}

fn is_project_root(dir: &Path, file_set: &HashSet<PathBuf>) -> bool {
    MetadataDetector::contains_manifest(dir, file_set)
}

fn build_parent_map(directories: &[PathBuf]) -> HashMap<PathBuf, Vec<PathBuf>> {
    let mut map: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new();
    for d in directories {
        if let Some(parent) = d.parent() {
            map.entry(parent.to_path_buf()).or_default().push(d.clone());
        }
    }
    map
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

    fn detect(root: &Path) -> Vec<Module> {
        let scan = RepositoryScanner::with_defaults().scan(root).unwrap();
        ModuleDetector::default().detect(&scan)
    }

    #[test]
    fn feature_folders() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/auth/mod.rs", "pub mod login;");
        write(root, "src/auth/login.rs", "pub fn login() {}");
        write(root, "src/database/db.rs", "pub fn connect() {}");
        write(root, "src/main.rs", "mod auth;");

        let modules = detect(root);
        assert_eq!(modules.len(), 2);
        assert_eq!(modules[0].name, "auth");
        assert_eq!(modules[0].kind, ModuleKind::Feature);
        assert_eq!(modules[0].root, "src/auth");
        assert_eq!(modules[0].files.len(), 2);
        assert_eq!(modules[1].name, "database");
    }

    #[test]
    fn layered_architecture() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/controllers/users.ts", "export {}");
        write(root, "src/models/user.ts", "export {}");
        write(root, "src/routes/index.ts", "export {}");
        write(root, "src/services/auth.ts", "export {}");

        let modules = detect(root);
        let kinds: Vec<ModuleKind> = modules.iter().map(|m| m.kind).collect();
        assert_eq!(
            kinds,
            vec![
                ModuleKind::Layer,
                ModuleKind::Layer,
                ModuleKind::Layer,
                ModuleKind::Layer
            ]
        );
    }

    #[test]
    fn monorepo_packages() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", "{}");
        write(root, "apps/web/package.json", "{}");
        write(root, "apps/web/src/index.ts", "export {}");
        write(root, "apps/api/package.json", "{}");
        write(root, "apps/api/src/main.ts", "export {}");
        write(root, "packages/ui/package.json", "{}");
        write(root, "packages/ui/src/button.tsx", "export {}");

        let modules = detect(root);
        let names: Vec<&str> = modules.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, vec!["api", "web", "ui"]);
        assert!(modules.iter().all(|m| m.kind == ModuleKind::Package));
    }

    #[test]
    fn empty_directories_not_modules() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/main.rs", "fn main(){}");
        std::fs::create_dir_all(root.join("src/empty")).unwrap();

        let modules = detect(root);
        assert!(modules.is_empty());
    }

    #[test]
    fn media_asset_folder_not_a_module() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/main.ts", "console.log('x')");
        write(root, "media/icon.svg", "<svg/>");
        write(root, "media/logo.png", "png");
        let modules = detect(root);
        assert!(modules.iter().all(|m| m.name != "media"));
    }

    #[test]
    fn min_files_threshold() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/auth/mod.rs", "pub mod x;");
        let scan = RepositoryScanner::with_defaults().scan(root).unwrap();
        let modules = ModuleDetector { min_files: 2 }.detect(&scan);
        assert!(modules.is_empty());
    }
}
