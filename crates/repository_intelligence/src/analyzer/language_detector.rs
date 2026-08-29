//! Language detection from file extensions and well-known filenames.

use std::collections::BTreeMap;
use std::path::Path;

use crate::models::Language;

use super::scanner::ScannedFile;

/// Maps extensions to a language.
pub const LANGUAGE_EXTENSIONS: &[(Language, &[&str])] = &[
    (Language::Rust, &["rs"]),
    (Language::TypeScript, &["ts", "tsx", "mts", "cts"]),
    (Language::JavaScript, &["js", "jsx", "mjs", "cjs"]),
    (Language::Python, &["py", "pyi"]),
    (Language::Go, &["go"]),
    (Language::Java, &["java"]),
    (Language::CSharp, &["cs"]),
    (Language::C, &["c", "h"]),
    (
        Language::Cpp,
        &["cpp", "cc", "cxx", "hpp", "hh", "hxx", "cppm"],
    ),
    (Language::Json, &["json", "jsonc", "json5"]),
    (Language::Yaml, &["yaml", "yml"]),
    (Language::Toml, &["toml", "tml"]),
    (Language::Markdown, &["md", "markdown"]),
];

/// Well-known filenames that do not carry a recognised extension.
pub const LANGUAGE_FILENAMES: &[(Language, &[&str])] = &[
    (
        Language::TypeScript,
        &["tsconfig.json", "tsconfig.eslint.json"],
    ),
    (
        Language::Json,
        &["package.json", "package-lock.json", ".prettierrc"],
    ),
    (Language::Toml, &["Cargo.toml", "pyproject.toml"]),
    (Language::Markdown, &["README", "CHANGELOG", "LICENSE"]),
];

/// Detects [`Language`] from a relative or absolute file path.
#[derive(Debug, Default, Clone, Copy)]
pub struct LanguageDetector;

impl LanguageDetector {
    /// Detect the language of `path` by extension, falling back to known
    /// filenames.
    pub fn detect(&self, path: &Path) -> Option<Language> {
        if let Some(ext) = path.extension().and_then(|e| e.to_str()) {
            let ext = ext.to_ascii_lowercase();
            for (language, extensions) in LANGUAGE_EXTENSIONS {
                if extensions.contains(&ext.as_str()) {
                    return Some(*language);
                }
            }
        }

        // Fallback: well-known filenames.
        let name = path.file_name()?.to_str()?;
        for (language, names) in LANGUAGE_FILENAMES {
            if names.contains(&name) {
                return Some(*language);
            }
        }
        None
    }

    /// Compute a language occurrence histogram for a set of files.
    pub fn count(&self, files: &[ScannedFile]) -> BTreeMap<Language, usize> {
        let mut counts: BTreeMap<Language, usize> = BTreeMap::new();
        for file in files {
            if let Some(language) = self.detect(&file.rel) {
                *counts.entry(language).or_insert(0) += 1;
            }
        }
        counts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_every_supported_language() {
        let detector = LanguageDetector;
        assert_eq!(
            detector.detect(Path::new("src/main.rs")),
            Some(Language::Rust)
        );
        assert_eq!(
            detector.detect(Path::new("a.ts")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            detector.detect(Path::new("component.tsx")),
            Some(Language::TypeScript)
        );
        assert_eq!(
            detector.detect(Path::new("app.js")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            detector.detect(Path::new("app.jsx")),
            Some(Language::JavaScript)
        );
        assert_eq!(
            detector.detect(Path::new("main.py")),
            Some(Language::Python)
        );
        assert_eq!(detector.detect(Path::new("main.go")), Some(Language::Go));
        assert_eq!(
            detector.detect(Path::new("Main.java")),
            Some(Language::Java)
        );
        assert_eq!(
            detector.detect(Path::new("Program.cs")),
            Some(Language::CSharp)
        );
        assert_eq!(detector.detect(Path::new("util.c")), Some(Language::C));
        assert_eq!(detector.detect(Path::new("util.h")), Some(Language::C));
        assert_eq!(detector.detect(Path::new("util.cpp")), Some(Language::Cpp));
        assert_eq!(detector.detect(Path::new("util.hpp")), Some(Language::Cpp));
        assert_eq!(
            detector.detect(Path::new("config.json")),
            Some(Language::Json)
        );
        assert_eq!(
            detector.detect(Path::new("config.yaml")),
            Some(Language::Yaml)
        );
        assert_eq!(
            detector.detect(Path::new("config.yml")),
            Some(Language::Yaml)
        );
        assert_eq!(
            detector.detect(Path::new("Cargo.toml")),
            Some(Language::Toml)
        );
        assert_eq!(
            detector.detect(Path::new("README.md")),
            Some(Language::Markdown)
        );
    }

    #[test]
    fn detects_uppercase_extensions() {
        let detector = LanguageDetector;
        assert_eq!(
            detector.detect(Path::new("Main.PY")),
            Some(Language::Python)
        );
        assert_eq!(
            detector.detect(Path::new("App.TS")),
            Some(Language::TypeScript)
        );
    }

    #[test]
    fn unknown_files_return_none() {
        let detector = LanguageDetector;
        assert_eq!(detector.detect(Path::new("Makefile")), None);
        assert_eq!(detector.detect(Path::new("data.csv")), None);
        assert_eq!(detector.detect(Path::new("no_extension")), None);
    }

    #[test]
    fn counts_language_histogram() {
        let detector = LanguageDetector;
        let files = vec![
            ScannedFile {
                path: "/repo/a.rs".into(),
                rel: "a.rs".into(),
                size: 1,
            },
            ScannedFile {
                path: "/repo/b.rs".into(),
                rel: "b.rs".into(),
                size: 1,
            },
            ScannedFile {
                path: "/repo/c.ts".into(),
                rel: "c.ts".into(),
                size: 1,
            },
            ScannedFile {
                path: "/repo/README".into(),
                rel: "README".into(),
                size: 1,
            },
        ];
        let counts = detector.count(&files);
        assert_eq!(counts[&Language::Rust], 2);
        assert_eq!(counts[&Language::TypeScript], 1);
        assert_eq!(counts[&Language::Markdown], 1);
    }
}
