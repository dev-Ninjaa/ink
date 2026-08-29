//! End-to-end integration tests against the committed fixture repositories
//! under `tests/repositories`.

use std::path::{Path, PathBuf};

use repository_intelligence::analyzer::Analyzer;
use repository_intelligence::models::{Framework, Language, ModuleKind, RelationshipKind};
use repository_intelligence::output::json::to_json;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/repositories")
        .join(name)
}

#[test]
fn monorepo_javascript() {
    let analysis = Analyzer::with_defaults()
        .analyze(fixture("monorepo_js"))
        .unwrap();

    // Languages
    assert_eq!(analysis.languages[&Language::TypeScript], 11);
    assert_eq!(analysis.languages[&Language::Json], 6);
    assert_eq!(analysis.languages[&Language::JavaScript], 1);
    assert_eq!(analysis.languages[&Language::Markdown], 1);

    // Frameworks: next + react (web), express (api), react (ui) across the repo.
    assert!(analysis.frameworks.contains(&Framework::NextJs));
    assert!(analysis.frameworks.contains(&Framework::React));
    assert!(analysis.frameworks.contains(&Framework::Express));
    assert!(!analysis.frameworks.contains(&Framework::Vite));

    // Package manager inferred from pnpm lockfile; npm NOT inferred.
    assert!(analysis
        .metadata
        .package_managers
        .iter()
        .any(|p| p == "pnpm"));
    assert!(!analysis
        .metadata
        .package_managers
        .iter()
        .any(|p| p == "npm"));

    // Project roots: root + 3 workspace members.
    assert_eq!(analysis.summary.project_roots, 4);
    for root in ["", "apps/api", "apps/web", "packages/ui"] {
        assert!(
            analysis.project_roots.iter().any(|p| p == root),
            "missing project root {root:?}"
        );
    }

    // Entry points: apps/api/src/index.ts (from package.json main + src/index),
    // apps/web/src/app/page.tsx, packages/ui/src/index.ts? -- ui has no
    // src/index.ts file so it must not appear.
    let paths: Vec<&str> = analysis
        .entry_points
        .iter()
        .map(|e| e.path.as_str())
        .collect();
    assert!(paths.contains(&"apps/api/src/index.ts"));
    assert!(paths.contains(&"apps/web/src/app/page.tsx"));
    assert!(!paths.contains(&"packages/ui/src/index.ts"));

    // Relationships: web page -> header via relative imports.
    assert!(analysis.relationships.iter().any(|r| {
        r.source == "apps/web/src/app/page.tsx"
            && r.target == "apps/web/src/components/header.tsx"
            && r.kind == RelationshipKind::Import
            && r.resolved
    }));
    // api index -> routes/user.
    assert!(analysis.relationships.iter().any(|r| {
        r.source == "apps/api/src/index.ts"
            && r.target == "apps/api/src/routes/user.ts"
            && r.kind == RelationshipKind::Import
    }));

    // Modules: workspace packages + web feature folders under src.
    assert!(analysis
        .modules
        .iter()
        .any(|m| { m.name == "web" && m.kind == ModuleKind::Package }));
    assert!(analysis
        .modules
        .iter()
        .any(|m| { m.name == "components" && m.root == "apps/web/src/components" }));

    // Deterministic + serializable.
    let first = to_json(&analysis).unwrap();
    let second = Analyzer::with_defaults()
        .analyze(fixture("monorepo_js"))
        .unwrap();
    let mut second = second;
    second.performance = analysis.performance;
    assert_eq!(first, to_json(&second).unwrap());
    let json: serde_json::Value = serde_json::from_str(&first).unwrap();
    assert_eq!(json["summary"]["files"].as_u64().unwrap(), 20);
}

#[test]
fn rust_service() {
    let analysis = Analyzer::with_defaults()
        .analyze(fixture("rust_service"))
        .unwrap();

    assert_eq!(analysis.languages[&Language::Rust], 6);
    assert_eq!(analysis.languages[&Language::Toml], 1);
    assert!(analysis.frameworks.contains(&Framework::Axum));
    assert!(analysis
        .metadata
        .package_managers
        .iter()
        .any(|p| p == "cargo"));

    // Entry point.
    assert_eq!(analysis.entry_points[0].path, "src/main.rs");
    assert!((analysis.entry_points[0].confidence - 0.98).abs() < 1e-6);

    // Relationships from main.rs.
    let main_rels: Vec<_> = analysis
        .relationships
        .iter()
        .filter(|r| r.source == "src/main.rs" && r.resolved)
        .collect();
    let targets: Vec<&str> = main_rels.iter().map(|r| r.target.as_str()).collect();
    assert!(targets.contains(&"src/routes/mod.rs"));
    assert!(targets.contains(&"src/routes/health.rs"));
    assert!(targets.contains(&"src/routes/user.rs"));

    // routes -> db & models, db -> models.
    assert!(analysis
        .relationships
        .iter()
        .any(|r| { r.source == "src/routes/user.rs" && r.target == "src/db.rs" }));
    assert!(analysis
        .relationships
        .iter()
        .any(|r| { r.source == "src/routes/user.rs" && r.target == "src/models.rs" }));
    assert!(analysis
        .relationships
        .iter()
        .any(|r| { r.source == "src/db.rs" && r.target == "src/models.rs" }));

    // Modules: routes layer.
    assert!(analysis
        .modules
        .iter()
        .any(|m| { m.name == "routes" && m.kind == ModuleKind::Layer }));
}

#[test]
fn python_service() {
    let analysis = Analyzer::with_defaults()
        .analyze(fixture("python_service"))
        .unwrap();

    assert_eq!(analysis.languages[&Language::Python], 7);
    assert!(analysis.frameworks.contains(&Framework::FastApi));
    assert!(analysis
        .metadata
        .package_managers
        .iter()
        .any(|p| p == "pip"));

    let paths: Vec<&str> = analysis
        .entry_points
        .iter()
        .map(|e| e.path.as_str())
        .collect();
    assert_eq!(paths, vec!["app/main.py"]);

    // Relative + absolute python imports.
    assert!(analysis.relationships.iter().any(|r| {
        r.source == "app/routers/users.py" && r.target == "app/models.py" && r.resolved
    }));
    assert!(analysis.relationships.iter().any(|r| {
        r.source == "app/routers/users.py" && r.target == "app/routers/deps.py" && r.resolved
    }));
    assert!(analysis.relationships.iter().any(|r| {
        r.source == "app/main.py" && r.target == "app/routers/__init__.py" && r.resolved
    }));

    // routers is a layered module.
    assert!(analysis
        .modules
        .iter()
        .any(|m| { m.name == "routers" && m.kind == ModuleKind::Layer }));
}

#[test]
fn fixture_repositories_work_with_all_backends() {
    for backend in [
        repository_intelligence::analyzer::scanner::ScanBackend::Ignore,
        repository_intelligence::analyzer::scanner::ScanBackend::Walkdir,
    ] {
        let mut config = repository_intelligence::AnalyzerConfig::default();
        config.scan_options.backend = backend;
        let analyzer = Analyzer::new(config.clone());

        let js = analyzer.analyze(fixture("monorepo_js")).unwrap();
        let rust = analyzer.analyze(fixture("rust_service")).unwrap();
        let py = analyzer.analyze(fixture("python_service")).unwrap();

        assert!(js.summary.files > 0);
        assert!(rust.summary.files > 0);
        assert!(py.summary.files > 0);
    }
}
