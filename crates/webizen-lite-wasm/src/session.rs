//! Session-scoped graph store for WASM Lite MCP.
//!
//! Hosts fetch bytes/text; tools here load into memory, query, and compile deontic
//! norms. No network, no filesystem.

use std::cell::RefCell;
use std::collections::HashMap;

use qualia_core_db::modalities;
use qualia_core_db::modalities::logic::deontic::{
    compile_norm_quin, evaluate_deontic_contract, DeonticVerdict, OP_FORBID, OP_OBLIGATE, OP_PERMIT,
};
use qualia_core_db::{q_hash, NQuin};
use serde_json::{json, Value};

use crate::{
    optional_u64, parse_quin, parse_u64, quin_json, required_str, required_u32, MAX_INPUT_QUINS,
    MAX_N3_EVENTS, MAX_QUERY_RESULTS,
};

const MAX_SESSION_GRAPHS: usize = 8;
const MAX_LEXICON_ENTRIES: usize = 8_192;
const Q42L_MAGIC: &[u8; 4] = b"Q42L";
const Q42L_VERSION: u16 = 1;

thread_local! {
    static GRAPHS: RefCell<HashMap<String, SessionGraph>> = RefCell::new(HashMap::new());
    static NEXT_ID: RefCell<u64> = const { RefCell::new(1) };
}

#[derive(Clone, Default)]
pub struct SessionGraph {
    pub id: String,
    pub label: String,
    pub source_format: String,
    pub quins: Vec<NQuin>,
    /// hash → surface string (IRIs and literal lexical forms)
    pub lexicon: HashMap<u64, String>,
}

fn allocate_id(hint: Option<&str>) -> String {
    if let Some(h) = hint {
        let t = h.trim();
        if !t.is_empty() && t.len() < 128 {
            return t.to_string();
        }
    }
    NEXT_ID.with(|n| {
        let mut v = n.borrow_mut();
        let id = format!("g{}", *v);
        *v += 1;
        id
    })
}

fn with_graphs_mut<R>(f: impl FnOnce(&mut HashMap<String, SessionGraph>) -> R) -> R {
    GRAPHS.with(|g| f(&mut g.borrow_mut()))
}

fn with_graphs<R>(f: impl FnOnce(&HashMap<String, SessionGraph>) -> R) -> R {
    GRAPHS.with(|g| f(&g.borrow()))
}

fn insert_graph(graph: SessionGraph) -> Result<SessionGraph, String> {
    with_graphs_mut(|map| {
        if map.len() >= MAX_SESSION_GRAPHS && !map.contains_key(&graph.id) {
            return Err(format!(
                "session graph limit ({MAX_SESSION_GRAPHS}) reached; unload_graph first"
            ));
        }
        if graph.quins.len() > MAX_INPUT_QUINS {
            return Err(format!(
                "graph exceeds {MAX_INPUT_QUINS} Quins (got {})",
                graph.quins.len()
            ));
        }
        let id = graph.id.clone();
        map.insert(id, graph.clone());
        Ok(graph)
    })
}

fn graph_summary(g: &SessionGraph) -> Value {
    json!({
        "graphId": g.id,
        "label": g.label,
        "sourceFormat": g.source_format,
        "quinCount": g.quins.len(),
        "lexiconCount": g.lexicon.len()
    })
}

// ─── Public MCP tools ────────────────────────────────────────────────────────

pub fn list_graphs(_args: &Value) -> Result<Value, String> {
    with_graphs(|map| {
        let mut graphs: Vec<Value> = map.values().map(graph_summary).collect();
        graphs.sort_by(|a, b| {
            a["graphId"]
                .as_str()
                .unwrap_or("")
                .cmp(b["graphId"].as_str().unwrap_or(""))
        });
        Ok(json!({
            "graphCount": graphs.len(),
            "maxGraphs": MAX_SESSION_GRAPHS,
            "graphs": graphs,
            "queryMeta": { "tool": "list_graphs", "logicMode": "none" }
        }))
    })
}

