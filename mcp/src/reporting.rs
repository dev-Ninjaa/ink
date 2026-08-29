//! File-based reporting for Ink MCP tool results.
//!
//! When the `INK_REPORT_DIR` environment variable points at a directory, every
//! MCP tool call writes a timestamped JSON document and appends a human-readable
//! markdown section to `report.md` inside that directory. When the variable is
//! unset or empty, reporting is a no-op so the stdio transport stays clean.

use std::fs;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

/// Environment variable selecting the reporting directory.
const ENV_REPORT_DIR: &str = "INK_REPORT_DIR";

/// Writes tool results to disk when a report directory is configured.
#[derive(Debug, Clone)]
pub struct Reporter {
    dir: Option<PathBuf>,
}

impl Reporter {
    /// Construct a reporter from the `INK_REPORT_DIR` environment variable.
    pub fn from_env() -> Self {
        let dir = std::env::var(ENV_REPORT_DIR)
            .ok()
            .filter(|dir| !dir.trim().is_empty());
        Reporter {
            dir: dir.map(PathBuf::from),
        }
    }

    /// Whether reporting is active (a directory was configured).
    #[cfg(test)]
    pub fn enabled(&self) -> bool {
        self.dir.is_some()
    }

    /// Record one tool result: write `<tool>-<timestamp>.json` and append a
    /// markdown section to `report.md`. No-op when reporting is disabled.
    pub fn record(&self, tool: &str, result: &str) {
        let Some(dir) = &self.dir else {
            return;
        };
        let stamp = timestamp();
        if let Err(error) = fs::create_dir_all(dir) {
            eprintln!("[ink] reporting: failed to create report dir: {error}");
            return;
        }

        let mut json_path = dir.join(format!("{tool}-{}.json", stamp.file));
        let mut n = 2;
        while json_path.exists() {
            json_path = dir.join(format!("{tool}-{}-{n}.json", stamp.file));
            n += 1;
        }
        if let Err(error) = fs::write(&json_path, result) {
            eprintln!(
                "[ink] reporting: failed to write {}: {error}",
                json_path.display()
            );
        }

        let markdown = markdown_section(tool, result, &stamp.human);
        if let Err(error) = append_to(&dir.join("report.md"), &markdown) {
            eprintln!("[ink] reporting: failed to append report.md: {error}");
        }
    }
}

/// Append `contents` to `path`, creating the file when it does not exist.
fn append_to(path: &Path, contents: &str) -> std::io::Result<()> {
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    file.write_all(contents.as_bytes())
}

/// A point in time formatted for filenames and for the markdown report.
struct Stamp {
    /// Sortable, filename-safe representation.
    file: String,
    /// Human-readable UTC representation.
    human: String,
}

/// Format the current wall-clock time (UTC) without any external dependency.
fn timestamp() -> Stamp {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs();
    let millis = now.subsec_millis();
    let (y, mo, d) = civil_from_days((secs / 86_400) as i64);
    let hh = secs / 3_600 % 24;
    let mm = secs / 60 % 60;
    let ss = secs % 60;
    Stamp {
        file: format!("{y:04}{mo:02}{d:02}-{hh:02}{mm:02}{ss:02}-{millis:03}"),
        human: format!("{y:04}-{mo:02}-{d:02} {hh:02}:{mm:02}:{ss:02} UTC"),
    }
}

/// Convert days since the Unix epoch to a (year, month, day) civil date.
/// Howard Hinnant's `civil_from_days` algorithm.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1_460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y, m as u32, d as u32)
}

/// Build the markdown section for one tool result.
fn markdown_section(tool: &str, result: &str, when: &str) -> String {
    let mut out = format!("\n### {tool} — {when}\n\n");
    match serde_json::from_str::<Value>(result) {
        Ok(value) => out.push_str(&summary_for(tool, &value)),
        Err(_) => out.push_str("```\n(non-JSON result)\n```\n"),
    }
    out
}

/// Human-readable summary lines for a tool's JSON output.
fn summary_for(tool: &str, value: &Value) -> String {
    match tool {
        "analyze_repository" => analysis_summary(value),
        "build_dependency_graph" => graph_summary(value),
        "optimize_context" => context_summary(value),
        _ => format!(
            "```\n{}\n```\n",
            serde_json::to_string_pretty(value).unwrap_or_default()
        ),
    }
}

fn obj_get<'a>(value: &'a Value, key: &str) -> Option<&'a Value> {
    value.as_object()?.get(key)
}

fn num(value: Option<&Value>) -> String {
    value
        .and_then(|v| v.as_u64())
        .map(|n| n.to_string())
        .unwrap_or_else(|| "—".into())
}

fn text(value: Option<&Value>) -> String {
    value.and_then(|v| v.as_str()).unwrap_or("—").to_owned()
}

