//! Browser-local MCP endpoint for ontology sites such as `ns.webcivics.net`.
//!
//! The crate deliberately depends on Qualia's `wasm-ontology` profile: no
//! renderer, WebGPU, scientific solver bundle, LLM runtime, daemon, or network
//! stack is admitted to this binary.
//!
//! # Copyright
//!
//! Copyright (c) 2026 Timothy Charles Holborn  
//! Licensed under CC BY-NC-ND 4.0 (Attribution-NonCommercial-NoDerivatives),
//! matching the Webizen / ns.webcivics.net technical-work rights scope.
//! See the crate `LICENSE` file.

use qualia_core_db::modalities;
use qualia_core_db::NQuin;
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

mod session;

const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26"];
pub(crate) const MAX_INPUT_QUINS: usize = 4_096;
pub(crate) const MAX_QUERY_RESULTS: usize = 512;
pub(crate) const MAX_N3_EVENTS: usize = 512;

/// Handle one MCP JSON-RPC 2.0 message.
///
/// Notifications return an empty string because JSON-RPC notifications do not
/// have response objects.
#[wasm_bindgen]
pub fn mcp_jsonrpc(message: &str) -> String {
    let req: Value = match serde_json::from_str(message) {
        Ok(value) => value,
        Err(_) => return rpc_error(Value::Null, -32700, "Parse error"),
    };

    if req.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
        return rpc_error(
            req.get("id").cloned().unwrap_or(Value::Null),
            -32600,
            "Invalid Request",
        );
    }

    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    if req.get("id").is_none() {
        return String::new();
    }

    let id = req.get("id").cloned().unwrap_or(Value::Null);
    match method {
        "initialize" => initialize(id, req.get("params")),
        "ping" => rpc_ok(id, json!({})),
        "tools/list" => rpc_ok(id, json!({ "tools": tool_catalog() })),
        "tools/call" => {
            let params = req.get("params").cloned().unwrap_or(Value::Null);
            let name = params.get("name").and_then(Value::as_str).unwrap_or("");
            let args = params
                .get("arguments")
                .cloned()
                .unwrap_or_else(|| json!({}));
            match call_tool(name, &args) {
                Ok(result) => tool_result(id, result, false),
                Err(message) => tool_result(id, json!({ "error": message }), true),
            }
        }
        _ => rpc_error(id, -32601, "Method not found"),
    }
}

/// Crate/build version, for an embed to confirm that the bridge loaded.
#[wasm_bindgen]
pub fn version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

fn initialize(id: Value, params: Option<&Value>) -> String {
    let requested = params
        .and_then(|p| p.get("protocolVersion"))
        .and_then(Value::as_str);
    let negotiated = requested
        .filter(|version| SUPPORTED_PROTOCOL_VERSIONS.contains(version))
        .unwrap_or(LATEST_PROTOCOL_VERSION);

    rpc_ok(
        id,
        json!({
            "protocolVersion": negotiated,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": {
                "name": "webizen-lite-wasm",
                "title": "Qualia Ontology MCP",
                "version": env!("CARGO_PKG_VERSION")
            },
            "instructions": "Browser-local, read-only ontology parsing, bounded Quin queries, SHACL checks, and modal reasoning. u64 Quin fields are returned as decimal strings."
        }),
    )
}

fn call_tool(name: &str, args: &Value) -> Result<Value, String> {
    match name {
        "ontology_capabilities" => ontology_capabilities(),
        "hash_iri" => hash_iri(args),
        "parse_n3" => parse_n3(args),
        "query_quins" => query_quins(args),
        "validate_shacl" => validate_shacl(args),
        "evaluate_deontic" => evaluate_deontic(args),
        "evaluate_epistemic" => evaluate_epistemic(args),
        "route_paraconsistent" => route_paraconsistent(args),
        "evaluate_ltl" => evaluate_ltl(args),
        "check_subsumption" => check_subsumption(args),
        "deontic_govern" => deontic_govern(args),
        // Site-discovery helpers (no network — host/agent fetches, then calls WASM).
        "namespace_discovery_help" => namespace_discovery_help(args),
        "catalog_summarize" => catalog_summarize(args),
        "corpus_summarize" => corpus_summarize(args),
        "resolve_dataset_urls" => resolve_dataset_urls(args),
        "export_graph" => export_graph(args),
        // Session graph + query + deontic bridge (P2–P4)
        "load_graph" => session::load_graph(args),
        "load_q42" => session::load_q42(args),
        "list_graphs" => session::list_graphs(args),
        "unload_graph" => session::unload_graph(args),
        "query_graph" => session::query_graph(args),
        "query_sparql" => session::query_sparql(args),
        "export_q42lite" => session::export_q42lite(args),
        "compile_deontic_norms" => session::compile_deontic_norms(args),
        "evaluate_deontic_session" => session::evaluate_deontic_session(args),
        _ => Err(format!("unknown tool: {name}")),
    }
}

/// Static URL contract + agent bootstrap for ontology namespaces (default: ns.webcivics.net).
fn namespace_discovery_help(args: &Value) -> Result<Value, String> {
    let base = args
        .get("baseUrl")
        .and_then(Value::as_str)
        .unwrap_or("https://ns.webcivics.net")
        .trim_end_matches('/');
    Ok(json!({
        "profile": "wasm-ontology / webizen-lite-wasm",
        "network": "none — fetch documents with the host HTTP client, then call parse_n3 / catalog_summarize",
        "bootstrap": {
            "llmsTxt": format!("{base}/llms.txt"),
            "catalogJson": format!("{base}/catalog.json"),
            "catalogTurtle": format!("{base}/catalog.ttl"),
            "contextJsonLd": format!("{base}/context.jsonld"),
            "aiUsePolicy": format!("{base}/ai-use-policy.json"),
            "agentMcpGuide": format!("{base}/agent-mcp-guide.md"),
            "agentLegislationGuide": format!("{base}/agent-legislation-guide.md"),
            "agentConformance": format!("{base}/agent-conformance.md"),
            "auLegislationCorpus": format!("{base}/au-legislation-corpus.json"),
            "titleIndex": format!("{base}/search/title-index.json"),
            "auTitleIndex": format!("{base}/search/au-title-index.json"),
            "wasmLite": format!("{base}/wasm/webizen-lite/")
        },
        "urlContract": {
            "html": format!("{base}/institutions/un/{{slug}}/"),
            "n3": format!("{base}/institutions/un/{{slug}}.n3"),
            "turtle": format!("{base}/institutions/un/{{slug}}.ttl"),
            "jsonld": format!("{base}/institutions/un/{{slug}}.jsonld"),
            "coreExample": format!("{base}/core/agency/"),
            "directoryIndexes": [
                format!("{base}/institutions/"),
                format!("{base}/institutions/un/"),
                format!("{base}/institutions/au-fed-legislation/")
            ],
            "notes": [
                "Trailing slash marks the HTML documentation page.",
                "Extension paths (.n3/.ttl/.jsonld) are machine RDF projections.",
                "Directory paths list child instruments (no leaf .n3).",
                "Older /ontologies/... URLs redirect to short paths.",
                "Prefer title indexes for keyword discovery before loading large acts."
            ]
        },
        "exportFormats": {
            "jsonld": "Default for agents (application/ld+json)",
            "rdfjson": "W3C RDF/JSON Note (application/rdf+json) for bots without JSON-LD context",
            "turtle": "text/turtle",
            "n3": "text/n3 — use when N3 logic/rules/variables must be preserved",
            "yamlld": "YAML encoding of the same JSON-LD document model"
        },
        "logicLayers": {
            "A_ground": "Ground RDF triples — any serialisation",
            "B_as_data": "CML norms / LogicApplication as RDF (logicMode=as-data)",
            "C_executable": "N3 rules or .q42 + evaluate_* tools — not full fidelity in pure RDF/JSON"
        },
        "recommendedAgentFlow": [
            "1. GET ai-use-policy.json and llms.txt (policy first).",
            "2. Load WASM; tools/call namespace_discovery_help.",
            "3. GET catalog.json and/or au-legislation-corpus.json / search/*-index.json.",
            "4. tools/call catalog_summarize or corpus_summarize with titleContains (offline).",
            "5. Host GET preferred .n3 (or .q42 when published); prefer RDF over multi-MB HTML.",
            "6. parse_n3 / query_quins / evaluate_deontic as needed; export_graph for bot-readable RDF.",
            "7. Cite catalog canonicalUrl + official Register/ELI; treat cml:Proposed as hypothesis."
        ],
        "plan": "https://github.com/webcivics/qualia / docs/plans/wasm-lite-agent-query-plan.md (Qualia monorepo)",
        "mcp": {
            "transport": "in-process WASM (mcp_jsonrpc)",
            "methods": ["initialize", "ping", "tools/list", "tools/call"],
            "protocolVersions": SUPPORTED_PROTOCOL_VERSIONS
        }
    }))
}

