//! Import/export/module-reference extraction without a full parser.
//!
//! We deliberately avoid tree-sitter: for the phase-1 feature set a small set
//! of robust regular expressions over source lines is fast, dependency-free
//! and "good enough" to build a precise repository graph later. Every
//! extractor is layered so a future tree-sitter backend can be dropped in
//! without touching the [`Relationship`] contract.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use rayon::prelude::*;

use crate::models::{Language, Relationship, RelationshipKind};
use crate::util;

use super::language_detector::LanguageDetector;
use super::scanner::ScanResult;

fn js_import_from() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r#"\bimport\s+(?:[^;"'\n]*?\sfrom\s+)?["']([^"']+)["']"#).unwrap()
    })
}

fn js_export_from() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r#"\bexport\s+[^;"'\n]*?\sfrom\s+["']([^"']+)["']"#).unwrap()
    })
}

fn js_dynamic() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r#"\b(?:import|require)\s*\(\s*["']([^"']+)["']"#).unwrap())
}

fn rust_use() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?m)^\s*use\s+([^;]+);").unwrap())
}

fn rust_mod_decl() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| {
        regex::Regex::new(r"(?m)^\s*(?:pub\s*(?:\([^)]*\))?\s+)?mod\s+([a-zA-Z_][a-zA-Z0-9_]*)\s*;")
            .unwrap()
    })
}

fn py_import() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?m)^\s*import\s+([^\n#]+)").unwrap())
}

fn py_from_import() -> &'static regex::Regex {
    static RE: OnceLock<regex::Regex> = OnceLock::new();
    RE.get_or_init(|| regex::Regex::new(r"(?m)^\s*from\s+([.\w]+)\s+import\s+([^\n#]*)").unwrap())
}

/// Extracts file-to-file relationships from a repository scan.
#[derive(Debug, Clone)]
pub struct ImportExtractor {
    max_source_bytes: u64,
    aliases: Vec<(String, String)>,
}

impl Default for ImportExtractor {
    fn default() -> Self {
        ImportExtractor::new(util::DEFAULT_MAX_SOURCE_BYTES, Vec::new())
    }
}

impl ImportExtractor {
    /// Create an extractor with a source-size cap and custom specifier aliases.
    ///
    /// Each alias is a `(prefix, directory)` pair; a specifier starting with
    /// `prefix` is resolved relative to `directory` inside the nearest project
    /// root. The default aliases (`@/`, `@`, `~/`, `~` → `src`) are always
    /// applied and `custom` overrides them.
    pub fn new(max_source_bytes: u64, custom_aliases: Vec<(String, String)>) -> Self {
        let mut aliases: Vec<(String, String)> = vec![
            ("@/".to_string(), "src".to_string()),
            ("@".to_string(), "src".to_string()),
            ("~/".to_string(), "src".to_string()),
            ("~".to_string(), "src".to_string()),
        ];
        for (prefix, directory) in custom_aliases {
            if let Some(slot) = aliases.iter_mut().find(|(p, _)| p == &prefix) {
                *slot = (prefix, directory);
            } else {
                aliases.push((prefix, directory));
            }
        }
        ImportExtractor {
            max_source_bytes,
            aliases,
        }
    }

