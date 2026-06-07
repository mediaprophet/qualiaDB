//! Group chat relay — publish to daemon inbox and pull from peer relay endpoints.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::OnceLock;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::chat_session::{ChatError, Role};

static RELAY_POLLER_STARTED: AtomicBool = AtomicBool::new(false);

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_agent_of: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_did: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent_backend: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub outcome_sharing: Option<crate::chat_agents::OutcomeSharingPolicy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RelayCursor {
    per_session: HashMap<String, u64>,
}

fn cursor_path() -> PathBuf {
    crate::state::app_meta_dir().join("relay_cursors.json")
}

fn load_cursors() -> RelayCursor {
    fs_read_json(cursor_path()).unwrap_or(RelayCursor {
        per_session: HashMap::new(),
    })
}

fn save_cursors(cursors: &RelayCursor) -> Result<(), String> {
    let path = cursor_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(cursors).map_err(|e| e.to_string())?;
    std::fs::write(path, text).map_err(|e| e.to_string())
}

fn fs_read_json<T: for<'de> Deserialize<'de>>(path: PathBuf) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn local_relay_base_url() -> String {
    let port = crate::api::get_active_daemon_port();
    if let Some(profile) = profile_relay_base() {
        if !profile.is_empty() {
            return profile;
        }
    }
    format!("http://127.0.0.1:{port}")
}

fn profile_relay_base() -> Option<String> {
    let profile = crate::user_profile::load_profile();
    profile.relay_base_url.clone()
}

fn sign_envelope(envelope: &mut RelayEnvelope) {
    let payload = serde_json::json!({
        "session_id": envelope.session_id,
        "lamport": envelope.lamport,
        "role": envelope.role,
        "content": envelope.content,
        "author_did": envelope.author_did,
        "author_name": envelope.author_name,
        "reply_to_fragment": envelope.reply_to_fragment,
        "timestamp": envelope.timestamp,
        "sub_agent_of": envelope.sub_agent_of,
        "agent_did": envelope.agent_did,
        "model_id": envelope.model_id,
        "agent_backend": envelope.agent_backend,
        "outcome_sharing": envelope.outcome_sharing,
    });
    let payload_str = serde_json::to_string(&payload).unwrap_or_default();
    let state = match crate::state::APP_STATE.get() {
        Some(s) => s,
        None => return,
    };
    let vault = state.key_vault.lock().unwrap();
    let key = vault.derive_key(&format!("relay:{}", envelope.author_did));
    let sig = vault.sign_payload(&key, payload_str.as_bytes());
    envelope.signature_hex = hex::encode(sig.to_bytes());
}

pub fn message_to_envelope(
    session_id: &str,
    msg: &crate::chat_session::ChatMessage,
) -> RelayEnvelope {
    let profile = crate::user_profile::load_profile();
    let mut envelope = RelayEnvelope {
        session_id: session_id.to_string(),
        lamport: msg.lamport,
        role: msg.role.as_str().to_string(),
        content: msg.content.clone(),
        author_did: msg
            .author_did
            .clone()
            .unwrap_or_else(|| profile.public_did.clone()),
        author_name: msg.author_name.clone(),
        reply_to_fragment: msg.reply_to_fragment.clone(),
        timestamp: msg.timestamp,
        signature_hex: String::new(),
        sub_agent_of: msg.sub_agent_of.clone(),
        agent_did: msg.agent_did.clone(),
        model_id: msg.model_id.clone(),
        agent_backend: msg.agent_backend.clone(),
        outcome_sharing: msg.outcome_sharing.clone(),
    };
    sign_envelope(&mut envelope);
    envelope
}

pub fn publish_session_message(
    storage_root: &Path,
    session_id: &str,
    lamport: u64,
) -> Result<(), String> {
    let session = crate::chat_session::load_session(storage_root, session_id)
        .map_err(|e| e.to_string())?;
    let msg = session
        .messages
        .iter()
        .find(|m| m.lamport == lamport)
        .ok_or_else(|| format!("message lamport {lamport} not found"))?;

    let envelope = message_to_envelope(session_id, msg);
    publish_envelope(&session.meta.participants, &envelope)
}

pub fn publish_envelope(
    participants: &[crate::chat_session::ChatParticipant],
    envelope: &RelayEnvelope,
) -> Result<(), String> {
    let local = local_relay_base_url();
    let mut endpoints: Vec<String> = vec![local.clone()];

    for p in participants {
        if let Some(contact) = crate::social_connect::find_contact_by_did(&p.did) {
            if let Some(url) = contact.relay_endpoint {
                if !url.is_empty() && !endpoints.contains(&url) {
                    endpoints.push(url);
                }
            }
        }
    }

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;

    let mut last_err = None;
    for base in endpoints {
        let url = format!("{}/chat/publish", base.trim_end_matches('/'));
        match client.post(&url).json(envelope).send() {
            Ok(resp) if resp.status().is_success() => {}
            Ok(resp) => {
                last_err = Some(format!("relay {url} returned {}", resp.status()));
            }
            Err(e) => {
                last_err = Some(format!("relay {url} failed: {e}"));
            }
        }
    }

    if let Some(err) = last_err {
        eprintln!("[chat_relay] {err}");
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct PullResponse {
    messages: Vec<RelayEnvelope>,
    latest_lamport: u64,
}

pub fn pull_from_relay(base_url: &str, session_id: &str, since_lamport: u64) -> Result<PullResponse, String> {
    let url = format!(
        "{}/chat/pull?session_id={}&since_lamport={}",
        base_url.trim_end_matches('/'),
        urlencoding_path(session_id),
        since_lamport
    );
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(|e| e.to_string())?;
    let resp = client.get(&url).send().map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("pull failed: {}", resp.status()));
    }
    resp.json::<PullResponse>().map_err(|e| e.to_string())
}