pub fn unload_graph(args: &Value) -> Result<Value, String> {
    let id = required_str(args, "graphId")?;
    with_graphs_mut(|map| {
        let removed = map.remove(id).is_some();
        Ok(json!({
            "graphId": id,
            "removed": removed,
            "remaining": map.len(),
            "queryMeta": { "tool": "unload_graph" }
        }))
    })
}

/// Load a graph into the session.
///
/// `format`:
/// - `n3` — `source` N3 text (ground triples → Quins + lexicon)
/// - `quins` — `quins` array of Quin objects
/// - `q42lite` — `bytesBase64` of Q42L pack (see `encode_q42lite`)
/// - `q42` — alias: try Q42L; full native Q42 v3 is not supported in WASM (fail with message)
pub fn load_graph(args: &Value) -> Result<Value, String> {
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("n3")
        .to_ascii_lowercase();
    let label = args
        .get("label")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let id_hint = args.get("graphId").and_then(Value::as_str);
    let graph_id = allocate_id(id_hint);

    let (quins, lexicon, source_format) = match format.as_str() {
        "n3" => {
            let source = required_str(args, "source")?;
            let (q, lex, truncated) = n3_to_quins(source)?;
            if truncated {
                // still load what we have
            }
            (q, lex, "n3".to_string())
        }
        "quins" => {
            let arr = args
                .get("quins")
                .and_then(Value::as_array)
                .ok_or_else(|| "quins must be an array".to_string())?;
            if arr.len() > MAX_INPUT_QUINS {
                return Err(format!("quins exceeds {MAX_INPUT_QUINS}"));
            }
            let mut q = Vec::with_capacity(arr.len());
            for (i, v) in arr.iter().enumerate() {
                q.push(parse_quin(v, &format!("quins[{i}]"))?);
            }
            (q, HashMap::new(), "quins".to_string())
        }
        "q42lite" | "q42-lite" | "q42l" => {
            let b64 = required_str(args, "bytesBase64")?;
            let bytes = decode_base64(b64)?;
            decode_q42lite(&bytes)?
        }
        "q42" => {
            // Prefer Q42L if magic matches; otherwise refuse full v3 mmap volumes.
            let b64 = required_str(args, "bytesBase64")?;
            let bytes = decode_base64(b64)?;
            if bytes.len() >= 4 && &bytes[0..4] == Q42L_MAGIC {
                decode_q42lite(&bytes)?
            } else if bytes.len() >= 4 && bytes[0..3] == *b"Q42" {
                return Err(
                    "native Q42 v3 volumes require memmap/lz4 and are not loadable in wasm-ontology; \
                     export Q42L via encode_q42lite / load_graph format=q42lite, or load N3"
                        .into(),
                );
            } else {
                return Err("unrecognised q42 payload (expected Q42L magic)".into());
            }
        }
        other => {
            return Err(format!(
                "unsupported load format '{other}' (use n3|quins|q42lite)"
            ))
        }
    };

    let graph = SessionGraph {
        id: graph_id.clone(),
        label,
        source_format,
        quins,
        lexicon,
    };
    let stored = insert_graph(graph)?;
    Ok(json!({
        "graphId": stored.id,
        "label": stored.label,
        "sourceFormat": stored.source_format,
        "quinCount": stored.quins.len(),
        "lexiconCount": stored.lexicon.len(),
        "queryMeta": {
            "tool": "load_graph",
            "format": format,
            "note": "Session-scoped; unload_graph to free. Full Q42 v3 not supported in WASM."
        }
    }))
}

/// Alias documented in the plan.
pub fn load_q42(args: &Value) -> Result<Value, String> {
    let mut args = args.clone();
    if args.get("format").is_none() {
        if let Some(obj) = args.as_object_mut() {
            obj.insert("format".into(), json!("q42lite"));
        }
    }
    load_graph(&args)
}