/// Summarise a DCAT catalog JSON string (fetched by the host) for agent navigation.
fn catalog_summarize(args: &Value) -> Result<Value, String> {
    let catalog_str = required_str(args, "catalogJson")?;
    let catalog: Value = serde_json::from_str(catalog_str)
        .map_err(|e| format!("catalogJson is not valid JSON: {e}"))?;
    let category_filter = args
        .get("categoryPrefix")
        .and_then(Value::as_str)
        .map(|s| s.trim_matches('/').to_string());
    let title_contains = args
        .get("titleContains")
        .and_then(Value::as_str)
        .map(|s| s.to_ascii_lowercase());
    let id_prefix = args
        .get("idPrefix")
        .and_then(Value::as_str)
        .map(|s| s.to_string());
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(64)
        .min(512) as usize;

    let datasets = catalog
        .get("datasets")
        .and_then(Value::as_array)
        .ok_or_else(|| "catalogJson.datasets must be an array".to_string())?;

    let mut categories: std::collections::BTreeMap<String, usize> =
        std::collections::BTreeMap::new();
    let mut selected = Vec::new();
    let mut matched = 0usize;

    for ds in datasets {
        let category = ds
            .get("category")
            .and_then(Value::as_str)
            .unwrap_or("uncategorized")
            .to_string();
        *categories.entry(category.clone()).or_insert(0) += 1;

        if let Some(ref prefix) = category_filter {
            if category != *prefix && !category.starts_with(&format!("{prefix}/")) {
                continue;
            }
        }
        let id = ds.get("id").and_then(Value::as_str).unwrap_or("");
        if let Some(ref pref) = id_prefix {
            if !id.starts_with(pref.as_str()) {
                continue;
            }
        }
        let title = ds.get("title").and_then(Value::as_str).unwrap_or("");
        if let Some(ref needle) = title_contains {
            if !title.to_ascii_lowercase().contains(needle.as_str()) {
                continue;
            }
        }
        matched += 1;
        if selected.len() < limit {
            selected.push(json!({
                "id": ds.get("id"),
                "title": ds.get("title"),
                "category": category,
                "canonicalUrl": ds.get("canonicalUrl"),
                "n3Url": ds.get("n3Url"),
                "turtleUrl": ds.get("turtleUrl"),
                "jsonldUrl": ds.get("jsonldUrl"),
                "tripleCount": ds.get("tripleCount"),
                "registerId": ds.get("registerId")
            }));
        }
    }

    Ok(json!({
        "title": catalog.get("title"),
        "baseUrl": catalog.get("baseUrl"),
        "generatedAt": catalog.get("generatedAt"),
        "datasetCount": catalog.get("datasetCount").cloned().unwrap_or(json!(datasets.len())),
        "categoryCounts": categories,
        "categoryFilter": category_filter,
        "titleContains": title_contains,
        "idPrefix": id_prefix,
        "matchedCount": matched,
        "returnedCount": selected.len(),
        "truncated": matched > selected.len(),
        "datasets": selected,
        "queryMeta": {
            "tool": "catalog_summarize",
            "projectedFromN3": false,
            "logicMode": "none"
        }
    }))
}

/// Summarise a fetched AU (or similar) legislation corpus JSON for title/id search.
///
/// Accepts either `{ "datasets": [ … ] }` (preferred) or a bare JSON array of dataset objects.
fn corpus_summarize(args: &Value) -> Result<Value, String> {
    let corpus_str = required_str(args, "corpusJson")?;
    let corpus: Value = serde_json::from_str(corpus_str)
        .map_err(|e| format!("corpusJson is not valid JSON: {e}"))?;
    let title_contains = args
        .get("titleContains")
        .and_then(Value::as_str)
        .map(|s| s.to_ascii_lowercase());
    let id_prefix = args
        .get("idPrefix")
        .and_then(Value::as_str)
        .map(|s| s.to_string());
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(64)
        .min(512) as usize;

    let datasets: &[Value] = if let Some(arr) = corpus.get("datasets").and_then(Value::as_array) {
        arr
    } else if let Some(arr) = corpus.as_array() {
        arr
    } else {
        return Err("corpusJson must be an object with datasets[] or a bare array".into());
    };

    let mut selected = Vec::new();
    let mut matched = 0usize;
    for ds in datasets {
        let id = ds
            .get("id")
            .or_else(|| ds.get("registerId"))
            .and_then(Value::as_str)
            .unwrap_or("");
        if let Some(ref pref) = id_prefix {
            if !id.starts_with(pref.as_str()) {
                continue;
            }
        }
        let title = ds.get("title").and_then(Value::as_str).unwrap_or("");
        if let Some(ref needle) = title_contains {
            if !title.to_ascii_lowercase().contains(needle.as_str()) {
                continue;
            }
        }
        matched += 1;
        if selected.len() < limit {
            selected.push(json!({
                "id": ds.get("id").or_else(|| ds.get("registerId")),
                "registerId": ds.get("registerId"),
                "title": ds.get("title"),
                "canonicalUrl": ds.get("canonicalUrl"),
                "n3Url": ds.get("n3Url"),
                "officialSource": ds.get("officialSource"),
                "rdfTriples": ds.get("rdfTriples").or_else(|| ds.get("tripleCount")),
                "n3Bytes": ds.get("n3Bytes"),
                "curationStatus": ds.get("curationStatus")
            }));
        }
    }

    Ok(json!({
        "jurisdiction": corpus.get("jurisdiction"),
        "datasetCount": corpus.get("datasetCount").cloned().unwrap_or(json!(datasets.len())),
        "titleContains": title_contains,
        "idPrefix": id_prefix,
        "matchedCount": matched,
        "returnedCount": selected.len(),
        "truncated": matched > selected.len(),
        "datasets": selected,
        "queryMeta": {
            "tool": "corpus_summarize",
            "projectedFromN3": false,
            "logicMode": "none"
        }
    }))
}

/// Export ground RDF triples in a bot-readable serialisation.
///
/// Input triples: `{ "s", "p", "o" }` where `o` is an IRI string or
/// `{ "type": "uri"|"literal"|"bnode", "value", "lang"?, "datatype"? }`.
fn export_graph(args: &Value) -> Result<Value, String> {
    let format = args
        .get("format")
        .and_then(Value::as_str)
        .unwrap_or("jsonld")
        .to_ascii_lowercase();
    let logic_mode = args
        .get("logicMode")
        .and_then(Value::as_str)
        .unwrap_or("none")
        .to_ascii_lowercase();
    if !matches!(
        logic_mode.as_str(),
        "none" | "as-data" | "evaluate" | "native-n3"
    ) {
        return Err("logicMode must be none|as-data|evaluate|native-n3".into());
    }
    let context_url = args
        .get("contextUrl")
        .and_then(Value::as_str)
        .unwrap_or("https://ns.webcivics.net/context.jsonld");
    let triples_raw = args
        .get("triples")
        .and_then(Value::as_array)
        .ok_or_else(|| "triples must be an array".to_string())?;
    if triples_raw.len() > MAX_QUERY_RESULTS {
        return Err(format!(
            "triples exceeds the {MAX_QUERY_RESULTS}-item export bound"
        ));
    }

    let mut triples = Vec::with_capacity(triples_raw.len());
    for (i, t) in triples_raw.iter().enumerate() {
        triples.push(parse_export_triple(t, i)?);
    }

    let dropped = match logic_mode.as_str() {
        "none" | "as-data" => vec![
            "n3:formulae".to_string(),
            "n3:variables".to_string(),
            "n3:=> rules".to_string(),
        ],
        "evaluate" => vec!["n3:formulae".to_string(), "n3:variables".to_string()],
        "native-n3" => Vec::new(),
        _ => Vec::new(),
    };

    let (media_type, body) = match format.as_str() {
        "jsonld" | "application/ld+json" => {
            ("application/ld+json", export_jsonld(&triples, context_url)?)
        }
        "rdfjson" | "rdf/json" | "application/rdf+json" | "rj" => {
            ("application/rdf+json", export_rdf_json(&triples)?)
        }
        "turtle" | "ttl" | "text/turtle" => ("text/turtle", Value::String(export_turtle(&triples))),
        "n3" | "text/n3" => ("text/n3", Value::String(export_n3(&triples))),
        "yamlld" | "yaml-ld" | "application/ld+yaml" => {
            let jsonld = export_jsonld(&triples, context_url)?;
            (
                "application/ld+yaml",
                Value::String(json_to_yaml_like(&jsonld)),
            )
        }
        other => {
            return Err(format!(
                "unsupported format '{other}' (use jsonld|rdfjson|turtle|n3|yamlld)"
            ))
        }
    };

    Ok(json!({
        "format": format,
        "mediaType": media_type,
        "tripleCount": triples.len(),
        "body": body,
        "queryMeta": {
            "tool": "export_graph",
            "graphFormat": media_type,
            "logicMode": logic_mode,
            "projectedFromN3": logic_mode != "native-n3",
            "dropped": dropped,
            "truncated": false,
            "note": "Ground triples only. N3 logic/rules require format=n3 with logicMode=native-n3 or evaluate_* tools on Quins/.q42."
        }
    }))
}

