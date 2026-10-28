//! Exact Studio export request aliases and error propagation support.
use crate::types::McpSystemError;
use serde_json::Value;

// Studio.export's Core owner uses `id` and `format`; this MCP surface also
// historically accepts `artifact_id` and `target_format`. Bind each alias to
// exactly one value before calling the owner instead of defaulting silently.
pub(super) fn studio_export_args(payload: &Value) -> Result<Value, McpSystemError> {
    let mut fields = payload.as_object().ok_or(McpSystemError::InvalidParameters)?.clone();
    for (canonical, alias) in [("id", "artifact_id"), ("format", "target_format")] {
        for key in [canonical, alias] {
            if fields.get(key).is_some_and(|value| !value.is_string()) {
                return Err(McpSystemError::InvalidParameters);
            }
        }
        if let Some(alias_value) = fields.get(alias).cloned() {
            match fields.get(canonical) {
                Some(value) if value != &alias_value => {
                    return Err(McpSystemError::ExecutionFailed(format!("Studio.export has conflicting {canonical} and {alias}")));
                }
                None => { fields.insert(canonical.into(), alias_value); }
                _ => {}
            }
        }
    }
    Ok(Value::Object(fields))
}

#[cfg(test)]
#[path = "poet_export_tests.rs"]
mod tests;
