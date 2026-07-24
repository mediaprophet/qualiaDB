//! Browser-local MCP endpoint for ontology sites such as `ns.webcivics.net`.
//!
//! The crate deliberately depends on Qualia's `wasm-ontology` profile: no
//! renderer, WebGPU, scientific solver bundle, LLM runtime, daemon, or network
//! stack is admitted to this binary.

use qualia_core_db::modalities;
use qualia_core_db::NQuin;
use serde_json::{json, Value};
use wasm_bindgen::prelude::*;

const LATEST_PROTOCOL_VERSION: &str = "2025-11-25";
const SUPPORTED_PROTOCOL_VERSIONS: &[&str] = &["2025-11-25", "2025-06-18", "2025-03-26"];
const MAX_INPUT_QUINS: usize = 4_096;
const MAX_QUERY_RESULTS: usize = 512;
const MAX_N3_EVENTS: usize = 512;

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
        "resolve_dataset_urls" => resolve_dataset_urls(args),
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
            "agentLegislationGuide": format!("{base}/agent-legislation-guide.md")
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
                "Older /ontologies/... URLs redirect to short paths."
            ]
        },
        "recommendedAgentFlow": [
            "1. GET llms.txt and ai-use-policy.json (policy first).",
            "2. GET catalog.json; optionally filter by category (institutions/un, core, …).",
            "3. GET the .n3 (or .ttl) for each selected dataset.",
            "4. Call MCP tools/call parse_n3 with the document body (offline).",
            "5. Use hash_iri on document IRIs for Quin grounding; evaluate_deontic when norms apply.",
            "6. Never invent articles; cite canonicalUrl from the catalog."
        ],
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
        "matchedCount": matched,
        "returnedCount": selected.len(),
        "truncated": matched > selected.len(),
        "datasets": selected
    }))
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

fn parse_quin(value: &Value, field: &str) -> Result<NQuin, String> {
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

fn quin_json(quin: &NQuin) -> Value {
    json!({
        "subject": quin.subject.to_string(),
        "predicate": quin.predicate.to_string(),
        "object": quin.object.to_string(),
        "context": quin.context.to_string(),
        "metadata": quin.metadata.to_string(),
        "parity": quin.parity.to_string()
    })
}

fn required_str<'a>(object: &'a Value, field: &str) -> Result<&'a str, String> {
    object
        .get(field)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{field} must be a string"))
}

fn required_u64(object: &Value, field: &str) -> Result<u64, String> {
    object
        .get(field)
        .ok_or_else(|| format!("{field} is required"))
        .and_then(|value| parse_u64(value, field))
}

fn optional_u64(object: &Value, field: &str) -> Result<Option<u64>, String> {
    object
        .get(field)
        .map(|value| parse_u64(value, field))
        .transpose()
}

fn required_u32(object: &Value, field: &str) -> Result<u32, String> {
    let value = required_u64(object, field)?;
    u32::try_from(value).map_err(|_| format!("{field} exceeds u32"))
}

fn required_u8(object: &Value, field: &str) -> Result<u8, String> {
    let value = required_u64(object, field)?;
    u8::try_from(value).map_err(|_| format!("{field} exceeds u8"))
}

fn parse_u64(value: &Value, field: &str) -> Result<u64, String> {
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
        tool("catalog_summarize", "Summarise a fetched DCAT catalog.json string: category counts and dataset URL list (no network).", json!({
            "type": "object",
            "required": ["catalogJson"],
            "properties": {
                "catalogJson": { "type": "string" },
                "categoryPrefix": { "type": "string", "description": "e.g. institutions/un" },
                "limit": { "type": "integer", "minimum": 1, "maximum": 512 }
            }
        })),
        tool("resolve_dataset_urls", "Expand a short path like /institutions/un/api-1977 into HTML/N3/TTL/JSON-LD URLs.", json!({
            "type": "object",
            "required": ["path"],
            "properties": {
                "path": { "type": "string" },
                "baseUrl": { "type": "string" }
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
}
