//! MCP tools for Knowledge Graph, SPARQL, Ontologies, and Identifiers.
//!
//! Migrated from legacy `mcp_stub_impls.rs` to full working library module.
//! Designed according to `AGENTS.md` §0-B (<500 lines).

use crate::types::{parse_tool_args, qualia_storage_path, McpSystemError};
use qualia_core_db::NQuin;
use serde_json::{json, Value};

/// Helper to serialize quins to N-Triples text without runtime allocations in inner loop.
pub fn quins_to_ntriples(quins: &[NQuin]) -> String {
    let mut buf = Vec::new();
    let _ = qualia_core_db::resolver::format_ntriples_to(quins, &mut buf);
    String::from_utf8(buf).unwrap_or_default()
}

/// Serialize a single `NQuin` to JSON.
pub fn quin_to_json(q: &NQuin) -> Value {
    json!({
        "subject": q.subject,
        "predicate": q.predicate,
        "object": q.object,
        "context": q.context,
        "metadata": q.metadata,
        "parity": q.parity,
    })
}

/// Helper to resolve codebase audit path.
pub fn resolve_audit_path(path_str: Option<&str>) -> std::path::PathBuf {
    if let Some(p) = path_str {
        if p == "audit" || p == "codebase" {
            let direct = std::path::PathBuf::from(r"C:\github\qualiaDB\.audit\codebase.q42");
            if direct.exists() {
                return direct;
            }
            let local = std::path::PathBuf::from(".audit").join("codebase.q42");
            if local.exists() {
                return local;
            }
        }
        return std::path::PathBuf::from(p);
    }
    std::path::PathBuf::from(r"C:\github\qualiaDB\.audit\codebase.q42")
}

/// Execute SPARQL range query on a .q42 vault (such as codebase.q42).
pub fn query_q42_vault(
    vault: &std::path::Path,
    query_str: &str,
    limit: usize,
) -> Result<Value, McpSystemError> {
    use qualia_core_db::q42_volume::{
        LocalFileRangeSource, Q42RangeVolume, MAX_COMPRESSED_SUPERBLOCK_SIZE, SUPERBLOCK_SIZE,
    };
    use qualia_core_db::sparql_ast::BindingRow;
    use qualia_core_db::sparql_executor::{
        execute_range_triple_select_page_into, Q42RangeTripleSelectCursor,
        Q42RangeTripleSelectPlan,
    };

    let (query, ctx) = qualia_core_db::sparql_parser::parse_sparql(query_str)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let plan = qualia_core_db::sparql_planner::QueryPlanner::plan(&query, &ctx)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let source = LocalFileRangeSource::open(vault).map_err(|_| McpSystemError::InvalidParameters)?;
    let root = Q42RangeVolume::open(source).map_err(|_| McpSystemError::InvalidParameters)?;
    let lexicon = qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok();

    let range_plan = Q42RangeTripleSelectPlan::from_execution_plan(&plan)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    const PAGE_ROWS: usize = 128;
    let mut compressed = [0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
    let mut decoded = [0u8; SUPERBLOCK_SIZE];
    let mut quins = [qualia_core_db::NQuin::default(); PAGE_ROWS];
    let mut rows = [BindingRow::default(); PAGE_ROWS];
    let mut cursor = Q42RangeTripleSelectCursor::default();
    let mut results = Vec::new();

    while results.len() < limit {
        let page = execute_range_triple_select_page_into(
            &root,
            range_plan,
            &ctx,
            cursor,
            &mut compressed,
            &mut decoded,
            &mut quins,
            &mut rows,
        )
        .map_err(|_| McpSystemError::ParseError)?;

        for row in &rows[..page.returned] {
            let mut row_obj = serde_json::Map::new();
            for (slot_idx, slot) in row.slots.iter().enumerate() {
                if let Some(v) = slot {
                    let val_str = lexicon
                        .as_ref()
                        .and_then(|l| l.lookup(*v))
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| format!("0x{v:016x}"));
                    row_obj.insert(format!("?v{slot_idx}"), Value::String(val_str));
                }
            }
            results.push(Value::Object(row_obj));
            if results.len() >= limit {
                break;
            }
        }

        match page.next_cursor {
            Some(next) => cursor = next,
            None => break,
        }
    }

    Ok(json!({
        "vault": vault.display().to_string(),
        "returned": results.len(),
        "bindings": results,
    }))
}

