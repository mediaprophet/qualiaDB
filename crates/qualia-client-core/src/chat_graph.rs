//! Chat graph — selectable fragments and reply edges forming a DAG off the linear chat.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use qualia_core_db::{q_hash, wal::WriteAheadLog, QualiaQuin};
use serde::{Deserialize, Serialize};

use crate::chat_session::{ChatError, Role};

const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;
const LAMPORT_SHIFT: u32 = 32;
const LAMPORT_MASK: u64 = 0x1FFF_FFFF;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGraphEdge {
    pub child_fragment_id: String,
    pub parent_fragment_id: String,
    pub reply_message_lamport: u64,
    pub created_at: u64,
    #[serde(default)]
    pub branch_type_id: Option<String>,
    #[serde(default)]
    pub branch_label: Option<String>,
    #[serde(default)]
    pub branch_emoji: Option<String>,
    #[serde(default)]
    pub wordnet_grounding_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatGraphSnapshot {
    pub fragments: Vec<ChatFragment>,
    pub edges: Vec<ChatGraphEdge>,
}

fn fragments_path(storage_root: &Path, session_id: &str) -> PathBuf {
    storage_root
        .join("Chats")
        .join(session_id)
        .join("fragments.jsonl")
}

fn edges_path(storage_root: &Path, session_id: &str) -> PathBuf {
    storage_root
        .join("Chats")
        .join(session_id)
        .join("graph_edges.jsonl")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn fragment_id_for_span(session_id: &str, lamport: u64, start: u32, end: u32) -> String {
    let raw = q_hash(&format!("frag:{session_id}:{lamport}:{start}:{end}"));
    format!("{raw:016x}")
}

fn session_subject_hash(session_id: &str) -> u64 {
    q_hash(&format!("chat:session:{session_id}"))
}

fn build_fragment_quin(session_id: &str, fragment_id_hex: &str, lamport: u64) -> QualiaQuin {
    let subject = session_subject_hash(session_id);
    let predicate = q_hash("chat:hasFragment");
    let object = u64::from_str_radix(fragment_id_hex, 16).unwrap_or(0) & OBJECT_HASH_MASK;
    let context = q_hash(&format!("msg:{lamport}"));
    let metadata = (lamport & LAMPORT_MASK) << LAMPORT_SHIFT;
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

fn build_reply_edge_quin(
    session_id: &str,
    child_fragment_id_hex: &str,
    parent_fragment_id_hex: &str,
    reply_lamport: u64,
) -> QualiaQuin {
    let subject = u64::from_str_radix(child_fragment_id_hex, 16).unwrap_or(0) & OBJECT_HASH_MASK;
    let predicate = q_hash("chat:repliesTo");
    let object = u64::from_str_radix(parent_fragment_id_hex, 16).unwrap_or(0) & OBJECT_HASH_MASK;
    let context = session_subject_hash(session_id);
    let metadata = (reply_lamport & LAMPORT_MASK) << LAMPORT_SHIFT;
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

fn append_jsonl<T: Serialize>(path: &Path, row: &T) -> Result<(), ChatError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(row)?)?;
    Ok(())
}

fn read_fragments(path: &Path) -> Result<Vec<ChatFragment>, ChatError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(&line)?);
    }
    Ok(out)
}

fn read_edges(path: &Path) -> Result<Vec<ChatGraphEdge>, ChatError> {
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        out.push(serde_json::from_str(&line)?);
    }
    Ok(out)
}

pub fn load_graph(storage_root: &Path, session_id: &str) -> Result<ChatGraphSnapshot, ChatError> {
    Ok(ChatGraphSnapshot {
        fragments: read_fragments(&fragments_path(storage_root, session_id))?,
        edges: read_edges(&edges_path(storage_root, session_id))?,
    })
}

pub fn create_fragment_from_selection(
    storage_root: &Path,
    session_id: &str,
    message_lamport: u64,
    message_content: &str,
    anchor_start: u32,
    anchor_end: u32,
) -> Result<ChatFragment, ChatError> {
    let start = anchor_start.min(message_content.len() as u32);
    let end = anchor_end.max(start).min(message_content.len() as u32);
    let anchor_text = message_content[start as usize..end as usize]
        .trim()
        .to_string();
    if anchor_text.is_empty() {
        return Err(ChatError::InvalidSession(
            "Selected fragment is empty.".to_string(),
        ));
    }

    let fragment_id = fragment_id_for_span(session_id, message_lamport, start, end);
    let profile = crate::user_profile::load_profile();
    let fragment = ChatFragment {
        fragment_id: fragment_id.clone(),
        message_lamport,
        anchor_start: start,
        anchor_end: end,
        anchor_text,
        author_did: profile
            .sharing
            .share_public_did
            .then(|| profile.public_did.clone()),
        author_name: profile
            .sharing
            .share_display_name
            .then(|| profile.display_name.clone()),
        created_at: unix_now(),
    };

    let frag_path = fragments_path(storage_root, session_id);
    let existing = read_fragments(&frag_path)?;
    if !existing.iter().any(|f| f.fragment_id == fragment_id) {
        append_jsonl(&frag_path, &fragment)?;
        let wal_path = storage_root.join("Chats").join(session_id).join("chat.wal");
        let quin = build_fragment_quin(session_id, &fragment_id, message_lamport);
        let mut wal = WriteAheadLog::open(&wal_path)
            .map_err(|e| ChatError::Wal(format!("Cannot open chat.wal: {e}")))?;
        wal.append_mutation(&quin)
            .map_err(|e| ChatError::Wal(format!("fragment quin append failed: {e}")))?;
    }

    Ok(fragment)
}

