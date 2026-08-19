//! Chat sessions

#![allow(non_snake_case)]

use super::*;

use std::path::Path;

pub fn create_chat_session(title: Option<String>) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::create_session(Path::new(&storage), title, None).map_err(|e| e.to_string())
}

pub fn list_chat_sessions() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let sessions =
        crate::chat_session::list_sessions(Path::new(&storage)).map_err(|e| e.to_string())?;
    serde_json::to_value(sessions).map_err(|e| e.to_string())
}

pub fn load_chat_session(id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session =
        crate::chat_session::load_session(Path::new(&storage), &id).map_err(|e| e.to_string())?;
    serde_json::to_value(session).map_err(|e| e.to_string())
}

pub fn append_chat_message(
    session_id: String,
    role: String,
    content: String,
) -> Result<u64, String> {
    append_chat_message_reply(session_id, role, content, None, None)
}

pub fn append_chat_message_reply(
    session_id: String,
    role: String,
    content: String,
    reply_to_fragment: Option<String>,
    branch_type_id: Option<String>,
) -> Result<u64, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let role = crate::chat_session::Role::from_str(&role).map_err(|e| e.to_string())?;
    crate::chat_session::append_message_with_author(
        Path::new(&storage),
        &session_id,
        role,
        &content,
        reply_to_fragment,
        None,
        None,
        None,
        branch_type_id,
    )
    .map_err(|e| e.to_string())
}

pub fn compact_chat_session(session_id: String) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let path = crate::chat_session::compact_session_to_q42(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().into_owned())
}

pub fn delete_chat_session(session_id: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::delete_session(Path::new(&storage), &session_id).map_err(|e| e.to_string())
}

pub fn rename_chat_session(session_id: String, title: String) -> Result<(), String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::chat_session::rename_session(Path::new(&storage), &session_id, &title)
        .map_err(|e| e.to_string())
}

pub fn get_last_chat_session_id() -> Option<String> {
    crate::chat_session::get_last_session_id()
}

pub fn set_last_chat_session_id(session_id: String) -> Result<(), String> {
    crate::chat_session::set_last_session_id(&session_id).map_err(|e| e.to_string())
}

pub fn compile_session_environment(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let catalog = load_workspace_catalog();
    let env = crate::context_binding::refresh_session_environment(
        Path::new(&storage),
        &catalog,
        &session_id,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_value(env).map_err(|e| e.to_string())
}

pub fn update_session_environment(
    session_id: String,
    ontology_ids: Vec<String>,
    prior_session_ids: Vec<String>,
    graph_mutation: bool,
    axiom_start_year: u32,
    axiom_end_year: u32,
    spatial_context: String,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let catalog = load_workspace_catalog();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let axiom_bounds = crate::context_binding::AxiomBounds {
        start_year: axiom_start_year.min(u16::MAX as u32) as u16,
        end_year: axiom_end_year.min(u16::MAX as u32) as u16,
        spatial_context_hash: 0,
        spatial_context_label: spatial_context.clone(),
    }
    .with_spatial_label(&spatial_context);

    let config = crate::context_binding::ChatEnvironmentConfig {
        session_id: session_id.clone(),
        ontology_ids,
        prior_session_ids,
        session_kind: session.meta.session_kind,
        participants: session.meta.participants.clone(),
        graph_mutation,
        axiom_bounds,
    };
    let env =
        crate::context_binding::compile_chat_environment(Path::new(&storage), &catalog, &config)
            .map_err(|e| e.to_string())?;
    env.save_to_session_dir(Path::new(&storage))
        .map_err(|e| e.to_string())?;
    serde_json::to_value(env).map_err(|e| e.to_string())
}

pub fn get_session_environment(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    serde_json::to_value(session.environment).map_err(|e| e.to_string())
}

pub fn list_installed_ontology_ids_for_chat() -> Vec<String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    crate::context_binding::list_installed_ontology_ids(Path::new(&storage))
}

pub fn run_chat_inference(session_id: String, prompt: String) -> Result<String, String> {
    let result = run_chat_inference_dispatch(&session_id, &prompt);
    if result.committed {
        Ok(result.text)
    } else {
        Err(result
            .block_reason
            .unwrap_or_else(|| "Inference blocked".to_string()))
    }
}

pub fn run_chat_inference_detailed(
    session_id: String,
    prompt: String,
) -> Result<serde_json::Value, String> {
    let result = run_chat_inference_dispatch(&session_id, &prompt);
    serde_json::to_value(result).map_err(|e| e.to_string())
}

/// Parse any `@mention` in the prompt and dispatch to the appropriate
/// inference path. If no mention is found, uses the default path. If a
/// mention resolves to a known roster agent, dispatches through
/// [`run_chat_inference_for_agent`] with the stripped prompt. If the
/// mention is unknown, returns an error result.
fn run_chat_inference_dispatch(
    session_id: &str,
    prompt: &str,
) -> crate::chat_inference::ChatInferenceResult {
    use crate::chat_agents::{PromptDispatch, resolve_prompt_dispatch};

    let storage = match crate::state::APP_STATE.get() {
        Some(state) => state.config.lock().map(|c| c.storage_path.clone()).unwrap_or_default(),
        None => {
            return crate::chat_inference::run_chat_inference_with_options(
                session_id, prompt, None,
            )
        }
    };

    match resolve_prompt_dispatch(std::path::Path::new(&storage), prompt) {
        PromptDispatch::Default => {
            crate::chat_inference::run_chat_inference_with_options(session_id, prompt, None)
        }
        PromptDispatch::Agent { agent, clean_prompt } => {
            // T9+: If the agent has a DAG pipeline, execute it through the
            // agent-turn handler. Otherwise, fall through to single-turn.
            if !agent.dag_pipeline_json.is_empty() {
                use crate::agent_turn_handler::{execute_agent_turn, AgentTurnConfig, AgentTurnOutcome};
                let mut config = AgentTurnConfig {
                    session_id: session_id.to_string(),
                    storage_path: storage.clone(),
                    agent_slug: agent.slug.clone(),
                    dag_pipeline: None, // use agent's stored pipeline
                };
                match execute_agent_turn(&mut config) {
                    AgentTurnOutcome::DagPipeline(result) => {
                        let mut r = crate::chat_inference::ChatInferenceResult::default();
                        r.text = result.summary();
                        r
                    }
                    AgentTurnOutcome::SingleTurn => {
                        // No DAG or non-local agent — fall through.
                        crate::chat_inference::run_chat_inference_for_agent(
                            session_id,
                            &clean_prompt,
                            Some(&agent.slug),
                            None,
                        )
                    }
                    AgentTurnOutcome::AgentUnavailable(reason) => {
                        let mut r = crate::chat_inference::ChatInferenceResult::default();
                        r.block_reason = Some(reason);
                        r
                    }
                    AgentTurnOutcome::InvalidDag(reason) => {
                        let mut r = crate::chat_inference::ChatInferenceResult::default();
                        r.block_reason = Some(reason);
                        r
                    }
                }
            } else {
                crate::chat_inference::run_chat_inference_for_agent(
                    session_id,
                    &clean_prompt,
                    Some(&agent.slug),
                    None,
                )
            }
        }
        PromptDispatch::UnknownAgent { slug, .. } => {
            let mut result = crate::chat_inference::ChatInferenceResult::default();
            result.block_reason = Some(format!("Unknown agent @{slug}"));
            result
        }
    }
}

pub fn cancel_chat_inference() {
    crate::chat_inference::request_cancel_inference();
}