/// Bounded graph query over a session graph (preferred over full SPARQL in lite).
pub fn query_graph(args: &Value) -> Result<Value, String> {
    let graph_id = required_str(args, "graphId")?;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(64)
        .min(MAX_QUERY_RESULTS as u64) as usize;
    let subject = optional_u64(args, "subject")?;
    let predicate = optional_u64(args, "predicate")?;
    let object = optional_u64(args, "object")?;
    let context = optional_u64(args, "context")?;
    let label_contains = args
        .get("labelContains")
        .and_then(Value::as_str)
        .map(|s| s.to_ascii_lowercase());
    let object_contains = args
        .get("objectContains")
        .and_then(Value::as_str)
        .map(|s| s.to_ascii_lowercase());

    with_graphs(|map| {
        let g = map
            .get(graph_id)
            .ok_or_else(|| format!("unknown graphId '{graph_id}'"))?;
        let mut matches = Vec::new();
        let mut match_count = 0usize;
        for q in &g.quins {
            if subject.is_some_and(|v| q.subject != v)
                || predicate.is_some_and(|v| q.predicate != v)
                || object.is_some_and(|v| q.object != v)
                || context.is_some_and(|v| q.context != v)
            {
                continue;
            }
            if let Some(ref needle) = object_contains {
                let hit = g
                    .lexicon
                    .get(&q.object)
                    .map(|s| s.to_ascii_lowercase().contains(needle.as_str()))
                    .unwrap_or(false);
                if !hit {
                    continue;
                }
            }
            if let Some(ref needle) = label_contains {
                let hit = [q.subject, q.predicate, q.object, q.context]
                    .iter()
                    .any(|h| {
                        g.lexicon
                            .get(h)
                            .map(|s| s.to_ascii_lowercase().contains(needle.as_str()))
                            .unwrap_or(false)
                    });
                if !hit {
                    continue;
                }
            }
            match_count += 1;
            if matches.len() < limit {
                matches.push(json!({
                    "quin": quin_json(q),
                    "labels": {
                        "subject": g.lexicon.get(&q.subject),
                        "predicate": g.lexicon.get(&q.predicate),
                        "object": g.lexicon.get(&q.object),
                        "context": g.lexicon.get(&q.context)
                    }
                }));
            }
        }
        Ok(json!({
            "graphId": graph_id,
            "matchCount": match_count,
            "returnedCount": matches.len(),
            "truncated": match_count > matches.len(),
            "matches": matches,
            "queryMeta": {
                "tool": "query_graph",
                "logicMode": "none",
                "note": "Preferred lite query API; use query_sparql only for simple SELECT patterns"
            }
        }))
    })
}

