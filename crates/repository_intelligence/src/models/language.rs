//! Supported programming/markup languages.

use std::fmt;
use std::str::FromStr;

use serde::de::{self, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Programming and markup languages recognised by the engine.
///
/// The variant set is deliberately closed for now; adding a new language is a
/// small, intentional change that flows through detection, analysis and
/// reporting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum Language {
    /// Rust
    Rust,
    /// TypeScript (`.ts`, `.tsx`, `.mts`, `.cts`)
    TypeScript,
    /// JavaScript (`.js`, `.jsx`, `.mjs`, `.cjs`)
    JavaScript,
    /// Python (`.py`, `.pyi`)
    Python,
    /// Go (`.go`)
    Go,
    /// Java (`.java`)
    Java,
    /// C# (`.cs`)
    CSharp,
    /// C (`.c`, `.h`)
    C,
    /// C++ (`.cpp`, `.cc`, `.cxx`, `.hpp`, `.hh`, `.hxx`)
    Cpp,
    /// JSON (`.json`, `.jsonc`)
    Json,
    /// YAML (`.yaml`, `.yml`)
    Yaml,
    /// TOML (`.toml`)
    Toml,
    /// Markdown (`.md`, `.markdown`)
    Markdown,
}

impl Language {
    /// Canonical lowercase string form, used in JSON output (`"rust"`,
    /// `"typescript"`, ...).
    pub fn as_str(self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::TypeScript => "typescript",
            Language::JavaScript => "javascript",
            Language::Python => "python",
            Language::Go => "go",
            Language::Java => "java",
            Language::CSharp => "csharp",
            Language::C => "c",
            Language::Cpp => "cpp",
            Language::Json => "json",
            Language::Yaml => "yaml",
            Language::Toml => "toml",
            Language::Markdown => "markdown",
        }
    }

    /// Iterate all supported languages in a stable order.
    pub fn all() -> impl Iterator<Item = Language> {
        [
            Language::Rust,
            Language::TypeScript,
            Language::JavaScript,
            Language::Python,
            Language::Go,
            Language::Java,
            Language::CSharp,
            Language::C,
            Language::Cpp,
            Language::Json,
            Language::Yaml,
            Language::Toml,
            Language::Markdown,
        ]
        .into_iter()
    }

    /// Whether this language has source-level import semantics that the
    /// import extractor understands (Rust, TypeScript, JavaScript, Python).
    pub fn supports_import_analysis(self) -> bool {
        matches!(
            self,
            Language::Rust | Language::TypeScript | Language::JavaScript | Language::Python
        )
    }
}

impl fmt::Display for Language {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Language {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let normalized = s.to_ascii_lowercase();
        Ok(match normalized.as_str() {
            "rust" | "rs" => Language::Rust,
            "typescript" | "ts" | "tsx" | "mts" | "cts" => Language::TypeScript,
            "javascript" | "js" | "jsx" | "mjs" | "cjs" => Language::JavaScript,
            "python" | "py" | "pyi" => Language::Python,
            "go" => Language::Go,
            "java" => Language::Java,
            "csharp" | "c#" => Language::CSharp,
            "c" => Language::C,
            "cpp" | "c++" | "cc" => Language::Cpp,
            "json" | "jsonc" => Language::Json,
            "yaml" | "yml" => Language::Yaml,
            "toml" => Language::Toml,
            "markdown" | "md" => Language::Markdown,
            _ => return Err(()),
        })
    }
}

impl Serialize for Language {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

struct LanguageVisitor;

impl<'de> Visitor<'de> for LanguageVisitor {
    type Value = Language;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("a supported language name")
    }

    fn visit_str<E>(self, value: &str) -> Result<Language, E>
    where
        E: de::Error,
    {
        Language::from_str(value).map_err(|_| de::Error::unknown_variant(value, &[]))
    }
}

impl<'de> Deserialize<'de> for Language {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        deserializer.deserialize_str(LanguageVisitor)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn as_str_matches_from_str() {
        for language in Language::all() {
            assert_eq!(Language::from_str(language.as_str()), Ok(language));
        }
    }

    #[test]
    fn unknown_languages_rejected() {
        assert!(Language::from_str("brainfuck").is_err());
        assert!(Language::from_str("").is_err());
    }

    #[test]
    fn import_analysis_capable_only_source_languages() {
        assert!(Language::Rust.supports_import_analysis());
        assert!(Language::TypeScript.supports_import_analysis());
        assert!(Language::JavaScript.supports_import_analysis());
        assert!(Language::Python.supports_import_analysis());
        assert!(!Language::Json.supports_import_analysis());
        assert!(!Language::Markdown.supports_import_analysis());
    }

    #[test]
    fn serializes_to_lowercase_string() {
        let json = serde_json::to_string(&Language::TypeScript).unwrap();
        assert_eq!(json, "\"typescript\"");
    }
}
