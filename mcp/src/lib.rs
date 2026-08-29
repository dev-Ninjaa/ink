//! Ink MCP server library crate.
//!
//! The binary (`src/main.rs`) is the entry point; the library exposes the
//! server handler and its tools/prompts/resources for integration testing.

pub mod engine;
pub mod handler;
pub mod prompts;
pub mod reporting;
pub mod resources;
pub mod state;
pub mod tools;

pub use handler::InkServer;
