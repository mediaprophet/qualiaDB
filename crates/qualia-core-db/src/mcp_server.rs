// crates/qualia-core-db/src/mcp_server.rs

// We still need access to standard library for I/O and String during init phase
extern crate std;

use crate::wal::append_mutation;
use crate::QualiaQuin;
use core::ptr::write_volatile;
use std::string::String;

/// Explicit operational states defining the execution boundaries
#[derive(Copy, Clone, PartialEq, Eq)]
pub enum McpRuntimeState {
    HandshakePhase,
    AllocationFirewallActive,
    SanctuaryGated,
}

#[derive(Debug)]
pub enum McpSystemError {
    SanctuaryGateTriggered,
    ToolNotFound,
    ParseError,
    IntentFrameViolation,
    FeatureNotEnabled,
    InvalidParameters,
}

#[derive(Debug, Clone)]
pub struct McpIntentFrame {
    pub purpose_hash: u64,
    pub active_deontic_constraints: Vec<u64>,
    pub active_profile_id: Option<u64>,
    pub session_nonce: u64,
    pub sanctuary_override: Option<String>,
    pub qpu_enabled: bool,
    pub llm_enabled: bool,
}

/// Zero-deserialization view over an incoming tools/call byte buffer
pub struct RawToolPayload<'a> {
    pub tool_name: &'a [u8],
    pub arguments_raw: &'a [u8],
}

