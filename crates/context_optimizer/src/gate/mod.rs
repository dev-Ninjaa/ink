//! Content eligibility gate: decide which candidate files are worth sending
//! to an LLM as context.
//!
//! Binary/media assets and generated lockfiles/bundles add noise and burn the
//! token budget, so they are classified up front and dropped before scoring.

/// How a candidate file is classified for context eligibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContentClass {
    /// Textual source, config or documentation worth including.
    Source,
    /// Binary or media asset (images, archives, executables, ...).
    NonText,
    /// Generated or heavy file with little value as LLM context
    /// (lockfiles, minified bundles).
    Generated,
}

/// Binary/media file extensions that are never useful as LLM context.
const BINARY_EXTENSIONS: &[&str] = &[
    // Images.
    "png", "jpg", "jpeg", "gif", "webp", "ico", "bmp", "tif", "tiff", "avif", "heic",
    // Audio / video.
    "mp3", "mp4", "wav", "ogg", "oga", "opus", "flac", "aac", "webm", "mov", "avi", "mkv",
    // Documents, archives and executables.
    "pdf", "zip", "gz", "tgz", "tar", "bz2", "xz", "7z", "rar", "exe", "dll", "so", "dylib", "o",
    "a", "lib", "bin", "wasm", "class", "jar", "war", "ttf", "otf", "woff", "woff2", "eot", "dat",
    "db", "sqlite", "sqlite3", "pak", "pyc", "pyo",
];

/// Generated file names (exact basenames) that are heavy and rarely relevant.
const GENERATED_FILENAMES: &[&str] = &[
    "package-lock.json",
    "npm-shrinkwrap.json",
    "pnpm-lock.yaml",
    "yarn.lock",
    "bun.lockb",
    "Cargo.lock",
    "composer.lock",
    "Gemfile.lock",
    "poetry.lock",
    "Pipfile.lock",
    "mix.lock",
    "pubspec.lock",
    "deno.lock",
    "flake.lock",
    "gradle.lockfile",
    "go.sum",
];

/// Generated file extensions (e.g. `app.min.js`).
const GENERATED_EXTENSIONS: &[&str] = &["lock", "min.js", "min.css"];

/// Classify a repository-relative path into its [`ContentClass`].
///
/// Path separators are normalized so Windows and POSIX paths classify
/// identically.
pub fn classify(path: &str) -> ContentClass {
    let normalized = path.replace('\\', "/");
    let basename = normalized.rsplit('/').next().unwrap_or(normalized.as_str());
    if GENERATED_FILENAMES.contains(&basename) {
        return ContentClass::Generated;
    }
    let ext = basename.rsplit('.').next().unwrap_or("");
    if BINARY_EXTENSIONS.contains(&ext) {
        return ContentClass::NonText;
    }
    if GENERATED_EXTENSIONS
        .iter()
        .any(|candidate| basename.ends_with(&format!(".{candidate}")))
    {
        return ContentClass::Generated;
    }
    ContentClass::Source
}

/// Whether a repository-relative path is eligible for the context bundle.
pub fn is_eligible(path: &str) -> bool {
    classify(path) == ContentClass::Source
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_files_are_eligible() {
        assert_eq!(classify("src/main.rs"), ContentClass::Source);
        assert_eq!(classify("src/app.tsx"), ContentClass::Source);
        assert_eq!(classify("README.md"), ContentClass::Source);
        assert_eq!(classify(".gitignore"), ContentClass::Source);
        assert_eq!(classify("src/data.json"), ContentClass::Source);
        assert!(is_eligible("src/main.rs"));
    }

    #[test]
    fn binary_and_media_are_non_text() {
        assert_eq!(classify("assets/logo.png"), ContentClass::NonText);
        assert_eq!(classify("public/favicon.ico"), ContentClass::NonText);
        assert_eq!(classify("docs/spec.pdf"), ContentClass::NonText);
        assert_eq!(classify("dist/app.wasm"), ContentClass::NonText);
        assert!(!is_eligible("assets/logo.png"));
    }

    #[test]
    fn generated_lockfiles_are_excluded() {
        assert_eq!(classify("package-lock.json"), ContentClass::Generated);
        assert_eq!(classify("Cargo.lock"), ContentClass::Generated);
        assert_eq!(classify("yarn.lock"), ContentClass::Generated);
        assert_eq!(classify("pnpm-lock.yaml"), ContentClass::Generated);
        assert_eq!(classify("app.min.js"), ContentClass::Generated);
        assert!(!is_eligible("package-lock.json"));
    }

    #[test]
    fn windows_paths_are_normalized() {
        assert_eq!(classify(r"assets\logo.png"), ContentClass::NonText);
    }
}