#[derive(Clone)]
struct ExportTerm {
    kind: ExportTermKind,
    value: String,
    lang: Option<String>,
    datatype: Option<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportTermKind {
    Uri,
    Literal,
    BNode,
}

struct ExportTriple {
    s: ExportTerm,
    p: ExportTerm,
    o: ExportTerm,
}

fn parse_export_triple(value: &Value, index: usize) -> Result<ExportTriple, String> {
    let obj = value
        .as_object()
        .ok_or_else(|| format!("triples[{index}] must be an object"))?;
    let s = parse_export_term(
        obj.get("s")
            .or_else(|| obj.get("subject"))
            .ok_or_else(|| format!("triples[{index}].s is required"))?,
        true,
    )?;
    let p = parse_export_term(
        obj.get("p")
            .or_else(|| obj.get("predicate"))
            .ok_or_else(|| format!("triples[{index}].p is required"))?,
        true,
    )?;
    let o = parse_export_term(
        obj.get("o")
            .or_else(|| obj.get("object"))
            .ok_or_else(|| format!("triples[{index}].o is required"))?,
        false,
    )?;
    if p.kind != ExportTermKind::Uri {
        return Err(format!("triples[{index}].p must be a URI"));
    }
    if s.kind == ExportTermKind::Literal {
        return Err(format!("triples[{index}].s cannot be a literal"));
    }
    Ok(ExportTriple { s, p, o })
}

fn parse_export_term(value: &Value, prefer_uri: bool) -> Result<ExportTerm, String> {
    if let Some(s) = value.as_str() {
        if s.starts_with("_:") {
            return Ok(ExportTerm {
                kind: ExportTermKind::BNode,
                value: s.to_string(),
                lang: None,
                datatype: None,
            });
        }
        if prefer_uri
            || s.starts_with("http://")
            || s.starts_with("https://")
            || s.starts_with("urn:")
        {
            return Ok(ExportTerm {
                kind: ExportTermKind::Uri,
                value: s.to_string(),
                lang: None,
                datatype: None,
            });
        }
        return Ok(ExportTerm {
            kind: ExportTermKind::Literal,
            value: s.to_string(),
            lang: None,
            datatype: None,
        });
    }
    let obj = value
        .as_object()
        .ok_or_else(|| "term must be a string or object".to_string())?;
    let raw_type = obj
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or(if prefer_uri { "uri" } else { "literal" });
    let kind = match raw_type {
        "uri" | "iri" | "url" => ExportTermKind::Uri,
        "literal" => ExportTermKind::Literal,
        "bnode" | "blank" => ExportTermKind::BNode,
        other => return Err(format!("unsupported term type '{other}'")),
    };
    let term_value = obj
        .get("value")
        .and_then(Value::as_str)
        .ok_or_else(|| "term.value must be a string".to_string())?
        .to_string();
    Ok(ExportTerm {
        kind,
        value: term_value,
        lang: obj
            .get("lang")
            .or_else(|| obj.get("language"))
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
        datatype: obj
            .get("datatype")
            .and_then(Value::as_str)
            .map(|s| s.to_string()),
    })
}

fn export_jsonld(triples: &[ExportTriple], context_url: &str) -> Result<Value, String> {
    use std::collections::BTreeMap;
    let mut by_subject: BTreeMap<String, BTreeMap<String, Vec<Value>>> = BTreeMap::new();
    for t in triples {
        let s_key = match t.s.kind {
            ExportTermKind::Uri | ExportTermKind::BNode => t.s.value.clone(),
            ExportTermKind::Literal => return Err("subject literal in JSON-LD export".into()),
        };
        let p_key = t.p.value.clone();
        let o_val = match t.o.kind {
            ExportTermKind::Uri => json!({ "@id": t.o.value }),
            ExportTermKind::BNode => json!({ "@id": t.o.value }),
            ExportTermKind::Literal => {
                let mut lit = json!({ "@value": t.o.value });
                if let Some(ref lang) = t.o.lang {
                    lit["@language"] = json!(lang);
                } else if let Some(ref dt) = t.o.datatype {
                    lit["@type"] = json!(dt);
                }
                lit
            }
        };
        by_subject
            .entry(s_key)
            .or_default()
            .entry(p_key)
            .or_default()
            .push(o_val);
    }
    let mut graph = Vec::new();
    for (s, preds) in by_subject {
        let mut node = json!({ "@id": s });
        for (p, objs) in preds {
            if objs.len() == 1 {
                node[p] = objs[0].clone();
            } else {
                node[p] = Value::Array(objs);
            }
        }
        graph.push(node);
    }
    Ok(json!({
        "@context": context_url,
        "@graph": graph
    }))
}

fn export_rdf_json(triples: &[ExportTriple]) -> Result<Value, String> {
    use std::collections::BTreeMap;
    // W3C RDF/JSON: { S: { P: [ {type,value,lang?,datatype?} ] } }
    let mut root: BTreeMap<String, BTreeMap<String, Vec<Value>>> = BTreeMap::new();
    for t in triples {
        let s_key = t.s.value.clone();
        let p_key = t.p.value.clone();
        let o_obj = match t.o.kind {
            ExportTermKind::Uri => json!({ "type": "uri", "value": t.o.value }),
            ExportTermKind::BNode => json!({ "type": "bnode", "value": t.o.value }),
            ExportTermKind::Literal => {
                let mut o = json!({ "type": "literal", "value": t.o.value });
                if let Some(ref lang) = t.o.lang {
                    o["lang"] = json!(lang);
                }
                if let Some(ref dt) = t.o.datatype {
                    o["datatype"] = json!(dt);
                }
                o
            }
        };
        root.entry(s_key)
            .or_default()
            .entry(p_key)
            .or_default()
            .push(o_obj);
    }
    let mut out = serde_json::Map::new();
    for (s, preds) in root {
        let mut pred_obj = serde_json::Map::new();
        for (p, arr) in preds {
            pred_obj.insert(p, Value::Array(arr));
        }
        out.insert(s, Value::Object(pred_obj));
    }
    Ok(Value::Object(out))
}

fn turtle_escape_iri(iri: &str) -> String {
    format!("<{}>", iri.replace('>', "\\>"))
}

fn turtle_escape_literal(s: &str) -> String {
    let escaped = s
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r");
    format!("\"{escaped}\"")
}

fn term_turtle(t: &ExportTerm) -> String {
    match t.kind {
        ExportTermKind::Uri => turtle_escape_iri(&t.value),
        ExportTermKind::BNode => {
            if t.value.starts_with("_:") {
                t.value.clone()
            } else {
                format!("_:{}", t.value)
            }
        }
        ExportTermKind::Literal => {
            let mut lit = turtle_escape_literal(&t.value);
            if let Some(ref lang) = t.lang {
                lit.push('@');
                lit.push_str(lang);
            } else if let Some(ref dt) = t.datatype {
                lit.push_str("^^");
                lit.push_str(&turtle_escape_iri(dt));
            }
            lit
        }
    }
}

fn export_turtle(triples: &[ExportTriple]) -> String {
    let mut out = String::new();
    for t in triples {
        out.push_str(&term_turtle(&t.s));
        out.push(' ');
        out.push_str(&term_turtle(&t.p));
        out.push(' ');
        out.push_str(&term_turtle(&t.o));
        out.push_str(" .\n");
    }
    out
}

fn export_n3(triples: &[ExportTriple]) -> String {
    // Ground-triple N3 projection (same as Turtle for this subset).
    export_turtle(triples)
}

/// Minimal YAML-like emitter for JSON values (no external yaml crate).
fn json_to_yaml_like(value: &Value) -> String {
    fn emit(value: &Value, indent: usize, out: &mut String) {
        let pad = "  ".repeat(indent);
        match value {
            Value::Null => out.push_str("null"),
            Value::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
            Value::Number(n) => out.push_str(&n.to_string()),
            Value::String(s) => {
                if s.contains('\n') || s.contains(':') || s.contains('#') {
                    out.push('"');
                    out.push_str(&s.replace('\\', "\\\\").replace('"', "\\\""));
                    out.push('"');
                } else {
                    out.push_str(s);
                }
            }
            Value::Array(arr) => {
                if arr.is_empty() {
                    out.push_str("[]");
                    return;
                }
                for (i, item) in arr.iter().enumerate() {
                    if i > 0 {
                        out.push('\n');
                        out.push_str(&pad);
                    }
                    out.push_str("- ");
                    match item {
                        Value::Object(_) | Value::Array(_) => {
                            out.push('\n');
                            emit(item, indent + 1, out);
                        }
                        _ => emit(item, indent + 1, out),
                    }
                }
            }
            Value::Object(map) => {
                if map.is_empty() {
                    out.push_str("{}");
                    return;
                }
                for (i, (k, v)) in map.iter().enumerate() {
                    if i > 0 {
                        out.push('\n');
                        out.push_str(&pad);
                    }
                    out.push_str(k);
                    out.push(':');
                    match v {
                        Value::Object(_) | Value::Array(_) => {
                            out.push('\n');
                            out.push_str(&"  ".repeat(indent + 1));
                            emit(v, indent + 1, out);
                        }
                        _ => {
                            out.push(' ');
                            emit(v, indent + 1, out);
                        }
                    }
                }
            }
        }
    }
    let mut out = String::new();
    emit(value, 0, &mut out);
    out.push('\n');
    out
}

/// Build canonical HTML + RDF URLs for a short namespace path (no network).
fn resolve_dataset_urls(args: &Value) -> Result<Value, String> {
    let base = args
        .get("baseUrl")
        .and_then(Value::as_str)
        .unwrap_or("https://ns.webcivics.net")
        .trim_end_matches('/');
    let mut path = required_str(args, "path")?.trim().to_string();
    if path.is_empty() {
        return Err("path must be non-empty".into());
    }
    if !path.starts_with('/') {
        path = format!("/{path}");
    }
    // Strip trailing slash and accidental extensions for the data path stem.
    let stem = path
        .trim_end_matches('/')
        .trim_end_matches(".n3")
        .trim_end_matches(".ttl")
        .trim_end_matches(".jsonld")
        .trim_end_matches(".json");
    Ok(json!({
        "dataPath": stem,
        "htmlUrl": format!("{base}{stem}/"),
        "n3Url": format!("{base}{stem}.n3"),
        "turtleUrl": format!("{base}{stem}.ttl"),
        "jsonldUrl": format!("{base}{stem}.jsonld"),
        "rawN3Url": format!("{base}/raw/ontologies{stem}.n3"),
        "note": "Directory indexes use the trailing-slash HTML URL only; leaf documents also publish RDF extensions."
    }))
}

fn ontology_capabilities() -> Result<Value, String> {
    Ok(json!({
        "profile": qualia_core_db::wasm_capabilities::compiled_profile(),
        "capabilities": qualia_core_db::wasm_capabilities::compiled_capabilities(),
        "limits": {
            "inputQuins": MAX_INPUT_QUINS,
            "queryResults": MAX_QUERY_RESULTS,
            "n3Events": MAX_N3_EVENTS
        },
        "excluded": [
            "portal-renderer",
            "webgpu",
            "scientific-solvers",
            "llm-inference",
            "native-daemon",
            "network-stack",
            "filesystem-storage"
        ]
    }))
}

fn hash_iri(args: &Value) -> Result<Value, String> {
    let iri = required_str(args, "iri")?;
    Ok(json!({
        "iri": iri,
        "hash": qualia_core_db::q_hash(iri).to_string()
    }))
}

fn parse_n3(args: &Value) -> Result<Value, String> {
    use modalities::logic::n3_parser::{N3Parser, StackEvent, Term};

    let source = required_str(args, "source")?;
    let mut events = Vec::new();
    let mut truncated = false;
    let mut parser = N3Parser::new(source);
    parser
        .parse_all_zero_heap(|event| {
            if events.len() >= MAX_N3_EVENTS {
                truncated = true;
                return Ok(());
            }
            let value = match event {
                StackEvent::StaticTriple(t) => json!({
                    "type": "triple",
                    "subject": term_json(t.subject),
                    "predicate": term_json(t.predicate),
                    "object": term_json(t.object)
                }),
                StackEvent::LogicRule(rule) => json!({
                    "type": "rule",
                    "ruleType": format!("{:?}", rule.rule_type),
                    "weight": rule.weight,
                    "premiseCount": rule.premise.len,
                    "conclusionCount": rule.conclusion.len
                }),
                StackEvent::AspBlock(block) => {
                    json!({ "type": "asp", "source": block })
                }
                StackEvent::DiffuseBlock(block) => {
                    json!({ "type": "diffuse", "source": block })
                }
            };
            events.push(value);
            Ok(())
        })
        .map_err(|error| error.to_string())?;

    fn term_json(term: Term<'_>) -> Value {
        match term {
            Term::Uri(value) => json!({ "kind": "iri", "value": value }),
            Term::Variable(value) => json!({ "kind": "variable", "value": value }),
            Term::Literal(value) => json!({ "kind": "literal", "value": value }),
            Term::Formula(value) => json!({ "kind": "formula", "value": value }),
        }
    }

    Ok(json!({
        "eventCount": events.len(),
        "truncated": truncated,
        "events": events
    }))
}

fn query_quins(args: &Value) -> Result<Value, String> {
    let quins = parse_quins(args)?;
    let subject = optional_u64(args, "subject")?;
    let predicate = optional_u64(args, "predicate")?;
    let object = optional_u64(args, "object")?;
    let context = optional_u64(args, "context")?;
    let limit = args
        .get("limit")
        .and_then(Value::as_u64)
        .unwrap_or(64)
        .min(MAX_QUERY_RESULTS as u64) as usize;

    let mut matches = Vec::new();
    let mut match_count = 0usize;
    for quin in &quins {
        if subject.is_some_and(|value| quin.subject != value)
            || predicate.is_some_and(|value| quin.predicate != value)
            || object.is_some_and(|value| quin.object != value)
            || context.is_some_and(|value| quin.context != value)
        {
            continue;
        }
        match_count += 1;
        if matches.len() < limit {
            matches.push(quin_json(quin));
        }
    }

    Ok(json!({
        "matchCount": match_count,
        "returnedCount": matches.len(),
        "truncated": match_count > matches.len(),
        "matches": matches
    }))
}

fn validate_shacl(args: &Value) -> Result<Value, String> {
    use qualia_core_db::shacl_compiler::{validate_shacl_property, ShaclConstraint, ShaclDatatype};

    let quins = parse_quins(args)?;
    let target_subject = required_u64(args, "targetSubject")?;
    let target_property = required_u64(args, "targetProperty")?;
    let raw_constraints = args
        .get("constraints")
        .and_then(Value::as_array)
        .ok_or_else(|| "constraints must be an array".to_string())?;
    if raw_constraints.len() > 32 {
        return Err("constraints exceeds the 32-item bound".to_string());
    }

    let mut constraints = Vec::with_capacity(raw_constraints.len());
    for raw in raw_constraints {
        let kind = raw
            .get("type")
            .and_then(Value::as_str)
            .ok_or_else(|| "constraint.type must be a string".to_string())?;
        let constraint = match kind {
            "datatype" => {
                let datatype = match required_str(raw, "datatype")? {
                    "string" => ShaclDatatype::String,
                    "integer" => ShaclDatatype::Integer,
                    "decimal" => ShaclDatatype::Decimal,
                    "boolean" => ShaclDatatype::Boolean,
                    "dateTime" => ShaclDatatype::DateTime,
                    other => return Err(format!("unsupported datatype: {other}")),
                };
                ShaclConstraint::Datatype(datatype)
            }
            "minCount" => ShaclConstraint::MinCount(required_u32(raw, "value")?),
            "maxCount" => ShaclConstraint::MaxCount(required_u32(raw, "value")?),
            "minLength" => ShaclConstraint::MinLength(required_u32(raw, "value")?),
            "maxLength" => ShaclConstraint::MaxLength(required_u32(raw, "value")?),
            "deonticObligate" => ShaclConstraint::DeonticObligate,
            "deonticPermit" => ShaclConstraint::DeonticPermit,
            "deonticForbid" => ShaclConstraint::DeonticForbid,
            "deonticNotExpired" => ShaclConstraint::DeonticNotExpired {
                now_unix: required_u32(raw, "nowUnix")?,
            },
            "epistemicKnowledge" => ShaclConstraint::EpistemicKnowledge {
                min_certainty: required_u8(raw, "minCertainty")?,
            },
            "epistemicBelief" => ShaclConstraint::EpistemicBelief {
                min_certainty: required_u8(raw, "minCertainty")?,
            },
            "commonKnowledge" => ShaclConstraint::CommonKnowledge,
            "in" => {
                let values = raw
                    .get("values")
                    .and_then(Value::as_array)
                    .ok_or_else(|| "in.values must be an array".to_string())?;
                if values.len() > 8 {
                    return Err("in.values exceeds the 8-item inline bound".to_string());
                }
                let mut inline = [0u64; 8];
                for (index, value) in values.iter().enumerate() {
                    inline[index] = parse_u64(value, "in.values")?;
                }
                ShaclConstraint::In {
                    count: values.len() as u8,
                    values: inline,
                }
            }
            other => return Err(format!("unsupported SHACL constraint: {other}")),
        };
        constraints.push(constraint);
    }

    Ok(json!({
        "conforms": validate_shacl_property(
            &quins,
            target_subject,
            target_property,
            &constraints
        )
    }))
}

fn evaluate_deontic(args: &Value) -> Result<Value, String> {
    use modalities::logic::deontic::{evaluate_deontic_contract, DeonticVerdict};

    let quins = parse_quins(args)?;
    let now = required_u32(args, "nowUnix")?;
    let mut out = vec![DeonticVerdict::default(); quins.len()];
    let count = evaluate_deontic_contract(&quins, now, &mut out)
        .map_err(|error| format!("deontic evaluation failed: {error:?}"))?;
    let verdicts: Vec<Value> = out[..count]
        .iter()
        .map(|verdict| {
            json!({
                "status": format!("{:?}", verdict.status),
                "opcode": verdict.opcode,
                "defeatKind": format!("{:?}", verdict.defeat_kind),
                "norm": quin_json(&verdict.norm)
            })
        })
        .collect();
    Ok(json!({ "verdictCount": count, "verdicts": verdicts }))
}

fn evaluate_epistemic(args: &Value) -> Result<Value, String> {
    use modalities::epistemic::{evaluate_epistemic_frame, EpistemicStatus, EpistemicVerdict};

    let quins = parse_quins(args)?;
    let agent = optional_u64(args, "agent")?.unwrap_or(0);
    let world = optional_u64(args, "world")?.unwrap_or(0);
    let empty = EpistemicVerdict {
        claim: NQuin::default(),
        status: EpistemicStatus::Skipped,
        certainty: 0,
    };
    let mut out = vec![empty; quins.len()];
    let count = evaluate_epistemic_frame(&quins, agent, world, &mut out)
        .map_err(|error| format!("epistemic evaluation failed: {error:?}"))?;
    let verdicts: Vec<Value> = out[..count]
        .iter()
        .map(|verdict| {
            json!({
                "status": format!("{:?}", verdict.status),
                "certainty": verdict.certainty,
                "claim": quin_json(&verdict.claim)
            })
        })
        .collect();
    Ok(json!({ "verdictCount": count, "verdicts": verdicts }))
}

fn route_paraconsistent(args: &Value) -> Result<Value, String> {
    let quins = parse_quins(args)?;
    let mut consistent = vec![NQuin::default(); quins.len()];
    let mut isolated = vec![NQuin::default(); quins.len()];
    let (consistent_count, isolated_count) =
        modalities::paraconsistent::route_paraconsistent(&quins, &mut consistent, &mut isolated)
            .map_err(|error| format!("paraconsistent routing failed: {error:?}"))?;

    Ok(json!({
        "consistent": consistent[..consistent_count].iter().map(quin_json).collect::<Vec<_>>(),
        "isolated": isolated[..isolated_count].iter().map(quin_json).collect::<Vec<_>>()
    }))
}

fn evaluate_ltl(args: &Value) -> Result<Value, String> {
    use modalities::temporal_ltl::{evaluate_ltl_trace, LtlFormula};

    let trace = parse_named_quins(args, "trace")?;
    let formula = args
        .get("formula")
        .ok_or_else(|| "formula is required".to_string())?;
    let kind = required_str(formula, "kind")?;
    let parsed = match kind {
        "globally" => LtlFormula::Globally(required_u64(formula, "property")?),
        "finally" => LtlFormula::Finally(required_u64(formula, "property")?),
        "next" => LtlFormula::Next(required_u64(formula, "property")?),
        "until" => LtlFormula::Until {
            ante: required_u64(formula, "ante")?,
            consequent: required_u64(formula, "consequent")?,
        },
        "release" => LtlFormula::Release {
            trigger: required_u64(formula, "trigger")?,
            invariant: required_u64(formula, "invariant")?,
        },
        other => return Err(format!("unsupported LTL formula kind: {other}")),
    };
    Ok(json!({ "holds": evaluate_ltl_trace(&trace, &parsed) }))
}

fn check_subsumption(args: &Value) -> Result<Value, String> {
    let tbox = parse_named_quins(args, "tbox")?;
    let sub = required_u64(args, "subClass")?;
    let sup = required_u64(args, "superClass")?;
    Ok(json!({
        "subsumed": modalities::dl::check_subsumption_quin(sub, sup, &tbox)
    }))
}

fn deontic_govern(args: &Value) -> Result<Value, String> {
    use modalities::interaction_governance::{map_policy, permits_execution, Governance};
    use modalities::logic::deontic::DeonticStatus;

    let status = match required_str(args, "status")?.to_ascii_lowercase().as_str() {
        "active" => DeonticStatus::Active,
        "defeated" => DeonticStatus::Defeated,
        "expired" => DeonticStatus::Expired,
        "pending" => DeonticStatus::Pending,
        "violated" => DeonticStatus::Violated,
        "discharged" => DeonticStatus::Discharged,
        "malformed" => DeonticStatus::Malformed,
        other => return Err(format!("invalid deontic status: {other}")),
    };
    let flag = |key: &str| args.get(key).and_then(Value::as_bool).unwrap_or(false);
    let mode = map_policy(
        status,
        Governance {
            non_derogable: flag("nonDerogable"),
            humanitarian: flag("humanitarian"),
            ambiguous: flag("ambiguous"),
        },
    );
    Ok(json!({
        "status": format!("{status:?}"),
        "policyMode": format!("{mode:?}"),
        "permitsExecution": permits_execution(mode)
    }))
}

fn parse_quins(args: &Value) -> Result<Vec<NQuin>, String> {
    parse_named_quins(args, "quins")
}

fn parse_named_quins(args: &Value, field: &str) -> Result<Vec<NQuin>, String> {
    let values = args
        .get(field)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{field} must be an array"))?;
    if values.len() > MAX_INPUT_QUINS {
        return Err(format!("{field} exceeds the {MAX_INPUT_QUINS}-Quin bound"));
    }
    values
        .iter()
        .map(|value| parse_quin(value, field))
        .collect()
}

pub(crate) fn parse_quin(value: &Value, field: &str) -> Result<NQuin, String> {
    if let Some(items) = value.as_array() {
        if items.len() != 6 {
            return Err(format!("{field} Quin arrays must contain six fields"));
        }
        return Ok(NQuin {
            subject: parse_u64(&items[0], "subject")?,
            predicate: parse_u64(&items[1], "predicate")?,
            object: parse_u64(&items[2], "object")?,
            context: parse_u64(&items[3], "context")?,
            metadata: parse_u64(&items[4], "metadata")?,
            parity: parse_u64(&items[5], "parity")?,
        });
    }

    let object = value
        .as_object()
        .ok_or_else(|| format!("{field} entries must be objects or six-item arrays"))?;
    let get = |name: &str| {
        object
            .get(name)
            .ok_or_else(|| format!("Quin.{name} is required"))
            .and_then(|value| parse_u64(value, name))
    };
    let subject = get("subject")?;
    let predicate = get("predicate")?;
    let object_value = get("object")?;
    let context = get("context")?;
    let metadata = object
        .get("metadata")
        .map(|value| parse_u64(value, "metadata"))
        .transpose()?
        .unwrap_or(0);
    let parity = object
        .get("parity")
        .map(|value| parse_u64(value, "parity"))
        .transpose()?
        .unwrap_or(subject ^ predicate ^ object_value ^ context);
    Ok(NQuin {
        subject,
        predicate,
        object: object_value,
        context,
        metadata,
        parity,
    })
}

pub(crate) fn quin_json(quin: &NQuin) -> Value {
    json!({
        "subject": quin.subject.to_string(),
        "predicate": quin.predicate.to_string(),
        "object": quin.object.to_string(),
        "context": quin.context.to_string(),
        "metadata": quin.metadata.to_string(),
        "parity": quin.parity.to_string()
    })
}

pub(crate) fn required_str<'a>(object: &'a Value, field: &str) -> Result<&'a str, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{field} must be a string"))
}