fn urlencoding_path(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            'A'..='Z' | 'a'..='z' | '0'..='9' | '-' | '_' | '.' | '~' => c.to_string(),
            _ => format!("%{:02X}", c as u8),
        })
        .collect()
}

fn ingest_remote_message(
    storage_root: &Path,
    session_id: &str,
    envelope: &RelayEnvelope,
) -> Result<bool, ChatError> {
    let profile = crate::user_profile::load_profile();
    if envelope.author_did == profile.public_did {
        return Ok(false);
    }

    let session = crate::chat_session::load_session(storage_root, session_id)?;
    if session.messages.iter().any(|m| {
        m.lamport == envelope.lamport
            && m.author_did.as_deref() == Some(envelope.author_did.as_str())
    }) {
        return Ok(false);
    }

    let role = Role::from_str(&envelope.role)?;

    if role == Role::Agent {
        let preview = crate::chat_session::ChatMessage {
            lamport: envelope.lamport,
            role,
            content: envelope.content.clone(),
            timestamp: envelope.timestamp,
            content_hash: 0,
            author_did: Some(envelope.author_did.clone()),
            author_name: envelope.author_name.clone(),
            reply_to_fragment: envelope.reply_to_fragment.clone(),
            source: Some(format!("relay:{}", envelope.author_did)),
            sub_agent_of: envelope.sub_agent_of.clone(),
            agent_did: envelope.agent_did.clone(),
            model_id: envelope.model_id.clone(),
            agent_backend: envelope.agent_backend.clone(),
            outcome_sharing: envelope.outcome_sharing.clone(),
        };
        crate::chat_agents::validate_ingested_agent_message(&preview, &session.meta.participants)
            .map_err(|e| ChatError::InvalidSession(e))?;
        if !crate::chat_agents::can_view_agent_outcome(
            &preview,
            &profile.public_did,
            &session.meta.participants,
        ) {
            return Ok(false);
        }
    }

    let lamport = crate::chat_session::append_relay_message_with_agent_meta(
        storage_root,
        session_id,
        role,
        &envelope.content,
        envelope.reply_to_fragment.clone(),
        Some(format!("relay:{}", envelope.author_did)),
        Some(envelope.author_did.clone()),
        envelope.author_name.clone(),
        envelope.sub_agent_of.clone(),
        envelope.agent_did.clone(),
        envelope.model_id.clone(),
        envelope.agent_backend.clone(),
        envelope.outcome_sharing.clone(),
    )?;
    let _ = lamport;
    notify_session_updated(session_id);

    Ok(true)
}

pub fn sync_session_relay(storage_root: &Path, session_id: &str) -> Result<usize, String> {
    let session = crate::chat_session::load_session(storage_root, session_id)
        .map_err(|e| e.to_string())?;

    let mut cursors = load_cursors();
    let since = *cursors.per_session.get(session_id).unwrap_or(&0);
    let mut ingested = 0usize;
    let mut latest = since;

    let mut endpoints = vec![local_relay_base_url()];
    for p in &session.meta.participants {
        if let Some(contact) = crate::social_connect::find_contact_by_did(&p.did) {
            if let Some(url) = contact.relay_endpoint {
                if !url.is_empty() && !endpoints.contains(&url) {
                    endpoints.push(url);
                }
            }
        }
    }

    for base in endpoints {
        let pull = match pull_from_relay(&base, session_id, since) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("[chat_relay] pull {base}: {e}");
                continue;
            }
        };
        latest = latest.max(pull.latest_lamport);
        for env in pull.messages {
            if ingest_remote_message(storage_root, session_id, &env).unwrap_or(false) {
                ingested += 1;
            }
        }
    }

    cursors.per_session.insert(session_id.to_string(), latest);
    save_cursors(&cursors)?;
    Ok(ingested)
}

pub fn sync_all_group_sessions() -> Result<usize, String> {
    let state = crate::state::APP_STATE.get().ok_or("APP_STATE not initialized")?;
    let storage = state.config.lock().unwrap().storage_path.clone();
    let storage_path = Path::new(&storage);

    let sessions = crate::chat_session::list_sessions(storage_path).map_err(|e| e.to_string())?;
    let mut total = 0usize;
    for summary in sessions {
        if summary.session_kind != crate::chat_session::SessionKind::Group {
            continue;
        }
        total += sync_session_relay(storage_path, &summary.id)?;
    }
    Ok(total)
}

pub fn start_relay_poller() {
    if RELAY_POLLER_STARTED.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(|| {
        loop {
            if let Err(e) = sync_all_group_sessions() {
                eprintln!("[chat_relay] poller: {e}");
            }
            std::thread::sleep(Duration::from_secs(4));
        }
    });
}

static RELAY_NOTIFY: OnceLock<std::sync::Mutex<Option<tokio::sync::broadcast::Sender<String>>>> = OnceLock::new();

pub fn relay_notify_channel() -> tokio::sync::broadcast::Sender<String> {
    RELAY_NOTIFY
        .get_or_init(|| {
            let (tx, _) = tokio::sync::broadcast::channel(64);
            std::sync::Mutex::new(Some(tx))
        })
        .lock()
        .unwrap()
        .clone()
        .expect("relay notify tx")
}

pub fn notify_session_updated(session_id: &str) {
    let _ = relay_notify_channel().send(session_id.to_string());
}
