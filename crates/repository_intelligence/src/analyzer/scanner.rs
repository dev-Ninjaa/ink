//! Filesystem scanner: recursive, gitignore-aware, parallel repository
//! traversal with project-root detection.
//!
//! Two interchangeable backends are provided:
//!
//! * [`ScanBackend::Ignore`] — default; uses the `ignore` crate so `.gitignore`
//!   rules are honoured, and parallelises entry processing with rayon.
//! * [`ScanBackend::Walkdir`] — deterministic sequential traversal built on
//!   `walkdir`, useful for maximal determinism or platforms without rayon.
//!
//! Both produce identical [`ScanResult`]s (output is sorted by path so it is
//! deterministic regardless of backend).

use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use rayon::prelude::*;

use crate::error::{Error, Result};
use crate::util;

use super::metadata_detector::MANIFEST_NAMES;

/// Which traversal engine to use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScanBackend {
    /// `ignore`-based traversal with `.gitignore` support and parallel
    /// entry processing.
    Ignore,
    /// Sequential `walkdir` traversal.
    ///
    /// `walkdir` cannot apply `.gitignore` rules itself, so when
    /// [`ScanOptions::respect_gitignore`] is enabled this backend delegates to
    /// the `ignore` crate's sequential walker to keep results identical to the
    /// [`Ignore`](Self::Ignore) backend. Pure `walkdir` traversal is only used
    /// when gitignore respect is disabled.
    Walkdir,
}

/// Tunable knobs for repository traversal.
#[derive(Debug, Clone)]
pub struct ScanOptions {
    /// Directory names (case-sensitive) that are never descended into.
    pub ignored_dirs: BTreeSet<String>,
    /// Maximum traversal depth (relative to the root). `None` means unbounded.
    pub max_depth: Option<usize>,
    /// Whether symlinks should be followed.
    pub follow_links: bool,
    /// Whether `.gitignore` / `.ignore` rules should be applied.
    pub respect_gitignore: bool,
    /// Whether to parallelise entry processing with rayon.
    pub parallel: bool,
    /// Whether hidden files/directories should be scanned.
    pub include_hidden: bool,
    /// Traversal backend.
    pub backend: ScanBackend,
}

impl Default for ScanOptions {
    fn default() -> Self {
        ScanOptions {
            ignored_dirs: default_ignored_dirs(),
            max_depth: None,
            follow_links: false,
            respect_gitignore: true,
            parallel: true,
            include_hidden: true,
            backend: ScanBackend::Ignore,
        }
    }
}