pub(crate) fn required_u64(object: &Value, field: &str) -> Result<u64, String> {
    object
        .get(field)
        .ok_or_else(|| format!("{field} is required"))
        .and_then(|value| parse_u64(value, field))
}

pub(crate) fn optional_u64(object: &Value, field: &str) -> Result<Option<u64>, String> {
    object
        .get(field)
        .map(|value| parse_u64(value, field))
        .transpose()
}

pub(crate) fn required_u32(object: &Value, field: &str) -> Result<u32, String> {
    let value = required_u64(object, field)?;
    u32::try_from(value).map_err(|_| format!("{field} exceeds u32"))
}

fn required_u8(object: &Value, field: &str) -> Result<u8, String> {
    let value = required_u64(object, field)?;
    u8::try_from(value).map_err(|_| format!("{field} exceeds u8"))
}

pub(crate) fn parse_u64(value: &Value, field: &str) -> Result<u64, String> {
    if let Some(number) = value.as_u64() {
        return Ok(number);
    }
    let text = value
        .as_str()
        .ok_or_else(|| format!("{field} must be a u64 number or string"))?;
    if let Some(hex) = text.strip_prefix("0x").or_else(|| text.strip_prefix("0X")) {
        u64::from_str_radix(hex, 16).map_err(|_| format!("{field} is not valid hexadecimal"))
    } else {
        text.parse::<u64>()
            .map_err(|_| format!("{field} is not a valid u64"))
    }
}

