//! Flutter FRB — chat session persistence.

use flutter_rust_bridge::frb;
use qualia_client_core::api as core;

#[frb]
#[derive(Debug, Clone)]
pub struct ChatSessionSummary {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: u64,
    pub session_kind: String,
    pub participant_count: u64,
    pub session_did: String,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatSessionShareTarget {
    pub session_id: String,
    pub session_did: String,
    pub title: String,
    pub session_kind: String,
    pub participant_count: u64,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatParticipant {
    pub did: String,
    pub display_name: String,
    pub actor_id: String,
    pub role: String,
    pub joined_at: u64,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub lamport: u64,
    pub role: String,
    pub content: String,
    pub timestamp: u64,
    pub author_name: Option<String>,
    pub author_did: Option<String>,
    pub reply_to_fragment: Option<String>,
    pub sub_agent_of: Option<String>,
    pub agent_did: Option<String>,
    pub author_display: Option<String>,
    pub model_id: Option<String>,
}

#[frb]
pub fn create_chat_session(title: Option<String>) -> Result<String, String> {
    core::create_chat_session(title)
}

#[frb]
pub fn ensure_chat_session() -> Result<String, String> {
    core::ensure_chat_session()
}

#[frb]
pub fn list_chat_sessions() -> Result<Vec<ChatSessionSummary>, String> {
    let json = core::list_chat_sessions()?;
    let raw: Vec<serde_json::Value> = serde_json::from_value(json).map_err(|e| e.to_string())?;
    raw.into_iter()
        .map(|v| {
            Ok(ChatSessionSummary {
                id: v["id"].as_str().unwrap_or_default().to_string(),
                title: v["title"].as_str().unwrap_or_default().to_string(),
                created_at: v["created_at"].as_u64().unwrap_or(0),
                updated_at: v["updated_at"].as_u64().unwrap_or(0),
                message_count: v["message_count"].as_u64().unwrap_or(0),
                session_kind: v["session_kind"].as_str().unwrap_or("solo").to_string(),
                participant_count: v["participant_count"].as_u64().unwrap_or(0),
                session_did: v["session_did"].as_str().unwrap_or_default().to_string(),
            })
        })
        .collect()
}

#[frb]
pub fn list_chat_session_share_targets() -> Result<Vec<ChatSessionShareTarget>, String> {
    let json = core::list_chat_session_share_targets()?;
    let arr = json.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|v| ChatSessionShareTarget {
            session_id: v["session_id"].as_str().unwrap_or_default().to_string(),
            session_did: v["session_did"].as_str().unwrap_or_default().to_string(),
            title: v["title"].as_str().unwrap_or_default().to_string(),
            session_kind: v["session_kind"]
                .as_str()
                .or_else(|| v["kind"].as_str())
                .unwrap_or("solo")
                .to_string(),
            participant_count: v["participant_count"].as_u64().unwrap_or(0),
        })
        .collect())
}

#[frb]
pub fn get_chat_session_did(session_id: String) -> Result<String, String> {
    core::get_chat_session_did(session_id)
}

#[frb]
pub fn load_chat_session_messages(id: String) -> Result<Vec<ChatMessage>, String> {
    let json = core::load_chat_session(id)?;
    let messages = json["messages"]
        .as_array()
        .cloned()
        .unwrap_or_default();
    messages
        .into_iter()
        .map(|m| {
            Ok(ChatMessage {
                lamport: m["lamport"].as_u64().unwrap_or(0),
                role: m["role"].as_str().unwrap_or("user").to_string(),
                content: m["content"].as_str().unwrap_or_default().to_string(),
                timestamp: m["timestamp"].as_u64().unwrap_or(0),
                author_name: m["author_name"].as_str().map(|s| s.to_string()),
                author_did: m["author_did"].as_str().map(|s| s.to_string()),
                reply_to_fragment: m["reply_to_fragment"].as_str().map(|s| s.to_string()),
                sub_agent_of: m["sub_agent_of"].as_str().map(|s| s.to_string()),
                agent_did: m["agent_did"].as_str().map(|s| s.to_string()),
                author_display: m["author_name"].as_str().map(|s| s.to_string()),
                model_id: m["model_id"].as_str().map(|s| s.to_string()),
            })
        })
        .collect()
}