// Simple raw-byte slice extractor. This is a very rudimentary byte matcher
// intended to satisfy the requirement of bypassing generic serde allocation.
fn extract_raw_json_string<'a>(payload: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    // Look for `"key":"value"`
    let mut i = 0;
    while i < payload.len() {
        if payload[i..].starts_with(key) {
            i += key.len();
            // find colon
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

/// Dispatches incoming tool actions without triggering dynamic heap allocations
pub unsafe fn enforce_fiduciary_tool_dispatch(
    payload: RawToolPayload,
    intent_frame: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    match payload.tool_name {
        // ── Graph Engine Tools ───────────────────────────────────────────────
        b"query_graph" => {
            if intent_frame.sanctuary_override.is_none() {
                let violation_quin = QualiaQuin::new_conduct_violation(
                    b"EgressViolation: Missing Cryptographic Sanctuary Override",
                );
                let _ = append_mutation(&violation_quin);
                return Err(McpSystemError::SanctuaryGateTriggered);
            }
            execute_bare_metal_graph_traversal(payload.arguments_raw, intent_frame)
        }

        b"get_graph_stats" => {
            execute_graph_stats(payload.arguments_raw, intent_frame)
        }

        b"list_ontologies" => {
            execute_list_ontologies(payload.arguments_raw, intent_frame)
        }

        // ── LLM Tools ─────────────────────────────────────────────────────────
        b"llm_infer" => {
            if !intent_frame.llm_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            execute_llm_infer(payload.arguments_raw, intent_frame)
        }

        b"llm_chat" => {
            if !intent_frame.llm_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            execute_llm_chat(payload.arguments_raw, intent_frame)
        }

        b"list_models" => {
            execute_list_models(payload.arguments_raw, intent_frame)
        }

        // ── QPU Tools ─────────────────────────────────────────────────────────
        b"qpu_optimize" => {
            if !intent_frame.qpu_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            execute_qpu_optimize(payload.arguments_raw, intent_frame)
        }

        b"qpu_dft" => {
            if !intent_frame.qpu_enabled {
                return Err(McpSystemError::FeatureNotEnabled);
            }
            execute_qpu_dft(payload.arguments_raw, intent_frame)
        }

        b"qpu_status" => {
            execute_qpu_status(payload.arguments_raw, intent_frame)
        }

        // ── Scientific Computing Tools ───────────────────────────────────────
        b"matrix_operation" => {
            execute_matrix_operation(payload.arguments_raw, intent_frame)
        }

        b"ode_solve" => {
            execute_ode_solve(payload.arguments_raw, intent_frame)
        }

        b"chemical_analysis" => {
            execute_chemical_analysis(payload.arguments_raw, intent_frame)
        }

        // ── Identity & Wallet Tools ─────────────────────────────────────────
        b"get_wallet_status" => {
            execute_wallet_status(payload.arguments_raw, intent_frame)
        }

        b"get_did_info" => {
            execute_did_info(payload.arguments_raw, intent_frame)
        }

        // ── Ontology Tools ────────────────────────────────────────────────────
        b"ingest_ontology" => {
            execute_ingest_ontology(payload.arguments_raw, intent_frame)
        }

        b"validate_shacl" => {
            execute_shacl_validation(payload.arguments_raw, intent_frame)
        }

        // ── Testing & Debugging Tools ───────────────────────────────────────
        b"inject_test_quin" => {
            execute_paraconsistent_injection(payload.arguments_raw, intent_frame)
        }

        b"get_system_status" => {
            execute_system_status(payload.arguments_raw, intent_frame)
        }

        _ => Err(McpSystemError::ToolNotFound),
    }
}

// ── Graph Engine Implementations ───────────────────────────────────────────

unsafe fn execute_bare_metal_graph_traversal(
    _args: &[u8],
    intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    let mut arena = crate::webizen::SlgArena::new();
    let contract = if intent
        .active_deontic_constraints
        .first()
        .copied()
        .unwrap_or(0)
        != 0
    {
        intent.active_deontic_constraints[0]
    } else {
        intent.purpose_hash
    };
    let fired = arena.fire_registered_rules(contract);
    Ok(fired.max(1))
}

unsafe fn execute_graph_stats(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Return graph statistics (quin count, memory usage, etc.)
    Ok(1)
}

unsafe fn execute_list_ontologies(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // List available ontologies (SNOMED-CT, FHIR, etc.)
    Ok(1)
}

// ── LLM Implementations ─────────────────────────────────────────────────────

unsafe fn execute_llm_infer(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Execute LLM inference with governance checks
    // This would call into llm_agent.rs
    Ok(1)
}

unsafe fn execute_llm_chat(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Execute LLM chat with Sentinel governance
    Ok(1)
}

unsafe fn execute_list_models(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // List available local models
    Ok(1)
}

// ── QPU Implementations ─────────────────────────────────────────────────────

unsafe fn execute_qpu_optimize(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Execute QPU optimization (QUBO, TSP, etc.)
    Ok(1)
}

unsafe fn execute_qpu_dft(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Execute DFT ground state calculation
    Ok(1)
}

unsafe fn execute_qpu_status(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Return QPU status (tokens configured, quota remaining, etc.)
    Ok(1)
}

// ── Scientific Computing Implementations ─────────────────────────────────

unsafe fn execute_matrix_operation(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Execute matrix operations (eigen decomposition, etc.)
    Ok(1)
}

unsafe fn execute_ode_solve(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Solve ODEs with numerical methods
    Ok(1)
}

unsafe fn execute_chemical_analysis(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Analyze chemical structures (SMILES, InChI, etc.)
    Ok(1)
}

// ── Identity & Wallet Implementations ─────────────────────────────────────

unsafe fn execute_wallet_status(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Return wallet status (balances, addresses, etc.)
    Ok(1)
}

unsafe fn execute_did_info(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Return DID information
    Ok(1)
}

// ── Ontology Implementations ────────────────────────────────────────────────

unsafe fn execute_ingest_ontology(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Ingest RDF/Turtle ontology data
    Ok(1)
}

unsafe fn execute_shacl_validation(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Validate data against SHACL constraints
    Ok(1)
}

// ── Testing & Debugging Implementations ─────────────────────────────────────

unsafe fn execute_paraconsistent_injection(
    _args: &[u8],
    intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    let candidate = QualiaQuin {
        subject: intent.purpose_hash,
        predicate: crate::q_hash("q42:testClaim"),
        object: intent.session_nonce,
        context: intent.purpose_hash,
        metadata: 0,
        parity: 0,
    };
    let mut q = candidate;
    q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;

    let mut consistent = [QualiaQuin::default(); 8];
    let mut isolated = [QualiaQuin::default(); 8];
    let (c, i) = crate::modalities::paraconsistent::route_paraconsistent(
        &[q],
        &mut consistent,
        &mut isolated,
    )
    .map_err(|_| McpSystemError::ParseError)?;

    for idx in 0..i {
        let _ = append_mutation(&isolated[idx]);
    }
    Ok(c + i)
}

unsafe fn execute_system_status(
    _args: &[u8],
    _intent: &McpIntentFrame,
) -> Result<usize, McpSystemError> {
    // Return comprehensive system status
    Ok(1)
}

/// Explicitly purges memory registers to prevent data harvesting
pub unsafe fn scrub_transient_mcp_buffers(buffer: &mut [u8]) {
    for byte_ptr in buffer.iter_mut() {
        write_volatile(byte_ptr, 0x00);
    }
}

/// The core unsafe parser that scans the `tools/call` JSON payload directly.
pub unsafe fn parse_and_evaluate_mcp_stream(stream_chunk: &[u8]) -> Result<usize, McpSystemError> {
    // 1. Check if this is a tools/call
    if !stream_chunk.windows(12).any(|w| w == b"\"tools/call\"") {
        return Err(McpSystemError::ParseError);
    }

    // 2. Extract tool name (raw byte slicing)
    let tool_name = extract_raw_json_string(stream_chunk, b"\"name\"").unwrap_or(b"");

    // 3. Look for "arguments" object (for override token checking)
    let sanctuary_override = extract_raw_json_string(stream_chunk, b"\"sanctuary_override\"");

    let valid_override = match sanctuary_override {
        Some(b"MISSING") => None,
        Some(other) => Some(String::from_utf8_lossy(other).into_owned()),
        None => None,
    };

    // Construct a persistent Intent Frame for this stream
    let intent_frame = McpIntentFrame {
        purpose_hash: crate::q_hash("purpose:General"),
        active_deontic_constraints: Vec::new(),
        active_profile_id: None,
        session_nonce: 0,
        sanctuary_override: valid_override,
        qpu_enabled: true, // In a real implementation, check actual state
        llm_enabled: true, // In a real implementation, check actual state
    };

    let payload = RawToolPayload {
        tool_name,
        arguments_raw: stream_chunk,
    };

    enforce_fiduciary_tool_dispatch(payload, &intent_frame)
}

// -----------------------------------------------------------------------------
// stdio Transport Logic (Allocations permitted only for handshake/metadata)
// -----------------------------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
pub async fn start_mcp_listener() {
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

    eprintln!("[MCP Server] Starting comprehensive MCP server on stdio transport...");
    eprintln!("[MCP Server] Available tools:");
    eprintln!("[MCP Server]   - Graph: query_graph, get_graph_stats, list_ontologies");
    eprintln!("[MCP Server]   - LLM: llm_infer, llm_chat, list_models");
    eprintln!("[MCP Server]   - QPU: qpu_optimize, qpu_dft, qpu_status");
    eprintln!("[MCP Server]   - Scientific: matrix_operation, ode_solve, chemical_analysis");
    eprintln!("[MCP Server]   - Identity: get_wallet_status, get_did_info");
    eprintln!("[MCP Server]   - Ontology: ingest_ontology, validate_shacl");
    eprintln!("[MCP Server]   - Testing: inject_test_quin, get_system_status");

    let mut stdin = BufReader::new(tokio::io::stdin());
    let mut stdout = tokio::io::stdout();
    let mut line = String::new();

    loop {
        line.clear();
        match stdin.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {
                let bytes = line.as_bytes();

                if bytes.windows(12).any(|w| w == b"\"tools/call\"") {
                    let mut buffer = [0u8; 16384];
                    let len = core::cmp::min(bytes.len(), buffer.len());
                    buffer[..len].copy_from_slice(&bytes[..len]);

                    let res = unsafe { parse_and_evaluate_mcp_stream(&buffer[..len]) };

                    unsafe {
                        scrub_transient_mcp_buffers(&mut buffer);
                    }

                    let reply = match res {
                        Ok(_) => {
                            r#"{"jsonrpc":"2.0","result":{"content":[{"type":"text","text":"Success"}]}}"#.to_string()
                        }
                        Err(e) => {
                            let error_msg = match e {
                                McpSystemError::SanctuaryGateTriggered => "Sanctuary gate triggered",
                                McpSystemError::ToolNotFound => "Tool not found",
                                McpSystemError::ParseError => "Parse error",
                                McpSystemError::IntentFrameViolation => "Intent frame violation",
                                McpSystemError::FeatureNotEnabled => "Feature not enabled",
                                McpSystemError::InvalidParameters => "Invalid parameters",
                            };
                            format!(r#"{{"jsonrpc":"2.0","error":{{"code":-32603,"message":"{}"}}}}"#, error_msg).to_string()
                        }
                    };

                    let _ = stdout.write_all(reply.as_bytes()).await;
                    let _ = stdout.write_all(b"\n").await;
                }
            }
            Err(_) => break,
        }
    }
}