fn tool_result(id: Value, result: Value, is_error: bool) -> String {
    let text = result.to_string();
    rpc_ok(
        id,
        json!({
            "content": [{ "type": "text", "text": text }],
            "structuredContent": result,
            "isError": is_error
        }),
    )
}

fn rpc_ok(id: Value, result: Value) -> String {
    json!({ "jsonrpc": "2.0", "id": id, "result": result }).to_string()
}

fn rpc_error(id: Value, code: i64, message: &str) -> String {
    json!({
        "jsonrpc": "2.0",
        "id": id,
        "error": { "code": code, "message": message }
    })
    .to_string()
}

fn tool_catalog() -> Value {
    json!([
        tool("ontology_capabilities", "List the exact capabilities and exclusions of this ontology WASM profile.", json!({ "type": "object" })),
        tool("hash_iri", "Hash an IRI with Qualia's canonical masked FNV-1a q_hash.", json!({
            "type": "object", "required": ["iri"], "properties": { "iri": { "type": "string" } }
        })),
        tool("parse_n3", "Parse bounded N3 triples, rules, ASP blocks, and diffuse blocks without network access.", json!({
            "type": "object", "required": ["source"], "properties": { "source": { "type": "string" } }
        })),
        tool("query_quins", "Filter an in-memory Quin array by optional subject, predicate, object, and context.", quin_tool_schema()),
        tool("validate_shacl", "Validate an in-memory Quin property against the WASM-safe SHACL constraint subset.", json!({
            "type": "object",
            "required": ["quins", "targetSubject", "targetProperty", "constraints"],
            "properties": {
                "quins": quin_array_schema(),
                "targetSubject": u64_schema(),
                "targetProperty": u64_schema(),
                "constraints": { "type": "array", "maxItems": 32, "items": { "type": "object" } }
            }
        })),
        tool("evaluate_deontic", "Evaluate obligation, permission, prohibition, expiry, and defeater Quins.", json!({
            "type": "object", "required": ["quins", "nowUnix"],
            "properties": { "quins": quin_array_schema(), "nowUnix": u64_schema() }
        })),
        tool("evaluate_epistemic", "Evaluate knowledge, belief, and common-knowledge Quins with optional agent/world filters.", json!({
            "type": "object", "required": ["quins"],
            "properties": {
                "quins": quin_array_schema(), "agent": u64_schema(), "world": u64_schema()
            }
        })),
        tool("route_paraconsistent", "Partition contradictory Quins into consistent and deterministic isolated contexts.", json!({
            "type": "object", "required": ["quins"], "properties": { "quins": quin_array_schema() }
        })),
        tool("evaluate_ltl", "Evaluate one stack-allocated LTL formula against a Quin trace.", json!({
            "type": "object", "required": ["trace", "formula"],
            "properties": { "trace": quin_array_schema(), "formula": { "type": "object" } }
        })),
        tool("check_subsumption", "Check transitive rdfs:subClassOf subsumption over a Quin TBox.", json!({
            "type": "object", "required": ["tbox", "subClass", "superClass"],
            "properties": {
                "tbox": quin_array_schema(), "subClass": u64_schema(), "superClass": u64_schema()
            }
        })),
        tool("deontic_govern", "Map a deontic verdict and governance flags to a Webizen policy mode.", json!({
            "type": "object", "required": ["status"],
            "properties": {
                "status": { "type": "string", "enum": ["active","defeated","expired","pending","violated","discharged","malformed"] },
                "nonDerogable": { "type": "boolean" },
                "humanitarian": { "type": "boolean" },
                "ambiguous": { "type": "boolean" }
            }
        })),
        tool("namespace_discovery_help", "Return the offline URL contract and recommended agent flow for ns.webcivics.net (or another baseUrl).", json!({
            "type": "object",
            "properties": { "baseUrl": { "type": "string", "description": "Default https://ns.webcivics.net" } }
        })),
        tool("catalog_summarize", "Summarise a fetched DCAT catalog.json string: category/title/id filters and dataset URLs (no network).", json!({
            "type": "object",
            "required": ["catalogJson"],
            "properties": {
                "catalogJson": { "type": "string" },
                "categoryPrefix": { "type": "string", "description": "e.g. institutions/un" },
                "titleContains": { "type": "string", "description": "Case-insensitive title substring" },
                "idPrefix": { "type": "string", "description": "Dataset id prefix match" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 512 }
            }
        })),
        tool("corpus_summarize", "Summarise a fetched legislation corpus JSON (e.g. au-legislation-corpus.json) by title/id (no network).", json!({
            "type": "object",
            "required": ["corpusJson"],
            "properties": {
                "corpusJson": { "type": "string" },
                "titleContains": { "type": "string" },
                "idPrefix": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 512 }
            }
        })),
        tool("export_graph", "Serialise ground RDF triples as jsonld (default), rdfjson, turtle, n3, or yamlld for bot consumers.", json!({
            "type": "object",
            "required": ["triples"],
            "properties": {
                "triples": {
                    "type": "array",
                    "maxItems": MAX_QUERY_RESULTS,
                    "items": {
                        "type": "object",
                        "required": ["s", "p", "o"],
                        "properties": {
                            "s": {},
                            "p": {},
                            "o": {},
                            "subject": {},
                            "predicate": {},
                            "object": {}
                        }
                    }
                },
                "format": {
                    "type": "string",
                    "enum": ["jsonld", "rdfjson", "turtle", "n3", "yamlld"],
                    "description": "Default jsonld"
                },
                "logicMode": {
                    "type": "string",
                    "enum": ["none", "as-data", "evaluate", "native-n3"],
                    "description": "Affects queryMeta.dropped honesty flags"
                },
                "contextUrl": {
                    "type": "string",
                    "description": "JSON-LD @context URL (default ns.webcivics.net/context.jsonld)"
                }
            }
        })),
        tool("resolve_dataset_urls", "Expand a short path like /institutions/un/api-1977 into HTML/N3/TTL/JSON-LD URLs.", json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string" },
                "baseUrl": { "type": "string" }
            }
        })),
        tool("load_graph", "Load N3, Quin JSON, or Q42L bytes into a session graph (no network).", json!({
            "type": "object",
            "properties": {
                "format": { "type": "string", "enum": ["n3", "quins", "q42lite", "q42"] },
                "source": { "type": "string", "description": "N3 text when format=n3" },
                "quins": quin_array_schema(),
                "bytesBase64": { "type": "string", "description": "Q42L payload when format=q42lite" },
                "graphId": { "type": "string" },
                "label": { "type": "string" }
            }
        })),
        tool("load_q42", "Load Q42L (wasm-safe) volume bytes; native Q42 v3 is rejected with guidance.", json!({
            "type": "object",
            "required": ["bytesBase64"],
            "properties": {
                "bytesBase64": { "type": "string" },
                "graphId": { "type": "string" },
                "label": { "type": "string" },
                "format": { "type": "string" }
            }
        })),
        tool("list_graphs", "List session-scoped graphs.", json!({ "type": "object" })),
        tool("unload_graph", "Drop a session graph by graphId.", json!({
            "type": "object", "required": ["graphId"],
            "properties": { "graphId": { "type": "string" } }
        })),
        tool("query_graph", "Filter a session graph by optional S/P/O/C hashes and label/object substring (lexicon).", json!({
            "type": "object",
            "required": ["graphId"],
            "properties": {
                "graphId": { "type": "string" },
                "subject": u64_schema(),
                "predicate": u64_schema(),
                "object": u64_schema(),
                "context": u64_schema(),
                "labelContains": { "type": "string" },
                "objectContains": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_QUERY_RESULTS }
            }
        })),
        tool("query_sparql", "SELECT-only SPARQL subset over a session graph (simple ?s ?p ?o BGP + optional FILTER CONTAINS on ?o).", json!({
            "type": "object",
            "required": ["graphId", "query"],
            "properties": {
                "graphId": { "type": "string" },
                "query": { "type": "string" },
                "limit": { "type": "integer", "minimum": 1, "maximum": MAX_QUERY_RESULTS }
            }
        })),
        tool("export_q42lite", "Serialise a session graph to Q42L base64 for transfer/tests.", json!({
            "type": "object",
            "required": ["graphId"],
            "properties": { "graphId": { "type": "string" } }
        })),
        tool("compile_deontic_norms", "Compile structured obligation/permit/forbid norms into Quins (optional storeAsGraph).", json!({
            "type": "object",
            "required": ["norms"],
            "properties": {
                "norms": { "type": "array", "maxItems": 256, "items": { "type": "object" } },
                "storeAsGraph": { "type": "boolean" },
                "graphId": { "type": "string" },
                "label": { "type": "string" }
            }
        })),
        tool("evaluate_deontic_session", "evaluate_deontic over a session graphId or quins array.", json!({
            "type": "object",
            "required": ["nowUnix"],
            "properties": {
                "nowUnix": u64_schema(),
                "graphId": { "type": "string" },
                "quins": quin_array_schema()
            }
        }))
    ])
}

