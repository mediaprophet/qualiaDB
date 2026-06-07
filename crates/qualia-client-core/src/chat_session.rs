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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SessionKind {
    #[default]
    Solo,
    Group,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatParticipant {
    pub did: String,
    pub display_name: String,
    pub actor_id: String,
    pub role: String,
    pub joined_at: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    User,
    Agent,
}

impl Role {
    pub fn as_str(self) -> &'static str {
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

/// Per-ontology scope summary compiled into the chat environment.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OntologyScopeSummary {
    pub id: String,
    pub name: String,
    pub quin_count: u64,
    pub q42_path: String,
}

/// Snapshot of chat inference scope and LLM-visible capabilities.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatEnvironment {
    pub session_id: String,
    pub active_model_profile_id: u64,
    pub ontology_ids: Vec<String>,
    pub prior_session_ids: Vec<String>,
    pub graph_scope_hashes: Vec<u64>,
    #[serde(default)]
    pub lexicon_prefixes: Vec<u64>,
    #[serde(default)]
    pub capability_briefing: String,
    #[serde(default)]
    pub model_id: Option<String>,
    #[serde(default)]
    pub model_modality: String,
    #[serde(default)]
    pub context_window: u32,
    #[serde(default)]
    pub engine_capabilities: Vec<String>,
    #[serde(default)]
    pub installed_qapps: Vec<String>,
    #[serde(default)]
    pub ontology_summaries: Vec<OntologyScopeSummary>,
    #[serde(default)]
    pub daemon_reachable: bool,
    #[serde(default)]
    pub session_kind: SessionKind,
    #[serde(default)]
    pub participants: Vec<ChatParticipant>,
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
    pub fn default_for_session(session_id: &str, storage_root: &Path) -> Self {
        let catalog = qualia_core_db::resource_catalog::load_default()
            .unwrap_or_else(|_| qualia_core_db::resource_catalog::ResourceCatalog::empty());
        crate::context_binding::compile_chat_environment(
            storage_root,
            &catalog,
            &crate::context_binding::ChatEnvironmentConfig {
                session_id: session_id.to_string(),
                ontology_ids: Vec::new(),
                prior_session_ids: Vec::new(),
                session_kind: SessionKind::Solo,
                participants: Vec::new(),
            },
        )
        .unwrap_or_else(|_| {
            let profile_id = active_model_profile_id();
            let session_scope = q_hash(&format!("chat:session:{session_id}"));
            Self {
                session_id: session_id.to_string(),
                active_model_profile_id: profile_id,
                ontology_ids: Vec::new(),
                prior_session_ids: Vec::new(),
                graph_scope_hashes: vec![session_scope],
                lexicon_prefixes: Vec::new(),
                capability_briefing: String::new(),
                model_id: None,
                model_modality: "text".to_string(),
                context_window: 4096,
                engine_capabilities: Vec::new(),
                installed_qapps: Vec::new(),
                ontology_summaries: Vec::new(),
                daemon_reachable: false,
                session_kind: SessionKind::Solo,
                participants: Vec::new(),
            }
        })
    }

    pub fn save_to_session_dir(&self, storage_root: &Path) -> Result<(), ChatError> {
        let path = session_dir(storage_root, &self.session_id).join("environment.json");
        fs::write(path, serde_json::to_string_pretty(self)?)?;
        Ok(())
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
    #[serde(default)]
    pub session_kind: SessionKind,
    #[serde(default)]
    pub participants: Vec<ChatParticipant>,
    #[serde(default)]
    pub owner_did: String,
    /// Stable DID for ontology / torrent sharing scoped to this chat session or group.
    #[serde(default)]
    pub session_did: String,
}

/// Target descriptor for ontology sharing UI (solo chats and group sessions).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSessionShareTarget {
    pub session_id: String,
    pub session_did: String,
    pub title: String,
    pub session_kind: SessionKind,
    pub participant_count: u64,
}

pub fn compile_session_did(session_id: &str, kind: SessionKind) -> String {
    let scope = match kind {
        SessionKind::Solo => "solo",
        SessionKind::Group => "group",
    };
    let digest = q_hash(&format!("qualia:chat:{scope}:{session_id}"));
    format!("did:qualia:chat:{scope}:{digest:016x}")
}

