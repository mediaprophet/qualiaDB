//! MCP tools for cooperative governance, jural correlatives, and values checking.
//!
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::types::McpSystemError;

fn map_db_error(e: qualia_core_db::mcp::mcp_server::McpSystemError) -> McpSystemError {
    match e {
        qualia_core_db::mcp::mcp_server::McpSystemError::InvalidParameters => {
            McpSystemError::InvalidParameters
        }
        qualia_core_db::mcp::mcp_server::McpSystemError::ParseError => McpSystemError::ParseError,
        qualia_core_db::mcp::mcp_server::McpSystemError::SanctuaryGateTriggered => {
            McpSystemError::SanctuaryGateTriggered
        }
        _ => McpSystemError::ToolNotReady,
    }
}

/// `values_check` tool — human-rights dignity rights guard.
pub fn values_check(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::values_check(args).map_err(map_db_error)
}

/// `values_evaluate` tool — deontic contract evaluation in values terms.
pub fn values_evaluate(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::values_evaluate(args).map_err(map_db_error)
}

/// `jural_correlate` tool — Hohfeldian jural correlation and opposites.
pub fn jural_correlate(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::jural_correlate(args).map_err(map_db_error)
}

/// `deontic_govern` tool — runtime PolicyMode mapping for Webizen VM.
pub fn deontic_govern(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::deontic_govern(args).map_err(map_db_error)
}

/// `mcp_cooperate` tool — Track-M agent cooperation gate.
pub fn mcp_cooperate(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_tool_impls::mcp_cooperate(args).map_err(map_db_error)
}
