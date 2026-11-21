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

/// Helper to resolve codebase audit or `.q42` vault path (checks CWD, SYNTH_OUTPUT_ROOT, SYNTH_INPUT_ROOT).
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
        let pb = std::path::PathBuf::from(p);
        if pb.exists() {
            return pb;
        }
        if !pb.is_absolute() {
            for env_key in ["SYNTH_OUTPUT_ROOT", "SYNTH_INPUT_ROOT"] {
                if let Ok(root) = std::env::var(env_key) {
                    let candidate = std::path::Path::new(root.trim()).join(p);
                    if candidate.exists() {
                        return candidate;
                    }
                }
            }
        }
        return pb;
    }
    std::path::PathBuf::from(r"C:\github\qualiaDB\.audit\codebase.q42")
}

/// Recover human-readable variable names (`?s`, `?label`, …) for each slot in `ctx`.
fn extract_variable_names(
    query_str: &str,
    ctx: &qualia_core_db::sparql_ast::SparqlQueryContext,
) -> Vec<String> {
    let mut names: Vec<String> = (0..ctx.variable_count)
        .map(|i| format!("?v{i}"))
        .collect();
    if let Ok(tokens) = qualia_core_db::sparql_library::sparql_grammar::tokenizer::tokenize(query_str) {
        for tok in tokens {
            if let qualia_core_db::sparql_library::sparql_grammar::tokenizer::Token::Var(var_name) = tok {
                let h = qualia_core_db::lexicon::generate_60bit_token(var_name.as_bytes());
                for (slot_idx, slot_hash) in ctx
                    .variable_hashes
                    .iter()
                    .take(ctx.variable_count)
                    .enumerate()
                {
                    if *slot_hash == h {
                        names[slot_idx] = var_name.clone();
                    }
                }
            }
        }
    }
    names
}

fn format_term_hash(hash: u64, lex_lookup: &dyn Fn(u64) -> Option<String>) -> String {
    if let Some(s) = lex_lookup(hash) {
        return s;
    }
    if let Some(lit) = qualia_core_db::resolver::classify_inline_literal(hash) {
        return lit.to_string();
    }
    if let Some(bytes) = qualia_core_db::resolver::resolve_hash(hash) {
        return String::from_utf8_lossy(bytes).into_owned();
    }
    format!("0x{hash:016x}")
}

