//! Peer agreements

#![allow(non_snake_case)]

use super::*;

use std::path::Path;

/// All peer agreements.
pub fn list_agreements() -> Result<serde_json::Value, String> {
    serde_json::to_value(crate::agreements::list_agreements()).map_err(|e| e.to_string())
}

/// Agreements a DID is party to (or that govern its relationship) — fills the directory's agreement slot.
pub fn agreements_for(did: String) -> Result<serde_json::Value, String> {
    serde_json::to_value(crate::agreements::agreements_for(&did)).map_err(|e| e.to_string())
}

/// Create a new **draft** agreement for a relationship, grounded in the non-derogable values floor
/// (defaults to UDHR), with a pending consent for each party. Returns the created agreement.
pub fn create_agreement(
    title: String,
    relationship_did: String,
    parties: Vec<String>,
) -> Result<serde_json::Value, String> {
    let now = mail_now_unix();
    let a = crate::agreements::Agreement {
        id: uuid::Uuid::new_v4().to_string(),
        title,
        relationship_did,
        parties: parties.clone(),
        values_anchors: vec!["urn:qualia:values:udhr".to_string()],
        undertakings: vec![],
        consents: parties
            .into_iter()
            .map(|did| crate::agreements::PartyConsent {
                did,
                consent: crate::agreements::ConsentState::Pending,
                signature_hex: None,
            })
            .collect(),
        stage: crate::agreements::FormationStage::Draft,
        jurisdiction: None,
        intents: Vec::new(),
        artifact_context: None,
        created_at: now,
        updated_at: now,
    };
    crate::agreements::upsert_agreement(a.clone())?;
    serde_json::to_value(a).map_err(|e| e.to_string())
}

/// Persist a full agreement (JSON) — for edits (undertakings, stage, etc.). Returns the refreshed list.
pub fn save_agreement(agreement_json: String) -> Result<serde_json::Value, String> {
    let a: crate::agreements::Agreement =
        serde_json::from_str(&agreement_json).map_err(|e| format!("bad agreement: {e}"))?;
    crate::agreements::upsert_agreement(a)?;
    list_agreements()
}

/// Set a party's consent on an agreement (`state`: pending / granted / withdrawn).
pub fn set_agreement_consent(
    id: String,
    did: String,
    state: String,
) -> Result<serde_json::Value, String> {
    let mut all = crate::agreements::list_agreements();
    let a = all
        .iter_mut()
        .find(|a| a.id == id)
        .ok_or_else(|| format!("unknown agreement '{id}'"))?;
    let cs = match state.to_lowercase().as_str() {
        "granted" => crate::agreements::ConsentState::Granted,
        "withdrawn" => crate::agreements::ConsentState::Withdrawn,
        _ => crate::agreements::ConsentState::Pending,
    };
    crate::agreements::set_consent(a, &did, cs);
    a.updated_at = mail_now_unix();
    let updated = a.clone();
    crate::agreements::upsert_agreement(updated)?;
    list_agreements()
}

pub fn get_chat_graph(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let graph = crate::chat_graph::load_graph(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let reactions =
        crate::chat_ontology::list_reactions(Path::new(&storage), &session_id).unwrap_or_default();
    let branch_types = crate::chat_ontology::list_branch_types(Path::new(&storage));
    serde_json::to_value(serde_json::json!({
        "fragments": graph.fragments,
        "edges": graph.edges,
        "messages": session.messages,
        "reactions": reactions,
        "branch_types": branch_types,
        "wordnet": crate::chat_ontology::resolve_wordnet_q42(Path::new(&storage))
            .map(|p| p.to_string_lossy().to_string()),
    }))
    .map_err(|e| e.to_string())
}

pub fn create_chat_fragment(
    session_id: String,
    message_lamport: u64,
    anchor_start: u32,
    anchor_end: u32,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let msg = session
        .messages
        .iter()
        .find(|m| m.lamport == message_lamport)
        .ok_or_else(|| format!("message {message_lamport} not found"))?;
    let fragment = crate::chat_graph::create_fragment_from_selection(
        Path::new(&storage),
        &session_id,
        message_lamport,
        &msg.content,
        anchor_start,
        anchor_end,
    )
    .map_err(|e| e.to_string())?;
    serde_json::to_value(fragment).map_err(|e| e.to_string())
}

pub fn sync_chat_relay(session_id: Option<String>) -> Result<u64, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    if let Some(id) = session_id {
        Ok(crate::chat_relay::sync_session_relay(Path::new(&storage), &id)? as u64)
    } else {
        Ok(crate::chat_relay::sync_all_group_sessions()? as u64)
    }
}