pub fn link_reply_to_fragment(
    storage_root: &Path,
    session_id: &str,
    parent_fragment_id: &str,
    reply_message_lamport: u64,
    child_fragment_id: Option<&str>,
    anchor_text: Option<&str>,
    reply_text: Option<&str>,
    branch_type_override: Option<&str>,
) -> Result<ChatGraphEdge, ChatError> {
    let child_id = child_fragment_id
        .map(|s| s.to_string())
        .unwrap_or_else(|| fragment_id_for_span(session_id, reply_message_lamport, 0, 0));

    let classification = if let Some(override_id) = branch_type_override {
        crate::chat_ontology::list_branch_types(storage_root)
            .into_iter()
            .find(|t| t.id == override_id)
            .map(|t| crate::chat_ontology::BranchClassification {
                branch_type_id: t.id,
                label: t.label,
                emoji: t.emoji,
                confidence: 1.0,
                wordnet_grounding_hash: t.wordnet_grounding_hash,
            })
    } else {
        match (anchor_text, reply_text) {
            (Some(a), Some(r)) => Some(crate::chat_ontology::classify_branch(storage_root, a, r)),
            _ => None,
        }
    };

    let edge = ChatGraphEdge {
        child_fragment_id: child_id.clone(),
        parent_fragment_id: parent_fragment_id.to_string(),
        reply_message_lamport,
        created_at: unix_now(),
        branch_type_id: classification.as_ref().map(|c| c.branch_type_id.clone()),
        branch_label: classification.as_ref().map(|c| c.label.clone()),
        branch_emoji: classification.as_ref().map(|c| c.emoji.clone()),
        wordnet_grounding_hash: classification.and_then(|c| c.wordnet_grounding_hash),
    };

    append_jsonl(&edges_path(storage_root, session_id), &edge)?;

    let wal_path = storage_root.join("Chats").join(session_id).join("chat.wal");
    let quin = build_reply_edge_quin(
        session_id,
        &child_id,
        parent_fragment_id,
        reply_message_lamport,
    );
    let mut wal = WriteAheadLog::open(&wal_path)
        .map_err(|e| ChatError::Wal(format!("Cannot open chat.wal: {e}")))?;
    wal.append_mutation(&quin)
        .map_err(|e| ChatError::Wal(format!("reply edge quin append failed: {e}")))?;

    Ok(edge)
}

pub fn build_thread_context_block(
    storage_root: &Path,
    session_id: &str,
    target_fragment_id: &str,
    max_depth: usize,
) -> Result<String, ChatError> {
    let graph = load_graph(storage_root, session_id)?;
    let session = crate::chat_session::load_session(storage_root, session_id)?;

    let mut lines = vec!["[Chat graph thread context]".to_string()];
    let mut current = target_fragment_id.to_string();
    let mut depth = 0;

    while depth < max_depth {
        let fragment = graph.fragments.iter().find(|f| f.fragment_id == current);
        if let Some(f) = fragment {
            lines.push(format!(
                "fragment {} (msg #{}) anchor=\"{}\"",
                f.fragment_id, f.message_lamport, f.anchor_text
            ));
            if let Some(msg) = session
                .messages
                .iter()
                .find(|m| m.lamport == f.message_lamport)
            {
                let author = msg.author_name.as_deref().unwrap_or(match msg.role {
                    Role::User => "user",
                    Role::Agent => "agent",
                });
                lines.push(format!("  full_message[{author}]: {}", msg.content));
            }
        }

        let parent_edge = graph.edges.iter().find(|e| e.child_fragment_id == current);
        match parent_edge {
            Some(e) => {
                current = e.parent_fragment_id.clone();
                depth += 1;
            }
            None => break,
        }
    }

    let child_edges: Vec<_> = graph
        .edges
        .iter()
        .filter(|e| e.parent_fragment_id == target_fragment_id)
        .collect();
    if !child_edges.is_empty() {
        lines.push("direct_replies:".to_string());
        for e in child_edges {
            if let Some(reply_msg) = session
                .messages
                .iter()
                .find(|m| m.lamport == e.reply_message_lamport)
            {
                let branch = e.branch_emoji.as_deref().unwrap_or("💬");
                let label = e.branch_label.as_deref().unwrap_or("Comment");
                lines.push(format!(
                    "  → {branch} {label} msg #{}: {}",
                    e.reply_message_lamport, reply_msg.content
                ));
            }
        }
    }

    Ok(lines.join("\n"))
}

pub fn append_message_with_reply(
    storage_root: &Path,
    session_id: &str,
    role: Role,
    content: &str,
    reply_to_fragment_id: Option<&str>,
) -> Result<u64, ChatError> {
    crate::chat_session::append_message_with_options(
        storage_root,
        session_id,
        role,
        content,
        reply_to_fragment_id.map(|s| s.to_string()),
        None,
    )
}