/// Execute a SPARQL query (SELECT/ASK/CONSTRUCT/DESCRIBE, with FILTER/JOIN/UNION/OPTIONAL/GROUP BY)
/// against a `.q42` vault (such as `fma.q42` or `codebase.q42`).
pub fn query_q42_vault(
    vault: &std::path::Path,
    query_str: &str,
    limit: usize,
) -> Result<Value, McpSystemError> {
    use qualia_core_db::q42_volume::{
        LocalFileRangeSource, Q42RangeVolume, Q42Volume, MAX_COMPRESSED_SUPERBLOCK_SIZE,
        SUPERBLOCK_SIZE,
    };
    use qualia_core_db::sparql_ast::{BindingRow, SparqlQuery, StringSink, TextResolver};
    use qualia_core_db::sparql_executor::{
        execute_range_triple_select_page_with_resolver_into, Q42RangeTripleSelectCursor,
        Q42RangeTripleSelectPlan, QueryExecutor,
    };

    let (query, ctx, literals) = qualia_core_db::sparql_parser::parse_sparql_full(query_str)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let plan = qualia_core_db::sparql_planner::QueryPlanner::plan(&query, &ctx)
        .map_err(|_| McpSystemError::InvalidParameters)?;

    let var_names = extract_variable_names(query_str, &ctx);

    // Zero-copy mmap lexicon view for unified v3/v4 volumes; fallback to sidecar loader only when needed.
    let mmap_vol = Q42Volume::open(vault).ok();
    let mmap_lex = mmap_vol.as_ref().and_then(|v| v.lex_view().ok());
    let fallback_lex = if mmap_lex.is_none() {
        qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(vault).ok()
    } else {
        None
    };

    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let seed = now_ms
        .wrapping_mul(0x9E37_79B9_7F4A_7C15)
        .wrapping_add(query_str.len() as u64 + 1);
    let sink = StringSink::new();
    let lex_closure = |h: u64| -> Option<String> {
        if let Some(view) = &mmap_lex {
            if let Some(s) = view.lookup_owned(h) {
                return Some(s);
            }
        }
        if let Some(fb) = &fallback_lex {
            if let Some(s) = fb.lookup(h) {
                return Some(s.to_string());
            }
        }
        qualia_core_db::daemon_graph::graph_lexicon_lookup(h)
    };
    let resolver = TextResolver::with_lexicon(&literals, &lex_closure)
        .with_sink(&sink)
        .with_env(now_ms, seed);

    let can_use_range = matches!(
        &query,
        SparqlQuery::Select(s)
            if !s.distinct
                && s.group_by_count == 0
                && s.aggregate_count == 0
                && s.order_by_count == 0
    ) && mmap_vol
        .as_ref()
        .and_then(|v| v.volume_manifest().ok().flatten())
        .is_none();

    let row_to_json = |row: &BindingRow| -> Value {
        let mut row_obj = serde_json::Map::new();
        for (slot_idx, slot) in row.slots.iter().enumerate() {
            if let Some(v) = slot {
                let val_str = resolver
                    .resolve_text(*v)
                    .unwrap_or_else(|| format_term_hash(*v, &lex_closure));
                let v_key = format!("?v{slot_idx}");
                if let Some(real_name) = var_names.get(slot_idx) {
                    row_obj.insert(real_name.clone(), Value::String(val_str.clone()));
                }
                row_obj.entry(v_key).or_insert(Value::String(val_str));
            }
        }
        Value::Object(row_obj)
    };

    if can_use_range {
        if let Ok(range_plan) = Q42RangeTripleSelectPlan::from_execution_plan(&plan) {
            let source =
                LocalFileRangeSource::open(vault).map_err(|_| McpSystemError::InvalidParameters)?;
            let root = Q42RangeVolume::open(source).map_err(|_| McpSystemError::InvalidParameters)?;

            const PAGE_ROWS: usize = 128;
            let mut compressed = [0u8; MAX_COMPRESSED_SUPERBLOCK_SIZE];
            let mut decoded = [0u8; SUPERBLOCK_SIZE];
            let mut quins = [qualia_core_db::NQuin::default(); PAGE_ROWS];
            let mut rows = [BindingRow::default(); PAGE_ROWS];
            let mut cursor = Q42RangeTripleSelectCursor::default();
            let mut results = Vec::new();

            while results.len() < limit {
                let page = execute_range_triple_select_page_with_resolver_into(
                    &root,
                    range_plan,
                    &ctx,
                    Some(resolver),
                    cursor,
                    &mut compressed,
                    &mut decoded,
                    &mut quins,
                    &mut rows,
                )
                .map_err(|_| McpSystemError::ParseError)?;

                for row in &rows[..page.returned] {
                    results.push(row_to_json(row));
                    if results.len() >= limit {
                        break;
                    }
                }

                match page.next_cursor {
                    Some(next) => cursor = next,
                    None => break,
                }
            }

            return Ok(json!({
                "vault": vault.display().to_string(),
                "variables": var_names,
                "returned": results.len(),
                "bindings": results,
            }));
        }
    }

    // Full SPARQL fallback over Q42 volume (multi-triple JOIN, OPTIONAL, UNION, GROUP BY, ASK, CONSTRUCT, DESCRIBE).
    let all_quins = qualia_core_db::q42_reader::read_q42_quins(vault)
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let executor = QueryExecutor::with_resolver(&all_quins, resolver);

    match &query {
        SparqlQuery::Ask(_) => {
            let ask_ok = executor
                .execute_ask(&plan, &ctx)
                .map_err(|_| McpSystemError::ParseError)?;
            Ok(json!({
                "vault": vault.display().to_string(),
                "boolean": ask_ok,
                "returned": usize::from(ask_ok),
                "bindings": [],
            }))
        }
        SparqlQuery::Construct(c) => {
            let rows = executor
                .execute_construct(&plan, &ctx, c.template_pattern)
                .map_err(|_| McpSystemError::ParseError)?;
            let results: Vec<Value> = rows.iter().take(limit).map(&row_to_json).collect();
            Ok(json!({
                "vault": vault.display().to_string(),
                "variables": ["?s", "?p", "?o"],
                "returned": results.len(),
                "bindings": results,
            }))
        }
        SparqlQuery::Describe(d) => {
            let rows = executor
                .execute_describe(&plan, &ctx, d)
                .map_err(|_| McpSystemError::ParseError)?;
            let results: Vec<Value> = rows.iter().take(limit).map(&row_to_json).collect();
            Ok(json!({
                "vault": vault.display().to_string(),
                "variables": ["?s", "?p", "?o"],
                "returned": results.len(),
                "bindings": results,
            }))
        }
        SparqlQuery::Select(_) => {
            let rows = executor
                .execute(&plan, &ctx)
                .map_err(|_| McpSystemError::ParseError)?;
            let results: Vec<Value> = rows.iter().take(limit).map(&row_to_json).collect();
            Ok(json!({
                "vault": vault.display().to_string(),
                "variables": var_names,
                "returned": results.len(),
                "bindings": results,
            }))
        }
    }
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
    let bindings: Vec<Value> = results[..truncated]
        .iter()
        .map(|q| {
            let mut row = serde_json::Map::new();
            for (idx, val) in [(0usize, q.subject), (1, q.predicate), (2, q.object)] {
                if val != 0 {
                    let s = format_term_hash(val, &qualia_core_db::daemon_graph::graph_lexicon_lookup);
                    row.insert(format!("?v{idx}"), Value::String(s));
                }
            }
            Value::Object(row)
        })
        .collect();
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
        "bindings": bindings,
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
        "utilization": if capacity > 0 { (count as f64) / (capacity as f64) } else { 0.0 },
        "empty": count == 0,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `list_ontologies` tool — lists startup ontology catalog entries and on-disk presence.
pub fn list_ontologies(_args: &[u8]) -> Result<String, McpSystemError> {
    let dir = qualia_core_db::ontology_loader::ontology_dir_path();
    let mut entries = Vec::new();
    for (filename, context_hash) in qualia_core_db::ontology_loader::startup_ontology_catalog() {
        let present = dir.as_ref().map(|d| d.join(filename).is_file()).unwrap_or(false);
        entries.push(json!({"filename": filename, "contextHash": context_hash, "present": present}));
    }
    let payload = json!({
        "ontologyDir": dir.map(|p| p.display().to_string()).unwrap_or_default(),
        "entries": entries,
        "graphQuinCount": qualia_core_db::daemon_graph::graph_quin_count(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `ingest_ontology` tool — ingests OWL/RDF/XML/TTL/N3/NT/q42 triples from a file or inline content into the daemon graph.
pub fn ingest_ontology(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let context_hash = v.get("context_hash").and_then(Value::as_u64).unwrap_or(0);
    let out_q42 = v.get("out_q42").and_then(Value::as_str);

    let (quins, path_str, sealed_q42) = if let Some(path_str) = v.get("path").and_then(Value::as_str) {
        let resolved = resolve_audit_path(Some(path_str));
        let p = resolved.as_path();
        let ctx = if context_hash == 0 { qualia_core_db::q_hash(path_str) } else { context_hash };
        if let Some(q42_dest) = out_q42 {
            qualia_core_db::ingest::streaming_import_rdf(
                &p.to_string_lossy(),
                q42_dest,
            )
            .map_err(|e| McpSystemError::ExecutionFailed(e.to_string()))?;
            let q = qualia_core_db::ontology_loader::load_q42_file(std::path::Path::new(q42_dest));
            (q, p.display().to_string(), Some(q42_dest.to_string()))
        } else if p.extension().and_then(|s| s.to_str()) == Some("q42") {
            (qualia_core_db::ontology_loader::load_q42_file(p), p.display().to_string(), None)
        } else {
            (qualia_core_db::ontology_loader::parse_ttl_to_quins(p, ctx), p.display().to_string(), None)
        }
    } else if let Some(content) = v.get("content").and_then(Value::as_str) {
        let ctx = if context_hash == 0 { qualia_core_db::q_hash("urn:inline:ontology") } else { context_hash };
        let (quins, lex) = qualia_core_db::ontology_loader::parse_ttl_bytes_to_quins_and_lex(content.as_bytes(), ctx)
            .map_err(McpSystemError::ExecutionFailed)?;
        qualia_core_db::daemon_graph::extend_graph_lexicon(lex);
        (quins, "inline".to_string(), None)
    } else {
        return Err(McpSystemError::InvalidParameters);
    };

    let count_before = qualia_core_db::daemon_graph::graph_quin_count();
    let ingested = quins.len();
    qualia_core_db::daemon_graph::extend_with_ontology_quins_slice(&quins);
    let count_after = qualia_core_db::daemon_graph::graph_quin_count();

    let payload = json!({
        "path": path_str,
        "out_q42": sealed_q42,
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
    let payload = json!({"did": did, "hash": parsed_hash, "hex": format!("0x{:016x}", parsed_hash), "msbSet": msb_set, "isTopologicalPointer": msb_set});
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `get_wallet_status` tool — inspects queued Interledger Protocol (ILP) micropayments.
pub fn get_wallet_status(_args: &[u8]) -> Result<String, McpSystemError> {
    let log_path = qualia_storage_path().join("pending_payments.ndjson");
    let (mut pending_count, mut total_amount_drops) = (0, 0u64);
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
    let payload = json!({"pendingPaymentsCount": pending_count, "totalAmountDrops": total_amount_drops, "journalPath": log_path.display().to_string(), "status": if pending_count > 0 { "unsettled" } else { "settled" }});
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

