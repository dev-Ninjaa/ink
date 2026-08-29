//! End-to-end integration tests.
//!
//! Spawns the real `ink_mcp` binary and drives the Model Context Protocol over
//! stdio with raw JSON-RPC frames, verifying the advertised surface: tools,
//! resources, prompts, and error handling. The tests use the *installed*
//! `ink_mcp` binary built from this crate (`CARGO_BIN_EXE_ink_mcp`), so they
//! exercise the real handler, engine bridge, and transports without mocking.

use std::{collections::BTreeSet, process::Stdio, time::Duration};

use serde_json::{json, Value};
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, Command},
};

const TIMEOUT: Duration = Duration::from_secs(30);

/// Spawn the real `ink_mcp` binary with stdio transport.
fn spawn_server() -> Child {
    let exe = env!("CARGO_BIN_EXE_ink_mcp");
    Command::new(exe)
        .arg("--transport")
        .arg("stdio")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .expect("spawn ink_mcp server")
}

/// MCP session over the spawned server's stdio pipes.
struct McpClient {
    writer: tokio::process::ChildStdin,
    reader: BufReader<tokio::process::ChildStdout>,
    child: Child,
    next_id: u64,
}

impl McpClient {
    async fn connect() -> anyhow::Result<(Self, Value)> {
        let mut child = spawn_server();
        let writer = child.stdin.take().expect("server stdin");
        let reader = BufReader::new(child.stdout.take().expect("server stdout"));

        let mut client = Self {
            writer,
            reader,
            child,
            next_id: 1,
        };
        let initialize = client
            .request(json!({
                "jsonrpc": "2.0",
                "id": 0,
                "method": "initialize",
                "params": {
                    "protocolVersion": "2024-11-05",
                    "capabilities": {},
                    "clientInfo": { "name": "integration-test", "version": "0.0.0" }
                }
            }))
            .await?;
        client
            .notify(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
            .await?;
        Ok((client, initialize))
    }

    /// Send a request, read responses until `id`, and return its `result`.
    async fn request(&mut self, message: Value) -> anyhow::Result<Value> {
        let response = self.raw_request(message).await?;
        if let Some(error) = response.get("error") {
            anyhow::bail!("RPC error: {error}");
        }
        Ok(response.get("result").cloned().unwrap_or(Value::Null))
    }

    /// Send a request and return the full JSON-RPC response (error or result).
    async fn raw_request(&mut self, message: Value) -> anyhow::Result<Value> {
        let id = message
            .get("id")
            .and_then(Value::as_u64)
            .expect("request id");
        self.write(&message).await?;
        self.read_until_id(id).await
    }

    /// Send a notification (no response expected).
    async fn notify(&mut self, message: Value) -> anyhow::Result<()> {
        self.write(&message).await
    }

    async fn write(&mut self, message: &Value) -> anyhow::Result<()> {
        let serialized = serde_json::to_string(message)?;
        self.writer.write_all(serialized.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;
        Ok(())
    }

    async fn read_until_id(&mut self, expected_id: u64) -> anyhow::Result<Value> {
        let deadline = tokio::time::Instant::now() + TIMEOUT;
        loop {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                anyhow::bail!("timed out waiting for response id {expected_id}");
            }
            let mut line = String::new();
            let read_result =
                tokio::time::timeout(remaining, self.reader.read_line(&mut line)).await;
            let Ok(Ok(read)) = read_result else {
                anyhow::bail!("failed reading from server");
            };
            if read == 0 {
                anyhow::bail!("server closed stdout while waiting for id {expected_id}");
            }
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let value: Value = serde_json::from_str(trimmed)?;
            if let Some(id) = value.get("id").and_then(Value::as_u64) {
                if id == expected_id {
                    return Ok(value);
                }
            }
        }
    }

    async fn next_call(&mut self, method: &str, params: Value) -> anyhow::Result<Value> {
        let id = self.next_id;
        self.next_id += 1;
        self.request(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": method,
            "params": params,
        }))
        .await
    }

    async fn shutdown(mut self) {
        let _ = tokio::time::timeout(Duration::from_secs(2), self.child.wait()).await;
        if self.child.id().is_some() {
            let _ = self.child.kill().await;
        }
    }
}

fn tool_names(tools: &Value) -> BTreeSet<String> {
    tools
        .get("tools")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|t| t.get("name").and_then(Value::as_str).map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn resource_template_uris(templates: &Value) -> BTreeSet<String> {
    templates
        .get("resourceTemplates")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|t| {
                    t.get("uriTemplate")
                        .and_then(Value::as_str)
                        .map(String::from)
                })
                .collect()
        })
        .unwrap_or_default()
}

