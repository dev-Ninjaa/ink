//! Output serialization: JSON and human-readable reports.

pub mod json;
pub mod report;

pub use json::{to_json, to_json_compact, write_json_file};
pub use report::{render_benchmark_report, render_report, BenchmarkRow};
