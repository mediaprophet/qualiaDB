//! HTTP relay inbox for group chat messages (daemon `/chat/publish` + `/chat/pull`).

#![cfg(not(target_arch = "wasm32"))]

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use serde::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::Filter;

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

pub fn chat_relay_routes(
    storage_path: String,
    vault: Arc<Mutex<crate::key_vault::KeyVault>>,
) -> impl Filter<Extract = impl warp::Reply, Error = warp::Rejection> + Clone {
    let storage_publish = storage_path.clone();
    let vault_publish = vault.clone();

    let publish = warp::path!("chat" / "publish")
        .and(warp::post())
        .and(warp::body::json())
        .and(warp::any().map(move || storage_publish.clone()))
        .and(warp::any().map(move || vault_publish.clone()))
        .and_then(
            |envelope: RelayEnvelope,
             storage: String,
             vault: Arc<Mutex<crate::key_vault::KeyVault>>| async move {
                if envelope.content.is_empty() || envelope.session_id.is_empty() {
                    return Ok::<_, std::convert::Infallible>(warp::reply::with_status(
                        warp::reply::json(&serde_json::json!({"error": "invalid envelope"})),
                        StatusCode::BAD_REQUEST,
                    ));
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
                                let vault = vault.lock().unwrap();
                                let key =
                                    vault.derive_key(&format!("relay:{}", envelope.author_did));
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
                                    return Ok(warp::reply::with_status(
                                        warp::reply::json(
                                            &serde_json::json!({"error": "signature invalid"}),
                                        ),
                                        StatusCode::UNAUTHORIZED,
                                    ));
                                }
                            }
                        }
                    }
                }

                match append_inbox(&storage, &envelope) {
                    Ok(()) => Ok(warp::reply::with_status(
                        warp::reply::json(
                            &serde_json::json!({"ok": true, "lamport": envelope.lamport}),
                        ),
                        StatusCode::OK,
                    )),
                    Err(e) => Ok(warp::reply::with_status(
                        warp::reply::json(&serde_json::json!({"error": e})),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )),
                }
            },
        );

    let pull = warp::path!("chat" / "pull")
        .and(warp::get())
        .and(warp::query::<std::collections::HashMap<String, String>>())
        .and(warp::any().map(move || storage_path.clone()))
        .and_then(
            |params: std::collections::HashMap<String, String>, storage: String| async move {
                let session_id = params.get("session_id").cloned().unwrap_or_default();
                let since = params
                    .get("since_lamport")
                    .and_then(|s| s.parse::<u64>().ok())
                    .unwrap_or(0);

                if session_id.is_empty() {
                    return Ok::<_, std::convert::Infallible>(warp::reply::with_status(
                        warp::reply::json(&serde_json::json!({"error": "session_id required"})),
                        StatusCode::BAD_REQUEST,
                    ));
                }

                match read_inbox(&storage, &session_id, since) {
                    Ok(resp) => Ok(warp::reply::with_status(
                        warp::reply::json(&resp),
                        StatusCode::OK,
                    )),
                    Err(e) => Ok(warp::reply::with_status(
                        warp::reply::json(&serde_json::json!({"error": e})),
                        StatusCode::INTERNAL_SERVER_ERROR,
                    )),
                }
            },
        );

    publish.or(pull)
}
