use super::*;
use serde_json::{json, Value};

pub struct AuthoringServer<B> {
    backend: B,
    tools: Vec<AuthoringTool>,
    limits: ProtocolLimits,
    lifecycle: Lifecycle,
}

fn error(id: Value, code: i64, message: &'static str) -> Value {
    json!({"jsonrpc":"2.0", "id":id, "error":{"code":code,"message":message}})
}
fn result(id: Value, value: Value) -> Value {
    json!({"jsonrpc":"2.0", "id":id, "result":value})
}
fn valid_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && name
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'.'))
}

fn metadata_only(params: Option<&Value>) -> bool {
    params.map_or(true, |p| {
        p.as_object().is_some_and(|object| {
            object
                .iter()
                .all(|(key, value)| key == "_meta" && value.is_object())
        })
    })
}

impl<B: AuthoringBackend> AuthoringServer<B> {
    pub fn new(backend: B, limits: ProtocolLimits) -> Result<Self, ProtocolError> {
        let limits = limits.validate()?;
        let tools = backend.tools();
        if tools.len() > limits.max_tools
            || !super::types::within_budget(&tools, limits.max_message_bytes / 2)
        {
            return Err(ProtocolError::InvalidCatalog);
        }
        for (index, tool) in tools.iter().enumerate() {
            if !valid_name(&tool.name)
                || tool.description.is_empty()
                || tool.input_schema.get("type").and_then(Value::as_str) != Some("object")
                || tools[..index]
                    .iter()
                    .any(|previous| previous.name == tool.name)
            {
                return Err(ProtocolError::InvalidCatalog);
            }
        }
        Ok(Self {
            backend,
            tools,
            limits,
            lifecycle: Lifecycle::Uninitialized,
        })
    }

    pub fn lifecycle(&self) -> Lifecycle {
        self.lifecycle
    }
    pub fn into_backend(self) -> B {
        self.backend
    }

    /// One UTF-8 JSON-RPC message. The stdio transport separately enforces a
    /// terminating newline and closes on oversized/invalid UTF-8 frames.
    /// Valid notifications never invoke a backend tool and never get responses.
    pub fn process(&mut self, line: &str) -> Option<Value> {
        if line.len() > self.limits.max_message_bytes {
            return Some(error(
                Value::Null,
                -32600,
                "Message exceeds configured limit",
            ));
        }
        let request: Value = match super::codec::parse(line) {
            Ok(value) => value,
            Err(_) => return Some(error(Value::Null, -32700, "Parse error")),
        };
        let object = match request.as_object() {
            Some(object) => object,
            None => return Some(error(Value::Null, -32600, "Invalid Request")),
        };
        let id = object.get("id");
        let valid_id = match id {
            None => true,
            Some(Value::String(value)) => value.len() <= self.limits.max_id_bytes,
            Some(Value::Number(value)) => value.is_i64() || value.is_u64(),
            _ => false,
        };
        if !valid_id {
            return Some(error(Value::Null, -32600, "Invalid Request identifier"));
        }
        let method = object.get("method").and_then(Value::as_str);
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0")
            || method.is_none()
            || method == Some("")
            || object.contains_key("result")
            || object.contains_key("error")
        {
            return Some(error(
                id.cloned().unwrap_or(Value::Null),
                -32600,
                "Invalid Request",
            ));
        }
        let method = method.unwrap_or("");
        let params = object.get("params");
        if id.is_none() {
            if method == "notifications/initialized"
                && self.lifecycle == Lifecycle::AwaitingInitialized
                && metadata_only(params)
            {
                self.lifecycle = Lifecycle::Ready;
            }
            // Includes initialize/tools/call with missing IDs: never dispatch.
            return None;
        }
        let id = id.cloned().unwrap_or(Value::Null);
        let response = match method {
            "initialize" => self.initialize(id.clone(), params),
            "ping" => {
                if params.map_or(true, Value::is_object) {
                    result(id.clone(), json!({}))
                } else {
                    error(id.clone(), -32602, "Invalid params")
                }
            }
            "notifications/initialized" => {
                error(id.clone(), -32600, "Initialized must be a notification")
            }
            _ if self.lifecycle != Lifecycle::Ready => {
                error(id.clone(), -32002, "Server not initialized")
            }
            "tools/list" => {
                if metadata_only(params) {
                    result(id.clone(), json!({"tools": self.tools}))
                } else {
                    error(id.clone(), -32602, "Invalid params or unsupported cursor")
                }
            }
            "tools/call" => self.call(id.clone(), params),
            _ => error(id.clone(), -32601, "Method not found"),
        };
        if super::types::within_budget(&response, self.limits.max_message_bytes) {
            Some(response)
        } else {
            // A backend operation may already have completed. Never substitute
            // a success receipt, truncate source or claim the action rolled back.
            Some(error(
                id,
                -32603,
                "Response exceeds configured limit; inspect operation state",
            ))
        }
    }