#[frb]
pub fn load_chat_session_title(id: String) -> Result<String, String> {
    let json = core::load_chat_session(id)?;
    Ok(json["meta"]["title"]
        .as_str()
        .unwrap_or("Chat")
        .to_string())
}

#[frb]
pub fn append_chat_message(session_id: String, role: String, content: String) -> Result<u64, String> {
    core::append_chat_message(session_id, role, content)
}

#[frb]
pub fn delete_chat_session(session_id: String) -> Result<(), String> {
    core::delete_chat_session(session_id)
}

#[frb]
pub fn rename_chat_session(session_id: String, title: String) -> Result<(), String> {
    core::rename_chat_session(session_id, title)
}

#[frb]
pub fn compact_chat_session(session_id: String) -> Result<String, String> {
    core::compact_chat_session(session_id)
}

#[frb]
pub fn get_last_chat_session_id() -> Option<String> {
    core::get_last_chat_session_id()
}

#[frb]
pub fn set_last_chat_session_id(session_id: String) -> Result<(), String> {
    core::set_last_chat_session_id(session_id)
}

#[frb]
pub fn compile_session_environment(session_id: String) -> Result<String, String> {
    let val = core::compile_session_environment(session_id)?;
    serde_json::to_string(&val).map_err(|e| e.to_string())
}

#[frb]
pub fn update_session_environment(
    session_id: String,
    ontology_ids: Vec<String>,
    prior_session_ids: Vec<String>,
    graph_mutation: bool,
) -> Result<String, String> {
    let val = core::update_session_environment(
        session_id,
        ontology_ids,
        prior_session_ids,
        graph_mutation,
    )?;
    serde_json::to_string(&val).map_err(|e| e.to_string())
}

#[frb]
pub fn get_session_environment(session_id: String) -> Result<String, String> {
    let val = core::get_session_environment(session_id)?;
    serde_json::to_string(&val).map_err(|e| e.to_string())
}

#[frb]
pub fn list_installed_ontology_ids_for_chat() -> Vec<String> {
    core::list_installed_ontology_ids_for_chat()
}

#[frb]
pub fn create_group_chat_session(
    title: Option<String>,
    participant_dids: Vec<String>,
) -> Result<String, String> {
    core::create_group_chat_session(title, participant_dids)
}

#[frb]
pub fn get_chat_participants(session_id: String) -> Result<Vec<ChatParticipant>, String> {
    let json = core::get_chat_participants(session_id)?;
    let arr = json.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|p| ChatParticipant {
            did: p["did"].as_str().unwrap_or_default().to_string(),
            display_name: p["display_name"].as_str().unwrap_or_default().to_string(),
            actor_id: p["actor_id"].as_str().unwrap_or_default().to_string(),
            role: p["role"].as_str().unwrap_or("member").to_string(),
            joined_at: p["joined_at"].as_u64().unwrap_or(0),
        })
        .collect())
}

#[frb]
pub fn add_chat_participant(session_id: String, participant_did: String) -> Result<Vec<ChatParticipant>, String> {
    let json = core::add_chat_participant(session_id, participant_did)?;
    let arr = json.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|p| ChatParticipant {
            did: p["did"].as_str().unwrap_or_default().to_string(),
            display_name: p["display_name"].as_str().unwrap_or_default().to_string(),
            actor_id: p["actor_id"].as_str().unwrap_or_default().to_string(),
            role: p["role"].as_str().unwrap_or("member").to_string(),
            joined_at: p["joined_at"].as_u64().unwrap_or(0),
        })
        .collect())
}

#[frb]
pub fn remove_chat_participant(session_id: String, participant_did: String) -> Result<Vec<ChatParticipant>, String> {
    let json = core::remove_chat_participant(session_id, participant_did)?;
    let arr = json.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|p| ChatParticipant {
            did: p["did"].as_str().unwrap_or_default().to_string(),
            display_name: p["display_name"].as_str().unwrap_or_default().to_string(),
            actor_id: p["actor_id"].as_str().unwrap_or_default().to_string(),
            role: p["role"].as_str().unwrap_or("member").to_string(),
            joined_at: p["joined_at"].as_u64().unwrap_or(0),
        })
        .collect())
}
