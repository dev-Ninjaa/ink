//! End-to-end integration tests against small scenario fixtures plus
//! dynamically generated repositories for edge cases.

use std::path::{Path, PathBuf};

use repository_intelligence::analyzer::Analyzer;
use repository_intelligence::models::{Framework, Language, ModuleKind, RelationshipKind};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn write(root: &Path, rel: &str, content: &str) {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(p, content).unwrap();
}

#[test]
fn nested_import_fixture() {
    let analysis = Analyzer::with_defaults()
        .analyze(fixture("nested_imports"))
        .unwrap();

    // JS chain: app -> nested/a -> core/b
    assert!(analysis
        .relationships
        .iter()
        .any(|r| { r.source == "app.ts" && r.target == "nested/a.ts" && r.resolved }));
    assert!(analysis
        .relationships
        .iter()
        .any(|r| { r.source == "nested/a.ts" && r.target == "core/b.ts" && r.resolved }));
    // Missing relative import is reported unresolved.
    assert!(analysis
        .relationships
        .iter()
        .any(|r| { r.source == "app.ts" && r.target == "./nope" && !r.resolved }));

    // Python chain: pkg/mod/impl -> pkg/shared via `from .. import shared`.
    assert!(analysis.relationships.iter().any(|r| {
        r.source == "pkg/mod/impl.py"
            && r.target == "pkg/shared.py"
            && r.kind == RelationshipKind::Import
            && r.resolved
    }));
    // mod package exports impl.
    assert!(analysis
        .relationships
        .iter()
        .any(|r| { r.source == "pkg/mod/__init__.py" && r.target == "pkg/mod/impl.py" }));
}

#[test]
fn every_supported_language_detected() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    let samples: &[(&str, Language)] = &[
        ("src/main.rs", Language::Rust),
        ("app.ts", Language::TypeScript),
        ("app.tsx", Language::TypeScript),
        ("app.js", Language::JavaScript),
        ("app.jsx", Language::JavaScript),
        ("main.py", Language::Python),
        ("main.go", Language::Go),
        ("Main.java", Language::Java),
        ("Program.cs", Language::CSharp),
        ("util.c", Language::C),
        ("util.h", Language::C),
        ("util.cpp", Language::Cpp),
        ("data.json", Language::Json),
        ("config.yaml", Language::Yaml),
        ("config.toml", Language::Toml),
        ("README.md", Language::Markdown),
    ];
    for (rel, language) in samples {
        write(root, rel, "x");
        let analysis = Analyzer::with_defaults().analyze(root).unwrap();
        assert_eq!(
            analysis.languages[language], 1,
            "{rel} should be counted as {language:?}"
        );
        std::fs::remove_file(root.join(rel)).unwrap();
    }
}

#[test]
fn every_supported_framework_detected() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();

    // JS
    write(
        root,
        "package.json",
        r#"{"dependencies":{"next":"1.0.0","react":"1.0.0","express":"1.0.0","vite":"1.0.0"},"devDependencies":{"@nestjs/core":"1.0.0"}}"#,
    );
    // Python
    write(
        root,
        "requirements.txt",
        "fastapi==0.1\nflask==3.0\ndjango==5.0\n",
    );
    // Rust
    write(
        root,
        "Cargo.toml",
        "[dependencies]\naxum = \"0.7\"\nactix-web = \"4\"\nrocket = \"0.5\"\n",
    );

    let analysis = Analyzer::with_defaults().analyze(root).unwrap();
    for framework in Framework::all() {
        assert!(
            analysis.frameworks.contains(&framework),
            "expected framework {framework:?} to be detected"
        );
    }
}

#[test]
fn monorepo_layout_and_modules_from_generated_repo() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "pnpm-workspace.yaml",
        "packages:\n  - apps/*\n  - packages/*\n",
    );
    write(root, "apps/web/package.json", "{}");
    write(
        root,
        "apps/web/src/pages/home.tsx",
        "export default function Home() {}",
    );
    write(
        root,
        "apps/web/src/features/auth/login.ts",
        "export const login = 1;",
    );
    write(root, "packages/shared/package.json", "{}");
    write(
        root,
        "packages/shared/src/format.ts",
        "export function fmt(s: string) { return s; }",
    );

    let analysis = Analyzer::with_defaults().analyze(root).unwrap();
    // Project roots: root, apps/web, packages/shared.
    assert_eq!(analysis.summary.project_roots, 3);
    // Modules: web + shared packages plus feature folders under src.
    assert!(analysis
        .modules
        .iter()
        .any(|m| { m.name == "web" && m.kind == ModuleKind::Package }));
    assert!(analysis
        .modules
        .iter()
        .any(|m| { m.name == "shared" && m.kind == ModuleKind::Package }));
    assert!(analysis
        .modules
        .iter()
        .any(|m| { m.name == "features" && m.kind == ModuleKind::Feature }));
    assert!(analysis
        .modules
        .iter()
        .any(|m| { m.name == "features" && m.root == "apps/web/src/features" }));
}

#[test]
fn missing_entry_points_for_library_only_repo() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(
        root,
        "math.ts",
        "export const add = (a: number, b: number) => a + b;",
    );
    write(
        root,
        "string.ts",
        "export const cap = (s: string) => s.toUpperCase();",
    );
    write(root, "README.md", "# utilities");

    let analysis = Analyzer::with_defaults().analyze(root).unwrap();
    assert!(analysis.entry_points.is_empty());
    // modules: flat layout without nested folders produces no modules.
    assert!(analysis.modules.is_empty());
}

#[test]
fn import_extraction_skips_large_files_and_binary() {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, "src/main.ts", "import { x } from './x';");
    write(root, "src/x.ts", "export const x = 1;");

    // A file larger than the cap must not be read for imports.
    let big = root.join("src/huge.ts");
    let blob = "export const y = 1;\n// ".repeat(1 << 20);
    std::fs::write(&big, blob).unwrap();

    let config = repository_intelligence::AnalyzerConfig {
        max_source_bytes: 1024 * 1024,
        ..Default::default()
    };
    let analysis = Analyzer::new(config).analyze(root).unwrap();
    assert!(analysis
        .relationships
        .iter()
        .any(|r| { r.source == "src/main.ts" && r.target == "src/x.ts" }));
    // huge.ts is still counted as a file even though it was not analysed.
    assert_eq!(analysis.summary.files, 3);
}

#[test]
fn deterministic_across_runs_on_fixture() {
    let a = Analyzer::with_defaults()
        .analyze(fixture("nested_imports"))
        .unwrap();
    let b = Analyzer::with_defaults()
        .analyze(fixture("nested_imports"))
        .unwrap();
    // Pin timing fields which legitimately vary between runs.
    let mut a = a;
    a.performance = b.performance;
    assert_eq!(
        serde_json::to_string(&a).unwrap(),
        serde_json::to_string(&b).unwrap()
    );
}