/// Minimal SELECT-only SPARQL over a session graph. Fail closed on unsupported syntax.
///
/// Supported:
/// - `SELECT * WHERE { ?s ?p ?o } LIMIT n`
/// - optional `FILTER(CONTAINS(LCASE(STR(?o)), "needle"))` (lexicon required for STR)
pub fn query_sparql(args: &Value) -> Result<Value, String> {
    let graph_id = required_str(args, "graphId")?;
    let query = required_str(args, "query")?;
    let qnorm = collapse_ws(query);
    let qupper = qnorm.to_ascii_uppercase();

    if qupper.contains("INSERT")
        || qupper.contains("DELETE")
        || qupper.contains("LOAD ")
        || qupper.contains("CLEAR ")
        || qupper.contains("DROP ")
        || qupper.contains("CREATE ")
        || qupper.contains("CONSTRUCT")
        || qupper.contains("DESCRIBE")
        || qupper.contains("ASK ")
    {
        return Err(
            "query_sparql is SELECT-only; UPDATE/CONSTRUCT/DESCRIBE/ASK are not supported".into(),
        );
    }
    if !qupper.contains("SELECT") || !qupper.contains("WHERE") {
        return Err("query_sparql requires SELECT … WHERE { … }".into());
    }

    // Detect simple BGP ?s ?p ?o
    let bgp_ok = qnorm.contains("?s")
        && qnorm.contains("?p")
        && qnorm.contains("?o")
        && (qnorm.contains("{ ?s ?p ?o }")
            || qnorm.contains("{?s ?p ?o}")
            || qnorm.contains("{ ?s ?p ?o.")
            || qnorm.contains("{?s ?p ?o."));
    if !bgp_ok {
        return Err(
            "unsupported SPARQL shape; use simple `SELECT * WHERE { ?s ?p ?o }` \
             optionally with FILTER(CONTAINS(LCASE(STR(?o)),\"…\")), or call query_graph"
                .into(),
        );
    }

    let mut limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(64)
        .min(MAX_QUERY_RESULTS as u64) as usize;
    if let Some(pos) = qupper.rfind("LIMIT") {
        let rest = qnorm[pos + 5..].trim();
        if let Some(num) = rest.split_whitespace().next() {
            if let Ok(n) = num.parse::<usize>() {
                limit = n.min(MAX_QUERY_RESULTS);
            }
        }
    }

    let mut object_contains: Option<String> = None;
    // FILTER(CONTAINS(LCASE(STR(?o)), "needle")) or single quotes
    if let Some(idx) = qupper.find("CONTAINS") {
        let slice = &qnorm[idx..];
        if let Some(q1) = slice.find('"') {
            if let Some(q2) = slice[q1 + 1..].find('"') {
                object_contains = Some(slice[q1 + 1..q1 + 1 + q2].to_ascii_lowercase());
            }
        } else if let Some(q1) = slice.find('\'') {
            if let Some(q2) = slice[q1 + 1..].find('\'') {
                object_contains = Some(slice[q1 + 1..q1 + 1 + q2].to_ascii_lowercase());
            }
        }
    }

    let mut proxy = json!({
        "graphId": graph_id,
        "limit": limit
    });
    if let Some(n) = object_contains {
        proxy["objectContains"] = json!(n);
    }
    let mut result = query_graph(&proxy)?;
    result["queryMeta"]["tool"] = json!("query_sparql");
    result["queryMeta"]["sparqlSubset"] = json!("bgp-spo+optional-filter-contains-object");
    result["queryMeta"]["unsupported"] = json!([
        "joins",
        "OPTIONAL",
        "UNION",
        "property paths",
        "named graphs",
        "UPDATE"
    ]);
    Ok(result)
}

/// Compile deontic norm Quins from structured inputs (party/property/action/opcode).
pub fn compile_deontic_norms(args: &Value) -> Result<Value, String> {
    let norms = args
        .get("norms")
        .and_then(Value::as_array)
        .ok_or_else(|| "norms must be an array".to_string())?;
    if norms.len() > 256 {
        return Err("norms exceeds 256-item bound".into());
    }
    let mut quins = Vec::with_capacity(norms.len());
    let mut compiled = Vec::with_capacity(norms.len());
    for (i, n) in norms.iter().enumerate() {
        let party = hash_or_u64(n, "party", "partyIri")?;
        let property = hash_or_u64(n, "property", "propertyIri")?;
        let action = hash_or_u64(n, "action", "actionIri")?;
        let contract = hash_or_u64(n, "contract", "contractIri").unwrap_or(0);
        let expiry = n
            .get("expiryUnix")
            .map(|v| parse_u64(v, "expiryUnix"))
            .transpose()?
            .unwrap_or(0);
        let expiry_u32 = u32::try_from(expiry).map_err(|_| "expiryUnix exceeds u32".to_string())?;
        let is_defeater = n
            .get("isDefeater")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let opcode = match n
            .get("opcode")
            .and_then(Value::as_str)
            .unwrap_or("obligate")
            .to_ascii_lowercase()
            .as_str()
        {
            "obligate" | "obligation" | "must" | "0x10" | "16" => OP_OBLIGATE,
            "permit" | "permission" | "may" | "0x11" | "17" => OP_PERMIT,
            "forbid" | "prohibition" | "must-not" | "0x12" | "18" => OP_FORBID,
            other => return Err(format!("norms[{i}].opcode unsupported: {other}")),
        };
        let q = compile_norm_quin(
            party,
            opcode,
            property,
            action,
            contract,
            expiry_u32,
            is_defeater,
        );
        compiled.push(json!({
            "index": i,
            "opcode": opcode,
            "isDefeater": is_defeater,
            "quin": quin_json(&q)
        }));
        quins.push(q);
    }

    // Optionally store into a session graph
    let store = args
        .get("storeAsGraph")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut graph_id = Value::Null;
    if store {
        let id = allocate_id(args.get("graphId").and_then(Value::as_str));
        let g = SessionGraph {
            id: id.clone(),
            label: args
                .get("label")
                .and_then(Value::as_str)
                .unwrap_or("deontic-norms")
                .to_string(),
            source_format: "deontic-compile".into(),
            quins: quins.clone(),
            lexicon: HashMap::new(),
        };
        insert_graph(g)?;
        graph_id = json!(id);
    }

    Ok(json!({
        "normCount": compiled.len(),
        "norms": compiled,
        "quins": quins.iter().map(quin_json).collect::<Vec<_>>(),
        "graphId": graph_id,
        "queryMeta": {
            "tool": "compile_deontic_norms",
            "logicMode": "evaluate",
            "opcodes": { "obligate": OP_OBLIGATE, "permit": OP_PERMIT, "forbid": OP_FORBID }
        }
    }))
}

