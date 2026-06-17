//! Full implementations for MCP tools that previously returned `tool_not_ready()`.

use super::mcp_tool_impls::{parse_quin_slice, parse_tool_args};
use super::McpSystemError;
use crate::NQuin;
use serde_json::{json, Value};

fn json_str<'a>(v: &'a Value, key: &str, default: &'a str) -> &'a str {
    v.get(key)
        .and_then(Value::as_str)
        .unwrap_or(default)
}

fn json_u64(v: &Value, key: &str, default: u64) -> u64 {
    v.get(key)
        .and_then(|x| x.as_u64().or_else(|| x.as_i64().map(|n| n as u64)))
        .unwrap_or(default)
}

fn qualia_storage_path() -> std::path::PathBuf {
    std::env::var("QUALIA_STORAGE_PATH").unwrap_or_else(|_| {
        std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .map(|h| format!("{h}/.qualia"))
            .unwrap_or_else(|_| ".qualia".to_string())
    })
    .into()
}

fn qapps_root() -> std::path::PathBuf {
    qualia_storage_path().join("Qapps")
}

fn list_qapp_dir_names() -> Vec<String> {
    let root = qapps_root();
    let mut names = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&root) {
        for entry in entries.filter_map(Result::ok) {
            if entry.path().is_dir() {
                let manifest = entry.path().join("qapp.json");
                if manifest.is_file() {
                    names.push(entry.file_name().to_string_lossy().into_owned());
                }
            }
        }
    }
    names.sort();
    names
}

fn resolve_qapp_dir(name: &str) -> Result<std::path::PathBuf, McpSystemError> {
    let dir = qapps_root().join(name);
    if dir.is_dir() && dir.join("qapp.json").is_file() {
        return Ok(dir);
    }
    for candidate in bundled_qapp_source_candidates(name) {
        if candidate.join("qapp.json").is_file() {
            return Ok(candidate);
        }
    }
    Err(McpSystemError::InvalidParameters)
}

fn bundled_qapp_source_candidates(qapp_name: &str) -> Vec<std::path::PathBuf> {
    let mut out = Vec::new();
    if let Ok(extra) = std::env::var("QUALIA_BUNDLED_QAPPS_DIR") {
        out.push(std::path::PathBuf::from(extra).join(qapp_name));
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(root) = exe.parent() {
            for rel in [
                format!("bundled/qapps/{qapp_name}"),
                format!("qapps/{qapp_name}"),
            ] {
                out.push(root.join(rel));
            }
        }
    }
    let repo = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    for rel in [
        format!("bundled/qapps/{qapp_name}"),
        format!("app-development/{qapp_name}"),
    ] {
        out.push(repo.join(rel));
    }
    out
}

fn read_qapp_manifest_value(qapp_name: &str) -> Result<Value, McpSystemError> {
    let dir = resolve_qapp_dir(qapp_name)?;
    let content = std::fs::read_to_string(dir.join("qapp.json"))
        .map_err(|_| McpSystemError::ParseError)?;
    serde_json::from_str(&content).map_err(|_| McpSystemError::ParseError)
}

fn quins_to_ntriples(quins: &[NQuin]) -> String {
    let mut buf = Vec::new();
    let _ = crate::resolver::format_ntriples_to(quins, &mut buf);
    String::from_utf8(buf).unwrap_or_default()
}

fn quin_to_json(q: &NQuin) -> Value {
    json!({
        "subject": q.subject,
        "predicate": q.predicate,
        "object": q.object,
        "context": q.context,
        "metadata": q.metadata,
        "parity": q.parity,
    })
}

// ── Graph ────────────────────────────────────────────────────────────────────