#[tokio::test]
async fn initialize_advertises_tools_resources_and_prompts() -> anyhow::Result<()> {
    let (client, result) = McpClient::connect().await?;
    client.shutdown().await;

    let capabilities = result.get("capabilities").expect("capabilities");
    assert!(capabilities.get("tools").is_some(), "tools capability");
    assert!(
        capabilities.get("resources").is_some(),
        "resources capability"
    );
    assert!(capabilities.get("prompts").is_some(), "prompts capability");

    let info = result.get("serverInfo").expect("serverInfo");
    assert_eq!(info.get("name").and_then(Value::as_str), Some("ink"));
    Ok(())
}

#[tokio::test]
async fn tools_list_exposes_all_ink_tools() -> anyhow::Result<()> {
    let (mut client, _) = McpClient::connect().await?;
    let result = client.next_call("tools/list", json!({})).await?;
    client.shutdown().await;

    let names = tool_names(&result);
    assert_eq!(
        names,
        BTreeSet::from([
            "analyze_repository".to_string(),
            "build_dependency_graph".to_string(),
            "optimize_context".to_string(),
            "schedule_agents".to_string(),
            "list_agents".to_string(),
            "advance_agents".to_string(),
            "get_cache_stats".to_string(),
            "clear_cache".to_string(),
            "generate_report".to_string(),
        ])
    );
    Ok(())
}

#[tokio::test]
async fn orchestration_tools_round_trip() -> anyhow::Result<()> {
    let (mut client, _) = McpClient::connect().await?;
    let root = std::env::var("CARGO_MANIFEST_DIR")?;

    let scheduled = client
        .next_call(
            "tools/call",
            json!({
                "name": "schedule_agents",
                "arguments": { "root": root, "max_agents": 2, "parallelism_enabled": true }
            }),
        )
        .await?;
    client.shutdown().await;

    assert_eq!(
        scheduled.get("isError").and_then(Value::as_bool),
        Some(false)
    );
    let text = scheduled
        .get("content")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .expect("tool text");
    let document: Value = serde_json::from_str(text)?;
    assert_eq!(
        document.get("accepted").and_then(Value::as_bool),
        Some(true)
    );
    assert!(document.get("agents").is_some());
    Ok(())
}

#[tokio::test]
async fn tools_call_reports_is_error_on_missing_root() -> anyhow::Result<()> {
    let (mut client, _) = McpClient::connect().await?;
    let result = client
        .next_call(
            "tools/call",
            json!({ "name": "analyze_repository", "arguments": {} }),
        )
        .await?;
    client.shutdown().await;

    // Missing argument -> the tool errors with isError true rather than failing
    // the JSON-RPC layer.
    assert_eq!(result.get("isError").and_then(Value::as_bool), Some(true));
    Ok(())
}

#[tokio::test]
async fn resources_lists_analysis_and_graph_templates() -> anyhow::Result<()> {
    let (mut client, _) = McpClient::connect().await?;
    let result = client
        .next_call("resources/templates/list", json!({}))
        .await?;
    client.shutdown().await;

    let uris = resource_template_uris(&result);
    assert_eq!(
        uris,
        BTreeSet::from([
            "ink://analysis/{root}".to_string(),
            "ink://graph/{root}".to_string(),
        ])
    );
    Ok(())
}

#[tokio::test]
async fn prompts_list_exposes_orchestrate_agent() -> anyhow::Result<()> {
    let (mut client, _) = McpClient::connect().await?;
    let result = client.next_call("prompts/list", json!({})).await?;
    client.shutdown().await;

    let names = result
        .get("prompts")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(|p| p.get("name").and_then(Value::as_str).map(String::from))
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    assert_eq!(names, BTreeSet::from(["orchestrate_agent".to_string()]));
    Ok(())
}

#[tokio::test]
async fn resources_read_analyzes_a_real_repository() -> anyhow::Result<()> {
    let (mut client, _) = McpClient::connect().await?;
    let root = std::env::var("CARGO_MANIFEST_DIR")?;
    let result = client
        .next_call(
            "resources/read",
            json!({ "uri": format!("ink://analysis/{root}") }),
        )
        .await?;
    client.shutdown().await;

    let contents = result
        .get("contents")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .and_then(|item| item.get("text"))
        .and_then(Value::as_str)
        .expect("resource text");
    let document: Value = serde_json::from_str(contents)?;
    assert!(document.get("analysis").is_some() || document.get("summary").is_some());
    Ok(())
}

#[tokio::test]
async fn resources_read_rejects_unknown_uri() -> anyhow::Result<()> {
    let (mut client, _) = McpClient::connect().await?;
    let id = client.next_id;
    client.next_id += 1;
    let response = client
        .raw_request(json!({
            "jsonrpc": "2.0",
            "id": id,
            "method": "resources/read",
            "params": { "uri": "ink://bogus/whatever" },
        }))
        .await?;
    client.shutdown().await;
    assert!(response.get("error").is_some(), "expected JSON-RPC error");
    Ok(())
}