/// Evaluate deontic norms: either `graphId` session graph or `quins` / compile result.
pub fn evaluate_deontic_session(args: &Value) -> Result<Value, String> {
    let now = required_u32(args, "nowUnix")?;
    let quins = if let Some(gid) = args.get("graphId").and_then(Value::as_str) {
        with_graphs(|map| {
            map.get(gid)
                .map(|g| g.quins.clone())
                .ok_or_else(|| format!("unknown graphId '{gid}'"))
        })?
    } else {
        let arr = args
            .get("quins")
            .and_then(Value::as_array)
            .ok_or_else(|| "provide graphId or quins".to_string())?;
        let mut q = Vec::with_capacity(arr.len());
        for (i, v) in arr.iter().enumerate() {
            q.push(parse_quin(v, &format!("quins[{i}]"))?);
        }
        q
    };
    if quins.len() > MAX_INPUT_QUINS {
        return Err(format!("quins exceeds {MAX_INPUT_QUINS}"));
    }
    let mut out = vec![DeonticVerdict::default(); quins.len().max(1)];
    let count = evaluate_deontic_contract(&quins, now, &mut out)
        .map_err(|e| format!("deontic evaluation failed: {e:?}"))?;
    let verdicts: Vec<Value> = out[..count]
        .iter()
        .map(|v| {
            json!({
                "status": format!("{:?}", v.status).to_ascii_lowercase(),
                "opcode": v.opcode,
                "defeatKind": format!("{:?}", v.defeat_kind),
                "norm": quin_json(&v.norm)
            })
        })
        .collect();
    Ok(json!({
        "verdictCount": count,
        "verdicts": verdicts,
        "nowUnix": now,
        "queryMeta": {
            "tool": "evaluate_deontic_session",
            "logicMode": "evaluate",
            "note": "Statuses are engine verdicts over loaded norms, not legal advice"
        }
    }))
}

// ─── Internals ───────────────────────────────────────────────────────────────

fn hash_or_u64(obj: &Value, num_field: &str, iri_field: &str) -> Result<u64, String> {
    if let Some(v) = obj.get(num_field) {
        return parse_u64(v, num_field);
    }
    if let Some(iri) = obj.get(iri_field).and_then(Value::as_str) {
        return Ok(q_hash(iri));
    }
    Err(format!("provide {num_field} or {iri_field}"))
}