    /// Extract all relationships from a scan. Output is sorted and de-duplicated.
    pub fn extract(&self, scan: &ScanResult) -> Vec<Relationship> {
        let detector = LanguageDetector;
        let file_set: HashSet<PathBuf> = scan.files.iter().map(|f| f.rel.clone()).collect();
        let roots = &scan.project_roots;

        let mut collected: Vec<Relationship> = scan
            .files
            .par_iter()
            .filter_map(|file| {
                let language = detector.detect(&file.rel)?;
                if !language.supports_import_analysis() {
                    return None;
                }
                if file.size > self.max_source_bytes {
                    return None;
                }
                let content = util::read_text_limited(&file.path, self.max_source_bytes)
                    .ok()
                    .flatten()?;
                let source_dir = file.rel.parent().unwrap_or(Path::new(""));
                let is_mod_file = file.rel.file_name().and_then(|n| n.to_str()) == Some("mod.rs");
                let project_root = nearest_project_root(roots, &file.rel);

                let extracted = extract_for_language(language, &content);
                let mut out = Vec::with_capacity(extracted.len());
                for (kind, specifier) in extracted {
                    let resolved = resolve_specifier(
                        language,
                        &specifier,
                        source_dir,
                        is_mod_file,
                        project_root,
                        &file_set,
                        &self.aliases,
                    );
                    match resolved {
                        Some(target) => out.push(Relationship {
                            source: util::forward_slashes(&file.rel),
                            target: util::forward_slashes(&target),
                            kind,
                            resolved: true,
                        }),
                        None => {
                            if reportable_unresolved(language, &specifier, &self.aliases) {
                                out.push(Relationship {
                                    source: util::forward_slashes(&file.rel),
                                    target: specifier,
                                    kind,
                                    resolved: false,
                                });
                            }
                        }
                    }
                }
                Some(out)
            })
            .collect::<Vec<Vec<Relationship>>>()
            .into_iter()
            .flatten()
            .collect();

        collected.sort_by(|a, b| {
            a.source
                .cmp(&b.source)
                .then_with(|| a.target.cmp(&b.target))
                .then_with(|| a.kind.cmp(&b.kind))
        });
        collected.dedup();
        collected
    }
}

fn nearest_project_root<'a>(roots: &'a [PathBuf], rel: &Path) -> Option<&'a PathBuf> {
    roots
        .iter()
        .filter(|root| rel.starts_with(root))
        .max_by_key(|root| root.components().count())
}

/// Extract raw `(kind, specifier)` pairs for a file's content.
fn extract_for_language(language: Language, content: &str) -> Vec<(RelationshipKind, String)> {
    match language {
        Language::Rust => extract_rust(content),
        Language::TypeScript | Language::JavaScript => extract_javascript(content),
        Language::Python => extract_python(content),
        _ => Vec::new(),
    }
}

fn extract_rust(content: &str) -> Vec<(RelationshipKind, String)> {
    let mut out = Vec::new();
    let mut seen: HashSet<(RelationshipKind, String)> = HashSet::new();

    for captures in rust_use().captures_iter(content) {
        let body = &captures[1];
        for path in expand_braces(body) {
            let path = path
                .split(" as ")
                .next()
                .unwrap_or(&path)
                .trim()
                .trim_end_matches('*');
            let path = path.trim();
            if path.is_empty() {
                continue;
            }
            if path == "self" || path.ends_with("::self") {
                continue;
            }
            let path = path.strip_suffix("::self").unwrap_or(path).trim();
            let spec = path.to_string();
            if seen.insert((RelationshipKind::Import, spec.clone())) {
                out.push((RelationshipKind::Import, spec));
            }
        }
    }

    for captures in rust_mod_decl().captures_iter(content) {
        let spec = captures[1].to_string();
        if seen.insert((RelationshipKind::ModuleReference, spec.clone())) {
            out.push((RelationshipKind::ModuleReference, spec));
        }
    }
    out
}

fn extract_javascript(content: &str) -> Vec<(RelationshipKind, String)> {
    let mut out = Vec::new();
    let mut seen: HashSet<(RelationshipKind, String)> = HashSet::new();

    for captures in js_export_from().captures_iter(content) {
        let spec = captures[1].to_string();
        if seen.insert((RelationshipKind::Export, spec.clone())) {
            out.push((RelationshipKind::Export, spec));
        }
    }
    for captures in js_import_from().captures_iter(content) {
        let spec = captures[1].to_string();
        if seen.insert((RelationshipKind::Import, spec.clone())) {
            out.push((RelationshipKind::Import, spec));
        }
    }
    for captures in js_dynamic().captures_iter(content) {
        let spec = captures[1].to_string();
        if seen.insert((RelationshipKind::Import, spec.clone())) {
            out.push((RelationshipKind::Import, spec));
        }
    }
    out
}

/// A parsed Python import. `level` is the number of leading dots; `module` is
/// the dotted path after the dots (may be empty, e.g. `from . import x`).
struct PyImport {
    level: usize,
    module: Option<String>,
}

