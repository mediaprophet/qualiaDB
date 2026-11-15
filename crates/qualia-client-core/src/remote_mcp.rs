//! Remote-MCP inference backend — reach an external provider (Claude / Google / X, or another
//! Webizen node) over the Model Context Protocol to run a completion on the person's behalf.
//!
//! Local inference is PREFERRED; this is the opt-in, costly path (Timothy's directive: local-first,
//! external-via-MCP when wanted/needed; future provider credentials slot in behind the same seam).
//! Native-only — it does network / process I/O and is never part of the wasm bundle.
//!
//! It issues an MCP `tools/call` to a configured inference tool (default `llm_infer`) and extracts the
//! text from the MCP content result. Three transports, mirroring how the rest of the platform speaks
//! MCP: **TCP** (newline-delimited JSON-RPC, exactly what the Webizen desktop MCP server on `:4245`
//! serves), **Stdio** (spawn an MCP server command), and **HTTP** (JSON-RPC POST).

use crate::agent_registry::McpTransport;
use std::io::{BufRead, BufReader, Write};
use std::time::Duration;

/// Default MCP tool name to call for inference (the Webizen MCP surface exposes `llm_infer`).
pub const DEFAULT_INFER_TOOL: &str = "llm_infer";

/// Outcome of one remote inference call: the text plus the lowering receipt
/// describing how the request reached the tool's schema.
#[derive(Debug)]
pub struct RemoteInferOutcome {
    pub text: String,
    pub lowering: crate::conditioning::McpLoweringReceipt,
}

/// Build the JSON-RPC `tools/call` request body for an inference call.
///
/// When the tool's schema supports a dedicated `system` argument the request
/// keeps role separation (`supports_system_role == Some(true)`); otherwise the
/// system text is flattened into `prompt` and the returned receipt records the
/// degradation. `None` means the capability was not declared or discoverable —
/// conservative flattening, reported as such.
fn build_infer_request(
    infer_tool: &str,
    model: Option<&str>,
    system: Option<&str>,
    prompt: &str,
    supports_system_role: Option<bool>,
) -> (serde_json::Value, crate::conditioning::McpLoweringReceipt) {
    let (mut args, mut receipt) = crate::conditioning::lower_mcp_tool_arguments(
        system,
        prompt,
        supports_system_role.unwrap_or(false),
    );
    if supports_system_role.is_none() && system.is_some() {
        receipt.degradation_reason =
            Some("system-role capability not declared or discoverable; flattened into prompt");
    }
    if let Some(m) = model {
        if !m.is_empty() {
            args["model"] = serde_json::json!(m);
        }
    }
    (
        serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "tools/call",
            "params": { "name": infer_tool, "arguments": args }
        }),
        receipt,
    )
}

/// Extract the text output from an MCP `tools/call` JSON-RPC response, tolerant of shape variation.
fn parse_infer_response(resp: &serde_json::Value) -> Result<String, String> {
    if let Some(err) = resp.get("error") {
        let msg = err
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("remote MCP error");
        return Err(format!("remote MCP error: {msg}"));
    }
    let result = resp
        .get("result")
        .ok_or_else(|| "remote MCP response missing `result`".to_string())?;

    // Canonical MCP content format: result.content = [{ type:"text", text:"…" }, …]
    if let Some(content) = result.get("content").and_then(|c| c.as_array()) {
        let mut out = String::new();
        for part in content {
            if let Some(t) = part.get("text").and_then(|v| v.as_str()) {
                out.push_str(t);
            }
        }
        if !out.is_empty() {
            return Ok(out);
        }
    }
    // Fallbacks for simpler servers.
    if let Some(t) = result.as_str() {
        return Ok(t.to_string());
    }
    for k in ["text", "output", "completion", "response"] {
        if let Some(t) = result.get(k).and_then(|v| v.as_str()) {
            return Ok(t.to_string());
        }
    }
    if result
        .get("isError")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
    {
        return Err("remote MCP tool reported an error".to_string());
    }
    Err("remote MCP response had no text content".to_string())
}

/// Run one inference over the configured MCP transport and return the
/// completion plus the role-lowering receipt.
///
/// `infer_tool` defaults to [`DEFAULT_INFER_TOOL`] when `None`.
/// `supports_system_role` is the tool-schema capability: `Some` from the
/// agent's declared configuration or [`remote_mcp_tool_supports_system_role`]
/// discovery, `None` to flatten conservatively. This is a blocking call — the
/// caller should run it off the UI thread (the desktop command wrapper uses
/// `spawn_blocking`).
pub fn remote_mcp_infer(
    transport: &McpTransport,
    infer_tool: Option<&str>,
    model: Option<&str>,
    system: Option<&str>,
    prompt: &str,
    supports_system_role: Option<bool>,
) -> Result<RemoteInferOutcome, String> {
    let tool = infer_tool.unwrap_or(DEFAULT_INFER_TOOL);
    let (req, lowering) = build_infer_request(tool, model, system, prompt, supports_system_role);
    let resp = match transport {
        McpTransport::Tcp { host, port } => call_tcp(host, *port, &req)?,
        McpTransport::Stdio { command, args } => call_stdio(command, args, &req)?,
        McpTransport::Http { url, credential_id } => {
            call_http(url, credential_id.as_deref(), &req)?
        }
    };
    Ok(RemoteInferOutcome {
        text: parse_infer_response(&resp)?,
        lowering,
    })
}