fn n3_to_quins(source: &str) -> Result<(Vec<NQuin>, HashMap<u64, String>, bool), String> {
    use modalities::logic::n3_parser::{N3Parser, StackEvent};

    let mut quins = Vec::new();
    let mut lexicon: HashMap<u64, String> = HashMap::new();
    let mut truncated = false;
    let mut event_count = 0usize;
    let mut parser = N3Parser::new(source);
    parser
        .parse_all_zero_heap(|event| {
            if event_count >= MAX_N3_EVENTS {
                truncated = true;
                return Ok(());
            }
            event_count += 1;
            if let StackEvent::StaticTriple(t) = event {
                if quins.len() >= MAX_INPUT_QUINS {
                    truncated = true;
                    return Ok(());
                }
                let (sh, ss) = term_hash_label(t.subject);
                let (ph, ps) = term_hash_label(t.predicate);
                let (oh, os) = term_hash_label(t.object);
                if lexicon.len() < MAX_LEXICON_ENTRIES {
                    lexicon.entry(sh).or_insert(ss);
                    lexicon.entry(ph).or_insert(ps);
                    lexicon.entry(oh).or_insert(os);
                }
                let context = 0u64;
                let metadata = 0u64;
                let parity = sh ^ ph ^ oh ^ context;
                quins.push(NQuin {
                    subject: sh,
                    predicate: ph,
                    object: oh,
                    context,
                    metadata,
                    parity,
                });
            }
            Ok(())
        })
        .map_err(|e| format!("N3 parse failed: {e:?}"))?;
    Ok((quins, lexicon, truncated))
}

fn term_hash_label(term: modalities::logic::n3_parser::Term<'_>) -> (u64, String) {
    use modalities::logic::n3_parser::Term;
    match term {
        Term::Uri(s) | Term::Literal(s) | Term::Variable(s) | Term::Formula(s) => {
            let owned = s.to_string();
            (q_hash(&owned), owned)
        }
    }
}

fn encode_q42lite(quins: &[NQuin], lexicon: &HashMap<u64, String>) -> Vec<u8> {
    let lex_json = if lexicon.is_empty() {
        Vec::new()
    } else {
        // store as array of [hash_str, string]
        let mut pairs = Vec::new();
        for (h, s) in lexicon {
            pairs.push(json!([h.to_string(), s]));
        }
        serde_json::to_vec(&pairs).unwrap_or_default()
    };
    let mut out = Vec::with_capacity(16 + lex_json.len() + quins.len() * 48);
    out.extend_from_slice(Q42L_MAGIC);
    out.extend_from_slice(&Q42L_VERSION.to_le_bytes());
    out.extend_from_slice(&0u16.to_le_bytes()); // flags
    out.extend_from_slice(&(quins.len() as u32).to_le_bytes());
    out.extend_from_slice(&(lex_json.len() as u32).to_le_bytes());
    out.extend_from_slice(&lex_json);
    for q in quins {
        out.extend_from_slice(&q.subject.to_le_bytes());
        out.extend_from_slice(&q.predicate.to_le_bytes());
        out.extend_from_slice(&q.object.to_le_bytes());
        out.extend_from_slice(&q.context.to_le_bytes());
        out.extend_from_slice(&q.metadata.to_le_bytes());
        out.extend_from_slice(&q.parity.to_le_bytes());
    }
    out
}