pub fn query_sparql(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let query = json_str(&v, "query", "");
    if query.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let limit = v
        .get("limit")
        .and_then(|x| x.as_u64())
        .unwrap_or(1_000) as usize;

    let graph = crate::daemon_graph::graph_read_guard();
    let (stats, results) =
        crate::daemon_query::execute_query_on_graph(query, graph.as_slice())
            .map_err(|e| match e {
            crate::daemon_query::QueryExecError::EmptyQuery
            | crate::daemon_query::QueryExecError::ParseError(_)
            | crate::daemon_query::QueryExecError::InvalidProgram => McpSystemError::InvalidParameters,
            crate::daemon_query::QueryExecError::OutputBufferFull => McpSystemError::ParseError,
            crate::daemon_query::QueryExecError::ClassifiedEgress => McpSystemError::SanctuaryGateTriggered,
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

pub fn get_graph_stats(_args: &[u8]) -> Result<String, McpSystemError> {
    let count = crate::daemon_graph::graph_quin_count();
    let capacity = crate::daemon_graph::MAX_GRAPH_QUINS;
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

#[cfg(not(target_arch = "wasm32"))]
pub fn list_ontologies(_args: &[u8]) -> Result<String, McpSystemError> {
    let dir = crate::ontology_loader::ontology_dir_path();
    let mut entries = Vec::new();
    for (filename, context_hash) in crate::ontology_loader::startup_ontology_catalog() {
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
        "ontologyDir": dir.as_ref().map(|p| p.display().to_string()),
        "entries": entries,
        "graphQuinCount": crate::daemon_graph::graph_quin_count(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

#[cfg(target_arch = "wasm32")]
pub fn list_ontologies(_args: &[u8]) -> Result<String, McpSystemError> {
    Err(McpSystemError::ToolNotReady)
}

// ── LLM ──────────────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub fn llm_infer(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let prompt = json_str(&v, "prompt", "");
    if prompt.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let graph_context = json_str(&v, "graph_context", "");
    let model_path = v
        .get("model_path")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| crate::resident_model::resident_gguf_path())
        .or_else(|| std::env::var("QUALIA_ACTIVE_GGUF").ok())
        .unwrap_or_default();

    if model_path.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }

    let agent = crate::llm_agent::LocalLlmAgent::new("did:qualia:mcp-agent", &model_path);
    let (text, provenance, tokens, semantic_quin) =
        agent.infer_local_model_streaming(prompt, graph_context, None::<fn(String)>);

    let payload = json!({
        "text": text,
        "tokensGenerated": tokens,
        "provenanceHashes": provenance,
        "semanticQuin": semantic_quin.map(|q| quin_to_json(&q)),
        "modelPath": model_path,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

#[cfg(target_arch = "wasm32")]
pub fn llm_infer(_args: &[u8]) -> Result<String, McpSystemError> {
    Err(McpSystemError::ToolNotReady)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn llm_chat(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let messages = v
        .get("messages")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    if messages.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let mut prompt = String::new();
    for msg in messages {
        let role = msg.get("role").and_then(Value::as_str).unwrap_or("user");
        let content = msg.get("content").and_then(Value::as_str).unwrap_or("");
        prompt.push_str(role);
        prompt.push_str(": ");
        prompt.push_str(content);
        prompt.push('\n');
    }
    let chat_args = serde_json::to_vec(&json!({
        "prompt": prompt,
        "model_path": v.get("model_path").and_then(Value::as_str).unwrap_or(""),
        "graph_context": v.get("graph_context").and_then(Value::as_str).unwrap_or(""),
    }))
    .map_err(|_| McpSystemError::ParseError)?;
    llm_infer(&chat_args)
}

#[cfg(target_arch = "wasm32")]
pub fn llm_chat(_args: &[u8]) -> Result<String, McpSystemError> {
    Err(McpSystemError::ToolNotReady)
}

#[cfg(not(target_arch = "wasm32"))]
pub fn list_models(_args: &[u8]) -> Result<String, McpSystemError> {
    let models_dir = qualia_storage_path().join("Models");
    let mut discovered = Vec::new();
    if models_dir.is_dir() {
        if let Ok(entries) = std::fs::read_dir(&models_dir) {
            for entry in entries.filter_map(Result::ok) {
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("gguf") {
                    discovered.push(json!({
                        "path": path.display().to_string(),
                        "name": path.file_name().and_then(|n| n.to_str()).unwrap_or(""),
                        "source": "storage",
                    }));
                }
            }
        }
    }

    let resident = match (
        crate::resident_model::resident_model_id(),
        crate::resident_model::resident_gguf_path(),
    ) {
        (Some(id), path) => Some(json!({
            "modelId": id,
            "path": path,
            "source": "resident",
        })),
        _ => None,
    };

    let payload = json!({
        "modelsDir": models_dir.display().to_string(),
        "discovered": discovered,
        "resident": resident,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

#[cfg(target_arch = "wasm32")]
pub fn list_models(_args: &[u8]) -> Result<String, McpSystemError> {
    Ok(r#"{"discovered":[],"resident":null}"#.to_string())
}

// ── QPU ──────────────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub fn qpu_optimize(args: &[u8]) -> Result<String, McpSystemError> {
    use crate::qubo_compiler::{solve_classical, QuboMatrix};
    use crate::solvers::qpu::pre_solver::ProblemDescription;
    use crate::solvers::qpu::pre_solver::PreSolver;

    let v = parse_tool_args(args)?;
    let problem_val = v
        .get("problem")
        .ok_or(McpSystemError::InvalidParameters)?;
    let problem: ProblemDescription =
        serde_json::from_value(problem_val.clone()).map_err(|_| McpSystemError::InvalidParameters)?;

    let mut solver = PreSolver::new();
    let job = solver
        .formulate(&problem)
        .map_err(|_| McpSystemError::ParseError)?;

    let mut matrix = QuboMatrix::new(problem.variables.len().max(1));
    if let Some(ham) = job.hamiltonian.as_deref() {
        if let Ok(form) =
            serde_json::from_str::<crate::solvers::qpu::pre_solver::QuboFormulation>(ham)
        {
            for (idx, coeff) in form.linear_terms {
                matrix.set_linear(idx as usize, coeff);
            }
            for (a, b, coeff) in form.quadratic_terms {
                matrix.set_quadratic(a as usize, b as usize, coeff);
            }
        }
    }

    let mut assignment = vec![0u8; matrix.num_vars];
    let energy = solve_classical(&matrix, &mut assignment);

    let payload = json!({
        "jobParameters": job,
        "classicalEnergy": energy,
        "assignment": assignment,
        "numVariables": matrix.num_vars,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

#[cfg(target_arch = "wasm32")]
pub fn qpu_optimize(_args: &[u8]) -> Result<String, McpSystemError> {
    Err(McpSystemError::ToolNotReady)
}

pub fn qpu_dft(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let resolution = v
        .get("grid_resolution")
        .and_then(|x| x.as_u64())
        .unwrap_or(8) as usize;
    let quins = if v.get("quins").is_some() {
        parse_quin_slice(&v, "quins")?
    } else {
        Vec::new()
    };

    let mut dft = crate::quantum_dft::ElectronDensity::new(resolution.max(2));
    let energy = dft.calculate_ground_state_energy(&quins);
    let payload = json!({
        "groundStateEnergy": energy,
        "gridResolution": dft.grid_resolution,
        "electronQuinCount": quins.iter().filter(|q| q.predicate == crate::q_hash("HAS_ELECTRON")).count(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

fn qpu_job_status_label(
    status: crate::specialized_libs::qpu_bridge::QPUJobStatus,
) -> &'static str {
    use crate::specialized_libs::qpu_bridge::QPUJobStatus;
    match status {
        QPUJobStatus::Queued => "queued",
        QPUJobStatus::Running => "running",
        QPUJobStatus::Completed => "completed",
        QPUJobStatus::Failed => "failed",
        QPUJobStatus::Cancelled => "cancelled",
        QPUJobStatus::Timeout => "timeout",
    }
}

pub fn qpu_status(_args: &[u8]) -> Result<String, McpSystemError> {
    use crate::specialized_libs::qpu_bridge::QPUBridgeManager;
    let bridge = QPUBridgeManager::new();
    let payload = json!({
        "connected": bridge.is_connected(),
        "jobStatus": qpu_job_status_label(bridge.get_job_status()),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

// ── Identity & wallet ────────────────────────────────────────────────────────

pub fn get_wallet_status(_args: &[u8]) -> Result<String, McpSystemError> {
    let pending_path = qualia_storage_path().join("pending_payments.ndjson");
    let mut queued = Vec::new();
    let mut line_count = 0usize;
    if pending_path.is_file() {
        if let Ok(content) = std::fs::read_to_string(&pending_path) {
            for line in content.lines() {
                let trimmed = line.trim();
                if trimmed.is_empty() {
                    continue;
                }
                line_count += 1;
                if let Ok(val) = serde_json::from_str::<Value>(trimmed) {
                    queued.push(val);
                }
            }
        }
    }
    let payload = json!({
        "pendingPaymentsPath": pending_path.display().to_string(),
        "queuedCount": line_count,
        "queued": queued,
        "storagePath": qualia_storage_path().display().to_string(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub fn get_did_info(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let did = json_str(&v, "did", "");
    if did.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let pointer = crate::identifier::parse_did_q42(did.as_bytes())
        .map_err(|_| McpSystemError::InvalidParameters)?;
    let payload = json!({
        "did": did,
        "pointer": pointer,
        "msbSet": (pointer >> 63) == 1,
        "payload60": pointer & 0x0FFF_FFFF_FFFF_FFFF,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

// ── Ontology ─────────────────────────────────────────────────────────────────

#[cfg(not(target_arch = "wasm32"))]
pub fn ingest_ontology(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let path_str = json_str(&v, "path", "");
    if path_str.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let path = std::path::Path::new(path_str);
    let context = json_u64(&v, "context_hash", 0);
    let graph_context = if context == 0 {
        crate::q_hash(path_str)
    } else {
        context
    };

    let quins = if path.extension().and_then(|s| s.to_str()) == Some("q42") {
        crate::ontology_loader::load_q42_file(path)
    } else {
        crate::ontology_loader::parse_ttl_to_quins(path, graph_context)
    };
    let added = quins.len();
    crate::daemon_graph::extend_with_ontology_quins_slice(&quins);

    let payload = json!({
        "path": path_str,
        "contextHash": graph_context,
        "quinsIngested": added,
        "graphQuinCount": crate::daemon_graph::graph_quin_count(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

#[cfg(target_arch = "wasm32")]
pub fn ingest_ontology(_args: &[u8]) -> Result<String, McpSystemError> {
    Err(McpSystemError::ToolNotReady)
}

fn parse_shacl_constraints(v: &Value) -> Result<Vec<crate::shacl_compiler::ShaclConstraint>, McpSystemError> {
    let arr = v
        .get("constraints")
        .and_then(Value::as_array)
        .ok_or(McpSystemError::InvalidParameters)?;
    let mut out = Vec::new();
    for item in arr {
        let kind = item
            .get("kind")
            .or_else(|| item.get("type"))
            .and_then(Value::as_str)
            .unwrap_or("");
        match kind {
            "datatype" | "Datatype" => {
                let dt = item
                    .get("datatype")
                    .and_then(Value::as_str)
                    .unwrap_or("xsd:string");
                let iri = format!("xsd:{dt}");
                if let Some(dt) = crate::shacl_compiler::ShaclDatatype::from_iri_hash(crate::q_hash(&iri)) {
                    out.push(crate::shacl_compiler::ShaclConstraint::Datatype(dt));
                }
            }
            "minCount" | "MinCount" => {
                let n = item.get("value").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
                out.push(crate::shacl_compiler::ShaclConstraint::MinCount(n));
            }
            "maxCount" | "MaxCount" => {
                let n = item.get("value").and_then(|x| x.as_u64()).unwrap_or(1) as u32;
                out.push(crate::shacl_compiler::ShaclConstraint::MaxCount(n));
            }
            "deonticObligate" => out.push(crate::shacl_compiler::ShaclConstraint::DeonticObligate),
            "deonticPermit" => out.push(crate::shacl_compiler::ShaclConstraint::DeonticPermit),
            "deonticForbid" => out.push(crate::shacl_compiler::ShaclConstraint::DeonticForbid),
            "epistemicKnowledge" => {
                let min = item.get("min_certainty").and_then(|x| x.as_u64()).unwrap_or(128) as u8;
                out.push(crate::shacl_compiler::ShaclConstraint::EpistemicKnowledge {
                    min_certainty: min,
                });
            }
            "commonKnowledge" => out.push(crate::shacl_compiler::ShaclConstraint::CommonKnowledge),
            _ => {}
        }
    }
    Ok(out)
}

pub fn validate_shacl(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let quins = parse_quin_slice(&v, "quins")?;
    let target_subject = json_u64(&v, "target_subject", 0);
    let target_property = json_u64(&v, "target_property", 0);
    if target_subject == 0 || target_property == 0 {
        return Err(McpSystemError::InvalidParameters);
    }
    let constraints = parse_shacl_constraints(&v)?;
    let valid = crate::shacl_compiler::validate_shacl_property(
        &quins,
        target_subject,
        target_property,
        &constraints,
    );
    let payload = json!({
        "valid": valid,
        "targetSubject": target_subject,
        "targetProperty": target_property,
        "constraintCount": constraints.len(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

// ── Qapps ────────────────────────────────────────────────────────────────────

pub fn list_qapps(_args: &[u8]) -> Result<String, McpSystemError> {
    let mut entries = Vec::new();
    for name in list_qapp_dir_names() {
        if let Ok(manifest) = read_qapp_manifest_value(&name) {
            let ext = manifest.get("x_qualia").cloned().unwrap_or(json!({}));
            entries.push(json!({
                "name": manifest.get("name").and_then(Value::as_str).unwrap_or(&name),
                "version": manifest.get("version").and_then(Value::as_str).unwrap_or(""),
                "displayName": ext.get("display_name").and_then(Value::as_str).unwrap_or(&name),
                "category": ext.get("category").and_then(Value::as_str).unwrap_or(""),
            }));
        }
    }
    let payload = json!({
        "qappsDir": qapps_root().display().to_string(),
        "count": entries.len(),
        "qapps": entries,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub fn get_qapp_manifest(qapp_name: &str) -> Result<String, McpSystemError> {
    let manifest = read_qapp_manifest_value(qapp_name)?;
    serde_json::to_string(&manifest).map_err(|_| McpSystemError::ParseError)
}

pub fn inspect_qapp_readiness(qapp_name: &str) -> Result<String, McpSystemError> {
    let dir = resolve_qapp_dir(qapp_name)?;
    let manifest = read_qapp_manifest_value(qapp_name)?;
    let ext = manifest.get("x_qualia").cloned().unwrap_or(json!({}));
    let mut checks = Vec::new();

    let manifest_path = dir.join("qapp.json");
    checks.push(json!({
        "kind": "manifest",
        "id": "qapp.json",
        "status": if manifest_path.is_file() { "ready" } else { "missing" },
    }));

    if let Some(entrypoints) = ext.get("entrypoints").and_then(Value::as_object) {
        for (key, value) in entrypoints {
            let rel = value.as_str().unwrap_or("index.html").split('#').next().unwrap_or("index.html");
            let exists = dir.join(rel).is_file();
            checks.push(json!({
                "kind": "entrypoint",
                "id": key,
                "path": rel,
                "status": if exists { "ready" } else { "missing" },
            }));
        }
    }

    if let Some(ontologies) = ext.get("required_ontologies").and_then(Value::as_array) {
        let graph_count = crate::daemon_graph::graph_quin_count();
        for ont in ontologies {
            let id = ont.as_str().unwrap_or("");
            checks.push(json!({
                "kind": "ontology",
                "id": id,
                "status": if graph_count > 0 { "maybe_ready" } else { "missing" },
                "detail": "Daemon graph ontology presence is approximate in core-db MCP path.",
            }));
        }
    }

    let ready = checks.iter().all(|c| {
        c.get("status")
            .and_then(Value::as_str)
            .is_some_and(|s| s == "ready" || s == "maybe_ready")
    });

    let payload = json!({
        "qappName": qapp_name,
        "ready": ready,
        "checks": checks,
        "graphQuinCount": crate::daemon_graph::graph_quin_count(),
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

pub fn list_qapp_updates(_args: &[u8]) -> Result<String, McpSystemError> {
    let mut offers = Vec::new();
    for name in list_qapp_dir_names() {
        let installed_dir = qapps_root().join(&name);
        let installed_version = read_qapp_manifest_value(&name)
            .ok()
            .and_then(|m| m.get("version").and_then(Value::as_str).map(str::to_string));
        let bundled = bundled_qapp_source_candidates(&name)
            .into_iter()
            .find(|p| p.join("qapp.json").is_file());
        let offered_version = bundled.as_ref().and_then(|p| {
            std::fs::read_to_string(p.join("qapp.json"))
                .ok()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .and_then(|m| m.get("version").and_then(Value::as_str).map(str::to_string))
        });
        let update_available = match (&installed_version, &offered_version) {
            (Some(i), Some(o)) => o != i,
            (None, Some(_)) => true,
            _ => false,
        };
        offers.push(json!({
            "qappName": name,
            "installedVersion": installed_version,
            "offeredVersion": offered_version,
            "updateAvailable": update_available,
            "offerSource": if bundled.is_some() { "bundled" } else { "none" },
            "installedPath": installed_dir.display().to_string(),
        }));
    }
    serde_json::to_string(&json!({ "offers": offers })).map_err(|_| McpSystemError::ParseError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_graph_stats_returns_json() {
        let out = get_graph_stats(b"{}").expect("stats");
        let v: Value = serde_json::from_str(&out).expect("json");
        assert!(v.get("quinCount").is_some());
        assert!(v.get("capacity").is_some());
    }

    #[test]
    fn get_did_info_parses_q42() {
        let args = br#"{"did":"did:q42:z6MkpTHR8VNs"}"#;
        let out = get_did_info(args).expect("did");
        let v: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["did"], "did:q42:z6MkpTHR8VNs");
        assert_eq!(v["msbSet"], true);
    }

    #[test]
    fn validate_shacl_min_count() {
        let subj = 1u64;
        let prop = 2u64;
        let args = serde_json::to_string(&json!({
            "quins": [{"subject": subj, "predicate": prop, "object": 3}],
            "target_subject": subj,
            "target_property": prop,
            "constraints": [{"kind": "minCount", "value": 1}],
        }))
        .unwrap();
        let out = validate_shacl(args.as_bytes()).expect("shacl");
        let v: Value = serde_json::from_str(&out).expect("json");
        assert_eq!(v["valid"], true);
    }

    #[test]
    fn list_qapps_returns_array() {
        let out = list_qapps(b"{}").expect("qapps");
        let v: Value = serde_json::from_str(&out).expect("json");
        assert!(v.get("qapps").and_then(Value::as_array).is_some());
    }
}