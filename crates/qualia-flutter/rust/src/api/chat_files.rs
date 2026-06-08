//! Flutter FRB — chat file attachments with sharing permissions.

use flutter_rust_bridge::frb;
use qualia_client_core::api as core;

#[frb]
#[derive(Debug, Clone)]
pub struct ChatFileSharing {
    pub visibility: String,
    pub allow_download: bool,
    pub allow_llm_context: bool,
    pub allow_relay_sync: bool,
    pub sensitivity_level: u8,
    pub allowed_dids: Vec<String>,
    pub expires_at: Option<u64>,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatFileRecord {
    pub file_id: String,
    pub original_name: String,
    pub mime_type: String,
    pub extension: String,
    pub sha256: String,
    pub byte_size: u64,
    pub page_count: Option<u32>,
    pub text_preview: String,
    pub author_did: String,
    pub author_name: Option<String>,
    pub message_lamport: Option<u64>,
    pub attached_at: u64,
    pub sharing: ChatFileSharing,
    pub parse_status: String,
    pub parse_error: Option<String>,
    pub sensitivity_level: u8,
    pub media_kind: String,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
    pub thumbnail_rel_path: Option<String>,
    pub vision_lexicon_id: Option<String>,
    pub vision_facet: Option<String>,
    pub vision_status: Option<String>,
}

#[frb]
#[derive(Debug, Clone)]
pub struct AttachChatFileResult {
    pub file: ChatFileRecord,
    pub message_lamport: u64,
}

#[frb]
#[derive(Debug, Clone)]
pub struct ChatFilePreview {
    pub mime_type: String,
    pub extension: String,
    pub page_count: Option<u32>,
    pub text_preview: String,
    pub parse_status: String,
    pub parse_error: Option<String>,
    pub media_kind: String,
    pub image_width: Option<u32>,
    pub image_height: Option<u32>,
}

fn parse_sharing(v: &serde_json::Value) -> ChatFileSharing {
    let visibility = v["visibility"].as_str().unwrap_or("session_participants");
    let inferred_sensitivity = match visibility {
        "owner_only" => 2,
        "session_participants" | "specific_dids" => 1,
        _ => 0,
    };
    ChatFileSharing {
        visibility: visibility.to_string(),
        allow_download: v["allow_download"].as_bool().unwrap_or(true),
        allow_llm_context: v["allow_llm_context"].as_bool().unwrap_or(true),
        allow_relay_sync: v["allow_relay_sync"].as_bool().unwrap_or(false),
        sensitivity_level: v["sensitivity_level"]
            .as_u64()
            .unwrap_or(inferred_sensitivity) as u8,
        allowed_dids: v["allowed_dids"]
            .as_array()
            .map(|a| {
                a.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default(),
        expires_at: v["expires_at"].as_u64(),
    }
}

fn parse_file_record(v: &serde_json::Value) -> ChatFileRecord {
    ChatFileRecord {
        file_id: v["file_id"].as_str().unwrap_or_default().to_string(),
        original_name: v["original_name"].as_str().unwrap_or_default().to_string(),
        mime_type: v["mime_type"].as_str().unwrap_or_default().to_string(),
        extension: v["extension"].as_str().unwrap_or_default().to_string(),
        sha256: v["sha256"].as_str().unwrap_or_default().to_string(),
        byte_size: v["byte_size"].as_u64().unwrap_or(0),
        page_count: v["page_count"].as_u64().map(|n| n as u32),
        text_preview: v["text_preview"].as_str().unwrap_or_default().to_string(),
        author_did: v["author_did"].as_str().unwrap_or_default().to_string(),
        author_name: v["author_name"].as_str().map(|s| s.to_string()),
        message_lamport: v["message_lamport"].as_u64(),
        attached_at: v["attached_at"].as_u64().unwrap_or(0),
        sharing: parse_sharing(&v["sharing"]),
        parse_status: v["parse_status"].as_str().unwrap_or("unknown").to_string(),
        parse_error: v["parse_error"].as_str().map(|s| s.to_string()),
        sensitivity_level: v["sensitivity_level"].as_u64().unwrap_or(0) as u8,
        media_kind: v["media_kind"].as_str().unwrap_or("document").to_string(),
        image_width: v["image_width"].as_u64().map(|n| n as u32),
        image_height: v["image_height"].as_u64().map(|n| n as u32),
        thumbnail_rel_path: v["thumbnail_rel_path"].as_str().map(|s| s.to_string()),
        vision_lexicon_id: v["vision_lexicon_id"].as_str().map(|s| s.to_string()),
        vision_facet: v["vision_facet"].as_str().map(|s| s.to_string()),
        vision_status: v["vision_status"].as_str().map(|s| s.to_string()),
    }
}

fn sharing_to_json(s: &ChatFileSharing) -> serde_json::Value {
    serde_json::json!({
        "visibility": s.visibility,
        "allow_download": s.allow_download,
        "allow_llm_context": s.allow_llm_context,
        "allow_relay_sync": s.allow_relay_sync,
        "sensitivity_level": s.sensitivity_level,
        "allowed_dids": s.allowed_dids,
        "expires_at": s.expires_at,
    })
}

#[frb]
pub fn default_chat_file_sharing(session_id: String) -> Result<ChatFileSharing, String> {
    let json = core::default_chat_file_sharing(session_id)?;
    Ok(parse_sharing(&json))
}

#[frb]
pub fn attach_chat_file(
    session_id: String,
    source_path: String,
    sharing: ChatFileSharing,
) -> Result<AttachChatFileResult, String> {
    let json = core::attach_chat_file(session_id, source_path, sharing_to_json(&sharing))?;
    Ok(AttachChatFileResult {
        file: parse_file_record(&json["file"]),
        message_lamport: json["message_lamport"].as_u64().unwrap_or(0),
    })
}

#[frb]
pub fn list_chat_files(session_id: String) -> Result<Vec<ChatFileRecord>, String> {
    let json = core::list_chat_files(session_id)?;
    Ok(json
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(parse_file_record)
        .collect())
}

#[frb]
pub fn set_chat_file_sharing(
    session_id: String,
    file_id: String,
    sharing: ChatFileSharing,
) -> Result<ChatFileRecord, String> {
    let json = core::set_chat_file_sharing(session_id, file_id, sharing_to_json(&sharing))?;
    Ok(parse_file_record(&json))
}

#[frb]
pub fn preview_chat_file(source_path: String) -> Result<ChatFilePreview, String> {
    let json = core::parse_chat_file_preview(source_path)?;
    let full = json["full_text"].as_str().unwrap_or("");
    let preview = if full.chars().count() > 400 {
        full.chars().take(400).collect::<String>() + "…"
    } else {
        full.to_string()
    };
    Ok(ChatFilePreview {
        mime_type: json["mime_type"].as_str().unwrap_or_default().to_string(),
        extension: json["extension"].as_str().unwrap_or_default().to_string(),
        page_count: json["page_count"].as_u64().map(|n| n as u32),
        text_preview: preview,
        parse_status: json["parse_status"].as_str().unwrap_or("unknown").to_string(),
        parse_error: json["parse_error"].as_str().map(|s| s.to_string()),
        media_kind: json["media_kind"].as_str().unwrap_or("document").to_string(),
        image_width: json["image_width"].as_u64().map(|n| n as u32),
        image_height: json["image_height"].as_u64().map(|n| n as u32),
    })
}

#[frb]
pub fn get_chat_file_local_path(
    session_id: String,
    file_id: String,
    variant: String,
) -> Result<String, String> {
    core::get_chat_file_local_path(session_id, file_id, variant)
}
