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
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatMessage {
    pub lamport: u64,
    pub role: String,
    pub content: String,
    pub timestamp: u64,
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
            })
        })
        .collect()
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
