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
    let job = crate::local_job_scheduler::LocalJobScheduler::global().enqueue(
        crate::local_job_scheduler::LocalJobKind::AgentTurn {
            session_id,
            agent_slug,
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
