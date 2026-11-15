//! MCP tools for zero-heap streaming parsing and serialization of RDF, JSON, and CSV formats.
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

/// `parse_rdf` tool — streaming zero-heap RDF/RDF-Star parser.
pub fn parse_rdf(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_format_impls::parse_rdf(args).map_err(map_db_error)
}

/// `parse_json` tool — streaming zero-heap JSON objects to Quins parser.
pub fn parse_json(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_format_impls::parse_json(args).map_err(map_db_error)
}

/// `parse_csv` tool — streaming zero-heap CSV to Quins parser.
pub fn parse_csv(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_format_impls::parse_csv(args).map_err(map_db_error)
}

/// `serialize_rdf` tool — zero-heap resolver-backed RDF/RDF-Star serializer.
pub fn serialize_rdf(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_format_impls::serialize_rdf(args).map_err(map_db_error)
}

/// `serialize_json` tool — zero-heap Quins to JSON array serializer.
pub fn serialize_json(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_format_impls::serialize_json(args).map_err(map_db_error)
}

/// `serialize_csv` tool — zero-heap Quins to CSV serializer.
pub fn serialize_csv(args: &[u8]) -> Result<String, McpSystemError> {
    qualia_core_db::mcp::mcp_format_impls::serialize_csv(args).map_err(map_db_error)
}
