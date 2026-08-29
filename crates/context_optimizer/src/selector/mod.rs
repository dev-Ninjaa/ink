//! Relevance scoring of candidate files against a request query.
//!
//! Files are scored with path-token matches, filename-stem matches, module
//! membership, language names and structural signals (entry points, graph
//! centrality and reachability). All scoring is deterministic.

use std::collections::HashSet;

/// Query tokens derived from a request query.
#[derive(Debug, Clone, Default)]
pub struct QueryTokens {
    tokens: Vec<String>,
}

impl QueryTokens {
    /// Tokenize a query string, discarding stopwords and very short tokens.
    pub fn new(query: &str) -> Self {
        QueryTokens {
            tokens: tokenize(query),
        }
    }

    /// Whether the query produced no usable tokens.
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty()
    }

    /// Iterate the query tokens.
    pub fn iter(&self) -> impl Iterator<Item = &str> {
        self.tokens.iter().map(String::as_str)
    }
}

/// Split `input` into lowercase alphanumeric tokens, dropping stopwords and
/// single-character tokens. CamelCase boundaries and punctuation are both
/// split so `DashboardProvider.ts` and `dashboard provider` yield the same
/// tokens.
pub fn tokenize(input: &str) -> Vec<String> {
    input
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .flat_map(split_camel)
        .map(|word| word.to_ascii_lowercase())
        .filter(|token| token.len() >= 2 && !is_stopword(token))
        .collect()
}

/// Split a single word at camelCase boundaries (`DashboardProvider` →
/// `["Dashboard", "Provider"]`, `HTTPServer` → `["HTTP", "Server"]`).
fn split_camel(word: &str) -> Vec<String> {
    let chars: Vec<char> = word.chars().collect();
    if chars.len() < 2 {
        return vec![word.to_owned()];
    }
    let mut parts = Vec::new();
    let mut start = 0;
    for index in 1..chars.len() {
        let previous = chars[index - 1];
        let current = chars[index];
        let lower_to_upper = previous.is_lowercase() && current.is_uppercase();
        let acronym_boundary = previous.is_uppercase()
            && current.is_uppercase()
            && index + 1 < chars.len()
            && chars[index + 1].is_lowercase();
        if lower_to_upper || acronym_boundary {
            parts.push(chars[start..index].iter().collect::<String>());
            start = index;
        }
    }
    parts.push(chars[start..].iter().collect::<String>());
    parts
}

/// Inputs describing one candidate file.
#[derive(Debug, Clone, Copy)]
pub struct FileSignals<'a> {
    /// Repository-relative path.
    pub path: &'a str,
    /// Detected language name, when known.
    pub language: Option<&'a str>,
    /// Owning module name, when known.
    pub module: Option<&'a str>,
    /// Whether the file is a Repository Intelligence entry point.
    pub is_entrypoint: bool,
    /// Whether the file is among the graph's central files.
    pub is_central: bool,
    /// Whether the file is reachable from entry points.
    pub is_reachable: bool,
}

/// Score `signals` against `query`.
///
/// Returns a raw score and the human-readable reasons that contributed to it.
/// Structural boosts are only applied when `structural_boost` is enabled.
pub fn score(
    signals: &FileSignals,
    query: &QueryTokens,
    structural_boost: bool,
) -> (f64, Vec<String>) {
    let path_tokens = path_tokens(signals.path);
    let stem_tokens = stem_tokens(signals.path);
    let module_tokens = signals.module.map(tokenize).unwrap_or_default();

    let mut score = 0.0;
    let mut reasons = Vec::new();
    let mut seen = HashSet::new();

    for token in query.iter() {
        if path_tokens.iter().any(|t| t == token) {
            score += 3.0;
            record(
                &mut reasons,
                &mut seen,
                format!("path token match: `{token}`"),
            );
        } else if token.len() >= 3 && path_tokens.iter().any(|t| t.starts_with(token)) {
            score += 1.5;
            record(
                &mut reasons,
                &mut seen,
                format!("path prefix match: `{token}`"),
            );
        }
        if stem_tokens.iter().any(|t| t == token) {
            score += 2.0;
            record(
                &mut reasons,
                &mut seen,
                format!("filename match: `{token}`"),
            );
        }
        if module_tokens.iter().any(|t| t == token) {
            score += 1.0;
            record(&mut reasons, &mut seen, format!("module match: `{token}`"));
        }
        if signals.language.is_some_and(|language| language == token) {
            score += 1.5;
            record(
                &mut reasons,
                &mut seen,
                format!("language match: `{token}`"),
            );
        }
    }

    if structural_boost {
        if signals.is_entrypoint {
            score += 2.0;
            record(&mut reasons, &mut seen, "entry point".to_owned());
        }
        if signals.is_central {
            score += 1.0;
            record(&mut reasons, &mut seen, "central file".to_owned());
        }
        if signals.is_reachable {
            score += 0.5;
            record(
                &mut reasons,
                &mut seen,
                "reachable from entry points".to_owned(),
            );
        }
    }

    (score, reasons)
}

