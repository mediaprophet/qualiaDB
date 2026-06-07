//! Flutter FRB — chat graph fragments and relay sync.

use flutter_rust_bridge::frb;
use qualia_client_core::api as core;

#[frb]
#[derive(Debug, Clone)]
pub struct ChatFragment {
    pub fragment_id: String,
    pub message_lamport: u64,
    pub anchor_start: u32,
    pub anchor_end: u32,
    pub anchor_text: String,
    pub author_did: Option<String>,
    pub author_name: Option<String>,
    pub created_at: u64,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatGraphEdge {
    pub child_fragment_id: String,
    pub parent_fragment_id: String,
    pub reply_message_lamport: u64,
    pub created_at: u64,
    pub branch_type_id: Option<String>,
    pub branch_label: Option<String>,
    pub branch_emoji: Option<String>,
    pub wordnet_grounding_hash: Option<String>,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatBranchType {
    pub id: String,
    pub label: String,
    pub emoji: String,
    pub description: String,
    pub wordnet_grounding_hash: Option<String>,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatReaction {
    pub message_lamport: u64,
    pub emoji: String,
    pub author_did: String,
    pub author_name: Option<String>,
    pub created_at: u64,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatGraphView {
    pub fragments: Vec<ChatFragment>,
    pub edges: Vec<ChatGraphEdge>,
}

#[frb]
pub fn get_chat_graph(session_id: String) -> Result<ChatGraphView, String> {
    let json = core::get_chat_graph(session_id)?;
    let fragments = json["fragments"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|f| ChatFragment {
            fragment_id: f["fragment_id"].as_str().unwrap_or_default().to_string(),
            message_lamport: f["message_lamport"].as_u64().unwrap_or(0),
            anchor_start: f["anchor_start"].as_u64().unwrap_or(0) as u32,
            anchor_end: f["anchor_end"].as_u64().unwrap_or(0) as u32,
            anchor_text: f["anchor_text"].as_str().unwrap_or_default().to_string(),
            author_did: f["author_did"].as_str().map(|s| s.to_string()),
            author_name: f["author_name"].as_str().map(|s| s.to_string()),
            created_at: f["created_at"].as_u64().unwrap_or(0),
        })
        .collect();
    let edges = json["edges"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|e| ChatGraphEdge {
            child_fragment_id: e["child_fragment_id"].as_str().unwrap_or_default().to_string(),
            parent_fragment_id: e["parent_fragment_id"].as_str().unwrap_or_default().to_string(),
            reply_message_lamport: e["reply_message_lamport"].as_u64().unwrap_or(0),
            created_at: e["created_at"].as_u64().unwrap_or(0),
            branch_type_id: e["branch_type_id"].as_str().map(|s| s.to_string()),
            branch_label: e["branch_label"].as_str().map(|s| s.to_string()),
            branch_emoji: e["branch_emoji"].as_str().map(|s| s.to_string()),
            wordnet_grounding_hash: e["wordnet_grounding_hash"].as_str().map(|s| s.to_string()),
        })
        .collect();
    Ok(ChatGraphView { fragments, edges })
}

#[frb]
pub fn create_chat_fragment(
    session_id: String,
    message_lamport: u64,
    anchor_start: u32,
    anchor_end: u32,
) -> Result<ChatFragment, String> {
    let json = core::create_chat_fragment(session_id, message_lamport, anchor_start, anchor_end)?;
    Ok(ChatFragment {
        fragment_id: json["fragment_id"].as_str().unwrap_or_default().to_string(),
        message_lamport: json["message_lamport"].as_u64().unwrap_or(0),
        anchor_start: json["anchor_start"].as_u64().unwrap_or(0) as u32,
        anchor_end: json["anchor_end"].as_u64().unwrap_or(0) as u32,
        anchor_text: json["anchor_text"].as_str().unwrap_or_default().to_string(),
        author_did: json["author_did"].as_str().map(|s| s.to_string()),
        author_name: json["author_name"].as_str().map(|s| s.to_string()),
        created_at: json["created_at"].as_u64().unwrap_or(0),
    })
}

#[frb]
pub fn append_chat_message_reply(
    session_id: String,
    role: String,
    content: String,
    reply_to_fragment: Option<String>,
    branch_type_id: Option<String>,
) -> Result<u64, String> {
    core::append_chat_message_reply(session_id, role, content, reply_to_fragment, branch_type_id)
}

#[frb]
pub fn sync_chat_relay(session_id: Option<String>) -> Result<u64, String> {
    core::sync_chat_relay(session_id)
}

#[frb]
pub fn list_chat_branch_types() -> Result<Vec<ChatBranchType>, String> {
    let json = core::list_chat_branch_types()?;
    let arr = json.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|t| ChatBranchType {
            id: t["id"].as_str().unwrap_or_default().to_string(),
            label: t["label"].as_str().unwrap_or_default().to_string(),
            emoji: t["emoji"].as_str().unwrap_or_default().to_string(),
            description: t["description"].as_str().unwrap_or_default().to_string(),
            wordnet_grounding_hash: t["wordnet_grounding_hash"].as_str().map(|s| s.to_string()),
        })
        .collect())
}

#[frb]
pub fn toggle_chat_reaction(
    session_id: String,
    message_lamport: u64,
    emoji: String,
) -> Result<Vec<ChatReaction>, String> {
    let json = core::toggle_chat_reaction(session_id, message_lamport, emoji)?;
    parse_reactions(json)
}

#[frb]
pub fn list_chat_reactions(session_id: String) -> Result<Vec<ChatReaction>, String> {
    let json = core::list_chat_reactions(session_id)?;
    parse_reactions(json)
}

#[frb]
pub fn wordnet_chat_ontology_status() -> Result<String, String> {
    let json = core::wordnet_chat_ontology_status()?;
    serde_json::to_string(&json).map_err(|e| e.to_string())
}

fn parse_reactions(json: serde_json::Value) -> Result<Vec<ChatReaction>, String> {
    let arr = json.as_array().cloned().unwrap_or_default();
    Ok(arr
        .into_iter()
        .map(|r| ChatReaction {
            message_lamport: r["message_lamport"].as_u64().unwrap_or(0),
            emoji: r["emoji"].as_str().unwrap_or_default().to_string(),
            author_did: r["author_did"].as_str().unwrap_or_default().to_string(),
            author_name: r["author_name"].as_str().map(|s| s.to_string()),
            created_at: r["created_at"].as_u64().unwrap_or(0),
        })
        .collect())
}
