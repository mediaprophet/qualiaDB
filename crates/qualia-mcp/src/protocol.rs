//! JSON-RPC 2.0 Protocol Handler for Model Context Protocol (MCP).
//!
//! Complies with MCP specification (protocol version "2024-11-05").
//! Strictly adheres to JSON-RPC 2.0 notification rules: notifications (requests without `id`)
//! NEVER receive a response, preventing client socket closure.
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::dispatch::enforce_fiduciary_tool_dispatch;
use crate::registry::tool_list_json;
use crate::resources::{get_qapp_surface_schema, QAPP_SURFACE_SCHEMA_URI};
use crate::types::{McpIntentFrame, McpSystemError, RawToolPayload};
use serde_json::{json, Value};

pub const MCP_PROTOCOL_VERSION: &str = "2024-11-05";

/// Handle a single incoming JSON-RPC 2.0 line.
/// Returns `Some(response_json)` if the request requires a response, or `None` if it was a notification.
pub fn process_jsonrpc_message(
    line: &str,
    intent_frame: &McpIntentFrame,
) -> Option<String> {
    let req: Value = match serde_json::from_str(line) {
        Ok(v) => v,
        Err(_) => {
            return Some(
                json!({
                    "jsonrpc": "2.0",
                    "id": null,
                    "error": {
                        "code": -32700,
                        "message": "Parse error"
                    }
                })
                .to_string(),
            );
        }
    };

    let id = req.get("id");
    let is_notification = id.is_none() || id == Some(&Value::Null);
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");

    // JSON-RPC 2.0: notifications never receive responses
    if is_notification {
        log::debug!("Received MCP notification: method='{}'", method);
        return None;
    }

    let req_id = id.cloned().unwrap_or(Value::Null);

    let result = match method {
        "initialize" => {
            let client_version = req
                .get("params")
                .and_then(|p| p.get("protocolVersion"))
                .and_then(Value::as_str)
                .unwrap_or(MCP_PROTOCOL_VERSION);

            let negotiated_version = match client_version {
                "2025-11-25" | "2025-06-18" | "2025-03-26" | "2024-11-05" => client_version,
                _ => MCP_PROTOCOL_VERSION,
            };

            log::info!(
                "MCP Handshake initialized with client protocolVersion='{}', negotiated='{}'",
                client_version,
                negotiated_version
            );

            Ok(json!({
                "protocolVersion": negotiated_version,
                "capabilities": {
                    "tools": { "listChanged": false },
                    "resources": { "subscribe": false, "listChanged": false },
                    "prompts": { "listChanged": false }
                },
                "serverInfo": {
                    "name": "qualia-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }))
        }

        "prompts/list" => Ok(json!({ "prompts": [] })),

        "ping" => Ok(json!({})),

        "tools/list" => Ok(tool_list_json()),

        "tools/call" => {
            let params = req.get("params").ok_or(McpSystemError::InvalidParameters);
            match params {
                Ok(p) => {
                    let tool_name = p.get("name").and_then(Value::as_str).unwrap_or("");
                    let default_args = json!({});
                    let arguments_val = p.get("arguments").unwrap_or(&default_args);
                    let arguments_raw = arguments_val.to_string();

                    let payload = RawToolPayload {
                        tool_name: tool_name.as_bytes(),
                        arguments_raw: arguments_raw.as_bytes(),
                    };

                    match enforce_fiduciary_tool_dispatch(payload, intent_frame) {
                        Ok(text_output) => Ok(json!({
                            "content": [
                                {
                                    "type": "text",
                                    "text": text_output
                                }
                            ],
                            "isError": false
                        })),
                        Err(e) => {
                            let error_msg = format!("Tool '{tool_name}' execution error: {e:?}");
                            Ok(json!({
                                "content": [
                                    {
                                        "type": "text",
                                        "text": error_msg
                                    }
                                ],
                                "isError": true
                            }))
                        }
                    }
                }
                Err(e) => Err(e),
            }
        }

        "resources/list" => Ok(json!({
            "resources": [
                {
                    "uri": QAPP_SURFACE_SCHEMA_URI,
                    "name": "Qualia QApp Host Surface Schema",
                    "mimeType": "application/json"
                }
            ]
        })),

        "resources/read" => {
            let uri = req
                .get("params")
                .and_then(|p| p.get("uri"))
                .and_then(Value::as_str)
                .unwrap_or("");

            if uri == QAPP_SURFACE_SCHEMA_URI {
                Ok(json!({
                    "contents": [
                        {
                            "uri": QAPP_SURFACE_SCHEMA_URI,
                            "mimeType": "application/json",
                            "text": get_qapp_surface_schema()
                        }
                    ]
                }))
            } else {
                Err(McpSystemError::InvalidParameters)
            }
        }

        _ => {
            return Some(
                json!({
                    "jsonrpc": "2.0",
                    "id": req_id,
                    "error": {
                        "code": -32601,
                        "message": format!("Method not found: '{method}'")
                    }
                })
                .to_string(),
            );
        }
    };

    match result {
        Ok(val) => Some(
            json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "result": val
            })
            .to_string(),
        ),
        Err(e) => Some(
            json!({
                "jsonrpc": "2.0",
                "id": req_id,
                "error": {
                    "code": -32602,
                    "message": format!("Invalid params: {e:?}")
                }
            })
            .to_string(),
        ),
    }
}