/// All path segments split into tokens (`src/feature/api-client.ts` →
/// `["src", "feature", "api", "client", "ts"]`; camelCase names are split too).
fn path_tokens(path: &str) -> Vec<String> {
    path.split('/')
        .flat_map(|segment| segment.split(['-', '_', '.']))
        .filter(|token| !token.is_empty())
        .flat_map(split_camel)
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

/// Tokens of the file name without its path.
fn stem_tokens(path: &str) -> Vec<String> {
    let stem = path.rsplit('/').next().unwrap_or(path);
    stem.split(['-', '_', '.'])
        .filter(|token| !token.is_empty())
        .flat_map(split_camel)
        .map(|token| token.to_ascii_lowercase())
        .collect()
}

fn record(reasons: &mut Vec<String>, seen: &mut HashSet<String>, reason: String) {
    if seen.insert(reason.clone()) {
        reasons.push(reason);
    }
}

fn is_stopword(token: &str) -> bool {
    matches!(
        token,
        "the"
            | "and"
            | "for"
            | "with"
            | "from"
            | "this"
            | "that"
            | "into"
            | "out"
            | "all"
            | "are"
            | "was"
            | "were"
            | "which"
            | "will"
            | "should"
            | "would"
            | "not"
            | "have"
            | "has"
            | "had"
            | "can"
            | "could"
            | "its"
            | "it"
            | "an"
            | "be"
            | "to"
            | "of"
            | "in"
            | "on"
            | "is"
            | "at"
            | "as"
            | "or"
            | "by"
            | "my"
            | "we"
            | "our"
            | "do"
            | "does"
            | "did"
            | "then"
            | "them"
            | "they"
            | "there"
            | "here"
            | "get"
            | "set"
            | "what"
            | "when"
            | "where"
            | "who"
            | "how"
            | "why"
            | "use"
            | "using"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signals(path: &str) -> FileSignals<'_> {
        FileSignals {
            path,
            language: None,
            module: None,
            is_entrypoint: false,
            is_central: false,
            is_reachable: false,
        }
    }

    #[test]
    fn tokenizes_and_filters_stopwords() {
        assert_eq!(tokenize("Fix the auth bug"), vec!["fix", "auth", "bug"]);
        assert_eq!(tokenize("auth"), vec!["auth"]);
        assert!(tokenize("the and for").is_empty());
    }

    #[test]
    fn camel_case_is_split() {
        assert_eq!(tokenize("DashboardProvider"), vec!["dashboard", "provider"]);
        assert_eq!(tokenize("HTTPServer"), vec!["http", "server"]);
        assert_eq!(
            tokenize("fix dashboard bug"),
            vec!["fix", "dashboard", "bug"]
        );
    }

    #[test]
    fn camel_case_filename_matches_query_exactly() {
        let query = QueryTokens::new("dashboard");
        let (score, reasons) = score(
            &signals("src/providers/DashboardProvider.ts"),
            &query,
            false,
        );
        assert!(score >= 3.0);
        assert!(reasons
            .iter()
            .any(|r| r.contains("path token match: `dashboard`")));
    }

    #[test]
    fn path_match_outranks_others() {
        let query = QueryTokens::new("auth");
        let (auth, _) = score(&signals("src/auth.rs"), &query, true);
        let (db, _) = score(&signals("src/db.rs"), &query, true);
        assert!(auth > db);
    }

    #[test]
    fn structural_boost_lifts_entry_points() {
        let query = QueryTokens::new("");
        let mut plain = signals("src/main.rs");
        let (base, _) = score(&plain, &query, true);
        plain.is_entrypoint = true;
        let (entry, _) = score(&plain, &query, true);
        assert!(entry > base);
        assert!((entry - base - 2.0).abs() < 1e-9);
    }

    #[test]
    fn language_and_module_matches_score() {
        let query = QueryTokens::new("rust models");
        let (score, reasons) = score(
            &FileSignals {
                path: "src/models/user.rs",
                language: Some("rust"),
                module: Some("models"),
                is_entrypoint: false,
                is_central: false,
                is_reachable: false,
            },
            &query,
            true,
        );
        assert!(score >= 3.5);
        assert!(reasons.iter().any(|r| r.contains("language match")));
        assert!(reasons.iter().any(|r| r.contains("module match")));
    }

    #[test]
    fn no_structural_boost_when_disabled() {
        let query = QueryTokens::new("");
        let signals = FileSignals {
            path: "src/main.rs",
            language: None,
            module: None,
            is_entrypoint: true,
            is_central: true,
            is_reachable: true,
        };
        let (score, reasons) = score(&signals, &query, false);
        assert_eq!(score, 0.0);
        assert!(reasons.is_empty());
    }
}
