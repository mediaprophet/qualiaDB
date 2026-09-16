//! MCP lowering adapter with explicit role degradation reporting.

use serde_json::{json, Value};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum McpLoweringMode {
    Structured,
    Flattened,
}

pub struct McpLoweringReceipt {
    pub mode: McpLoweringMode,
    pub role_degraded: bool,
    pub degradation_reason: Option<&'static str>,
}

/// Lower system prompt and user objective into MCP tool call arguments.
/// Accurately flags role degradation when flattening into a single string.
pub fn lower_mcp_tool_arguments(
    system_prompt: Option<&str>,
    user_prompt: &str,
    tool_schema_supports_system_role: bool,
) -> (Value, McpLoweringReceipt) {
    if tool_schema_supports_system_role {
        let val = json!({
            "system": system_prompt.unwrap_or(""),
            "prompt": user_prompt,
        });
        (
            val,
            McpLoweringReceipt {
                mode: McpLoweringMode::Structured,
                role_degraded: false,
                degradation_reason: None,
            },
        )
    } else {
        let flattened = match system_prompt {
            Some(sys) if !sys.trim().is_empty() => format!("{sys}\n\n{user_prompt}"),
            _ => user_prompt.to_string(),
        };
        let val = json!({
            "prompt": flattened,
        });
        (
            val,
            McpLoweringReceipt {
                mode: McpLoweringMode::Flattened,
                role_degraded: system_prompt.is_some(),
                degradation_reason: if system_prompt.is_some() {
                    Some("MCP tool schema lacks dedicated system parameter; flattened into prompt")
                } else {
                    None
                },
            },
        )
    }
}
