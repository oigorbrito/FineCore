use serde_json::{json, Value};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, ChildStdout};

use super::client::McpError;

pub(super) struct Wire {
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl Wire {
    pub(super) fn new(stdin: ChildStdin, stdout: ChildStdout) -> Self {
        Self {
            stdin,
            stdout: BufReader::new(stdout),
        }
    }

    pub(super) async fn send(&mut self, message: &Value) -> Result<(), McpError> {
        let mut line = message.to_string();
        line.push('\n');
        self.stdin
            .write_all(line.as_bytes())
            .await
            .map_err(map_stdio_io_error)?;
        self.stdin.flush().await.map_err(map_stdio_io_error)?;
        Ok(())
    }

    /// Read until the response for `id` arrives. Valid notifications are
    /// skipped, and server requests are answered inline so a `ping`
    /// mid-exchange cannot deadlock the lane. Non-JSON stdout diagnostics
    /// are tolerated; parsed JSON must be a valid JSON-RPC envelope.
    pub(super) async fn recv(&mut self, id: u64) -> Result<Value, McpError> {
        loop {
            let mut line = String::new();
            let bytes = self
                .stdout
                .read_line(&mut line)
                .await
                .map_err(map_stdio_io_error)?;
            if bytes == 0 {
                return Err(McpError::Protocol("server closed the connection".into()));
            }
            let Ok(message) = serde_json::from_str::<Value>(line.trim()) else {
                continue;
            };
            let object = message
                .as_object()
                .ok_or_else(|| McpError::Protocol("JSON-RPC message is not an object".into()))?;
            if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
                return Err(McpError::Protocol("invalid JSON-RPC version".into()));
            }
            if let Some(method) = object.get("method") {
                let method = method
                    .as_str()
                    .ok_or_else(|| McpError::Protocol("JSON-RPC method is not a string".into()))?;
                if let Some(request_id) = object.get("id") {
                    let reply = if method == "ping" {
                        json!({ "jsonrpc": "2.0", "id": request_id, "result": {} })
                    } else {
                        json!({ "jsonrpc": "2.0", "id": request_id,
                                "error": { "code": -32601, "message": "method not supported" } })
                    };
                    self.send(&reply).await?;
                }
                continue;
            }
            let response_id = object
                .get("id")
                .and_then(Value::as_u64)
                .ok_or_else(|| McpError::Protocol("JSON-RPC response has an invalid id".into()))?;
            if response_id != id {
                return Err(McpError::Protocol(format!(
                    "JSON-RPC response id {response_id} does not match request {id}"
                )));
            }
            let result = object.get("result");
            let error = object.get("error");
            if result.is_some() == error.is_some() {
                return Err(McpError::Protocol(
                    "JSON-RPC response must contain exactly one of result or error".into(),
                ));
            }
            if let Some(error) = error {
                let error = error
                    .as_object()
                    .ok_or_else(|| McpError::Protocol("JSON-RPC error is not an object".into()))?;
                if error.get("code").and_then(Value::as_i64).is_none() {
                    return Err(McpError::Protocol(
                        "JSON-RPC error code is not an integer".into(),
                    ));
                }
                let detail = error
                    .get("message")
                    .and_then(Value::as_str)
                    .ok_or_else(|| {
                        McpError::Protocol("JSON-RPC error message is not a string".into())
                    })?;
                return Err(McpError::Protocol(format!("server error: {detail}")));
            }
            return Ok(result.expect("validated result").clone());
        }
    }
}

fn map_stdio_io_error(error: std::io::Error) -> McpError {
    let closed = matches!(
        error.kind(),
        std::io::ErrorKind::BrokenPipe
            | std::io::ErrorKind::ConnectionAborted
            | std::io::ErrorKind::ConnectionReset
            | std::io::ErrorKind::UnexpectedEof
    );
    let context = if closed {
        "server closed the connection"
    } else {
        "stdio transport failed"
    };
    McpError::Protocol(format!("{context}: {error}"))
}