/// Discover whether the named inference tool's declared `inputSchema` accepts
/// a dedicated `system` argument, via a `tools/list` round-trip.
/// `None` = capability unknown (endpoint error or tool absent); callers fall
/// back to flattened arguments.
pub fn remote_mcp_tool_supports_system_role(
    transport: &McpTransport,
    tool_name: &str,
) -> Option<bool> {
    let req = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}
    });
    let response = match transport {
        McpTransport::Tcp { host, port } => call_tcp(host, *port, &req).ok()?,
        McpTransport::Stdio { command, args } => call_stdio(command, args, &req).ok()?,
        McpTransport::Http { url, credential_id } => {
            call_http(url, credential_id.as_deref(), &req).ok()?
        }
    };
    let tools = response
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(|tools| tools.as_array())?;
    let tool = tools
        .iter()
        .find(|tool| tool.get("name").and_then(|n| n.as_str()) == Some(tool_name))?;
    let properties = tool
        .get("inputSchema")
        .and_then(|schema| schema.get("properties"))
        .and_then(|props| props.as_object())?;
    Some(properties.contains_key("system"))
}

fn call_tcp(host: &str, port: u16, req: &serde_json::Value) -> Result<serde_json::Value, String> {
    use std::net::TcpStream;
    let stream =
        TcpStream::connect((host, port)).map_err(|e| format!("connect {host}:{port}: {e}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(120))).ok();
    stream.set_write_timeout(Some(Duration::from_secs(30))).ok();
    let mut writer = stream.try_clone().map_err(|e| e.to_string())?;
    let mut reader = BufReader::new(stream);
    let line = serde_json::to_string(req).map_err(|e| e.to_string())?;
    writer
        .write_all(line.as_bytes())
        .map_err(|e| e.to_string())?;
    writer.write_all(b"\n").map_err(|e| e.to_string())?;
    writer.flush().ok();
    // Read JSON-RPC response lines until one carries our result/error (skip any notifications).
    let mut buf = String::new();
    for _ in 0..100 {
        buf.clear();
        let n = reader
            .read_line(&mut buf)
            .map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            break;
        }
        let t = buf.trim();
        if t.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
            if v.get("result").is_some() || v.get("error").is_some() {
                return Ok(v);
            }
        }
    }
    Err("no JSON-RPC response from remote MCP (TCP)".into())
}

fn call_stdio(
    command: &str,
    args: &[String],
    req: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    use std::process::{Command, Stdio};
    let mut child = Command::new(command)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| format!("spawn {command}: {e}"))?;
    {
        let stdin = child.stdin.as_mut().ok_or("no stdin on MCP child")?;
        let line = serde_json::to_string(req).map_err(|e| e.to_string())?;
        stdin
            .write_all(line.as_bytes())
            .map_err(|e| e.to_string())?;
        stdin.write_all(b"\n").map_err(|e| e.to_string())?;
        stdin.flush().ok();
    }
    let stdout = child.stdout.take().ok_or("no stdout on MCP child")?;
    let mut reader = BufReader::new(stdout);
    let mut buf = String::new();
    let mut found = None;
    for _ in 0..200 {
        buf.clear();
        let n = reader
            .read_line(&mut buf)
            .map_err(|e| format!("read: {e}"))?;
        if n == 0 {
            break;
        }
        let t = buf.trim();
        if t.is_empty() {
            continue;
        }
        if let Ok(v) = serde_json::from_str::<serde_json::Value>(t) {
            if v.get("result").is_some() || v.get("error").is_some() {
                found = Some(v);
                break;
            }
        }
    }
    let _ = child.kill();
    let _ = child.wait();
    found.ok_or_else(|| "no JSON-RPC response from stdio MCP server".to_string())
}

