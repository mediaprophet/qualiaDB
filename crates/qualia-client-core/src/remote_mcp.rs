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

/// Build the JSON-RPC `tools/call` request body for an inference call.
///
/// The system prompt (if any) is prepended to the user prompt so the request works against any MCP
/// inference tool that accepts a single `prompt` string argument; `model` is passed through when set.
fn build_infer_request(
    infer_tool: &str,
    model: Option<&str>,
    system: Option<&str>,
    prompt: &str,
) -> serde_json::Value {
    let full = match system {
        Some(sys) if !sys.trim().is_empty() => format!("{sys}\n\n{prompt}"),
        _ => prompt.to_string(),
    };
    let mut args = serde_json::Map::new();
    args.insert("prompt".into(), serde_json::json!(full));
    if let Some(m) = model {
        if !m.is_empty() {
            args.insert("model".into(), serde_json::json!(m));
        }
    }
    serde_json::json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": { "name": infer_tool, "arguments": serde_json::Value::Object(args) }
    })
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

/// Run one inference over the configured MCP transport and return the completion text.
///
/// `infer_tool` defaults to [`DEFAULT_INFER_TOOL`] when `None`. This is a blocking call — the caller
/// should run it off the UI thread (the desktop command wrapper uses `spawn_blocking`).
pub fn remote_mcp_infer(
    transport: &McpTransport,
    infer_tool: Option<&str>,
    model: Option<&str>,
    system: Option<&str>,
    prompt: &str,
) -> Result<String, String> {
    let tool = infer_tool.unwrap_or(DEFAULT_INFER_TOOL);
    let req = build_infer_request(tool, model, system, prompt);
    let resp = match transport {
        McpTransport::Tcp { host, port } => call_tcp(host, *port, &req)?,
        McpTransport::Stdio { command, args } => call_stdio(command, args, &req)?,
        McpTransport::Http { url } => call_http(url, &req)?,
    };
    parse_infer_response(&resp)
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

fn call_http(url: &str, req: &serde_json::Value) -> Result<serde_json::Value, String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client
        .post(url)
        .header("content-type", "application/json")
        .header("accept", "application/json")
        .json(req)
        .send()
        .map_err(|e| format!("http post: {e}"))?;
    let status = resp.status();
    let text = resp.text().map_err(|e| e.to_string())?;
    if !status.is_success() {
        let snippet: String = text.chars().take(240).collect();
        return Err(format!("remote MCP HTTP {status}: {snippet}"));
    }
    serde_json::from_str(&text).map_err(|e| format!("parse http json: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_is_valid_tools_call() {
        let req = build_infer_request("llm_chat", Some("phi-3"), Some("Be terse."), "hi");
        assert_eq!(req["jsonrpc"], "2.0");
        assert_eq!(req["method"], "tools/call");
        assert_eq!(req["params"]["name"], "llm_chat");
        assert_eq!(req["params"]["arguments"]["model"], "phi-3");
        let prompt = req["params"]["arguments"]["prompt"].as_str().unwrap();
        assert!(prompt.starts_with("Be terse."));
        assert!(prompt.ends_with("hi"));
    }

    #[test]
    fn request_omits_empty_model_and_system() {
        let req = build_infer_request(DEFAULT_INFER_TOOL, None, None, "just this");
        assert!(req["params"]["arguments"].get("model").is_none());
        assert_eq!(req["params"]["arguments"]["prompt"], "just this");
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
