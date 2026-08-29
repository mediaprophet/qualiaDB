//! Local-model chat boundary for the standalone POET browser.

use std::path::Path;

use axum::{
    body::Bytes,
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Deserialize;

use crate::llm_agent::{AgentIntent, AgentRuntime, LocalLlmAgent, WebizenVerdict};
use crate::modalities::logic::n3_compiler::N3OutputMode;

pub const LLM_REQUEST_LIMIT_BYTES: usize = 128 * 1024;
const MAX_PROMPT_BYTES: usize = 32 * 1024;
const MAX_CONTEXT_BYTES: usize = 64 * 1024;

#[derive(Debug, Deserialize)]
pub struct PoetLlmRequest {
    pub model_path: String,
    pub prompt: String,
    #[serde(default)]
    pub graph_context: String,
    #[serde(default = "default_agent_did")]
    pub agent_did: String,
    #[serde(default)]
    pub principal_did: String,
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32,
    #[serde(default)]
    pub library_projects: Vec<String>,
    #[serde(default)]
    pub library_context_supplied: bool,
}

fn default_agent_did() -> String {
    "did:qualia:poet-local-agent".into()
}

fn default_max_tokens() -> u32 {
    256
}

/// Return only model files explicitly mounted, configured through
/// `QUALIA_MODEL_PATHS`, or present in QualiaDB's documented model locations.
/// Directories are scanned one level deep and the result is bounded.
pub async fn models_handler() -> Response {
    Json(serde_json::json!({
        "ok": true,
        "honesty": "live-local-catalogue",
        "data": { "models": discover_local_models() }
    }))
    .into_response()
}

#[derive(Deserialize)]
struct ModelLifecycleRequest {
    model_path: String,
}

pub async fn activate_model_handler(body: Bytes) -> Response {
    if body.len() > 4096 {
        return failure(
            StatusCode::PAYLOAD_TOO_LARGE,
            "model_lifecycle_payload",
            "Model activation requests are limited to 4 KiB",
        );
    }
    if super::poet_llm_jobs::has_active_jobs() {
        return failure(
            StatusCode::CONFLICT,
            "model_in_use",
            "Cancel or finish active local-model jobs before changing the resident model",
        );
    }
    let request: ModelLifecycleRequest = match serde_json::from_slice(&body) {
        Ok(request) => request,
        Err(error) => return failure(StatusCode::BAD_REQUEST, "invalid_json", error.to_string()),
    };
    let path = request.model_path.trim().to_string();
    if let Err(message) = validate_model_file(&path) {
        return failure(StatusCode::BAD_REQUEST, "invalid_model", message);
    }
    let model_id = crate::q_hash(&path);
    let mounted_path = path.clone();
    match tokio::task::spawn_blocking(move || {
        crate::resident_model::mount_resident_model(model_id, &mounted_path, false)
    })
    .await
    {
        Ok(Ok(report)) => Json(serde_json::json!({
            "ok": true,
            "honesty": "resident-model-active",
            "data": {
                "model_id": format!("model:{model_id:016x}"),
                "model_path": crate::resident_model::resident_gguf_path().unwrap_or(path),
                "mapped_bytes": report.mapped_bytes,
                "layers": report.n_layer,
                "heads": report.n_head,
                "kv_heads": report.n_kv_head,
                "directml": report.directml_enabled
            }
        }))
        .into_response(),
        Ok(Err(message)) => failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            "model_activation_failed",
            message,
        ),
        Err(error) => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "model_activation_worker",
            error.to_string(),
        ),
    }
}

pub async fn evict_model_handler() -> Response {
    if super::poet_llm_jobs::has_active_jobs() {
        return failure(
            StatusCode::CONFLICT,
            "model_in_use",
            "Cancel or finish active local-model jobs before evicting the resident model",
        );
    }
    let previous = crate::resident_model::resident_gguf_path();
    crate::resident_model::clear_resident_model();
    Json(serde_json::json!({
        "ok": true,
        "honesty": "resident-model-evicted",
        "data": { "previous_model_path": previous }
    }))
    .into_response()
}

fn validate_model_file(path: &str) -> Result<(), String> {
    let path = Path::new(path);
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if !matches!(extension.as_str(), "gguf" | "p64") {
        return Err("Select a local .gguf or .p64 model".into());
    }
    if !path.is_file() {
        return Err("The selected local model file does not exist".into());
    }
    Ok(())
}

