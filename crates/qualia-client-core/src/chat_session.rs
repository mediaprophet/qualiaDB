//! Chat session persistence — WAL quins + JSON sidecar under `{storage}/Chats/`.

use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use qualia_core_db::{q_hash, wal::WriteAheadLog, QualiaQuin};
use serde::{Deserialize, Serialize};

const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;
const LAMPORT_SHIFT: u32 = 32;
const LAMPORT_MASK: u64 = 0x1FFF_FFFF;

#[derive(Debug)]
pub enum ChatError {
    NotFound(String),
    InvalidSession(String),
    Wal(String),
    Io(std::io::Error),
    Json(serde_json::Error),
    Compact(String),
}

impl std::fmt::Display for ChatError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChatError::NotFound(id) => write!(f, "Chat session not found: {id}"),
            ChatError::InvalidSession(msg) => write!(f, "Invalid session: {msg}"),
            ChatError::Wal(msg) => write!(f, "WAL error: {msg}"),
            ChatError::Io(e) => write!(f, "IO error: {e}"),
            ChatError::Json(e) => write!(f, "JSON error: {e}"),
            ChatError::Compact(msg) => write!(f, "Compaction error: {msg}"),
        }
    }
}

impl From<std::io::Error> for ChatError {
    fn from(e: std::io::Error) -> Self {
        ChatError::Io(e)
    }
}

impl From<serde_json::Error> for ChatError {
    fn from(e: serde_json::Error) -> Self {
        ChatError::Json(e)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Agent,
}

impl Role {
    fn as_str(self) -> &'static str {
        match self {
            Role::User => "user",
            Role::Agent => "agent",
        }
    }

    pub fn from_str(s: &str) -> Result<Self, ChatError> {
        match s.to_lowercase().as_str() {
            "user" => Ok(Role::User),
            "agent" => Ok(Role::Agent),
            _ => Err(ChatError::InvalidSession(format!("unknown role: {s}"))),
        }
    }
}

/// Snapshot of chat inference scope (extended by `context_binding` in S4).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEnvironment {
    pub session_id: String,
    pub active_model_profile_id: u64,
    pub ontology_ids: Vec<String>,
    pub prior_session_ids: Vec<String>,
    pub graph_scope_hashes: Vec<u64>,
}

fn active_model_profile_id() -> u64 {
    let path = crate::state::app_meta_dir().join("active_model.json");
    if let Ok(text) = fs::read_to_string(path) {
        if let Ok(record) = serde_json::from_str::<crate::model_lifecycle::ActiveModelRecord>(&text)
        {
            return record.profile_id;
        }
    }
    0
}

