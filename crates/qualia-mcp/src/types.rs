//! Types and data structures for the Model Context Protocol (MCP) server.
//!
//! Designed according to `AGENTS.md` §0-B (focused, single-purpose, <500 lines).

use serde::{Deserialize, Serialize};

/// Explicit operational states defining the execution boundaries.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum McpRuntimeState {
    HandshakePhase,
    AllocationFirewallActive,
    SanctuaryGated,
}

/// System and dispatch errors surfaced through standard MCP/JSON-RPC channels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpSystemError {
    SanctuaryGateTriggered,
    ToolNotFound,
    ToolNotReady,
    ParseError,
    IntentFrameViolation,
    FeatureNotEnabled,
    InvalidParameters,
    ExecutionFailed(String),
}

impl core::fmt::Display for McpSystemError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::SanctuaryGateTriggered => write!(f, "Sanctuary gate triggered"),
            Self::ToolNotFound => write!(f, "Tool not found"),
            Self::ToolNotReady => write!(f, "Tool not implemented on the current MCP surface"),
            Self::ParseError => write!(f, "Parse error"),
            Self::IntentFrameViolation => write!(f, "Intent frame violation"),
            Self::FeatureNotEnabled => write!(f, "Feature not enabled"),
            Self::InvalidParameters => write!(f, "Invalid parameters"),
            Self::ExecutionFailed(msg) => write!(f, "Execution failed: {msg}"),
        }
    }
}

impl std::error::Error for McpSystemError {}

/// Operational intent and security boundary context for tool invocations.
#[derive(Debug, Clone)]
pub struct McpIntentFrame {
    pub purpose_hash: u64,
    pub active_deontic_constraints: Vec<u64>,
    pub active_profile_id: Option<u64>,
    pub session_nonce: u64,
    /// Cryptographic 32-byte egress-override token. Fails closed when missing or zero.
    pub sanctuary_override: Option<[u8; 32]>,
    pub qpu_enabled: bool,
    pub llm_enabled: bool,
}

impl Default for McpIntentFrame {
    fn default() -> Self {
        Self {
            purpose_hash: 0,
            active_deontic_constraints: Vec::new(),
            active_profile_id: None,
            session_nonce: 0,
            sanctuary_override: None,
            qpu_enabled: true,
            llm_enabled: true,
        }
    }
}

/// Zero-deserialization view over an incoming `tools/call` byte buffer.
pub struct RawToolPayload<'a> {
    pub tool_name: &'a [u8],
    pub arguments_raw: &'a [u8],
}

/// Static descriptor for an advertised MCP tool.
#[derive(Clone, Copy, Debug)]
pub struct McpToolDescriptor {
    pub name: &'static str,
    pub description: &'static str,
    pub input_schema: &'static str,
}

/// Extract a raw JSON string value for `key` without heap allocation.
pub fn extract_raw_json_string<'a>(payload: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let mut i = 0;
    while i < payload.len() {
        if payload[i..].starts_with(key) {
            i += key.len();
            while i < payload.len() && (payload[i] == b' ' || payload[i] == b':') {
                i += 1;
            }
            if i < payload.len() && payload[i] == b'"' {
                i += 1;
                let start = i;
                while i < payload.len() && payload[i] != b'"' {
                    i += 1;
                }
                return Some(&payload[start..i]);
            }
        }
        i += 1;
    }
    None
}

/// Root storage dir: `QUALIA_STORAGE_PATH`, else `~/.qualia`
/// (HOME/USERPROFILE), else `./.qualia`.
pub fn qualia_storage_path() -> std::path::PathBuf {
    std::env::var("QUALIA_STORAGE_PATH")
        .unwrap_or_else(|_| {
            std::env::var("HOME")
                .or_else(|_| std::env::var("USERPROFILE"))
                .map(|h| format!("{h}/.qualia"))
                .unwrap_or_else(|_| ".qualia".to_string())
        })
        .into()
}

/// Decodes a hex string into raw bytes.
pub fn hex_decode(s: &str) -> Result<Vec<u8>, McpSystemError> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err(McpSystemError::InvalidParameters);
    }
    (0..s.len())
        .step_by(2)
        .map(|i| {
            u8::from_str_radix(&s[i..i + 2], 16).map_err(|_| McpSystemError::InvalidParameters)
        })
        .collect()
}

/// Helper to parse tool arguments from raw JSON bytes.
pub fn parse_tool_args(args: &[u8]) -> Result<serde_json::Value, McpSystemError> {
    if args.is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_slice(args).map_err(|_| McpSystemError::ParseError)
}

/// Helper to parse a slice of quins from JSON.
pub fn parse_quin_slice(v: &serde_json::Value, field: &str) -> Result<Vec<qualia_core_db::NQuin>, McpSystemError> {
    let arr = v.get(field).and_then(serde_json::Value::as_array).ok_or(McpSystemError::InvalidParameters)?;
    let mut quins = Vec::with_capacity(arr.len());
    for item in arr {
        let subject = item.get("subject").and_then(serde_json::Value::as_u64).unwrap_or(0);
        let predicate = item.get("predicate").and_then(serde_json::Value::as_u64).unwrap_or(0);
        let object = item.get("object").and_then(serde_json::Value::as_u64).unwrap_or(0);
        let context = item.get("context").and_then(serde_json::Value::as_u64).unwrap_or(0);
        let metadata = item.get("metadata").and_then(serde_json::Value::as_u64).unwrap_or(0);
        let parity = item.get("parity").and_then(serde_json::Value::as_u64).unwrap_or(subject ^ predicate ^ object ^ context ^ metadata);
        quins.push(qualia_core_db::NQuin { subject, predicate, object, context, metadata, parity });
    }
    Ok(quins)
}
