//! Agent-oriented diagnostics and bounded, reversible smoke probes.
//!
//! Commands return structured JSON so an automated agent never has to infer
//! state from presentation text. The inference probe creates a temporary chat
//! session and removes it before returning.

use qualia_client_core::api;
use serde::Serialize;
use std::time::Instant;
use tauri::command;

const MAX_SAMPLE_CHARS: usize = 480;
/// An Instruments invocation is a bounded, one-shot tool request, not a
/// persistent peer conversation. The native model context has its own limit;
/// this keeps the desktop IPC payload bounded before it reaches that runtime.
const MAX_INSTRUMENT_PROMPT_CHARS: usize = 16 * 1024;

#[derive(Debug, Serialize)]
pub struct AgentQaSnapshot {
    pub schema_version: u32,
    pub captured_at_unix: u64,
    pub setup: serde_json::Value,
    pub config: serde_json::Value,
    pub active_model: Option<String>,
    pub model_status: serde_json::Value,
    pub hardware: serde_json::Value,
    pub daemon_status: String,
    pub mail_receiver: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct AgentQaModelProbe {
    pub schema_version: u32,
    pub passed: bool,
    pub active_model: Option<String>,
    pub committed: bool,
    pub duration_ms: u64,
    pub output_sample: String,
    pub block_reason: Option<String>,
    pub cleanup_succeeded: bool,
}

/// The result displayed by the OS-shell Instruments volume. `committed` is
/// false when the existing Webizen inference pipeline denied or otherwise
/// declined the request; callers must show `block_reason` instead of treating
/// it as a model reply.
#[derive(Debug, Serialize)]
pub struct InstrumentInferenceResult {
    pub committed: bool,
    pub text: String,
    pub block_reason: Option<String>,
    pub duration_ms: u64,
}

/// Read-only readiness information for the Instruments volume. This must not
/// claim that a model is usable merely because a UI tile exists.
#[derive(Debug, Serialize)]
pub struct InstrumentStatus {
    pub active_model: Option<String>,
    pub model_status: serde_json::Value,
    pub daemon_status: String,
    pub ready: bool,
}

#[command]
pub fn instrument_status() -> Result<InstrumentStatus, String> {
    let active_model = api::get_active_model();
    let model_status = api::get_model_lifecycle_status()?;
    Ok(InstrumentStatus {
        ready: active_model.is_some(),
        active_model,
        model_status,
        daemon_status: api::daemon_status(),
    })
}

#[command]
pub fn agent_qa_snapshot() -> Result<AgentQaSnapshot, String> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(|error| format!("Clock error: {error}"))?
        .as_secs();
    Ok(AgentQaSnapshot {
        schema_version: 1,
        captured_at_unix: now,
        setup: serde_json::to_value(api::get_setup_state()?).map_err(|e| e.to_string())?,
        config: serde_json::to_value(api::get_config()).map_err(|e| e.to_string())?,
        active_model: api::get_active_model(),
        model_status: api::get_model_lifecycle_status()?,
        hardware: serde_json::to_value(api::get_hardware_status()).map_err(|e| e.to_string())?,
        daemon_status: api::daemon_status(),
        mail_receiver: api::mail_receiver_status()
            .unwrap_or_else(|error| serde_json::json!({ "error": error })),
    })
}

#[command]
pub async fn agent_qa_test_active_model() -> Result<AgentQaModelProbe, String> {
    tauri::async_runtime::spawn_blocking(run_active_model_probe)
        .await
        .map_err(|error| format!("Model probe worker failed: {error}"))?
}

/// Run one native, Webizen-governed inference turn for the Instruments tool
/// plane. A temporary session is deliberately removed after the result so this
/// cannot appear as a person-to-person Talk thread.
#[command]
pub async fn instrument_infer(prompt: String) -> Result<InstrumentInferenceResult, String> {
    let prompt = validate_instrument_prompt(prompt)?;
    tauri::async_runtime::spawn_blocking(move || run_instrument_inference(prompt))
        .await
        .map_err(|error| format!("Instrument inference worker failed: {error}"))?
}

fn validate_instrument_prompt(prompt: String) -> Result<String, String> {
    let prompt = prompt.trim().to_string();
    if prompt.is_empty() {
        return Err("Enter a request for the local tool.".to_string());
    }
    if prompt.chars().count() > MAX_INSTRUMENT_PROMPT_CHARS {
        return Err(format!(
            "Instrument request exceeds the {} character limit.",
            MAX_INSTRUMENT_PROMPT_CHARS
        ));
    }
    Ok(prompt)
}

fn run_instrument_inference(prompt: String) -> Result<InstrumentInferenceResult, String> {
    let session_id = api::create_chat_session(Some("Webizen Instruments invocation".to_string()))?;
    let started = Instant::now();
    let response = api::run_chat_inference_detailed(session_id.clone(), prompt);
    let cleanup = api::delete_chat_session(session_id);
    let value = response?;
    if let Err(error) = cleanup {
        return Err(format!(
            "Inference completed but temporary session cleanup failed: {error}"
        ));
    }

    Ok(InstrumentInferenceResult {
        committed: value
            .get("committed")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(false),
        text: value
            .get("text")
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_string(),
        block_reason: value
            .get("block_reason")
            .and_then(serde_json::Value::as_str)
            .map(str::to_string),
        duration_ms: started.elapsed().as_millis() as u64,
    })
}

fn run_active_model_probe() -> Result<AgentQaModelProbe, String> {
    let active_model = api::get_active_model();
    if active_model.is_none() {
        return Ok(AgentQaModelProbe {
            schema_version: 1,
            passed: false,
            active_model,
            committed: false,
            duration_ms: 0,
            output_sample: String::new(),
            block_reason: Some("No active model".to_string()),
            cleanup_succeeded: true,
        });
    }

    let session_id = api::create_chat_session(Some("Webizen model readiness probe".to_string()))?;
    let started = Instant::now();
    let result = api::run_chat_inference_detailed(
        session_id.clone(),
        "Reply with exactly: WEBIZEN_MODEL_READY".to_string(),
    );
    let duration_ms = started.elapsed().as_millis() as u64;
    let cleanup_succeeded = api::delete_chat_session(session_id).is_ok();
    let value = result?;
    let committed = value
        .get("committed")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false);
    let output = value
        .get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let output_sample: String = output.chars().take(MAX_SAMPLE_CHARS).collect();
    let block_reason = value
        .get("block_reason")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);

    Ok(AgentQaModelProbe {
        schema_version: 1,
        passed: committed && !output.trim().is_empty() && cleanup_succeeded,
        active_model,
        committed,
        duration_ms,
        output_sample,
        block_reason,
        cleanup_succeeded,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sample_limit_is_bounded_by_characters() {
        let text = "é".repeat(MAX_SAMPLE_CHARS + 20);
        let bounded: String = text.chars().take(MAX_SAMPLE_CHARS).collect();
        assert_eq!(bounded.chars().count(), MAX_SAMPLE_CHARS);
    }

    #[test]
    fn instrument_prompt_is_bounded_and_trimmed() {
        assert_eq!(
            validate_instrument_prompt("  hello  ".to_string()).unwrap(),
            "hello"
        );
        assert!(validate_instrument_prompt(" ".to_string()).is_err());
        assert!(validate_instrument_prompt("x".repeat(MAX_INSTRUMENT_PROMPT_CHARS + 1)).is_err());
    }
}