fn tool(name: &str, description: &str, input_schema: Value) -> Value {
    json!({ "name": name, "description": description, "inputSchema": input_schema })
}

fn u64_schema() -> Value {
    json!({
        "oneOf": [
            { "type": "string", "pattern": "^(0[xX][0-9a-fA-F]+|[0-9]+)$" },
            { "type": "integer", "minimum": 0 }
        ]
    })
}

fn quin_array_schema() -> Value {
    json!({
        "type": "array",
        "maxItems": MAX_INPUT_QUINS,
        "items": {
            "type": "object",
            "required": ["subject", "predicate", "object", "context"],
            "properties": {
                "subject": u64_schema(),
                "predicate": u64_schema(),
                "object": u64_schema(),
                "context": u64_schema(),
                "metadata": u64_schema(),
                "parity": u64_schema()
            }
        }
    })
}

fn quin_tool_schema() -> Value {
    json!({
        "type": "object",
        "required": ["quins"],
        "properties": {
            "quins": quin_array_schema(),
            "subject": u64_schema(),
            "predicate": u64_schema(),
            "object": u64_schema(),
            "context": u64_schema(),
            "limit": { "type": "integer", "minimum": 0, "maximum": MAX_QUERY_RESULTS }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(name: &str, arguments: Value) -> Value {
        let response: Value = serde_json::from_str(&mcp_jsonrpc(
            &json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "tools/call",
                "params": { "name": name, "arguments": arguments }
            })
            .to_string(),
        ))
        .unwrap();
        response["result"]["structuredContent"].clone()
    }

    #[test]
    fn lists_the_ontology_profile() {
        let result = call("ontology_capabilities", json!({}));
        assert_eq!(result["profile"], "ontology-mcp-kernel");
        assert!(result["capabilities"]
            .as_array()
            .unwrap()
            .iter()
            .any(|value| value == "deontic-logic"));
    }

    #[test]
    fn namespace_discovery_help_lists_bootstrap_urls() {
        let result = call("namespace_discovery_help", json!({}));
        assert!(result["bootstrap"]["catalogJson"]
            .as_str()
            .unwrap()
            .contains("catalog.json"));
        assert!(result["recommendedAgentFlow"].as_array().unwrap().len() >= 4);
    }

    #[test]
    fn catalog_summarize_filters_by_category_prefix() {
        let catalog = json!({
            "title": "Test",
            "baseUrl": "https://ns.webcivics.net",
            "datasetCount": 2,
            "datasets": [
                {
                    "id": "a",
                    "title": "A",
                    "category": "institutions/un",
                    "canonicalUrl": "https://ns.webcivics.net/institutions/un/a/",
                    "n3Url": "https://ns.webcivics.net/institutions/un/a.n3"
                },
                {
                    "id": "b",
                    "title": "B",
                    "category": "core",
                    "canonicalUrl": "https://ns.webcivics.net/core/b/",
                    "n3Url": "https://ns.webcivics.net/core/b.n3"
                }
            ]
        });
        let result = call(
            "catalog_summarize",
            json!({
                "catalogJson": catalog.to_string(),
                "categoryPrefix": "institutions/un"
            }),
        );
        assert_eq!(result["matchedCount"], 1);
        assert_eq!(result["datasets"][0]["id"], "a");
    }

    #[test]
    fn resolve_dataset_urls_builds_extension_paths() {
        let result = call(
            "resolve_dataset_urls",
            json!({ "path": "institutions/un/api-1977" }),
        );
        assert_eq!(
            result["htmlUrl"],
            "https://ns.webcivics.net/institutions/un/api-1977/"
        );
        assert_eq!(
            result["n3Url"],
            "https://ns.webcivics.net/institutions/un/api-1977.n3"
        );
    }

    #[test]
    fn exact_u64_strings_round_trip_through_query() {
        let result = call(
            "query_quins",
            json!({
                "quins": [{
                    "subject": "18446744073709551615",
                    "predicate": "2",
                    "object": "3",
                    "context": "4"
                }],
                "subject": "18446744073709551615"
            }),
        );
        assert_eq!(result["matchCount"], 1);
        assert_eq!(result["matches"][0]["subject"], "18446744073709551615");
    }

    #[test]
    fn parses_n3_static_triple() {
        let result = call("parse_n3", json!({ "source": "<urn:s> <urn:p> <urn:o> ." }));
        assert_eq!(result["eventCount"], 1);
        assert_eq!(result["events"][0]["type"], "triple");
    }

    #[test]
    fn notifications_do_not_return_json_rpc_responses() {
        let response =
            mcp_jsonrpc(r#"{"jsonrpc":"2.0","method":"notifications/initialized","params":{}}"#);
        assert!(response.is_empty());
    }

    #[test]
    fn catalog_summarize_filters_by_title_contains() {
        let catalog = json!({
            "datasets": [
                { "id": "priv", "title": "Privacy Act 1988", "category": "institutions/au-fed-legislation",
                  "canonicalUrl": "https://ns.webcivics.net/institutions/au-fed-legislation/priv/",
                  "n3Url": "https://ns.webcivics.net/institutions/au-fed-legislation/priv.n3" },
                { "id": "crimes", "title": "Crimes Act", "category": "institutions/au-fed-legislation",
                  "canonicalUrl": "https://ns.webcivics.net/institutions/au-fed-legislation/crimes/",
                  "n3Url": "https://ns.webcivics.net/institutions/au-fed-legislation/crimes.n3" }
            ]
        });
        let result = call(
            "catalog_summarize",
            json!({
                "catalogJson": catalog.to_string(),
                "titleContains": "privacy"
            }),
        );
        assert_eq!(result["matchedCount"], 1);
        assert_eq!(result["datasets"][0]["id"], "priv");
    }

    #[test]
    fn corpus_summarize_finds_consumer_data_right() {
        let corpus = json!({
            "jurisdiction": "AU",
            "datasetCount": 2,
            "datasets": [
                {
                    "id": "C2019A00063",
                    "title": "Treasury Laws Amendment (Consumer Data Right) Act 2019",
                    "n3Url": "https://ns.webcivics.net/institutions/au-fed-legislation/C2019A00063.n3",
                    "officialSource": "https://www.legislation.gov.au/C2019A00063/latest/downloads"
                },
                {
                    "id": "C1915A00006",
                    "title": "Crimes Act 1915",
                    "n3Url": "https://ns.webcivics.net/institutions/au-fed-legislation/C1915A00006.n3"
                }
            ]
        });
        let result = call(
            "corpus_summarize",
            json!({
                "corpusJson": corpus.to_string(),
                "titleContains": "Consumer Data Right"
            }),
        );
        assert_eq!(result["matchedCount"], 1);
        assert_eq!(result["datasets"][0]["id"], "C2019A00063");
    }

    #[test]
    fn export_graph_jsonld_and_rdfjson() {
        let triples = json!([{
            "s": "https://example.org/about",
            "p": "http://purl.org/dc/terms/title",
            "o": { "type": "literal", "value": "Anna's Homepage", "lang": "en" }
        }]);
        let jsonld = call(
            "export_graph",
            json!({ "triples": triples, "format": "jsonld" }),
        );
        assert_eq!(jsonld["mediaType"], "application/ld+json");
        assert_eq!(jsonld["tripleCount"], 1);
        assert!(jsonld["body"]["@graph"].as_array().unwrap().len() == 1);

        let rdfjson = call(
            "export_graph",
            json!({ "triples": triples, "format": "rdfjson" }),
        );
        assert_eq!(rdfjson["mediaType"], "application/rdf+json");
        let about = &rdfjson["body"]["https://example.org/about"];
        assert!(
            about["http://purl.org/dc/terms/title"].as_array().unwrap()[0]["value"]
                .as_str()
                .unwrap()
                .contains("Anna")
        );
    }

    #[test]
    fn export_graph_turtle_n3_yamlld() {
        let triples = json!([{
            "s": "urn:s",
            "p": "urn:p",
            "o": "urn:o"
        }]);
        let ttl = call(
            "export_graph",
            json!({ "triples": triples, "format": "turtle" }),
        );
        assert!(ttl["body"].as_str().unwrap().contains("<urn:s>"));
        let n3 = call(
            "export_graph",
            json!({ "triples": triples, "format": "n3" }),
        );
        assert_eq!(n3["mediaType"], "text/n3");
        let yaml = call(
            "export_graph",
            json!({ "triples": triples, "format": "yamlld" }),
        );
        assert_eq!(yaml["mediaType"], "application/ld+yaml");
        assert!(yaml["body"].as_str().unwrap().contains("@graph"));
    }

    #[test]
    fn load_n3_query_graph_and_sparql_subset() {
        let loaded = call(
            "load_graph",
            json!({
                "format": "n3",
                "graphId": "demo-access",
                "label": "demo",
                // Object IRIs encode searchable labels into the lexicon (hash of full IRI).
                "source": "<urn:entity> <urn:p/prefLabel> <urn:label/APP12-access-to-personal-information> . <urn:entity> <urn:p/note> <urn:label/must-provide-access-to-personal-information> . <urn:other> <urn:p/prefLabel> <urn:label/Crimes-Act> ."
            }),
        );
        assert_eq!(loaded["graphId"], "demo-access");
        assert!(
            loaded["quinCount"].as_u64().unwrap() >= 2,
            "load_graph result: {loaded}"
        );

        let hits = call(
            "query_graph",
            json!({
                "graphId": "demo-access",
                "objectContains": "access-to-personal",
                "limit": 10
            }),
        );
        assert!(
            hits["matchCount"].as_u64().unwrap() >= 1,
            "query_graph: {hits}"
        );

        let sparql = call(
            "query_sparql",
            json!({
                "graphId": "demo-access",
                "query": "SELECT * WHERE { ?s ?p ?o } FILTER(CONTAINS(LCASE(STR(?o)), \"access-to-personal\")) LIMIT 20"
            }),
        );
        assert!(
            sparql["matchCount"].as_u64().unwrap() >= 1,
            "query_sparql: {sparql}"
        );

        let listed = call("list_graphs", json!({}));
        assert!(listed["graphCount"].as_u64().unwrap() >= 1);

        let unloaded = call("unload_graph", json!({ "graphId": "demo-access" }));
        assert_eq!(unloaded["removed"], true);
    }

    #[test]
    fn q42lite_round_trip_and_deontic_bridge() {
        let loaded = call(
            "load_graph",
            json!({
                "format": "n3",
                "graphId": "q42-src",
                "source": "<urn:a> <urn:p> <urn:b> ."
            }),
        );
        assert_eq!(loaded["quinCount"], 1);
        let exported = call("export_q42lite", json!({ "graphId": "q42-src" }));
        let b64 = exported["bytesBase64"].as_str().unwrap();
        assert!(!b64.is_empty());

        let reloaded = call(
            "load_q42",
            json!({
                "bytesBase64": b64,
                "graphId": "q42-dst",
                "label": "roundtrip"
            }),
        );
        assert_eq!(reloaded["quinCount"], 1);

        // Active obligation (no expiry)
        let compiled = call(
            "compile_deontic_norms",
            json!({
                "storeAsGraph": true,
                "graphId": "norms-1",
                "norms": [{
                    "partyIri": "did:example:holder",
                    "propertyIri": "https://ns.webcivics.net/values/requires",
                    "actionIri": "urn:action:give-access",
                    "contractIri": "urn:contract:app12",
                    "opcode": "obligate",
                    "expiryUnix": 0
                }]
            }),
        );
        assert_eq!(compiled["normCount"], 1);
        let verdicts = call(
            "evaluate_deontic_session",
            json!({ "graphId": "norms-1", "nowUnix": 1_700_000_000u64 }),
        );
        assert_eq!(verdicts["verdictCount"], 1);
        let status = verdicts["verdicts"][0]["status"].as_str().unwrap();
        assert!(
            status == "active" || status == "Active",
            "expected active, got {status}"
        );

        // Expired
        let expired = call(
            "compile_deontic_norms",
            json!({
                "norms": [{
                    "partyIri": "did:example:holder",
                    "propertyIri": "urn:prop",
                    "actionIri": "urn:act",
                    "opcode": "obligate",
                    "expiryUnix": 100
                }]
            }),
        );
        let v2 = call(
            "evaluate_deontic_session",
            json!({
                "quins": expired["quins"],
                "nowUnix": 1_000_000u64
            }),
        );
        let st2 = v2["verdicts"][0]["status"]
            .as_str()
            .unwrap()
            .to_ascii_lowercase();
        assert_eq!(st2, "expired");

        // Defeater
        let with_def = call(
            "compile_deontic_norms",
            json!({
                "storeAsGraph": true,
                "graphId": "norms-def",
                "norms": [
                    {
                        "partyIri": "did:example:holder",
                        "propertyIri": "urn:disclose",
                        "actionIri": "urn:data",
                        "contractIri": "urn:c",
                        "opcode": "obligate",
                        "expiryUnix": 0
                    },
                    {
                        "partyIri": "did:example:holder",
                        "propertyIri": "urn:disclose",
                        "actionIri": "urn:data",
                        "contractIri": "urn:c",
                        "opcode": "permit",
                        "isDefeater": true,
                        "expiryUnix": 0
                    }
                ]
            }),
        );
        assert_eq!(with_def["normCount"], 2);
        let v3 = call(
            "evaluate_deontic_session",
            json!({ "graphId": "norms-def", "nowUnix": 1 }),
        );
        let statuses: Vec<String> = v3["verdicts"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["status"].as_str().unwrap().to_ascii_lowercase())
            .collect();
        assert!(
            statuses.iter().any(|s| s == "defeated"),
            "expected a defeated verdict, got {statuses:?}"
        );

        let _ = call("unload_graph", json!({ "graphId": "q42-src" }));
        let _ = call("unload_graph", json!({ "graphId": "q42-dst" }));
        let _ = call("unload_graph", json!({ "graphId": "norms-1" }));
        let _ = call("unload_graph", json!({ "graphId": "norms-def" }));
    }
}