fn ensure_session_did(meta: &mut SessionMeta) -> String {
    if meta.session_did.is_empty() {
        meta.session_did = compile_session_did(&meta.id, meta.session_kind);
    }
    meta.session_did.clone()
}

fn persist_session_did_if_needed(storage_root: &Path, meta: &mut SessionMeta) -> Result<(), ChatError> {
    let before = meta.session_did.clone();
    let _ = ensure_session_did(meta);
    if meta.session_did != before {
        let path = session_meta_path(storage_root, &meta.id);
        fs::write(path, serde_json::to_string_pretty(meta)?)?;
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub lamport: u64,
    pub role: Role,
    pub content: String,
    pub timestamp: u64,
    pub content_hash: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_did: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub author_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reply_to_fragment: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// Human principal DID — agent messages are sub-agents of this participant.
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
pub struct ChatSessionSummary {
    pub id: String,
    pub title: String,
    pub created_at: u64,
    pub updated_at: u64,
    pub message_count: u64,
    #[serde(default)]
    pub session_kind: SessionKind,
    #[serde(default)]
    pub participant_count: u64,
    #[serde(default)]
    pub session_did: String,
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

    let profile = crate::user_profile::load_profile();
    let owner_did = profile.public_did.clone();

    let session_did = compile_session_did(&id, SessionKind::Solo);
    let meta = SessionMeta {
        id: id.clone(),
        title: title.clone(),
        created_at: now,
        updated_at: now,
        message_count: 0,
        next_lamport: 1,
        environment_ref: "environment.json".to_string(),
        session_kind: SessionKind::Solo,
        participants: Vec::new(),
        owner_did,
        session_did,
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
    let _ = crate::chat_agents::ensure_local_agent_config(storage_root, &id);
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
        let mut meta: SessionMeta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
        persist_session_did_if_needed(storage_root, &mut meta)?;
        out.push(ChatSessionSummary {
            id: meta.id,
            title: meta.title,
            created_at: meta.created_at,
            updated_at: meta.updated_at,
            message_count: meta.message_count,
            session_kind: meta.session_kind,
            participant_count: meta.participants.len() as u64,
            session_did: meta.session_did,
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
    let mut meta: SessionMeta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
    persist_session_did_if_needed(storage_root, &mut meta)?;
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
    append_message_with_options(storage_root, id, role, content, None, None)
}

pub fn append_message_with_options(
    storage_root: &Path,
    id: &str,
    role: Role,
    content: &str,
    reply_to_fragment: Option<String>,
    source: Option<String>,
) -> Result<u64, ChatError> {
    append_message_with_author(
        storage_root,
        id,
        role,
        content,
        reply_to_fragment,
        source,
        None,
        None,
        None,
    )
}

pub fn append_message_with_author(
    storage_root: &Path,
    id: &str,
    role: Role,
    content: &str,
    reply_to_fragment: Option<String>,
    source: Option<String>,
    override_author_did: Option<String>,
    override_author_name: Option<String>,
    branch_type_override: Option<String>,
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

    let (author_did, author_name) =
        if override_author_did.is_some() || override_author_name.is_some() {
        (override_author_did, override_author_name)
    } else if meta.session_kind == SessionKind::Group && role == Role::User {
        let profile = crate::user_profile::load_profile();
        (
            Some(profile.public_did),
            profile
                .sharing
                .share_display_name
                .then(|| profile.display_name),
        )
    } else {
        (None, None)
    };

    let relay_ingest = source.as_ref().is_some_and(|s| s.starts_with("relay:"));

    let mut msg = ChatMessage {
        lamport,
        role,
        content: content.to_string(),
        timestamp: meta.updated_at,
        content_hash,
        author_did,
        author_name,
        reply_to_fragment: reply_to_fragment.clone(),
        source,
        sub_agent_of: None,
        agent_did: None,
        model_id: None,
        agent_backend: None,
        outcome_sharing: None,
    };

    if role == Role::Agent && !relay_ingest {
        let _ = crate::chat_agents::decorate_local_agent_message(storage_root, id, &mut msg);
    }

    let mut jsonl = OpenOptions::new()
        .create(true)
        .append(true)
        .open(messages_path(storage_root, id))?;
    writeln!(jsonl, "{}", serde_json::to_string(&msg)?)?;

    fs::write(meta_path, serde_json::to_string_pretty(&meta)?)?;

    if let Some(parent_id) = reply_to_fragment.as_deref() {
        let graph = crate::chat_graph::load_graph(storage_root, id).ok();
        let anchor = graph
            .as_ref()
            .and_then(|g| g.fragments.iter().find(|f| f.fragment_id == parent_id))
            .map(|f| f.anchor_text.as_str());
        let _ = crate::chat_graph::link_reply_to_fragment(
            storage_root,
            id,
            parent_id,
            lamport,
            None,
            anchor,
            Some(content),
            branch_type_override.as_deref(),
        );
    }

    if meta.session_kind == SessionKind::Group && !relay_ingest {
        if role != Role::Agent || crate::chat_agents::can_relay_agent_outcome(&msg, &meta.participants) {
            let _ = crate::chat_relay::publish_session_message(storage_root, id, lamport);
        }
    }

    set_last_session_id(id)?;
    Ok(lamport)
}

/// Ingest a relayed message with pre-set sub-agent metadata (skips local decoration / re-publish).
pub fn append_relay_message_with_agent_meta(
    storage_root: &Path,
    id: &str,
    role: Role,
    content: &str,
    reply_to_fragment: Option<String>,
    source: Option<String>,
    author_did: Option<String>,
    author_name: Option<String>,
    sub_agent_of: Option<String>,
    agent_did: Option<String>,
    model_id: Option<String>,
    agent_backend: Option<String>,
    outcome_sharing: Option<crate::chat_agents::OutcomeSharingPolicy>,
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
        author_did,
        author_name,
        reply_to_fragment,
        source,
        sub_agent_of,
        agent_did,
        model_id,
        agent_backend,
        outcome_sharing,
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

fn owner_participant(profile: &crate::user_profile::UserProfile) -> ChatParticipant {
    ChatParticipant {
        did: profile.public_did.clone(),
        display_name: profile.display_name.clone(),
        actor_id: "self".to_string(),
        role: "owner".to_string(),
        joined_at: unix_now(),
    }
}

fn contacts_for_dids(dids: &[String]) -> Vec<ChatParticipant> {
    let contacts = crate::social_connect::list_chat_contacts();
    let now = unix_now();
    dids.iter()
        .filter_map(|did| {
            contacts.iter().find(|c| c.did == *did).map(|c| ChatParticipant {
                did: c.did.clone(),
                display_name: c.display_name.clone(),
                actor_id: c.actor_id.clone(),
                role: "member".to_string(),
                joined_at: now,
            })
        })
        .collect()
}

fn sync_participants_to_environment(
    storage_root: &Path,
    id: &str,
    meta: &SessionMeta,
) -> Result<(), ChatError> {
    let env_path = environment_path(storage_root, id);
    let mut environment = if env_path.is_file() {
        serde_json::from_str(&fs::read_to_string(&env_path)?)?
    } else {
        ChatEnvironment::default_for_session(id, storage_root)
    };
    environment.session_kind = meta.session_kind;
    environment.participants = meta.participants.clone();
    fs::write(env_path, serde_json::to_string_pretty(&environment)?)?;
    Ok(())
}

pub fn create_group_session(
    storage_root: &Path,
    title: Option<String>,
    participant_dids: &[String],
) -> Result<String, ChatError> {
    let profile = crate::user_profile::load_profile();
    if !profile.sharing.allow_group_chat_invites {
        return Err(ChatError::InvalidSession(
            "Group chat invites are disabled in your profile.".to_string(),
        ));
    }

    let mut participants = vec![owner_participant(&profile)];
    participants.extend(contacts_for_dids(participant_dids));

    if participants.len() < 2 {
        return Err(ChatError::InvalidSession(
            "Select at least one friend to start a group chat.".to_string(),
        ));
    }

    fs::create_dir_all(chats_dir(storage_root))?;
    let id = new_session_id();
    let dir = session_dir(storage_root, &id);
    fs::create_dir_all(&dir)?;

    let now = unix_now();
    let title = title.unwrap_or_else(|| {
        let names: Vec<_> = participants
            .iter()
            .filter(|p| p.role != "owner")
            .map(|p| p.display_name.as_str())
            .take(3)
            .collect();
        if names.is_empty() {
            "Group chat".to_string()
        } else {
            format!("Group: {}", names.join(", "))
        }
    });

    let mut environment = ChatEnvironment::default_for_session(&id, storage_root);
    environment.session_kind = SessionKind::Group;
    environment.participants = participants.clone();

    let session_did = compile_session_did(&id, SessionKind::Group);
    let meta = SessionMeta {
        id: id.clone(),
        title: title.clone(),
        created_at: now,
        updated_at: now,
        message_count: 0,
        next_lamport: 1,
        environment_ref: "environment.json".to_string(),
        session_kind: SessionKind::Group,
        participants,
        owner_did: profile.public_did,
        session_did,
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
    let _ = crate::chat_agents::ensure_local_agent_config(storage_root, &id);
    Ok(id)
}

pub fn add_participant(
    storage_root: &Path,
    id: &str,
    participant_did: &str,
) -> Result<Vec<ChatParticipant>, ChatError> {
    let meta_path = session_meta_path(storage_root, id);
    if !meta_path.is_file() {
        return Err(ChatError::NotFound(id.to_string()));
    }
    let mut meta: SessionMeta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
    if meta.session_kind != SessionKind::Group {
        meta.session_kind = SessionKind::Group;
    }
    if meta.participants.iter().any(|p| p.did == participant_did) {
        return Ok(meta.participants.clone());
    }

    let added = contacts_for_dids(&[participant_did.to_string()]);
    let Some(participant) = added.into_iter().next() else {
        return Err(ChatError::InvalidSession(format!(
            "No contact found for DID {participant_did}. Add them via Profile → Add Friend first."
        )));
    };

    meta.participants.push(participant);
    meta.updated_at = unix_now();
    fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
    sync_participants_to_environment(storage_root, id, &meta)?;
    Ok(meta.participants.clone())
}

pub fn remove_participant(
    storage_root: &Path,
    id: &str,
    participant_did: &str,
) -> Result<Vec<ChatParticipant>, ChatError> {
    let meta_path = session_meta_path(storage_root, id);
    if !meta_path.is_file() {
        return Err(ChatError::NotFound(id.to_string()));
    }
    let mut meta: SessionMeta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
    if meta.owner_did == participant_did {
        return Err(ChatError::InvalidSession(
            "Cannot remove the session owner.".to_string(),
        ));
    }
    meta.participants.retain(|p| p.did != participant_did);
    meta.updated_at = unix_now();
    fs::write(&meta_path, serde_json::to_string_pretty(&meta)?)?;
    sync_participants_to_environment(storage_root, id, &meta)?;
    Ok(meta.participants.clone())
}

pub fn get_participants(storage_root: &Path, id: &str) -> Result<Vec<ChatParticipant>, ChatError> {
    let meta_path = session_meta_path(storage_root, id);
    if !meta_path.is_file() {
        return Err(ChatError::NotFound(id.to_string()));
    }
    let meta: SessionMeta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
    Ok(meta.participants.clone())
}

/// Sessions and groups that can be selected as share targets (each has a stable `session_did`).
pub fn list_session_share_targets(storage_root: &Path) -> Result<Vec<ChatSessionShareTarget>, ChatError> {
    let summaries = list_sessions(storage_root)?;
    Ok(summaries
        .into_iter()
        .map(|s| ChatSessionShareTarget {
            session_id: s.id,
            session_did: s.session_did,
            title: s.title,
            session_kind: s.session_kind,
            participant_count: s.participant_count,
        })
        .collect())
}

pub fn get_session_did(storage_root: &Path, session_id: &str) -> Result<String, ChatError> {
    let meta_path = session_meta_path(storage_root, session_id);
    if !meta_path.is_file() {
        return Err(ChatError::NotFound(session_id.to_string()));
    }
    let mut meta: SessionMeta = serde_json::from_str(&fs::read_to_string(&meta_path)?)?;
    persist_session_did_if_needed(storage_root, &mut meta)?;
    Ok(meta.session_did)
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