fn decode_q42lite(bytes: &[u8]) -> Result<(Vec<NQuin>, HashMap<u64, String>, String), String> {
    if bytes.len() < 16 {
        return Err("Q42L payload too short".into());
    }
    if &bytes[0..4] != Q42L_MAGIC {
        return Err("bad Q42L magic".into());
    }
    let version = u16::from_le_bytes([bytes[4], bytes[5]]);
    if version != Q42L_VERSION {
        return Err(format!("unsupported Q42L version {version}"));
    }
    let quin_count = u32::from_le_bytes([bytes[8], bytes[9], bytes[10], bytes[11]]) as usize;
    let lex_len = u32::from_le_bytes([bytes[12], bytes[13], bytes[14], bytes[15]]) as usize;
    if quin_count > MAX_INPUT_QUINS {
        return Err(format!(
            "Q42L quin_count {quin_count} exceeds {MAX_INPUT_QUINS}"
        ));
    }
    let mut off = 16usize;
    let mut lexicon = HashMap::new();
    if lex_len > 0 {
        if bytes.len() < off + lex_len {
            return Err("Q42L lexicon truncated".into());
        }
        let lex_slice = &bytes[off..off + lex_len];
        off += lex_len;
        if let Ok(Value::Array(pairs)) = serde_json::from_slice::<Value>(lex_slice) {
            for p in pairs {
                if let Some(arr) = p.as_array() {
                    if arr.len() >= 2 {
                        if let (Some(h), Some(s)) = (arr[0].as_str(), arr[1].as_str()) {
                            if let Ok(hv) = h.parse::<u64>() {
                                if lexicon.len() < MAX_LEXICON_ENTRIES {
                                    lexicon.insert(hv, s.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    let need = off + quin_count * 48;
    if bytes.len() < need {
        return Err("Q42L quins truncated".into());
    }
    let mut quins = Vec::with_capacity(quin_count);
    for i in 0..quin_count {
        let b = &bytes[off + i * 48..off + (i + 1) * 48];
        let rd = |o: usize| u64::from_le_bytes(b[o..o + 8].try_into().unwrap());
        quins.push(NQuin {
            subject: rd(0),
            predicate: rd(8),
            object: rd(16),
            context: rd(24),
            metadata: rd(32),
            parity: rd(40),
        });
    }
    Ok((quins, lexicon, "q42lite".to_string()))
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    // Minimal base64 decode (standard alphabet, ignore whitespace).
    const T: &[u8; 256] = &{
        let mut t = [0xffu8; 256];
        let mut i = 0u8;
        while i < 26 {
            t[(b'A' + i) as usize] = i;
            t[(b'a' + i) as usize] = 26 + i;
            i += 1;
        }
        i = 0;
        while i < 10 {
            t[(b'0' + i) as usize] = 52 + i;
            i += 1;
        }
        t[b'+' as usize] = 62;
        t[b'/' as usize] = 63;
        t
    };
    let clean: Vec<u8> = input.bytes().filter(|b| !b.is_ascii_whitespace()).collect();
    if clean.len() % 4 != 0 {
        return Err("invalid base64 length".into());
    }
    let mut out = Vec::with_capacity(clean.len() / 4 * 3);
    for chunk in clean.chunks_exact(4) {
        let a = T[chunk[0] as usize];
        let b = T[chunk[1] as usize];
        let c = if chunk[2] == b'=' {
            0
        } else {
            T[chunk[2] as usize]
        };
        let d = if chunk[3] == b'=' {
            0
        } else {
            T[chunk[3] as usize]
        };
        if a == 0xff
            || b == 0xff
            || (chunk[2] != b'=' && c == 0xff)
            || (chunk[3] != b'=' && d == 0xff)
        {
            return Err("invalid base64 character".into());
        }
        out.push((a << 2) | (b >> 4));
        if chunk[2] != b'=' {
            out.push((b << 4) | (c >> 2));
        }
        if chunk[3] != b'=' {
            out.push((c << 6) | d);
        }
    }
    Ok(out)
}

fn encode_base64(data: &[u8]) -> String {
    const A: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::new();
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = if chunk.len() > 1 { chunk[1] as u32 } else { 0 };
        let b2 = if chunk.len() > 2 { chunk[2] as u32 } else { 0 };
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(A[((n >> 18) & 63) as usize] as char);
        out.push(A[((n >> 12) & 63) as usize] as char);
        if chunk.len() > 1 {
            out.push(A[((n >> 6) & 63) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(A[(n & 63) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

fn collapse_ws(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_space = false;
    for c in s.chars() {
        if c.is_whitespace() {
            if !prev_space {
                out.push(' ');
                prev_space = true;
            }
        } else {
            out.push(c);
            prev_space = false;
        }
    }
    out
}

/// MCP helper: return base64 Q42L of an existing session graph (for transfer docs/tests).
pub fn export_q42lite(args: &Value) -> Result<Value, String> {
    let graph_id = required_str(args, "graphId")?;
    with_graphs(|map| {
        let g = map
            .get(graph_id)
            .ok_or_else(|| format!("unknown graphId '{graph_id}'"))?;
        let bytes = encode_q42lite(&g.quins, &g.lexicon);
        Ok(json!({
            "graphId": graph_id,
            "format": "q42lite",
            "byteLength": bytes.len(),
            "bytesBase64": encode_base64(&bytes),
            "quinCount": g.quins.len(),
            "queryMeta": { "tool": "export_q42lite", "magic": "Q42L" }
        }))
    })
}