impl ChatEnvironment {
    pub fn default_for_session(session_id: &str, _storage_root: &Path) -> Self {
        let profile_id = active_model_profile_id();
        let session_scope = q_hash(&format!("chat:session:{session_id}"));
        Self {
            session_id: session_id.to_string(),
            active_model_profile_id: profile_id,
            ontology_ids: Vec::new(),
            prior_session_ids: Vec::new(),
            graph_scope_hashes: vec![session_scope],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionMeta {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: u64,
    pub next_lamport: u64,
    pub environment_ref: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub lamport: u64,
    pub role: Role,
    pub content: String,
    pub timestamp: u64,
    pub content_hash: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionSummary {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSession {
    pub meta: SessionMeta,
    pub environment: ChatEnvironment,
    pub messages: Vec<ChatMessage>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LastSessionPrefs {
    last_session_id: Option<String>,
}

pub fn chats_dir(storage_root: &Path) -> PathBuf {
    storage_root.join("Chats")
}

fn session_dir(storage_root: &Path, id: &str) -> PathBuf {
    chats_dir(storage_root).join(id)
}

fn session_meta_path(storage_root: &Path, id: &str) -> PathBuf {
    session_dir(storage_root, id).join("session.json")
}

fn environment_path(storage_root: &Path, id: &str) -> PathBuf {
    session_dir(storage_root, id).join("environment.json")
}

fn messages_path(storage_root: &Path, id: &str) -> PathBuf {
    session_dir(storage_root, id).join("messages.jsonl")
}

fn wal_path(storage_root: &Path, id: &str) -> PathBuf {
    session_dir(storage_root, id).join("chat.wal")
}

fn q42_path(storage_root: &Path, id: &str) -> PathBuf {
    session_dir(storage_root, id).join("chat.q42")
}

fn last_session_prefs_path() -> PathBuf {
    crate::state::app_meta_dir().join("chat_prefs.json")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn new_session_id() -> String {
    let ts = unix_now();
    let rnd: u64 = rand::random();
    format!("{ts}-{rnd:016x}")
}

fn session_subject_hash(session_id: &str) -> u64 {
    q_hash(&format!("chat:session:{session_id}"))
}

fn message_object_hash(lamport: u64) -> u64 {
    q_hash(&format!("msg:{lamport}")) & OBJECT_HASH_MASK
}

fn role_context_hash(role: Role) -> u64 {
    q_hash(&format!("chat:role:{}", role.as_str()))
}

fn build_message_quin(session_id: &str, role: Role, lamport: u64, content_hash: u64) -> QualiaQuin {
    let subject = session_subject_hash(session_id);
    let predicate = q_hash("chat:hasMessage");
    let object = message_object_hash(lamport);
    let context = role_context_hash(role);
    let metadata = (lamport & LAMPORT_MASK) << LAMPORT_SHIFT | (content_hash & 0xFFFF_FFFF);
    let parity = subject ^ predicate ^ object ^ context ^ metadata;
    QualiaQuin {
        subject,
        predicate,
        object,
        context,
        metadata,
        parity,
    }
}

fn write_quins_to_q42(quins: &[QualiaQuin], out_path: &Path) -> Result<u64, ChatError> {
    let mut out_file = File::create(out_path)?;
    let mut written_count = 0u64;
    let mut block_id: u64 = 0;
    let mut buffer = Vec::with_capacity(393_216);

    for quin in quins {
        buffer.extend_from_slice(bytemuck::bytes_of(quin));
        written_count += 1;
        if buffer.len() >= 393_216 {
            let compressed = lz4_flex::compress_prepend_size(&buffer);
            out_file.write_all(&block_id.to_le_bytes())?;
            out_file.write_all(&(compressed.len() as u32).to_le_bytes())?;
            out_file.write_all(&(buffer.len() as u32).to_le_bytes())?;
            out_file.write_all(&compressed)?;
            buffer.clear();
            block_id += 1;
        }
    }

    if !buffer.is_empty() {
        let compressed = lz4_flex::compress_prepend_size(&buffer);
        out_file.write_all(&block_id.to_le_bytes())?;
        out_file.write_all(&(compressed.len() as u32).to_le_bytes())?;
        out_file.write_all(&(buffer.len() as u32).to_le_bytes())?;
        out_file.write_all(&compressed)?;
    }

    Ok(written_count)
}

pub fn get_last_session_id() -> Option<String> {
    let path = last_session_prefs_path();
    let text = fs::read_to_string(path).ok()?;
    let prefs: LastSessionPrefs = serde_json::from_str(&text).ok()?;
    prefs.last_session_id
}

pub fn set_last_session_id(id: &str) -> Result<(), ChatError> {
    let path = last_session_prefs_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let prefs = LastSessionPrefs {
        last_session_id: Some(id.to_string()),
    };
    fs::write(path, serde_json::to_string_pretty(&prefs)?)?;
    Ok(())
}

pub fn create_session(
    storage_root: &Path,
    title: Option<String>,
    env: Option<ChatEnvironment>,
) -> Result<String, ChatError> {
    fs::create_dir_all(chats_dir(storage_root))?;
    let id = new_session_id();
    let dir = session_dir(storage_root, &id);
    fs::create_dir_all(&dir)?;

    let now = unix_now();
    let title = title.unwrap_or_else(|| "New chat".to_string());
    let environment = env.unwrap_or_else(|| ChatEnvironment::default_for_session(&id, storage_root));

    let meta = SessionMeta {
        id: id.clone(),
        title: title.clone(),
        created_at: now,
        updated_at: now,
        message_count: 0,
        next_lamport: 1,
        environment_ref: "environment.json".to_string(),
    };

    fs::write(session_meta_path(storage_root, &id), serde_json::to_string_pretty(&meta)?)?;
    fs::write(
        environment_path(storage_root, &id),
        serde_json::to_string_pretty(&environment)?,
    )?;
    fs::File::create(messages_path(storage_root, &id))?;
    let _ = WriteAheadLog::open(wal_path(storage_root, &id))
        .map_err(|e| ChatError::Wal(format!("Cannot create chat.wal: {e}")))?;

    set_last_session_id(&id)?;
    Ok(id)
}

pub fn list_sessions(storage_root: &Path) -> Result<Vec<ChatSessionSummary>, ChatError> {
    let root = chats_dir(storage_root);
    if !root.is_dir() {
        return Ok(Vec::new());
    }

    let mut out = Vec::new();
    for entry in fs::read_dir(&root)?.filter_map(Result::ok) {
        if !entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            continue;
        }
        let id = entry.file_name().to_string_lossy().into_owned();
        let meta_path = session_meta_path(storage_root, &id);
        if !meta_path.is_file() {
            continue;
        }
        let meta: SessionMeta = serde_json::from_str(&fs::read_to_string(meta_path)?)?;
        out.push(ChatSessionSummary {
            id: meta.id,
            title: meta.title,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            message_count: meta.message_count,
        });
    }

    out.sort_by(|a, b| b.updated_at.cmp(&a.updated_at));
    Ok(out)
}

fn read_messages_jsonl(path: &Path) -> Result<Vec<ChatMessage>, ChatError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut messages = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        messages.push(serde_json::from_str(&line)?);
    }
    Ok(messages)
}

pub fn load_session(storage_root: &Path, id: &str) -> Result<ChatSession, ChatError> {
    let meta_path = session_meta_path(storage_root, id);
    if !meta_path.is_file() {
        return Err(ChatError::NotFound(id.to_string()));
    }
    let meta: SessionMeta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
    let env_path = environment_path(storage_root, id);
    let environment = if env_path.is_file() {
        serde_json::from_str(&fs::read_to_string(env_path)?)?
    } else {
        ChatEnvironment::default_for_session(id, storage_root)
    };
    let messages = read_messages_jsonl(&messages_path(storage_root, id))?;
    Ok(ChatSession {
        meta,
        environment,
        messages,
    })
}

pub fn append_message(
    storage_root: &Path,
    id: &str,
    role: Role,
    content: &str,
) -> Result<u64, ChatError> {
    let meta_path = session_meta_path(storage_root, id);
    if !meta_path.is_file() {
        return Err(ChatError::NotFound(id.to_string()));
    }

    let mut meta: SessionMeta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
    let lamport = meta.next_lamport;
    meta.next_lamport += 1;
    meta.message_count += 1;
    meta.updated_at = unix_now();

    let content_hash = q_hash(content) & OBJECT_HASH_MASK;
    let quin = build_message_quin(id, role, lamport, content_hash);

    let mut wal = WriteAheadLog::open(wal_path(storage_root, id))
        .map_err(|e| ChatError::Wal(format!("Cannot open chat.wal: {e}")))?;
    wal.append_mutation(&quin)
        .map_err(|e| ChatError::Wal(format!("append failed: {e}")))?;

    let msg = ChatMessage {
        lamport,
        role,
        content: content.to_string(),
        timestamp: meta.updated_at,
        content_hash,
    };

    let mut jsonl = OpenOptions::new()
        .create(true)
        .append(true)
        .open(messages_path(storage_root, id))?;
    writeln!(jsonl, "{}", serde_json::to_string(&msg)?)?;

    fs::write(meta_path, serde_json::to_string_pretty(&meta)?)?;
    set_last_session_id(id)?;
    Ok(lamport)
}

pub fn compact_session_to_q42(storage_root: &Path, id: &str) -> Result<PathBuf, ChatError> {
    let dir = session_dir(storage_root, id);
    if !dir.is_dir() {
        return Err(ChatError::NotFound(id.to_string()));
    }

    let wal_file = wal_path(storage_root, id);
    let mut wal = WriteAheadLog::open(&wal_file)
        .map_err(|e| ChatError::Wal(format!("Cannot open chat.wal: {e}")))?;
    let quins = wal
        .recover()
        .map_err(|e| ChatError::Compact(format!("WAL recover failed: {e}")))?;

    if quins.is_empty() {
        return Err(ChatError::Compact("No quins in session WAL".to_string()));
    }

    let out = q42_path(storage_root, id);
    let count = write_quins_to_q42(&quins, &out)?;
    if count == 0 {
        return Err(ChatError::Compact("Wrote zero quins".to_string()));
    }
    Ok(out)
}

pub fn delete_session(storage_root: &Path, id: &str) -> Result<(), ChatError> {
    let dir = session_dir(storage_root, id);
    if !dir.is_dir() {
        return Err(ChatError::NotFound(id.to_string()));
    }
    fs::remove_dir_all(&dir)?;

    if get_last_session_id().as_deref() == Some(id) {
        let path = last_session_prefs_path();
        let prefs = LastSessionPrefs {
            last_session_id: None,
        };
        let _ = fs::write(path, serde_json::to_string_pretty(&prefs)?);
    }
    Ok(())
}

pub fn rename_session(storage_root: &Path, id: &str, title: &str) -> Result<(), ChatError> {
    let meta_path = session_meta_path(storage_root, id);
    if !meta_path.is_file() {
        return Err(ChatError::NotFound(id.to_string()));
    }
    let mut meta: SessionMeta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
    meta.title = title.trim().to_string();
    meta.updated_at = unix_now();
    fs::write(meta_path, serde_json::to_string_pretty(&meta)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_storage() -> PathBuf {
        let mut dir = env::temp_dir();
        dir.push(format!("qualia-chat-test-{}", rand::random::<u32>()));
        dir
    }

    #[test]
    fn create_list_append_load_delete_roundtrip() {
        let storage = temp_storage();
        let id = create_session(&storage, Some("Test".into()), None).unwrap();
        assert_eq!(get_last_session_id().as_deref(), Some(id.as_str()));
        let sessions = list_sessions(&storage).unwrap();
        assert_eq!(sessions.len(), 1);
        assert_eq!(sessions[0].id, id);

        let lamport = append_message(&storage, &id, Role::User, "hello").unwrap();
        assert_eq!(lamport, 1);
        append_message(&storage, &id, Role::Agent, "hi there").unwrap();

        let session = load_session(&storage, &id).unwrap();
        assert_eq!(session.messages.len(), 2);
        assert_eq!(session.messages[0].content, "hello");

        let q42 = compact_session_to_q42(&storage, &id).unwrap();
        assert!(q42.is_file());

        delete_session(&storage, &id).unwrap();
        assert!(load_session(&storage, &id).is_err());
        let _ = fs::remove_dir_all(&storage);
    }

}
