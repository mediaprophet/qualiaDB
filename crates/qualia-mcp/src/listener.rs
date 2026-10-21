//! Async stdio listener loop for Model Context Protocol (MCP) server.
//!
//! Follows `AGENTS.md` §0-B (<500 lines).

use crate::protocol::process_jsonrpc_message;
use crate::types::McpIntentFrame;
use std::io::Write;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Starts the MCP server on standard input/output with specified hardware flags.
pub async fn start_mcp_listener_with_flags(
    enable_qpu: bool,
    enable_llm: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let intent_frame = McpIntentFrame {
        purpose_hash: 0,
        active_deontic_constraints: Vec::new(),
        active_profile_id: None,
        session_nonce: 0x4242,
        sanctuary_override: None,
        qpu_enabled: enable_qpu,
        llm_enabled: enable_llm,
    };

    let stdin = tokio::io::stdin();
    let reader = BufReader::new(stdin);
    let mut lines = reader.lines();

    log::info!("Qualia MCP Server listening on stdio (qpu={}, llm={})...", enable_qpu, enable_llm);

    while let Ok(Some(line)) = lines.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(response) = process_jsonrpc_message(trimmed, &intent_frame) {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(response.as_bytes());
            let _ = stdout.write_all(b"\n");
            let _ = stdout.flush();
        }
    }

    log::info!("Qualia MCP stdio stream closed, shutting down.");
    Ok(())
}
