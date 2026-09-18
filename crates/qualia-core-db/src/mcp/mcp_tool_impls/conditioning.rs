use super::*;

fn json_to_vibe_value(v: &Value) -> vibe::Value {
    match v {
        Value::Null => vibe::Value::Null,
        Value::Bool(b) => vibe::Value::Bool(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                vibe::Value::I64(i)
            } else if let Some(u) = n.as_u64() {
                vibe::Value::U64(u)
            } else {
                vibe::Value::F64(n.as_f64().unwrap_or(0.0))
            }
        }
        Value::String(s) => vibe::Value::String(s.clone()),
        Value::Array(items) => vibe::Value::List(items.iter().map(json_to_vibe_value).collect()),
        Value::Object(map) => {
            let mut rec = std::collections::BTreeMap::new();
            for (k, val) in map {
                rec.insert(k.clone(), json_to_vibe_value(val));
            }
            vibe::Value::Record(rec)
        }
    }
}

fn vibe_value_to_json(v: &vibe::Value) -> Value {
    match v {
        vibe::Value::Null => Value::Null,
        vibe::Value::Bool(b) => Value::Bool(*b),
        vibe::Value::I64(i) => Value::Number((*i).into()),
        vibe::Value::U64(u) => Value::Number((*u).into()),
        vibe::Value::F64(f) => serde_json::Number::from_f64(*f)
            .map(Value::Number)
            .unwrap_or(Value::Null),
        vibe::Value::String(s) | vibe::Value::Iri(s) => Value::String(s.clone()),
        vibe::Value::List(items) => Value::Array(items.iter().map(vibe_value_to_json).collect()),
        vibe::Value::Record(map) => {
            let mut obj = serde_json::Map::new();
            for (k, val) in map {
                obj.insert(k.clone(), vibe_value_to_json(val));
            }
            Value::Object(obj)
        }
        _ => Value::String(format!("{:?}", v)),
    }
}

/// MCP tool: `conditioning_validate`
/// Validate a conditioning profile specification against schema and structural rules.
pub fn conditioning_validate(args: &[u8]) -> Result<String, McpSystemError> {
    let raw = parse_tool_args(args)?;
    let vibe_args = json_to_vibe_value(&raw);
    let span = vibe::Span { start: 0, end: 0 };
    match crate::poet_host::invoke::inference::conditioning_validate(
        &vibe_args, span,
    ) {
        Ok(result) => Ok(vibe_value_to_json(&result).to_string()),
        Err(_) => Err(McpSystemError::InvalidParameters),
    }
}

/// MCP tool: `conditioning_compile`
/// Compile a conditioning profile into an execution contract and budget plan.
pub fn conditioning_compile(args: &[u8]) -> Result<String, McpSystemError> {
    let raw = parse_tool_args(args)?;
    let vibe_args = json_to_vibe_value(&raw);
    let span = vibe::Span { start: 0, end: 0 };
    match crate::poet_host::invoke::inference::conditioning_compile(
        &vibe_args, span,
    ) {
        Ok(result) => Ok(vibe_value_to_json(&result).to_string()),
        Err(_) => Err(McpSystemError::InvalidParameters),
    }
}