fn parse_py_specifier(spec: &str) -> PyImport {
    let trimmed = spec.trim();
    let dots = trimmed.chars().take_while(|c| *c == '.').count();
    let module = trimmed[dots..].trim();
    PyImport {
        level: dots,
        module: if module.is_empty() {
            None
        } else {
            Some(module.to_string())
        },
    }
}

fn extract_python(content: &str) -> Vec<(RelationshipKind, String)> {
    let mut out = Vec::new();
    let mut seen: HashSet<(RelationshipKind, String)> = HashSet::new();

    for captures in py_from_import().captures_iter(content) {
        let spec = captures[1].trim();
        let parsed = parse_py_specifier(spec);
        let names: Vec<&str> = captures[2]
            .split(',')
            .map(|s| s.split(" as ").next().unwrap_or(s).trim())
            .filter(|s| !s.is_empty() && *s != "*")
            .collect();

        if parsed.module.is_none() {
            // `from . import x, y` — each imported name is a potential
            // sibling submodule.
            for name in names {
                let key = format!("{}{}", ".".repeat(parsed.level), name);
                if seen.insert((RelationshipKind::Import, key.clone())) {
                    out.push((RelationshipKind::Import, key));
                }
            }
        } else {
            let spec_full = format!(
                "{}{}",
                ".".repeat(parsed.level),
                parsed.module.as_deref().unwrap_or("")
            );
            if seen.insert((RelationshipKind::Import, spec_full.clone())) {
                out.push((RelationshipKind::Import, spec_full));
            }
        }
    }

    for captures in py_import().captures_iter(content) {
        for token in captures[1].split(',') {
            let token = token.split(" as ").next().unwrap_or(token).trim();
            if token.is_empty() {
                continue;
            }
            let spec = token.to_string();
            if seen.insert((RelationshipKind::Import, spec.clone())) {
                out.push((RelationshipKind::Import, spec));
            }
        }
    }
    out
}

fn resolve_specifier(
    language: Language,
    specifier: &str,
    source_dir: &Path,
    is_mod_file: bool,
    project_root: Option<&PathBuf>,
    file_set: &HashSet<PathBuf>,
    aliases: &[(String, String)],
) -> Option<PathBuf> {
    match language {
        Language::Rust => resolve_rust(specifier, source_dir, is_mod_file, project_root, file_set),
        Language::TypeScript | Language::JavaScript => {
            resolve_javascript(specifier, source_dir, project_root, file_set, aliases)
        }
        Language::Python => resolve_python(specifier, source_dir, project_root, file_set),
        _ => None,
    }
}

/// Whether a specifier is an npm-style scoped package (`@scope/name`).
///
/// Scoped packages are external dependencies, not internal aliases. The `@`
/// alias prefix must only apply to `@/...` or single-segment alias forms, so
/// `@vscode/test-electron` is never mistaken for `src/vscode/test-electron`.
fn is_scoped_package(specifier: &str) -> bool {
    let Some(rest) = specifier.strip_prefix('@') else {
        return false;
    };
    let Some(slash) = rest.find('/') else {
        return false;
    };
    !rest[..slash].is_empty()
}

