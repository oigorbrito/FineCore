//! Offline, asserted host assembly: prepare, review, and archive a release checklist.
//!
//! Exercises SDK state, tools/policy, memory/skills, real local stdio MCP, sessions,
//! streamed run events, recovery, and session-owned background orchestration.
//! Models are deterministic test doubles, not providers. This is NOT coverage of
//! every provider/integration: no credentials, HTTP MCP, web, remote skills,
//! containers, or language kernels. The OS process demo requires a POSIX shell.
//! The same executable has a private MCP-server mode; no Python/Node is needed.

mod support;

use std::io::{BufRead, Write};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::Duration;

use async_trait::async_trait;
use orca_harness_core::testing::{call, ScriptedModel};
use orca_harness_sdk::integrations::mcp::StdioLaunch;
use orca_harness_sdk::orchestration::{ProcessSpawn, RunState, SubagentModel, SubagentRequest};
use orca_harness_sdk::*;
use serde_json::{json, Value};
use support::TempWorkspace;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;
const WAIT: Duration = Duration::from_secs(10);

#[tokio::main]
async fn main() -> Result {
    if std::env::args().nth(1).as_deref() == Some("--release-mcp") {
        return serve_mcp();
    }
    let workspace = TempWorkspace::new("host-assembly");
    let harness = Harness::builder()
        .workspace(workspace.path())
        .state_dir(workspace.path().join(".orca"))
        .build()?;
    assert_eq!(harness.workspace().root(), workspace.path().canonicalize()?);
    println!(
        "Release workspace: {}",
        harness.workspace().root().display()
    );
    let mcp = harness.mcp();
    let status = mcp
        .connect_stdio(
            "release",
            &StdioLaunch {
                command: std::env::current_exe()?.to_string_lossy().into_owned(),
                args: vec!["--release-mcp".into()],
                env: Default::default(),
                cwd: None,
                environment: Default::default(),
            },
        )
        .await?;
    assert!(status.healthy);
    assert_eq!(status.tool_count, 1);

    // Always disconnect, even when a fallible phase returns an error. Sessions
    // likewise shut down before propagating phase errors; workspace is RAII-owned.
    let result = async {
        let id = prepare(&harness, mcp.clone()).await?;
        recover_and_archive(&harness, &id).await?;
        interrupted_checks(&harness).await?;
        review(&harness).await
    }
    .await;
    assert!(mcp.disconnect("release"));
    assert!(mcp.servers().is_empty());
    result?;
    println!("All host assembly assertions passed; sessions closed and MCP disconnected.");
    Ok(())
}

/// A minimal JSON-lines MCP peer: preserve request IDs and implement a real
/// tools/call, rather than shadowing a remote tool with a host FnTool.
fn serve_mcp() -> Result {
    let mut out = std::io::stdout().lock();
    for line in std::io::stdin().lock().lines() {
        let request: Value = serde_json::from_str(&line?)?;
        let Some(id) = request.get("id") else {
            continue;
        };
        let result = match request["method"].as_str().unwrap_or("") {
            "initialize" => {
                json!({"protocolVersion":"2025-06-18", "capabilities":{"tools":{}},
                "serverInfo":{"name":"release-fixture","version":"1"}})
            }
            "tools/list" => {
                json!({"tools":[{"name":"check", "description":"Check a release tag",
                "inputSchema":{"type":"object","properties":{"tag":{"type":"string"}},"required":["tag"]}}]})
            }
            "tools/call" => {
                assert_eq!(request["params"]["name"], "check");
                let tag = request["params"]["arguments"]["tag"].as_str().unwrap();
                json!({
                    "content": [{
                        "type": "text",
                        "text": format!("verified tag {tag}"),
                    }],
                    "isError": false,
                })
            }
            _ => {
                writeln!(
                    out,
                    "{}",
                    json!({"jsonrpc":"2.0","id":id,"error":{"code":-32601,"message":"unsupported method"}})
                )?;
                out.flush()?;
                continue;
            }
        };
        writeln!(out, "{}", json!({"jsonrpc":"2.0","id":id,"result":result}))?;
        out.flush()?;
    }
    Ok(())
}

fn step(calls: Vec<ToolCall>) -> ModelResponse {
    ModelResponse::ToolCalls {
        content: None,
        calls,
        usage: Some(Usage {
            input_tokens: 10,
            output_tokens: 3,
            ..Default::default()
        }),
    }
}

fn tool_output(messages: &[Message], id: &str) -> Value {
    messages
        .iter()
        .find_map(|m| match m {
            Message::Tool { results } => results.iter().find(|r| r.call_id == id).map(|r| {
                assert!(!r.is_error, "{r:?}");
                r.output.clone()
            }),
            _ => None,
        })
        .expect("tool result must be in transcript")
}

include!("host_assembly/phases.rs");