/// Directories that are always pruned from traversal, regardless of any
/// `.gitignore`, because they are either VCS metadata or build/artifact
/// output and would skew language/module detection.
pub fn default_ignored_dirs() -> BTreeSet<String> {
    [
        ".git",
        ".hg",
        ".svn",
        ".bzr",
        "node_modules",
        "bower_components",
        "target",
        "dist",
        "build",
        "out",
        ".next",
        ".nuxt",
        ".output",
        ".cache",
        ".zig-cache",
        ".turbo",
        ".vercel",
        ".parcel-cache",
        ".pnpm-store",
        ".venv",
        "venv",
        ".tox",
        ".nox",
        "__pycache__",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        ".hypothesis",
        ".idea",
        ".vscode",
        ".gradle",
        ".cargo",
        ".stack-work",
        ".dart_tool",
        ".terraform",
        "coverage",
        "Pods",
        ".build",
        "DerivedData",
        "CMakeFiles",
        ".expo",
        ".docusaurus",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

/// A file discovered during scanning.
#[derive(Debug, Clone)]
pub struct ScannedFile {
    /// Absolute path on disk.
    pub path: PathBuf,
    /// Repository-relative path.
    pub rel: PathBuf,
    /// Size in bytes.
    pub size: u64,
}

/// Aggregate counters for a scan.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScanStats {
    /// Number of files.
    pub files: u64,
    /// Number of directories (excluding the root).
    pub directories: u64,
    /// Total bytes of all files.
    pub bytes: u64,
    /// Number of entries that could not be read (permissions, races, ...).
    pub scan_errors: u64,
}

/// Result of one repository scan.
#[derive(Debug, Clone)]
pub struct ScanResult {
    /// Absolute root path.
    pub root: PathBuf,
    /// Discovered files (sorted by relative path).
    pub files: Vec<ScannedFile>,
    /// Discovered directories, relative to root, excluding the root itself
    /// (sorted).
    pub directories: Vec<PathBuf>,
    /// Project roots (directories containing a manifest) relative to root,
    /// including the root itself (sorted).
    pub project_roots: Vec<PathBuf>,
    /// Aggregate counters.
    pub stats: ScanStats,
    /// Wall-clock traversal time in nanoseconds.
    pub duration_ns: u64,
}

/// The repository scanner.
#[derive(Debug, Clone)]
pub struct RepositoryScanner {
    options: ScanOptions,
}

impl RepositoryScanner {
    /// Create a scanner with the given options.
    pub fn new(options: ScanOptions) -> Self {
        RepositoryScanner { options }
    }

    /// Create a scanner with default options.
    pub fn with_defaults() -> Self {
        RepositoryScanner {
            options: ScanOptions::default(),
        }
    }

    /// Access the effective scan options.
    pub fn options(&self) -> &ScanOptions {
        &self.options
    }

    /// Scan a repository rooted at `root`.
    pub fn scan(&self, root: impl AsRef<Path>) -> Result<ScanResult> {
        let root = root.as_ref();
        if !root.is_dir() {
            return Err(Error::InvalidRoot(root.to_path_buf()));
        }

        let started = Instant::now();
        let (raw, scan_errors) = match self.options.backend {
            ScanBackend::Ignore => self.collect_ignore(root),
            // Walkdir cannot apply `.gitignore` rules by itself; when gitignore
            // respect is requested, delegate to the `ignore` crate's sequential
            // walker so results stay identical to the Ignore backend.
            ScanBackend::Walkdir if self.options.respect_gitignore => self.collect_ignore_seq(root),
            ScanBackend::Walkdir => self.collect_walkdir(root),
        };
        let duration_ns = started.elapsed().as_nanos() as u64;

        Ok(post_process(root, raw, scan_errors, duration_ns))
    }

    fn collect_ignore(&self, root: &Path) -> (Vec<RawEntry>, u64) {
        self.collect_ignore_with(root, self.options.parallel)
    }

    fn collect_ignore_seq(&self, root: &Path) -> (Vec<RawEntry>, u64) {
        self.collect_ignore_with(root, false)
    }

    fn collect_ignore_with(&self, root: &Path, parallel: bool) -> (Vec<RawEntry>, u64) {
        let mut builder = ignore::WalkBuilder::new(root);
        builder
            .follow_links(self.options.follow_links)
            .hidden(!self.options.include_hidden)
            .git_ignore(self.options.respect_gitignore)
            .git_exclude(self.options.respect_gitignore)
            .git_global(self.options.respect_gitignore)
            .ignore(self.options.respect_gitignore)
            .require_git(false);
        if let Some(depth) = self.options.max_depth {
            builder.max_depth(Some(depth));
        }

        let ignored = self.options.ignored_dirs.clone();
        builder.filter_entry(move |entry| {
            entry.depth() == 0 || !is_ignored_name(entry.file_name(), &ignored)
        });
        let walker = builder.build();

        let errors = AtomicU64::new(0);
        let entries: Vec<RawEntry> = if parallel {
            walker
                .par_bridge()
                .filter_map(|entry| to_raw(entry, &errors))
                .collect()
        } else {
            walker.filter_map(|entry| to_raw(entry, &errors)).collect()
        };
        (entries, errors.load(Ordering::Relaxed))
    }

    fn collect_walkdir(&self, root: &Path) -> (Vec<RawEntry>, u64) {
        let max_depth = self.options.max_depth.unwrap_or(usize::MAX);
        let walker = walkdir::WalkDir::new(root)
            .follow_links(self.options.follow_links)
            .min_depth(0)
            .max_depth(max_depth);

        let ignored = self.options.ignored_dirs.clone();
        let mut entries = Vec::new();
        let mut errors = 0u64;

        for entry in walker.into_iter().filter_entry(move |entry| {
            entry.depth() == 0 || !is_ignored_name(entry.file_name(), &ignored)
        }) {
            match entry {
                Ok(d) => {
                    if !self.options.include_hidden && d.depth() > 0 {
                        if let Some(name) = d.file_name().to_str() {
                            if name.starts_with('.') {
                                continue;
                            }
                        }
                    }
                    let file_type = d.file_type();
                    let is_dir = file_type.is_dir();
                    let is_file = file_type.is_file();
                    if !is_dir && !is_file {
                        continue;
                    }
                    let size = if is_dir {
                        0
                    } else {
                        d.metadata().map(|m| m.len()).unwrap_or(0)
                    };
                    entries.push(RawEntry {
                        path: d.path().to_path_buf(),
                        is_dir,
                        size,
                    });
                }
                Err(_) => errors += 1,
            }
        }
        (entries, errors)
    }
}

fn is_ignored_name(name: &std::ffi::OsStr, ignored: &BTreeSet<String>) -> bool {
    name.to_str().is_some_and(|n| ignored.contains(n))
}

/// A raw, unclassified filesystem entry from a backend.
#[derive(Debug, Clone)]
struct RawEntry {
    path: PathBuf,
    is_dir: bool,
    size: u64,
}

fn to_raw(
    entry: std::result::Result<ignore::DirEntry, ignore::Error>,
    errors: &AtomicU64,
) -> Option<RawEntry> {
    match entry {
        Ok(d) => {
            let file_type = d.file_type()?;
            let is_dir = file_type.is_dir();
            let is_file = file_type.is_file();
            if !is_dir && !is_file {
                return None;
            }
            let size = if is_dir {
                0
            } else {
                d.metadata().map(|m| m.len()).unwrap_or(0)
            };
            Some(RawEntry {
                path: d.path().to_path_buf(),
                is_dir,
                size,
            })
        }
        Err(_) => {
            errors.fetch_add(1, Ordering::Relaxed);
            None
        }
    }
}

fn post_process(root: &Path, raw: Vec<RawEntry>, scan_errors: u64, duration_ns: u64) -> ScanResult {
    let mut files: Vec<ScannedFile> = Vec::with_capacity(raw.len());
    let mut directories: Vec<PathBuf> = Vec::new();
    let mut file_set: HashSet<PathBuf> = HashSet::with_capacity(raw.len());

    let mut bytes = 0u64;
    let mut dir_count = 0u64;

    for entry in raw {
        if entry.is_dir {
            let rel = rel_of(root, &entry.path);
            directories.push(rel);
            dir_count += 1;
        } else {
            let rel = rel_of(root, &entry.path);
            bytes += entry.size;
            file_set.insert(rel.clone());
            files.push(ScannedFile {
                path: entry.path,
                rel,
                size: entry.size,
            });
        }
    }

    drop(file_set);
    files.sort_by(|a, b| a.rel.cmp(&b.rel));
    directories.sort();

    let mut project_roots = BTreeSet::new();
    project_roots.insert(PathBuf::new());
    for file in &files {
        if MANIFEST_NAMES
            .iter()
            .any(|name| file.rel.file_name().is_some_and(|n| n == *name))
        {
            let parent = file.rel.parent().unwrap_or(Path::new(""));
            project_roots.insert(util::normalize_lexical(parent));
        }
    }

    let stats = ScanStats {
        files: files.len() as u64,
        directories: dir_count,
        bytes,
        scan_errors,
    };

    ScanResult {
        root: root.to_path_buf(),
        files,
        directories,
        project_roots: project_roots.into_iter().collect(),
        stats,
        duration_ns,
    }
}

fn rel_of(root: &Path, path: &Path) -> PathBuf {
    util::normalize_lexical(path.strip_prefix(root).unwrap_or(path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write(root: &Path, rel: &str, content: &str) {
        let p = root.join(rel);
        if let Some(parent) = p.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(p, content).unwrap();
    }

    #[test]
    fn scans_basic_layout() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/main.rs", "fn main() {}");
        write(root, "src/lib.rs", "pub fn f() {}");
        write(root, "README.md", "# hi");
        write(root, "src/nested/deep/file.ts", "export const x = 1;");
        write(root, "src/sub/keep/.env.example", "TOKEN=abc");

        for backend in [ScanBackend::Ignore, ScanBackend::Walkdir] {
            let opts = ScanOptions {
                backend,
                ..ScanOptions::default()
            };
            let result = RepositoryScanner::new(opts).scan(root).unwrap();

            assert_eq!(result.stats.files, 5);
            assert!(result.directories.contains(&PathBuf::from("src")));
            assert!(result
                .directories
                .contains(&PathBuf::from("src/nested/deep")));
            assert_eq!(result.project_roots, vec![PathBuf::new()]);
            let paths: Vec<_> = result.files.iter().map(|f| f.rel.clone()).collect();
            let mut sorted = paths.clone();
            sorted.sort();
            assert_eq!(paths, sorted);
        }
    }

    #[test]
    fn counts_bytes() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "a.txt", "abcde");
        write(root, "b.txt", "xy");
        let result = RepositoryScanner::with_defaults().scan(root).unwrap();
        assert_eq!(result.stats.bytes, 7);
    }

    #[test]
    fn skips_ignored_directories() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/main.rs", "fn main() {}");
        write(root, "node_modules/pkg/index.js", "x");
        write(root, "target/debug/app", "x");
        write(root, ".git/HEAD", "ref");

        for backend in [ScanBackend::Ignore, ScanBackend::Walkdir] {
            let opts = ScanOptions {
                backend,
                ..ScanOptions::default()
            };
            let result = RepositoryScanner::new(opts).scan(root).unwrap();
            assert_eq!(result.stats.files, 1);
            assert_eq!(result.files[0].rel, PathBuf::from("src/main.rs"));
        }
    }

    #[test]
    fn detects_project_roots_from_manifests() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", "{}");
        write(root, "apps/web/package.json", "{}");
        write(root, "packages/ui/Cargo.toml", "[package]");
        let result = RepositoryScanner::with_defaults().scan(root).unwrap();
        assert_eq!(
            result.project_roots,
            vec![
                PathBuf::new(),
                PathBuf::from("apps/web"),
                PathBuf::from("packages/ui"),
            ]
        );
    }

    #[test]
    fn respects_gitignore_when_enabled() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, ".gitignore", "ignored_file.ts\n");
        write(root, "kept.ts", "export const kept = 1;");
        write(root, "ignored_file.ts", "export const gone = 1;");

        for backend in [ScanBackend::Ignore, ScanBackend::Walkdir] {
            for respect in [true, false] {
                let opts = ScanOptions {
                    respect_gitignore: respect,
                    include_hidden: true,
                    backend,
                    ..ScanOptions::default()
                };
                let result = RepositoryScanner::new(opts).scan(root).unwrap();
                let names: Vec<_> = result.files.iter().map(|f| f.rel.clone()).collect();
                assert_eq!(
                    names.contains(&PathBuf::from("ignored_file.ts")),
                    !respect,
                    "backend={backend:?} respect={respect}"
                );
            }
        }
    }

    #[test]
    fn gitignore_parity_between_backends() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, ".gitignore", ".hidden-out/\n");
        write(root, "src/main.rs", "fn main(){}");
        write(root, ".hidden-out/a.ts", "export const x = 1;");
        write(root, ".hidden-out/b.ts", "export const y = 1;");

        let mut opts = ScanOptions::default();
        let ignore = RepositoryScanner::new(opts.clone()).scan(root).unwrap();
        opts.backend = ScanBackend::Walkdir;
        let walkdir = RepositoryScanner::new(opts).scan(root).unwrap();

        assert_eq!(ignore.stats.files, walkdir.stats.files);
        let a: Vec<_> = ignore.files.iter().map(|f| f.rel.clone()).collect();
        let b: Vec<_> = walkdir.files.iter().map(|f| f.rel.clone()).collect();
        assert_eq!(a, b);
    }

    #[test]
    fn skips_zig_cache_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/main.rs", "fn main(){}");
        write(root, ".zig-cache/o/deps.zig", "pub fn d(){}");
        for backend in [ScanBackend::Ignore, ScanBackend::Walkdir] {
            let opts = ScanOptions {
                backend,
                ..ScanOptions::default()
            };
            let result = RepositoryScanner::new(opts).scan(root).unwrap();
            assert_eq!(result.stats.files, 1);
            assert_eq!(result.files[0].rel, PathBuf::from("src/main.rs"));
        }
    }

    #[test]
    fn rejects_missing_root() {
        let err = RepositoryScanner::with_defaults()
            .scan(Path::new("definitely/not/here"))
            .unwrap_err();
        assert!(matches!(err, Error::InvalidRoot(_)));
    }
}