fn call_http(
    url: &str,
    credential_id: Option<&str>,
    req: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let mut request = client
        .post(url)
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .json(req);
    if let Some(connection) = credential_id.filter(|value| !value.trim().is_empty()) {
        // The credential exists only in the platform keychain and is read at
        // dispatch time after the caller has obtained consent.
        let secret = crate::provider_credentials::bearer_credential(connection)?;
        request = request.bearer_auth(secret);
    }
    let resp = request.send().map_err(|e| format!("http post: {e}"))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| e.to_string())?;
    if !status.is_success() {
        let snippet: String = text.chars().take(240).collect();
        return Err(format!("remote MCP HTTP {status}: {snippet}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("parse http json: {e}"))
}

/// Test an MCP endpoint without asking it to generate text.  It sends the
/// standard `tools/list` request and only reports that a response was received.
pub fn remote_mcp_probe(transport: &McpTransport) -> Result<usize, String> {
    let req = serde_json::json!({
        "jsonrpc": "2.0", "id": 1, "method": "tools/list", "params": {}
    });
    let response = match transport {
        McpTransport::Tcp { host, port } => call_tcp(host, *port, &req)?,
        McpTransport::Stdio { command, args } => call_stdio(command, args, &req)?,
        McpTransport::Http { url, credential_id } => {
            call_http(url, credential_id.as_deref(), &req)?
        }
    };
    if let Some(error) = response.get("error") {
        let message = error
            .get("message")
            .and_then(|value| value.as_str())
            .unwrap_or("MCP error");
        return Err(format!("MCP tools/list failed: {message}"));
    }
    let tools = response
        .get("result")
        .and_then(|result| result.get("tools"))
        .and_then(|tools| tools.as_array())
        .ok_or_else(|| "MCP tools/list response had no tools array".to_string())?;
    Ok(tools.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_valid_tools_call() {
        let (req, receipt) =
            build_infer_request("llm_chat", Some("phi-3"), Some("Be terse."), "hi", None);
        assert_eq!(req["jsonrpc"], "2.0");
        assert_eq!(req["method"], "tools/call");
        assert_eq!(req["params"]["name"], "llm_chat");
        assert_eq!(req["params"]["arguments"]["model"], "phi-3");
        let prompt = req["params"]["arguments"]["prompt"].as_str().unwrap();
        assert!(prompt.starts_with("Be terse."));
        assert!(prompt.ends_with("hi"));
        assert_eq!(
            receipt.mode,
            crate::conditioning::McpLoweringMode::Flattened
        );
        assert!(receipt.role_degraded);
        assert_eq!(
            receipt.degradation_reason,
            Some("system-role capability not declared or discoverable; flattened into prompt")
        );
    }

    #[test]
    fn request_omits_empty_model_and_system() {
        let (req, receipt) = build_infer_request(DEFAULT_INFER_TOOL, None, None, "just this", None);
        assert!(req["params"]["arguments"].get("model").is_none());
        assert_eq!(req["params"]["arguments"]["prompt"], "just this");
        assert!(!receipt.role_degraded);
    }

    #[test]
    fn structured_request_keeps_roles_when_schema_supports_system() {
        let (req, receipt) =
            build_infer_request("llm_chat", None, Some("Be terse."), "hi", Some(true));
        assert_eq!(req["params"]["arguments"]["system"], "Be terse.");
        assert_eq!(req["params"]["arguments"]["prompt"], "hi");
        assert_eq!(
            receipt.mode,
            crate::conditioning::McpLoweringMode::Structured
        );
        assert!(!receipt.role_degraded);
        assert!(receipt.degradation_reason.is_none());
    }

    #[test]
    fn declared_unsupported_schema_flattens_with_reason() {
        let (req, receipt) =
            build_infer_request("llm_chat", None, Some("Be terse."), "hi", Some(false));
        assert!(req["params"]["arguments"].get("system").is_none());
        assert!(req["params"]["arguments"]["prompt"]
            .as_str()
            .unwrap()
            .starts_with("Be terse."));
        assert!(receipt.role_degraded);
        assert_eq!(
            receipt.degradation_reason,
            Some("MCP tool schema lacks dedicated system parameter; flattened into prompt")
        );
    }

    #[test]
    fn parses_mcp_content_array() {
        let resp = serde_json::json!({
            "jsonrpc": "2.0", "id": 1,
            "result": { "content": [ {"type":"text","text":"Hello"}, {"type":"text","text":", world"} ] }
        });
        assert_eq!(parse_infer_response(&resp).unwrap(), "Hello, world");
    }

    #[test]
    fn parses_simple_fallbacks() {
        let a = serde_json::json!({ "result": "plain string" });
        assert_eq!(parse_infer_response(&a).unwrap(), "plain string");
        let b = serde_json::json!({ "result": { "text": "keyed text" } });
        assert_eq!(parse_infer_response(&b).unwrap(), "keyed text");
    }

    #[test]
    fn surfaces_errors() {
        let e = serde_json::json!({ "error": { "code": -32000, "message": "boom" } });
        assert!(parse_infer_response(&e).unwrap_err().contains("boom"));
        let ie = serde_json::json!({ "result": { "isError": true, "content": [] } });
        assert!(parse_infer_response(&ie).is_err());
    }
}
