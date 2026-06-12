//! HTTP relay inbox for group chat messages (daemon `/chat/publish` + `/chat/pull`).

#![cfg(not(target_arch = "wasm32"))]

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayEnvelope {
    pub session_id: String,
    pub lamport: u64,
    pub role: String,
    pub content: String,
    pub author_did: String,
    pub author_name: Option<String>,
    pub reply_to_fragment: Option<String>,
    pub timestamp: u64,
    pub signature_hex: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RelayPullResponse {
    pub messages: Vec<RelayEnvelope>,
    pub latest_lamport: u64,
}

#[derive(Clone)]
pub struct ChatState {
    pub storage_path: String,
    pub vault: Arc<Mutex<crate::key_vault::KeyVault>>,
}

fn relay_root(storage_path: &str) -> PathBuf {
    PathBuf::from(storage_path).join("ChatRelay")
}

fn inbox_path(storage_path: &str, session_id: &str) -> PathBuf {
    relay_root(storage_path)
        .join(session_id)
        .join("inbox.jsonl")
}

fn append_inbox(storage_path: &str, envelope: &RelayEnvelope) -> Result<(), String> {
    let path = inbox_path(storage_path, &envelope.session_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let line = serde_json::to_string(envelope).map_err(|e| e.to_string())?;
    writeln!(file, "{line}").map_err(|e| e.to_string())
}

fn read_inbox(
    storage_path: &str,
    session_id: &str,
    since_lamport: u64,
) -> Result<RelayPullResponse, String> {
    let path = inbox_path(storage_path, session_id);
    if !path.is_file() {
        return Ok(RelayPullResponse {
            messages: Vec::new(),
            latest_lamport: since_lamport,
        });
    }

    let file = File::open(&path).map_err(|e| e.to_string())?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();
    let mut latest = since_lamport;

    for line in reader.lines() {
        let line = line.map_err(|e| e.to_string())?;
        if line.trim().is_empty() {
            continue;
        }
        let env: RelayEnvelope = serde_json::from_str(&line).map_err(|e| e.to_string())?;
        if env.lamport > since_lamport {
            messages.push(env.clone());
        }
        latest = latest.max(env.lamport);
    }

    messages.sort_by_key(|m| m.lamport);
    Ok(RelayPullResponse {
        messages,
        latest_lamport: latest,
    })
}

async fn publish_handler(
    State(state): State<ChatState>,
    Json(envelope): Json<RelayEnvelope>,
) -> impl IntoResponse {
    if envelope.content.is_empty() || envelope.session_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "invalid envelope"})),
        );
    }

    if !envelope.signature_hex.is_empty() {
        if let Ok(sig_bytes) = hex::decode(&envelope.signature_hex) {
            if sig_bytes.len() == 64 {
                let payload = serde_json::json!({
                    "session_id": envelope.session_id,
                    "lamport": envelope.lamport,
                    "role": envelope.role,
                    "content": envelope.content,
                    "author_did": envelope.author_did,
                    "author_name": envelope.author_name,
                    "reply_to_fragment": envelope.reply_to_fragment,
                    "timestamp": envelope.timestamp,
                });
                if let Ok(payload_str) = serde_json::to_string(&payload) {
                    let vault = state.vault.lock().unwrap();
                    let key = vault.derive_key(&format!("relay:{}", envelope.author_did));
                    let pk = ed25519_dalek::VerifyingKey::from(&key);
                    let mut sig_arr = [0u8; 64];
                    sig_arr.copy_from_slice(&sig_bytes);
                    if crate::key_vault::KeyVault::verify_signature(
                        pk.as_bytes(),
                        payload_str.as_bytes(),
                        &sig_arr,
                    )
                    .is_err()
                    {
                        return (
                            StatusCode::UNAUTHORIZED,
                            Json(serde_json::json!({"error": "signature invalid"})),
                        );
                    }
                }
            }
        }
    }

    match append_inbox(&state.storage_path, &envelope) {
        Ok(()) => (
            StatusCode::OK,
            Json(serde_json::json!({"ok": true, "lamport": envelope.lamport})),
        ),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})),
        ),
    }
}

async fn pull_handler(
    State(state): State<ChatState>,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let session_id = params.get("session_id").cloned().unwrap_or_default();
    let since = params
        .get("since_lamport")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    if session_id.is_empty() {
        return (
            StatusCode::BAD_REQUEST,
            Json(serde_json::json!({"error": "session_id required"})).into_response(),
        );
    }

    match read_inbox(&state.storage_path, &session_id, since) {
        Ok(resp) => (StatusCode::OK, Json(serde_json::json!(resp)).into_response()),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": e})).into_response(),
        ),
    }
}

pub fn chat_relay_routes(
    storage_path: String,
    vault: Arc<Mutex<crate::key_vault::KeyVault>>,
) -> Router {
    let state = ChatState { storage_path, vault };
    Router::new()
        .route("/publish", post(publish_handler))
        .route("/pull", get(pull_handler))
        .with_state(state)
}
