//! MCP tools for Local LLM GGUF Autoregressive Inference and Chat.
//!
//! Migrated from legacy `mcp_stub_impls.rs` to dedicated library module.
//! Designed according to `AGENTS.md` §0-B (<500 lines).

use crate::types::{hex_decode, parse_tool_args, qualia_storage_path, McpSystemError};
use serde_json::{json, Value};

/// Helper to serialize a single `NQuin` to JSON.
fn quin_to_json(q: &qualia_core_db::NQuin) -> Value {
    json!({
        "subject": q.subject,
        "predicate": q.predicate,
        "object": q.object,
        "context": q.context,
        "metadata": q.metadata,
        "parity": q.parity,
    })
}

/// `llm_infer` tool — runs local GGUF model autoregressive inference.
pub fn llm_infer(args: &[u8]) -> Result<String, McpSystemError> {
    let v = parse_tool_args(args)?;
    let prompt = v.get("prompt").and_then(Value::as_str).unwrap_or("");
    if prompt.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }
    let graph_context = v.get("graph_context").and_then(Value::as_str).unwrap_or("");
    let model_path = v
        .get("model_path")
        .and_then(Value::as_str)
        .map(str::to_string)
        .or_else(|| qualia_core_db::resident_model::resident_gguf_path())
        .or_else(|| std::env::var("QUALIA_ACTIVE_GGUF").ok())
        .unwrap_or_default();

    if model_path.is_empty() {
        return Err(McpSystemError::InvalidParameters);
    }

    let sampler_cbor = v.get("sampler_cbor").and_then(Value::as_str).unwrap_or("");
    let sampler_cfg = if sampler_cbor.is_empty() {
        None
    } else {
        hex_decode(sampler_cbor)
            .ok()
            .as_deref()
            .and_then(qualia_core_db::sampler::SamplerConfig::from_cbor)
            .filter(|c| !c.is_greedy())
    };
    let sampler_installed = sampler_cfg.is_some();
    if sampler_installed {
        qualia_core_db::llm_bench::set_sampler_config(sampler_cfg);
    }

    let agent = qualia_core_db::llm_agent::LocalLlmAgent::new("did:qualia:mcp-agent", &model_path);
    let (text, provenance, tokens, semantic_quin) =
        agent.infer_local_model_streaming(prompt, graph_context, None::<fn(String)>);

    if sampler_installed {
        qualia_core_db::llm_bench::set_sampler_config(None);
    }

    let payload = json!({
        "text": text,
        "tokensGenerated": tokens,
        "provenanceHashes": provenance,
        "semanticQuin": semantic_quin.map(|q| quin_to_json(&q)),
        "modelPath": model_path,
        "sampled": sampler_installed,
    });
    serde_json::to_string(&payload).map_err(|_| McpSystemError::ParseError)
}

/// `llm_chat` tool — multi-turn chat completion via the local GGUF inference engine.
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

/// `list_models` tool — discovers GGUF models under local storage and reports any resident mounted model.
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
        qualia_core_db::resident_model::resident_model_id(),
        qualia_core_db::resident_model::resident_gguf_path(),
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