/// `query_sparql` tool — executes SPARQL queries against live in-memory daemon graph or a .q42 vault (e.g. codebase.q42).
pub fn query_sparql(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let query = v.get("query").and_then(Value::as_str).unwrap_or("");
    if query.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let limit = v.get("limit").and_then(|x| x.as_u64()).unwrap_or(1_000) as usize;

    let vault_opt = v
        .get("vault_path")
        .or_else(|| v.get("q42_path"))
        .and_then(Value::as_str);

    if let Some(v_path) = vault_opt {
        let resolved = resolve_audit_path(Some(v_path));
        let result = query_q42_vault(&resolved, query, limit)?;
        return serde_json::to_string(&result).map_err(|_| McpSystemError::ParseError);
    }

    let graph = qualia_core_db::daemon_graph::graph_read_guard();
    let (stats, results) = qualia_core_db::daemon_query::execute_query_on_graph(query, graph.as_slice())
        .map_err(|e| match e {
            qualia_core_db::daemon_query::QueryExecError::EmptyQuery
            | qualia_core_db::daemon_query::QueryExecError::ParseError(_)
            | qualia_core_db::daemon_query::QueryExecError::InvalidProgram => {
                McpSystemError::InvalidParameters
            }
            qualia_core_db::daemon_query::QueryExecError::OutputBufferFull => McpSystemError::ParseError,
            qualia_core_db::daemon_query::QueryExecError::ClassifiedEgress => {
                McpSystemError::SanctuaryGateTriggered
            }
        })?;

    let truncated = results.len().min(limit);
    let payload = json!({
        "query": query,
        "matchCount": stats.match_count,
        "returned": truncated,
        "stats": {
            "vmCycles": stats.vm_cycles,
            "directJumpOps": stats.direct_jump_ops,
            "lexiconLookupOps": stats.lexicon_lookup_ops,
            "matchCount": stats.match_count,
        },
        "ntriples": quins_to_ntriples(&results[..truncated]),
        "quins": results[..truncated].iter().map(quin_to_json).collect::<Vec<_>>(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `get_graph_stats` tool — returns resident daemon-graph quin count, capacity, and utilization.
pub fn get_graph_stats(_args: &[u8]) -> Result<String, McpSystemError> {
    let count = qualia_core_db::daemon_graph::graph_quin_count();
    let capacity = qualia_core_db::daemon_graph::MAX_GRAPH_QUINS;
    let payload = json!({
        "quinCount": count,
        "capacity": capacity,
        "utilization": if capacity > 0 {
            (count as f64) / (capacity as f64)
        } else {
            0.0
        },
        "empty": count == 0,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `list_ontologies` tool — lists startup ontology catalog entries and on-disk presence.
pub fn list_ontologies(_args: &[u8]) -> Result<String, McpSystemError> {
    let dir = qualia_core_db::ontology_loader::ontology_dir_path();
    let mut entries = Vec::new();
    for (filename, context_hash) in qualia_core_db::ontology_loader::startup_ontology_catalog() {
        let present = dir
            .as_ref()
            .map(|d| d.join(filename).is_file())
            .unwrap_or(false);
        entries.push(json!({
            "filename": filename,
            "contextHash": context_hash,
            "present": present,
        }));
    }
    let payload = json!({
        "ontologyDir": dir.map(|p| p.display().to_string()).unwrap_or_default(),
        "entries": entries,
        "graphQuinCount": qualia_core_db::daemon_graph::graph_quin_count(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `ingest_ontology` tool — ingests RDF Turtle triples from a file or inline content into the daemon graph.
pub fn ingest_ontology(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let context_hash = v.get("context_hash").and_then(Value::as_u64).unwrap_or(0);

    let (quins, path_str) = if let Some(path_str) = v.get("path").and_then(Value::as_str) {
        let p = std::path::Path::new(path_str);
        let ctx = if context_hash == 0 {
            qualia_core_db::q_hash(path_str)
        } else {
            context_hash
        };
        let q = if p.extension().and_then(|s| s.to_str()) == Some("q42") {
            qualia_core_db::ontology_loader::load_q42_file(p)
        } else {
            qualia_core_db::ontology_loader::parse_ttl_to_quins(p, ctx)
        };
        (q, path_str.to_string())
    } else if let Some(content) = v.get("content").and_then(Value::as_str) {
        let ctx = if context_hash == 0 {
            qualia_core_db::q_hash("urn:inline:ontology")
        } else {
            context_hash
        };
        use rio_api::parser::TriplesParser;
        let mut quins = Vec::new();
        let mut parser = rio_turtle::TurtleParser::new(content.as_bytes(), None);
        let _ = parser.parse_all(&mut |t: rio_api::model::Triple| -> Result<(), std::io::Error> {
            let s = qualia_core_db::q_hash(&t.subject.to_string());
            let p = qualia_core_db::q_hash(&t.predicate.to_string());
            let o = qualia_core_db::q_hash(&t.object.to_string());
            quins.push(qualia_core_db::NQuin {
                subject: s,
                predicate: p,
                object: o,
                context: ctx,
                metadata: 0,
                parity: s ^ p ^ o ^ ctx,
            });
            Ok(())
        });
        (quins, "inline".to_string())
    } else {
        return Err(McpSystemError::InvalidParameters);
    };

    let count_before = qualia_core_db::daemon_graph::graph_quin_count();
    let ingested = quins.len();
    qualia_core_db::daemon_graph::extend_with_ontology_quins_slice(&quins);
    let count_after = qualia_core_db::daemon_graph::graph_quin_count();

    let payload = json!({
        "path": path_str,
        "ingestedQuins": ingested,
        "graphCountBefore": count_before,
        "graphCountAfter": count_after,
        "success": true,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `get_did_info` tool — resolves a `did:q42:...` identifier into its topological pointer hash and bit flags.
pub fn get_did_info(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let did = v.get("did").and_then(Value::as_str).unwrap_or("");
    if !did.starts_with("did:q42:") {
        return Err(McpSystemError::InvalidParameters);
    }

    let parsed_hash = qualia_core_db::identifier::parse_did_q42(did.as_bytes())
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let msb_set = (parsed_hash >> 63) == 1;

    let payload = json!({
        "did": did,
        "hash": parsed_hash,
        "hex": format!("0x{:016x}", parsed_hash),
        "msbSet": msb_set,
        "isTopologicalPointer": msb_set,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `get_wallet_status` tool — inspects queued Interledger Protocol (ILP) micropayments.
pub fn get_wallet_status(_args: &[u8]) -> Result<String, McpSystemError> {
    let log_path = qualia_storage_path().join("pending_payments.ndjson");
    let mut pending_count = 0;
    let mut total_amount_drops = 0u64;

    if let Ok(content) = std::fs::read_to_string(&log_path) {
        for line in content.lines() {
            if let Ok(record) = serde_json::from_str::<Value>(line) {
                pending_count += 1;
                if let Some(amt) = record.get("amount").and_then(Value::as_u64) {
                    total_amount_drops += amt;
                }
            }
        }
    }

    let payload = json!({
        "pendingPaymentsCount": pending_count,
        "totalAmountDrops": total_amount_drops,
        "journalPath": log_path.display().to_string(),
        "status": if pending_count > 0 { "unsettled" } else { "settled" },
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `codebase_audit` tool — fast query into the C:\github\qualiaDB\.audit\codebase.q42 database.
/// Supports inspecting functions, signatures, methods, and files across the entire QualiaDB ecosystem.
pub fn codebase_audit(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let limit = v.get("limit").and_then(|x| x.as_u64()).unwrap_or(25) as usize;
    let vault_str = v.get("vault_path").and_then(Value::as_str);
    let vault = resolve_audit_path(vault_str);

    let custom_query = v.get("query").and_then(Value::as_str);
    let query_str = if let Some(q) = custom_query {
        q.to_string()
    } else {
        format!("SELECT ?fn ?sig WHERE {{ ?fn <https://audit.dev/ns#signature> ?sig }} LIMIT {limit}")
    };

    let result = query_q42_vault(&vault, &query_str, limit)?;
    serde_json::to_string(&result).map_err(|_| McpSystemError::ParseError)
}

