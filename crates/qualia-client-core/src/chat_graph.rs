//! Chat graph — selectable fragments and reply edges forming a DAG off the linear chat.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use qualia_core_db::{q_hash, wal::WriteAheadLog, NQuin};
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

fn build_fragment_quin(session_id: &str, fragment_id_hex: &str, lamport: u64) -> NQuin {
    let subject = session_subject_hash(session_id);
    let predicate = q_hash("chat:hasFragment");
    let object = u64::from_str_radix(fragment_id_hex, 16).unwrap_or(0) & OBJECT_HASH_MASK;
    let context = q_hash(&format!("msg:{lamport}"));
    let metadata = (lamport & LAMPORT_MASK) << LAMPORT_SHIFT;
    let parity = subject ^ predicate ^ object ^ context ^ metadata;
    NQuin {
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
) -> NQuin {
    let subject = u64::from_str_radix(child_fragment_id_hex, 16).unwrap_or(0) & OBJECT_HASH_MASK;
    let predicate = q_hash("chat:repliesTo");
    let object = u64::from_str_radix(parent_fragment_id_hex, 16).unwrap_or(0) & OBJECT_HASH_MASK;
    let context = session_subject_hash(session_id);
    let metadata = (reply_lamport & LAMPORT_MASK) << LAMPORT_SHIFT;
    let parity = subject ^ predicate ^ object ^ context ^ metadata;
    NQuin {
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

/// Formats a thread context block with branch taxonomy and WordNet synset info.
///
/// Streaming counterpart of [`build_thread_context_block`]: walks the ancestor
/// chain of `target_fragment_id` up to `max_depth`, then the target's direct
/// replies, writing each entry directly into `writer` as
/// `[Branch: <label> / WordNet: 0x<hash>] anchor="<text>" author="<did>"`.
/// The caller owns the rendered output, so the function stays caller-buffered;
/// only the cold Tier-2 graph/session load allocates internally.
pub fn format_thread_context_to<W: std::fmt::Write>(
    storage_root: &Path,
    session_id: &str,
    target_fragment_id: &str,
    max_depth: usize,
    writer: &mut W,
) -> Result<(), ChatError> {
    let graph = load_graph(storage_root, session_id)?;
    let session = crate::chat_session::load_session(storage_root, session_id)?;

    writeln!(writer, "[Chat graph thread context]")?;

    // Ancestor chain: the edge with `child_fragment_id == current` carries the
    // dialectical branch classification that produced this fragment.
    let mut current = target_fragment_id.to_string();
    let mut depth = 0usize;
    while depth < max_depth {
        let fragment = graph.fragments.iter().find(|f| f.fragment_id == current);
        let Some(fragment) = fragment else { break };
        let edge = graph
            .edges
            .iter()
            .find(|e| e.child_fragment_id == fragment.fragment_id);
        let (label, wordnet) = edge
            .map(|e| {
                (
                    e.branch_label.as_deref().unwrap_or("Comment"),
                    e.wordnet_grounding_hash.as_deref().unwrap_or("none"),
                )
            })
            .unwrap_or(("Root", "none"));
        let author = fragment.author_did.as_deref().unwrap_or("unknown");
        writeln!(
            writer,
            "[Branch: {label} / WordNet: {wordnet}] anchor=\"{}\" author=\"{author}\"",
            fragment.anchor_text,
        )?;
        match edge {
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
        writeln!(writer, "direct_replies:")?;
        for e in child_edges {
            let label = e.branch_label.as_deref().unwrap_or("Comment");
            let wordnet = e.wordnet_grounding_hash.as_deref().unwrap_or("none");
            if let Some(reply_msg) = session
                .messages
                .iter()
                .find(|m| m.lamport == e.reply_message_lamport)
            {
                let author = reply_msg.author_name.as_deref().unwrap_or(match reply_msg.role {
                    Role::User => "user",
                    Role::Agent => "agent",
                });
                writeln!(
                    writer,
                    "  -> [Branch: {label} / WordNet: {wordnet}] anchor=\"{}\" author=\"{author}\"",
                    reply_msg.content,
                )?;
            }
        }
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    fn temp_storage() -> PathBuf {
        let mut dir = env::temp_dir();
        dir.push(format!("qualia-chatgraph-test-{}", rand::random::<u32>()));
        dir
    }

    #[test]
    fn thread_context_formats_branch_taxonomy_and_replies() {
        let storage = temp_storage();
        let session_id =
            crate::chat_session::create_session(&storage, Some("Graph".into()), None).unwrap();
        let lamport = crate::chat_session::append_message(
            &storage,
            &session_id,
            Role::User,
            "we should adopt the plan",
        )
        .unwrap();
        let reply_lamport = crate::chat_session::append_message(
            &storage,
            &session_id,
            Role::Agent,
            "I disagree with that step",
        )
        .unwrap();

        let frag = create_fragment_from_selection(
            &storage,
            &session_id,
            lamport,
            "we should adopt the plan",
            0,
            9,
        )
        .unwrap();
        let edge = link_reply_to_fragment(
            &storage,
            &session_id,
            &frag.fragment_id,
            reply_lamport,
            None,
            Some("adopt the plan"),
            Some("I disagree with that step"),
            Some("objection"),
        )
        .unwrap();
        assert_eq!(edge.branch_label.as_deref(), Some("Objection"));

        let mut out = String::new();
        format_thread_context_to(&storage, &session_id, &frag.fragment_id, 6, &mut out).unwrap();
        assert!(out.contains("[Chat graph thread context]"));
        assert!(out.contains("[Branch: Root / WordNet: none] anchor=\"we should\""));
        assert!(out.contains("direct_replies:"));
        assert!(out.contains("[Branch: Objection / WordNet:"));
        assert!(out.contains("I disagree with that step"));

        let _ = std::fs::remove_dir_all(&storage);
    }

    #[test]
    fn thread_context_walks_ancestors_to_max_depth() {
        let storage = temp_storage();
        let session_id =
            crate::chat_session::create_session(&storage, Some("Graph".into()), None).unwrap();
        let lamport_a = crate::chat_session::append_message(
            &storage,
            &session_id,
            Role::User,
            "first proposal text",
        )
        .unwrap();
        let lamport_b = crate::chat_session::append_message(
            &storage,
            &session_id,
            Role::User,
            "a clarifying follow up",
        )
        .unwrap();

        let frag_a = create_fragment_from_selection(
            &storage,
            &session_id,
            lamport_a,
            "first proposal text",
            0,
            5,
        )
        .unwrap();
        let frag_b = create_fragment_from_selection(
            &storage,
            &session_id,
            lamport_b,
            "a clarifying follow up",
            0,
            12,
        )
        .unwrap();
        link_reply_to_fragment(
            &storage,
            &session_id,
            &frag_a.fragment_id,
            lamport_b,
            Some(&frag_b.fragment_id),
            Some("proposal"),
            Some("a clarifying follow up"),
            Some("clarification"),
        )
        .unwrap();

        let mut out = String::new();
        format_thread_context_to(&storage, &session_id, &frag_b.fragment_id, 6, &mut out).unwrap();
        // Child fragment annotated with the Clarification branch; ancestor follows.
        assert!(out.contains("[Branch: Clarification / WordNet:"));
        assert!(out.contains("anchor=\"a clarifying\""));
        assert!(out.contains("anchor=\"first\""));

        // Depth 1 shows only the target fragment, not the ancestor.
        let mut shallow = String::new();
        format_thread_context_to(&storage, &session_id, &frag_b.fragment_id, 1, &mut shallow)
            .unwrap();
        assert!(shallow.contains("a clarifying"));
        assert!(!shallow.contains("anchor=\"first\""));

        let _ = std::fs::remove_dir_all(&storage);
    }
}
