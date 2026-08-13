//! Local job scheduler

#![allow(non_snake_case)]

use super::*;

use std::path::Path;

/// Schedule one agent turn as a background job (queued, off the chat thread). Routed local-first; a
/// remote-MCP agent's turn is sent out over MCP. Returns the created job as JSON.
pub fn schedule_agent_job(
    session_id: String,
    agent_slug: Option<String>,
    prompt: String,
) -> Result<serde_json::Value, String> {
    let agent_updated_at_unix = if let Some(slug) = agent_slug.as_deref() {
        let state = crate::state::APP_STATE
            .get()
            .ok_or("Application not initialized")?;
        let storage = state
            .config
            .lock()
            .map_err(|error| error.to_string())?
            .storage_path
            .clone();
        Some(
            crate::agent_registry::get_agent(Path::new(&storage), slug)
                .ok_or_else(|| format!("unknown agent @{slug}"))?
                .updated_at_unix,
        )
    } else {
        None
    };
    let job = crate::local_job_scheduler::LocalJobScheduler::global().enqueue(
        crate::local_job_scheduler::LocalJobKind::AgentTurn {
            session_id,
            agent_slug,
            agent_updated_at_unix,
            prompt,
        },
    )?;
    serde_json::to_value(job).map_err(|e| e.to_string())
}

/// Snapshot of the local job queue (jobs + status counts).
pub fn list_local_jobs() -> Result<serde_json::Value, String> {
    let snap = crate::local_job_scheduler::LocalJobScheduler::global().snapshot()?;
    serde_json::to_value(snap).map_err(|e| e.to_string())
}

/// Cancel a job by id (queued → cancelled; running → cooperative cancel).
pub fn cancel_local_job(id: String) -> Result<bool, String> {
    crate::local_job_scheduler::LocalJobScheduler::global().cancel(&id)
}

/// Re-run a finished job with the same bounded inputs.
pub fn retry_local_job(id: String) -> Result<serde_json::Value, String> {
    let job = crate::local_job_scheduler::LocalJobScheduler::global().retry(&id)?;
    serde_json::to_value(job).map_err(|e| e.to_string())
}

/// Clear completed/failed/cancelled history without affecting active work.
pub fn clear_finished_local_jobs() -> Result<usize, String> {
    crate::local_job_scheduler::LocalJobScheduler::global().clear_finished()
}

pub fn schedule_model_download(
    url: String,
    filename: String,
    model_id: String,
) -> Result<serde_json::Value, String> {
    let job = crate::local_job_scheduler::LocalJobScheduler::global().enqueue(
        crate::local_job_scheduler::LocalJobKind::ModelDownload {
            url,
            filename,
            model_id,
        },
    )?;
    serde_json::to_value(job).map_err(|e| e.to_string())
}

pub fn schedule_model_activation(model_name: String) -> Result<serde_json::Value, String> {
    let job = crate::local_job_scheduler::LocalJobScheduler::global()
        .enqueue(crate::local_job_scheduler::LocalJobKind::ModelActivation { model_name })?;
    serde_json::to_value(job).map_err(|e| e.to_string())
}

pub fn schedule_anatomy_asset_acquire(model: String) -> Result<serde_json::Value, String> {
    // Validate before queueing so typo failures are immediate and visible at the initiating control.
    crate::wellfair::api::parse_anatomy_model(&model)?;
    let job = crate::local_job_scheduler::LocalJobScheduler::global()
        .enqueue(crate::local_job_scheduler::LocalJobKind::AnatomyAssetAcquire { model })?;
    serde_json::to_value(job).map_err(|e| e.to_string())
}

/// Enqueue a job for a specific apparatus (`did:q42:device:…`). Empty target → this install.
/// Remote devices fail closed until multi-device dispatch is live.
pub fn schedule_job_on_device(
    kind_json: String,
    target_device_id: Option<String>,
) -> Result<serde_json::Value, String> {
    let kind: crate::local_job_scheduler::LocalJobKind =
        serde_json::from_str(&kind_json).map_err(|e| format!("invalid job kind: {e}"))?;
    let job = crate::local_job_scheduler::LocalJobScheduler::global()
        .enqueue_for_device(kind, target_device_id)?;
    serde_json::to_value(job).map_err(|e| e.to_string())
}

pub fn ensure_chat_session() -> Result<String, String> {
    if let Some(id) = get_last_chat_session_id() {
        let state = crate::state::APP_STATE.get().unwrap();
        let storage = state.config.lock().unwrap().storage_path.clone();
        if crate::chat_session::load_session(Path::new(&storage), &id).is_ok() {
            return Ok(id);
        }
    }
    create_chat_session(None)
}

pub fn create_group_chat_session(
    title: Option<String>,
    participant_dids: Vec<String>,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::create_group_session(Path::new(&storage), title, &participant_dids)
        .map_err(|e| e.to_string())
}

pub fn add_chat_participant(
    session_id: String,
    participant_did: String,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let participants =
        crate::chat_session::add_participant(Path::new(&storage), &session_id, &participant_did)
            .map_err(|e| e.to_string())?;
    serde_json::to_value(participants).map_err(|e| e.to_string())
}

pub fn remove_chat_participant(
    session_id: String,
    participant_did: String,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let participants =
        crate::chat_session::remove_participant(Path::new(&storage), &session_id, &participant_did)
            .map_err(|e| e.to_string())?;
    serde_json::to_value(participants).map_err(|e| e.to_string())
}

pub fn get_chat_participants(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let participants = crate::chat_session::get_participants(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(participants).map_err(|e| e.to_string())
}

pub fn get_local_agent_config(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let cfg = crate::chat_agents::load_local_agent_config(Path::new(&storage), &session_id)?;
    serde_json::to_value(cfg).map_err(|e| e.to_string())
}

pub fn update_agent_outcome_sharing(
    session_id: String,
    policy_json: String,
) -> Result<serde_json::Value, String> {
    let policy: crate::chat_agents::OutcomeSharingPolicy =
        serde_json::from_str(&policy_json).map_err(|e| e.to_string())?;
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let cfg = crate::chat_agents::update_outcome_sharing(Path::new(&storage), &session_id, policy)?;
    serde_json::to_value(cfg).map_err(|e| e.to_string())
}

pub fn get_default_outcome_sharing(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let policy = crate::chat_agents::default_outcome_sharing(session.meta.session_kind);
    serde_json::to_value(policy).map_err(|e| e.to_string())
}