fn discover_local_models() -> Vec<serde_json::Value> {
    const MAX_MODELS: usize = 64;
    const KNOWN_MODELS: &[&str] = &[
        r"C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.f16.p64",
        r"C:\LLM_Models\P64\smollm2-360m-instruct-q8_0.p64",
        r"C:\LLM_Models\GGUF\smollm2-360m-instruct-q8_0.gguf",
        r"C:\LLM_Models\GGUF\lmstudio-community\smollm2-360m-instruct-q8_0.gguf",
    ];

    let resident = crate::resident_model::resident_gguf_path();
    let mut candidates = Vec::new();
    if let Some(path) = resident.as_deref() {
        push_model_path(&mut candidates, Path::new(path), MAX_MODELS);
    }
    if let Some(configured) = std::env::var_os("QUALIA_MODEL_PATHS") {
        for path in std::env::split_paths(&configured).take(MAX_MODELS) {
            if path.is_dir() {
                if let Ok(entries) = std::fs::read_dir(path) {
                    for entry in entries.flatten().take(MAX_MODELS) {
                        push_model_path(&mut candidates, &entry.path(), MAX_MODELS);
                    }
                }
            } else {
                push_model_path(&mut candidates, &path, MAX_MODELS);
            }
            if candidates.len() >= MAX_MODELS {
                break;
            }
        }
    }
    for path in KNOWN_MODELS {
        push_model_path(&mut candidates, Path::new(path), MAX_MODELS);
    }

    candidates.sort();
    candidates.dedup();
    candidates
        .into_iter()
        .map(|path| {
            let metadata = std::fs::metadata(&path).ok();
            let path_text = path.to_string_lossy().into_owned();
            let name = path
                .file_name()
                .and_then(|value| value.to_str())
                .unwrap_or("local model");
            let format = path
                .extension()
                .and_then(|value| value.to_str())
                .unwrap_or_default()
                .to_ascii_lowercase();
            serde_json::json!({
                "id": format!("model:{:016x}", crate::q_hash(&path_text)),
                "name": name,
                "path": path_text,
                "format": format,
                "bytes": metadata.map(|value| value.len()).unwrap_or(0),
                "resident": resident.as_deref() == Some(path.to_string_lossy().as_ref())
            })
        })
        .collect()
}

fn push_model_path(out: &mut Vec<std::path::PathBuf>, path: &Path, limit: usize) {
    if out.len() >= limit || !path.is_file() {
        return;
    }
    let extension = path
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or_default();
    if !matches!(extension.to_ascii_lowercase().as_str(), "gguf" | "p64") {
        return;
    }
    out.push(path.canonicalize().unwrap_or_else(|_| path.to_path_buf()));
}

fn failure(status: StatusCode, code: &'static str, message: impl Into<String>) -> Response {
    (
        status,
        Json(serde_json::json!({
            "ok": false,
            "honesty": "unavailable",
            "code": code,
            "diagnostic": message.into()
        })),
    )
        .into_response()
}