fn float(value: Option<&Value>) -> String {
    value
        .and_then(|v| v.as_f64())
        .map(|n| format!("{n:.2}"))
        .unwrap_or_else(|| "—".into())
}

/// Escape a value for use inside a markdown table cell.
fn esc(value: &str) -> String {
    value.replace('|', "\\|").replace('\n', " ")
}

fn analysis_summary(value: &Value) -> String {
    let mut out = String::new();
    out.push_str(&format!("- **root:** {}\n", text(obj_get(value, "root"))));
    out.push_str(&format!(
        "- **analyzer:** {}\n",
        text(obj_get(value, "analyzer_version"))
    ));
    if let Some(summary) = obj_get(value, "summary") {
        out.push_str(&format!(
            "- **files:** {} · **dirs:** {} · **bytes:** {}\n",
            num(obj_get(summary, "files")),
            num(obj_get(summary, "directories")),
            num(obj_get(summary, "bytes"))
        ));
    }
    if let Some(perf) = obj_get(value, "performance") {
        out.push_str(&format!(
            "- **time:** {} ms · **throughput:** {} files/s\n",
            float(obj_get(perf, "total_duration_ms")),
            float(obj_get(perf, "files_per_second"))
        ));
    }
    if let Some(languages) = value.get("languages").and_then(|l| l.as_object()) {
        let mut counts: Vec<String> = languages
            .iter()
            .map(|(lang, count)| format!("{lang}={}", count.as_u64().unwrap_or(0)))
            .collect();
        counts.sort();
        out.push_str(&format!("- **languages:** {}\n", counts.join(", ")));
    }
    if let Some(entry_points) = value.get("entry_points").and_then(|e| e.as_array()) {
        let mut list: Vec<String> = entry_points
            .iter()
            .map(|e| {
                format!(
                    "{} ({})",
                    text(obj_get(e, "path")),
                    float(obj_get(e, "confidence"))
                )
            })
            .collect();
        if list.len() > 5 {
            list.truncate(5);
            list.push("…".into());
        }
        out.push_str(&format!("- **entry points:** {}\n", list.join(", ")));
    }
    let module_count = value
        .get("modules")
        .and_then(|m| m.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    let relationship_count = value
        .get("relationships")
        .and_then(|r| r.as_array())
        .map(|a| a.len())
        .unwrap_or(0);
    out.push_str(&format!(
        "- **modules:** {module_count} · **relationships:** {relationship_count}\n"
    ));
    out
}

fn graph_summary(value: &Value) -> String {
    let mut out = String::new();
    if let Some(stats) = obj_get(value, "statistics") {
        out.push_str(&format!(
            "- **nodes:** {} · **edges:** {} · **modules:** {} · **module edges:** {}\n",
            num(obj_get(stats, "node_count")),
            num(obj_get(stats, "edge_count")),
            num(obj_get(stats, "module_count")),
            num(obj_get(stats, "module_edge_count"))
        ));
        out.push_str(&format!(
            "- **entrypoints:** {} · **file cycles:** {} · **module cycles:** {}\n",
            num(obj_get(stats, "entrypoint_count")),
            num(obj_get(stats, "file_cycle_count")),
            num(obj_get(stats, "module_cycle_count"))
        ));
        out.push_str(&format!(
            "- **max depth:** {} · **avg depth:** {} · **density:** {}\n",
            num(obj_get(stats, "maximum_depth")),
            float(obj_get(stats, "average_depth")),
            float(obj_get(stats, "graph_density"))
        ));
    }
    if let Some(central) = value.get("central_files").and_then(|c| c.as_array()) {
        let mut list: Vec<String> = central
            .iter()
            .take(5)
            .map(|node| {
                format!(
                    "{} (deg {})",
                    text(obj_get(node, "id")),
                    num(obj_get(node, "total_degree"))
                )
            })
            .collect();
        if central.len() > 5 {
            list.push("…".into());
        }
        out.push_str(&format!("- **central files:** {}\n", list.join(", ")));
    }
    if let Some(reach) = obj_get(value, "reachability") {
        let reachable = reach
            .get("reachable_nodes")
            .and_then(|r| r.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        let unreachable = reach
            .get("unreachable_nodes")
            .and_then(|r| r.as_array())
            .map(|a| a.len())
            .unwrap_or(0);
        out.push_str(&format!(
            "- **reachability:** {} reachable · {} unreachable\n",
            reachable, unreachable
        ));
    }
    out
}

fn context_summary(value: &Value) -> String {
    let mut out = String::new();
    out.push_str(&format!("- **root:** {}\n", text(obj_get(value, "root"))));
    out.push_str(&format!("- **query:** {}\n", text(obj_get(value, "query"))));
    if let Some(metrics) = obj_get(value, "metrics") {
        out.push_str(&format!(
            "- **files:** {} considered → {} selected\n",
            num(obj_get(metrics, "files_considered")),
            num(obj_get(metrics, "files_selected"))
        ));
        let reduction = obj_get(metrics, "token_reduction_percent")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        out.push_str(&format!(
            "- **tokens:** {} → {} ({reduction:.1}% reduction)\n",
            num(obj_get(metrics, "tokens_before")),
            num(obj_get(metrics, "tokens_after"))
        ));

        let dropped = [
            ("budget", "files_dropped_budget"),
            ("duplicates", "files_dropped_duplicates"),
            ("non-text", "files_dropped_non_text"),
            ("generated", "files_dropped_generated"),
            ("low relevance", "files_dropped_low_relevance"),
            ("excluded", "files_excluded"),
        ];
        let mut parts: Vec<String> = Vec::new();
        for (label, key) in dropped {
            let count = obj_get(metrics, key).and_then(|v| v.as_u64()).unwrap_or(0);
            if count > 0 {
                parts.push(format!("{label}={count}"));
            }
        }
        if !parts.is_empty() {
            out.push_str(&format!("- **dropped:** {}\n", parts.join(", ")));
        }
    }
    if let Some(selected) = value.get("selected").and_then(|s| s.as_array()) {
        out.push_str("- **selected files:**\n");
        out.push_str("  | path | score | relevance | tokens |\n");
        out.push_str("  |---|---:|---:|---:|\n");
        for file in selected.iter().take(20) {
            out.push_str(&format!(
                "  | {} | {} | {} | {} |\n",
                esc(&text(obj_get(file, "path"))),
                float(obj_get(file, "score")),
                float(obj_get(file, "relevance")),
                num(obj_get(file, "tokens"))
            ));
        }
        if selected.len() > 20 {
            out.push_str(&format!("  | … and {} more | | | |\n", selected.len() - 20));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epoch_is_1970_utc() {
        let (y, mo, d) = civil_from_days(0);
        assert_eq!((y, mo, d), (1970, 1, 1));
    }

    #[test]
    fn known_date_rounds_correctly() {
        let days = 20_694;
        let (y, mo, d) = civil_from_days(days);
        assert_eq!((y, mo, d), (2026, 8, 29));
    }

    #[test]
    fn disabled_reporter_records_nothing() {
        std::env::remove_var(ENV_REPORT_DIR);
        assert!(!Reporter::from_env().enabled());
    }

    #[test]
    fn record_writes_json_and_appends_report() {
        let dir = tempfile::tempdir().unwrap();
        let reporter = Reporter {
            dir: Some(dir.path().to_path_buf()),
        };
        reporter.record("analyze_repository", r#"{"summary":{"files":2}}"#);
        reporter.record("analyze_repository", r#"{"summary":{"files":3}}"#);

        let json_files: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".json"))
            .collect();
        assert_eq!(json_files.len(), 2);

        let report = fs::read_to_string(dir.path().join("report.md")).unwrap();
        assert_eq!(report.matches("### analyze_repository").count(), 2);
    }

    #[test]
    fn markdown_summary_extracts_optimizer_metrics() {
        let value = serde_json::json!({
            "root": "/repo",
            "query": "auth flow",
            "metrics": {
                "files_considered": 10,
                "files_selected": 2,
                "files_dropped_budget": 7,
                "files_dropped_duplicates": 1,
                "tokens_before": 1000,
                "tokens_after": 100,
                "token_reduction_percent": 90.0
            },
            "selected": [
                {"path": "src/auth.rs", "score": 2.0, "relevance": 1.0, "tokens": 50}
            ]
        });
        let summary = context_summary(&value);
        assert!(summary.contains("10 considered → 2 selected"));
        assert!(summary.contains("90.0% reduction"));
        assert!(summary.contains("budget=7"));
        assert!(summary.contains("| src/auth.rs |"));
    }

    #[test]
    fn graph_summary_reads_statistics() {
        let value = serde_json::json!({
            "statistics": {
                "node_count": 46, "edge_count": 112, "module_count": 10,
                "module_edge_count": 22, "entrypoint_count": 1,
                "file_cycle_count": 1, "module_cycle_count": 1,
                "maximum_depth": 9, "average_depth": 3.5, "graph_density": 0.1
            },
            "central_files": [{"id": "RuntimeManager.ts", "total_degree": 12}],
            "reachability": {"reachable_nodes": ["a"], "unreachable_nodes": ["b"]}
        });
        let summary = graph_summary(&value);
        assert!(summary.contains("**nodes:** 46 · **edges:** 112"));
        assert!(summary.contains("RuntimeManager.ts (deg 12)"));
        assert!(summary.contains("1 reachable · 1 unreachable"));
    }
}