    fn initialize(&mut self, id: Value, params: Option<&Value>) -> Value {
        if self.lifecycle != Lifecycle::Uninitialized {
            return error(id, -32600, "Already initialized");
        }
        let Some(params) = params.and_then(Value::as_object) else {
            return error(id, -32602, "Invalid initialize params");
        };
        if !params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .is_some_and(|s| !s.is_empty())
            || !params.get("capabilities").is_some_and(Value::is_object)
            || !params.get("clientInfo").is_some_and(|info| {
                info.get("name")
                    .and_then(Value::as_str)
                    .is_some_and(|s| !s.is_empty())
                    && info
                        .get("version")
                        .and_then(Value::as_str)
                        .is_some_and(|s| !s.is_empty())
            })
        {
            return error(id, -32602, "Invalid initialize params");
        }
        // Supported requested version is echoed. All other requests negotiate
        // the sole supported version; incompatible clients should disconnect.
        self.lifecycle = Lifecycle::AwaitingInitialized;
        result(
            id,
            json!({"protocolVersion":AUTHORING_PROTOCOL_VERSION,
            "capabilities":{"tools":{"listChanged":false}},
            "serverInfo":{"name":"qualia-mcp-authoring", "version":env!("CARGO_PKG_VERSION")}}),
        )
    }

    fn call(&mut self, id: Value, params: Option<&Value>) -> Value {
        let Some(params) = params.and_then(Value::as_object) else {
            return error(id, -32602, "Invalid tool params");
        };
        let Some(name) = params.get("name").and_then(Value::as_str) else {
            return error(id, -32602, "Tool name required");
        };
        if params.contains_key("task") {
            return error(id, -32602, "Task execution unsupported");
        }
        if !self.tools.iter().any(|tool| tool.name == name) {
            return error(id, -32602, "Unknown tool");
        }
        let args = match params.get("arguments") {
            None => json!({}),
            Some(value) if value.is_object() => value.clone(),
            _ => return error(id, -32602, "Tool arguments must be an object"),
        };
        let (value, is_error) = match self.backend.call(name, args) {
            Ok(value) if value.is_object() => (value, false),
            Ok(_) => (
                json!({"error":{"code":"invalid_backend_result", "message":"Backend result must be an object"}}),
                true,
            ),
            Err(tool_error) => {
                if tool_error.code.is_empty()
                    || tool_error.code.len() > 128
                    || tool_error.message.is_empty()
                    || tool_error.message.len() > 4096
                {
                    (
                        json!({"error":{"code":"backend_error", "message":"Operation failed"}}),
                        true,
                    )
                } else {
                    (json!({"error":tool_error}), true)
                }
            }
        };
        // Bound before making the compatible text serialization of the object.
        if !super::types::within_budget(&value, self.limits.max_message_bytes / 8) {
            return error(
                id,
                -32603,
                "Result exceeds configured limit; inspect operation state",
            );
        }
        let text = match serde_json::to_string(&value) {
            Ok(text) => text,
            Err(_) => return error(id, -32603, "Result serialization failed"),
        };
        result(
            id,
            json!({"content":[{"type":"text", "text":text}],
            "structuredContent":value, "isError":is_error}),
        )
    }
}