pub(crate) fn decode_request(body: &Bytes) -> Result<PoetLlmRequest, Response> {
    if body.len() > LLM_REQUEST_LIMIT_BYTES {
        return Err(failure(
            StatusCode::PAYLOAD_TOO_LARGE,
            "payload_too_large",
            "Local LLM requests are limited to 128 KiB",
        ));
    }
    let mut request: PoetLlmRequest = serde_json::from_slice(body)
        .map_err(|error| failure(StatusCode::BAD_REQUEST, "invalid_json", error.to_string()))?;
    request.model_path = request.model_path.trim().to_string();
    request.prompt = request.prompt.trim().to_string();
    request.agent_did = request.agent_did.trim().to_string();
    request.principal_did = request.principal_did.trim().to_string();
    if request.prompt.is_empty() || request.prompt.len() > MAX_PROMPT_BYTES {
        return Err(failure(
            StatusCode::BAD_REQUEST,
            "invalid_prompt",
            "Prompt must contain 1..=32768 bytes",
        ));
    }
    if request.graph_context.len() > MAX_CONTEXT_BYTES {
        return Err(failure(
            StatusCode::BAD_REQUEST,
            "context_too_large",
            "Grounding context is limited to 64 KiB",
        ));
    }
    if request.agent_did.is_empty() || !request.agent_did.starts_with("did:") {
        return Err(failure(
            StatusCode::BAD_REQUEST,
            "invalid_agent",
            "Agent identity must be a DID",
        ));
    }
    if !(1..=256).contains(&request.max_tokens) {
        return Err(failure(
            StatusCode::BAD_REQUEST,
            "invalid_token_budget",
            "Local model token budget must be between 1 and 256",
        ));
    }
    if request.library_projects.len() > 32
        || request
            .library_projects
            .iter()
            .any(|project| project.is_empty() || project.len() > 128)
    {
        return Err(failure(
            StatusCode::BAD_REQUEST,
            "invalid_library_scope",
            "Library grounding supports at most 32 non-empty project tags of 128 bytes",
        ));
    }
    if let Err(message) = validate_model_file(&request.model_path) {
        return Err(failure(StatusCode::BAD_REQUEST, "model_not_found", message));
    }
    if let Err(message) = super::poet_record_api::authorize_agent_run(
        &request.agent_did,
        &request.principal_did,
        &request.model_path,
        &request.library_projects,
        request.library_context_supplied,
        request.max_tokens,
    ) {
        return Err(failure(
            StatusCode::FORBIDDEN,
            "agent_authority_denied",
            message,
        ));
    }
    Ok(request)
}

pub async fn generate_handler(body: Bytes) -> Response {
    let request = match decode_request(&body) {
        Ok(request) => request,
        Err(response) => return response,
    };
    match tokio::task::spawn_blocking(move || run_local_turn(request)).await {
        Ok(Ok(value)) => Json(value).into_response(),
        Ok(Err(message)) => failure(
            StatusCode::UNPROCESSABLE_ENTITY,
            "inference_failed",
            message,
        ),
        Err(error) => failure(
            StatusCode::INTERNAL_SERVER_ERROR,
            "inference_worker_failed",
            error.to_string(),
        ),
    }
}

fn run_local_turn(request: PoetLlmRequest) -> Result<serde_json::Value, String> {
    let agent = LocalLlmAgent::new(&request.agent_did, &request.model_path);
    let context_hash = crate::q_hash(&request.graph_context);
    let intent = AgentIntent {
        intent_predicate: crate::q_hash("llm:ReadGraph"),
        requested_graph_scope: if request.graph_context.is_empty() {
            Vec::new()
        } else {
            vec![context_hash]
        },
        context_namespaces: Vec::new(),
        requires_network: false,
        ilp_offer_micro_cents: 0,
        principal_did_hash: crate::q_hash(&request.principal_did),
        mcp_intent_frame_hash: crate::q_hash("poet:local-chat-turn"),
        output_mode: N3OutputMode::FreeText,
        clearance_ceiling: 0,
        max_sentinel_depth: 32,
        active_profile: None,
    };
    match agent.validate_intent(&intent) {
        WebizenVerdict::Permit => {}
        verdict => return Err(format!("Webizen rejected the model intent: {verdict:?}")),
    }

    let output = agent
        .infer(&request.prompt, &request.graph_context)
        .map_err(|error| format!("{error:?}"))?;
    match agent.validate_output(&output) {
        WebizenVerdict::Permit | WebizenVerdict::Sanitised { .. } => {}
        verdict => return Err(format!("Webizen rejected the model output: {verdict:?}")),
    }
    let verified =
        crate::inference::post_turn_verify::maybe_verify_turn(&request.prompt, &output.text);
    let checks = verified
        .checks
        .iter()
        .map(|check| {
            serde_json::json!({
                "id": check.id,
                "ok": check.ok,
                "detail": check.detail
            })
        })
        .collect::<Vec<_>>();

    Ok(serde_json::json!({
        "ok": true,
        "honesty": "live-local-model",
        "assertion_status": "model_assertion_requires_verification",
        "agent_did": request.agent_did,
        "model_path": request.model_path,
        "text": verified.final_text,
        "draft": output.text,
        "tokens_generated": output.tokens_generated,
        "inference_duration_ms": output.inference_duration_ms,
        "provenance_hashes": output.provenance_quins,
        "context_hash": context_hash,
        "context_supplied": !request.graph_context.is_empty(),
        "repaired": verified.repaired,
        "checks": checks,
        "semantic_quin": output.semantic_quin
    }))
}