pub fn start_chat_relay_poller() {
    crate::chat_relay::start_relay_poller();
}

pub fn list_chat_branch_types() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let types = crate::chat_ontology::list_branch_types(Path::new(&storage));
    serde_json::to_value(types).map_err(|e| e.to_string())
}

pub fn classify_chat_branch(
    anchor_text: String,
    reply_text: String,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let c = crate::chat_ontology::classify_branch(Path::new(&storage), &anchor_text, &reply_text);
    serde_json::to_value(c).map_err(|e| e.to_string())
}

pub fn toggle_chat_reaction(
    session_id: String,
    message_lamport: u64,
    emoji: String,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let reactions = crate::chat_ontology::toggle_reaction(
        Path::new(&storage),
        &session_id,
        message_lamport,
        &emoji,
    )?;
    serde_json::to_value(reactions).map_err(|e| e.to_string())
}

pub fn list_chat_reactions(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let reactions = crate::chat_ontology::list_reactions(Path::new(&storage), &session_id)?;
    serde_json::to_value(reactions).map_err(|e| e.to_string())
}

pub fn wordnet_chat_ontology_status() -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let path = crate::chat_ontology::resolve_wordnet_q42(Path::new(&storage));
    Ok(serde_json::json!({
        "available": path.is_some(),
        "q42_path": path.as_ref().map(|p| p.to_string_lossy().to_string()),
        "lex_path": path.as_ref().and_then(|p| crate::chat_ontology::resolve_wordnet_lex(p).map(|l| l.to_string_lossy().to_string())),
    }))
}

pub fn default_chat_file_sharing(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let session = crate::chat_session::load_session(Path::new(&storage), &session_id)
        .map_err(|e| e.to_string())?;
    let sharing = crate::chat_files::default_sharing_for_session(session.meta.session_kind);
    serde_json::to_value(sharing).map_err(|e| e.to_string())
}

pub fn attach_chat_file(
    session_id: String,
    source_path: String,
    sharing_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let sharing: crate::chat_files::ChatFileSharing =
        serde_json::from_value(sharing_json).map_err(|e| e.to_string())?;
    let result = crate::chat_files::attach_chat_file(
        Path::new(&storage),
        &session_id,
        Path::new(&source_path),
        sharing,
    )?;
    serde_json::to_value(result).map_err(|e| e.to_string())
}

pub fn list_chat_files(session_id: String) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let files = crate::chat_files::list_chat_files(Path::new(&storage), &session_id, None)?;
    serde_json::to_value(files).map_err(|e| e.to_string())
}

pub fn set_chat_file_sharing(
    session_id: String,
    file_id: String,
    sharing_json: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let sharing: crate::chat_files::ChatFileSharing =
        serde_json::from_value(sharing_json).map_err(|e| e.to_string())?;
    let updated = crate::chat_files::set_chat_file_sharing(
        Path::new(&storage),
        &session_id,
        &file_id,
        sharing,
    )?;
    serde_json::to_value(updated).map_err(|e| e.to_string())
}

pub fn get_chat_file_local_path(
    session_id: String,
    file_id: String,
    variant: String,
) -> Result<String, String> {
    let state = crate::state::APP_STATE.get().unwrap();
    let storage = state.config.lock().unwrap().storage_path.clone();
    let path = crate::chat_files::resolve_chat_file_path(
        Path::new(&storage),
        &session_id,
        &file_id,
        &variant,
        None,
    )?;
    Ok(path.to_string_lossy().to_string())
}

pub fn parse_chat_file_preview(source_path: String) -> Result<serde_json::Value, String> {
    let path = Path::new(&source_path);
    if !path.is_file() {
        return Err(format!("File not found: {}", path.display()));
    }
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .and_then(|mut f| std::io::Read::read_to_end(&mut f, &mut bytes))
        .map_err(|e| e.to_string())?;
    let name = path.file_name().and_then(|s| s.to_str()).unwrap_or("file");
    let parsed = crate::chat_files::parse_document_bytes(name, &bytes);
    serde_json::to_value(parsed).map_err(|e| e.to_string())
}