/// Whether an unresolved specifier is still worth reporting. We only report
/// genuinely internal attempts (relative paths, aliases, `crate`/`super`
/// paths, `mod` declarations) and never bare external dependencies.
fn reportable_unresolved(
    language: Language,
    specifier: &str,
    aliases: &[(String, String)],
) -> bool {
    match language {
        Language::Rust => specifier.starts_with("crate::") || specifier.starts_with("super::"),
        Language::TypeScript | Language::JavaScript => {
            !is_scoped_package(specifier)
                && (specifier.starts_with("./")
                    || specifier.starts_with("../")
                    || aliases
                        .iter()
                        .any(|(prefix, _)| specifier.starts_with(prefix)))
        }
        Language::Python => specifier.starts_with('.') && specifier.len() > 1,
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Rust resolution
// ---------------------------------------------------------------------------

fn resolve_rust(
    specifier: &str,
    source_dir: &Path,
    is_mod_file: bool,
    project_root: Option<&PathBuf>,
    file_set: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let mut spec = specifier;
    let mut base = source_dir.to_path_buf();

    if let Some(rest) = spec.strip_prefix("crate::") {
        base = rust_crate_root(source_dir, project_root);
        spec = rest;
    } else {
        let mut sup = 0usize;
        while let Some(rest) = spec.strip_prefix("super::") {
            sup += 1;
            spec = rest;
        }
        // `super` in a leaf module file (`src/core/cache.rs`) refers to the
        // containing directory (`src/core`), so the first `super::` does not
        // pop. In a `mod.rs` it refers to the parent of the enclosing
        // directory, so each `super::` pops one level.
        let pops = if is_mod_file {
            sup
        } else {
            sup.saturating_sub(1)
        };
        for _ in 0..pops {
            let parent = base.parent()?;
            base = parent.to_path_buf();
        }
    }

    rust_module_candidates(&base, spec, file_set)
}

/// Determine the crate root directory (the module namespace for `crate::`)
/// for a Rust source file. For standard Cargo layouts this is
/// `<project_root>/src`; otherwise the project root itself.
fn rust_crate_root(source_dir: &Path, project_root: Option<&PathBuf>) -> PathBuf {
    let project_root = project_root.cloned().unwrap_or_default();
    let src_dir = project_root.join("src");
    if source_dir.starts_with(&src_dir) || source_dir == src_dir {
        src_dir
    } else {
        project_root
    }
}

fn rust_module_candidates(
    base: &Path,
    dotted: &str,
    file_set: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let segments: Vec<&str> = dotted.split("::").filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return None;
    }
    // `use crate::models::User` — the trailing segments are item names, not
    // module paths. Try the longest module prefix that actually resolves to a
    // file: `models::User` → `models`.
    for end in (1..=segments.len()).rev() {
        let mut cur = base.to_path_buf();
        for segment in &segments[..end] {
            cur.push(segment);
        }
        let file = cur.with_extension("rs");
        if file_set.contains(&file) {
            return Some(file);
        }
        let module_dir = cur.join("mod.rs");
        if file_set.contains(&module_dir) {
            return Some(module_dir);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// JavaScript / TypeScript resolution
// ---------------------------------------------------------------------------

fn resolve_javascript(
    specifier: &str,
    source_dir: &Path,
    project_root: Option<&PathBuf>,
    file_set: &HashSet<PathBuf>,
    aliases: &[(String, String)],
) -> Option<PathBuf> {
    // Scoped npm packages (`@scope/name`) are external dependencies; never
    // Scoped npm packages (`@scope/name`) are external dependencies. The bare
    // `@` default alias must not apply to them, but a more specific configured
    // alias (e.g. `@components` -> `src/components`) is still honoured.
    let is_scoped = is_scoped_package(specifier);
    let alias_match = aliases.iter().find(|(prefix, _)| {
        specifier.starts_with(prefix.as_str()) && !(is_scoped && prefix.as_str() == "@")
    });
    let base = if specifier.starts_with("./") || specifier.starts_with("../") {
        util::normalize_lexical(&source_dir.join(specifier))
    } else if specifier.starts_with('/')
        || specifier.contains(':')
        || !specifier.contains('.') && !specifier.contains('/') && !specifier.starts_with('@')
    {
        // absolute path, protocol, or bare module — not internal.
        if let Some((prefix, directory)) = aliases
            .iter()
            .find(|(prefix, _)| specifier.starts_with(prefix.as_str()))
        {
            let rest = &specifier[prefix.len()..];
            let rest = rest.strip_prefix('/').unwrap_or(rest);
            util::normalize_lexical(
                &project_root
                    .cloned()
                    .unwrap_or_default()
                    .join(directory)
                    .join(rest),
            )
        } else {
            return None;
        }
    } else if let Some((prefix, directory)) = alias_match {
        let rest = &specifier[prefix.len()..];
        let rest = rest.strip_prefix('/').unwrap_or(rest);
        util::normalize_lexical(
            &project_root
                .cloned()
                .unwrap_or_default()
                .join(directory)
                .join(rest),
        )
    } else {
        // scoped package or bare import — not internal.
        return None;
    };

    js_module_candidates(&base, file_set)
}

fn js_module_candidates(base: &Path, file_set: &HashSet<PathBuf>) -> Option<PathBuf> {
    const EXTENSIONS: [&str; 6] = [".ts", ".tsx", ".js", ".jsx", ".mjs", ".cjs"];
    if file_set.contains(base) {
        return Some(base.to_path_buf());
    }
    for extension in EXTENSIONS {
        let candidate = base.with_extension(extension.trim_start_matches('.'));
        if file_set.contains(&candidate) {
            return Some(candidate);
        }
    }
    for extension in EXTENSIONS {
        let candidate = base.join(format!("index{extension}"));
        if file_set.contains(&candidate) {
            return Some(candidate);
        }
    }
    None
}

// ---------------------------------------------------------------------------
// Python resolution
// ---------------------------------------------------------------------------

fn resolve_python(
    specifier: &str,
    source_dir: &Path,
    project_root: Option<&PathBuf>,
    file_set: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let parsed = parse_py_specifier(specifier);

    if parsed.level > 0 {
        // Relative import: climb `level - 1` package boundaries from the
        // package base directory (the nearest ancestor with `__init__.py`).
        let base = python_package_dir(source_dir, file_set);
        let mut dir = base;
        for _ in 1..parsed.level {
            let parent = dir.parent()?;
            dir = parent.to_path_buf();
        }
        let module = parsed.module.as_deref().unwrap_or("");
        python_module_candidates(&dir, module, file_set)
    } else {
        // Absolute import: resolve from the project root.
        let root = project_root.cloned().unwrap_or_default();
        python_module_candidates(&root, parsed.module.as_deref().unwrap_or(""), file_set)
    }
}

fn python_package_dir(source_dir: &Path, file_set: &HashSet<PathBuf>) -> PathBuf {
    let mut base = source_dir.to_path_buf();
    while !file_set.contains(&base.join("__init__.py")) {
        let Some(parent) = base.parent() else {
            break;
        };
        if parent == base {
            break;
        }
        base = parent.to_path_buf();
    }
    base
}

fn python_module_candidates(
    dir: &Path,
    dotted: &str,
    file_set: &HashSet<PathBuf>,
) -> Option<PathBuf> {
    let segments: Vec<&str> = dotted.split('.').filter(|s| !s.is_empty()).collect();
    if segments.is_empty() {
        return None;
    }
    let mut cur = dir.to_path_buf();
    for (index, segment) in segments.iter().enumerate() {
        cur.push(segment);
        if index == segments.len() - 1 {
            let module_file = cur.with_extension("py");
            if file_set.contains(&module_file) {
                return Some(module_file);
            }
            let package_init = cur.join("__init__.py");
            if file_set.contains(&package_init) {
                return Some(package_init);
            }
        } else {
            let package_init = cur.join("__init__.py");
            if !file_set.contains(&package_init) {
                return None;
            }
        }
    }
    None
}

/// Expand Rust brace groups: `a::{b, c}` → `["a::b", "a::c"]`.
fn expand_braces(path: &str) -> Vec<String> {
    let mut current = vec![path.trim().to_string()];
    loop {
        let mut next = Vec::new();
        let mut expanded_any = false;
        for item in &current {
            match first_brace_group(item) {
                Some((before, inside, after)) => {
                    expanded_any = true;
                    for part in split_top_level(&inside) {
                        next.push(format!("{before}{part}{after}"));
                    }
                }
                None => next.push(item.clone()),
            }
        }
        current = next;
        if !expanded_any {
            break;
        }
    }
    current
}

fn first_brace_group(s: &str) -> Option<(String, String, String)> {
    let open = s.find('{')?;
    let mut depth = 0usize;
    let mut close = None;
    for (index, ch) in s[open..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                if depth == 0 {
                    break;
                }
                depth -= 1;
                if depth == 0 {
                    close = Some(open + index);
                    break;
                }
            }
            _ => {}
        }
    }
    let close = close?;
    Some((
        s[..open].to_string(),
        s[open + 1..close].to_string(),
        s[close + 1..].to_string(),
    ))
}

fn split_top_level(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (index, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                parts.push(s[start..index].trim().to_string());
                start = index + 1;
            }
            _ => {}
        }
    }
    parts.push(s[start..].trim().to_string());
    parts.into_iter().filter(|p| !p.is_empty()).collect()
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

    fn relationships(root: &Path) -> Vec<Relationship> {
        let scan = RepositoryScanner::with_defaults().scan(root).unwrap();
        ImportExtractor::default().extract(&scan)
    }

    #[test]
    fn relative_typescript_imports() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "src/page.tsx",
            "import Navbar from './components/Navbar';",
        );
        write(
            root,
            "src/components/Navbar.tsx",
            "export default function N(){}",
        );
        write(root, "src/components/Navbar.css", ".nav {}");

        let rels = relationships(root);
        assert!(rels.iter().any(|r| {
            r.source == "src/page.tsx"
                && r.target == "src/components/Navbar.tsx"
                && r.kind == RelationshipKind::Import
                && r.resolved
        }));
    }

    #[test]
    fn alias_imports_resolve_to_project_root() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", "{}");
        write(root, "src/app/page.tsx", "import { api } from '@/lib/api';");
        write(root, "src/lib/api.ts", "export const api = 1;");

        let rels = relationships(root);
        assert!(rels
            .iter()
            .any(|r| { r.source == "src/app/page.tsx" && r.target == "src/lib/api.ts" }));
    }

    #[test]
    fn nested_and_index_imports() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "src/a.ts",
            "import b from './b';\nimport c from './features/index';",
        );
        write(root, "src/b.ts", "export default 1;");
        write(root, "src/features/index.ts", "export default 2;");

        let rels = relationships(root);
        let targets: Vec<_> = rels
            .iter()
            .filter(|r| r.source == "src/a.ts" && r.resolved)
            .map(|r| r.target.as_str())
            .collect();
        assert!(targets.contains(&"src/b.ts"));
        assert!(targets.contains(&"src/features/index.ts"));
    }

    #[test]
    fn javascript_requires_and_exports() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "server.js",
            "const a = require('./lib/a');\nexport { default } from './b';",
        );
        write(root, "lib/a.js", "module.exports = 1;");
        write(root, "b.js", "export default 2;");

        let rels = relationships(root);
        assert!(rels.iter().any(|r| {
            r.source == "server.js" && r.target == "lib/a.js" && r.kind == RelationshipKind::Import
        }));
        assert!(rels.iter().any(|r| {
            r.source == "server.js" && r.target == "b.js" && r.kind == RelationshipKind::Export
        }));
    }

    #[test]
    fn bare_imports_are_skipped_not_reported() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "app.ts",
            "import React from 'react';\nimport { join } from 'node:path';",
        );
        let rels = relationships(root);
        assert!(rels.is_empty());
    }

    #[test]
    fn scoped_packages_are_external_not_aliases() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(
            root,
            "src/app.ts",
            "import { run } from '@vscode/test-electron';\nimport { x } from '@org/lib';",
        );
        write(root, "src/vscode/test-electron.ts", "export const run = 1;");
        let rels = relationships(root);
        // Neither is resolved to `src/vscode/...` nor reported unresolved.
        assert!(rels.is_empty());
    }

    #[test]
    fn scoped_like_custom_alias_still_resolves() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "package.json", "{}");
        write(
            root,
            "src/app.ts",
            "import { button } from '@components/Button';",
        );
        write(root, "src/components/Button.ts", "export const button = 1;");
        let extractor = ImportExtractor::new(
            crate::util::DEFAULT_MAX_SOURCE_BYTES,
            vec![("@components".to_string(), "src/components".to_string())],
        );
        let scan = RepositoryScanner::with_defaults().scan(root).unwrap();
        let rels = extractor.extract(&scan);
        assert!(rels.iter().any(|r| {
            r.source == "src/app.ts" && r.target == "src/components/Button.ts" && r.resolved
        }));
    }

    #[test]
    fn rust_super_from_leaf_file_resolves_in_same_directory() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "Cargo.toml", "[package]\nname=\"demo\"");
        write(
            root,
            "src/core/cache.rs",
            "use super::registry::RegistryResponse;\nuse super::super::shared::Shared;",
        );
        write(root, "src/core/registry.rs", "pub struct RegistryResponse;");
        write(root, "src/shared.rs", "pub struct Shared;");

        let rels = relationships(root);
        assert!(rels.iter().any(|r| {
            r.source == "src/core/cache.rs"
                && r.target == "src/core/registry.rs"
                && r.kind == RelationshipKind::Import
                && r.resolved
        }));
        // `super::super` from a leaf climbs one extra level.
        assert!(rels.iter().any(|r| {
            r.source == "src/core/cache.rs" && r.target == "src/shared.rs" && r.resolved
        }));
    }

    #[test]
    fn rust_super_from_mod_file_pops_one_level() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "Cargo.toml", "[package]\nname=\"demo\"");
        write(root, "src/core/mod.rs", "use super::shared::Shared;");
        write(root, "src/shared.rs", "pub struct Shared;");

        let rels = relationships(root);
        assert!(rels.iter().any(|r| {
            r.source == "src/core/mod.rs"
                && r.target == "src/shared.rs"
                && r.kind == RelationshipKind::Import
                && r.resolved
        }));
    }

    #[test]
    fn unresolved_relative_imports_reported() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/missing.ts", "import x from './does-not-exist';");
        let rels = relationships(root);
        assert!(rels.iter().any(|r| {
            r.source == "src/missing.ts" && r.target == "./does-not-exist" && !r.resolved
        }));
    }

    #[test]
    fn rust_crate_and_mod_resolution() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "Cargo.toml", "[package]\nname=\"demo\"");
        write(
            root,
            "src/main.rs",
            "mod routes;\nuse crate::routes::user;\nuse crate::db;",
        );
        write(root, "src/routes/mod.rs", "pub mod user;\npub mod health;");
        write(root, "src/routes/user.rs", "pub fn handle() {}");
        write(root, "src/routes/health.rs", "pub fn health() {}");
        write(root, "src/db.rs", "pub fn connect() {}");

        let rels = relationships(root);
        assert!(rels.iter().any(|r| {
            r.source == "src/main.rs"
                && r.target == "src/routes/mod.rs"
                && r.kind == RelationshipKind::ModuleReference
        }));
        assert!(rels
            .iter()
            .any(|r| { r.source == "src/main.rs" && r.target == "src/routes/user.rs" }));
        assert!(rels
            .iter()
            .any(|r| { r.source == "src/main.rs" && r.target == "src/db.rs" }));
        assert!(rels.iter().any(|r| {
            r.source == "src/routes/mod.rs"
                && r.target == "src/routes/user.rs"
                && r.kind == RelationshipKind::ModuleReference
        }));
        assert!(rels
            .iter()
            .any(|r| { r.source == "src/routes/mod.rs" && r.target == "src/routes/health.rs" }));
    }

    #[test]
    fn rust_brace_group_use() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "src/main.rs", "use crate::a::{b, c};");
        write(root, "src/a/b.rs", "pub fn b(){}");
        write(root, "src/a/c.rs", "pub fn c(){}");

        let rels = relationships(root);
        assert!(rels.iter().any(|r| r.target == "src/a/b.rs"));
        assert!(rels.iter().any(|r| r.target == "src/a/c.rs"));
    }

    #[test]
    fn python_relative_and_absolute() {
        let dir = tempdir().unwrap();
        let root = dir.path();
        write(root, "app/__init__.py", "");
        write(root, "app/routers/__init__.py", "");
        write(
            root,
            "app/routers/users.py",
            "from ..models import User\nfrom . import deps",
        );
        write(root, "app/models.py", "class User: pass");
        write(root, "app/routers/deps.py", "def dep(): pass");
        write(
            root,
            "app/main.py",
            "import app.models\nfrom app.models import User",
        );

        let rels = relationships(root);
        assert!(rels
            .iter()
            .any(|r| { r.source == "app/routers/users.py" && r.target == "app/models.py" }));
        assert!(rels
            .iter()
            .any(|r| { r.source == "app/routers/users.py" && r.target == "app/routers/deps.py" }));
        assert!(rels
            .iter()
            .any(|r| { r.source == "app/main.py" && r.target == "app/models.py" }));
    }

    #[test]
    fn expansion_is_correct() {
        let expanded = expand_braces("crate::{a, b}::x");
        assert_eq!(expanded, vec!["crate::a::x", "crate::b::x"]);
        let nested = expand_braces("a::{b, {c, d}}");
        assert_eq!(nested, vec!["a::b", "a::c", "a::d"]);
    }
}
