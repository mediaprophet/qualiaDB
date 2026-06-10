//! Chat discourse ontology — branch types grounded in WordNet when `wordnet.q42` is present.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use qualia_core_db::q_hash;
use serde::{Deserialize, Serialize};

use crate::resource_import;

static WORDNET_LEX: OnceLock<Option<qualia_core_db::q42_lex::Q42Lexicon>> = OnceLock::new();

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatBranchType {
    pub id: String,
    pub label: String,
    pub emoji: String,
    pub description: String,
    pub wordnet_lemmas: Vec<String>,
    pub keywords: Vec<String>,
    #[serde(default)]
    pub wordnet_grounding_hash: Option<String>,
    #[serde(default)]
    pub wordnet_gloss: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BranchClassification {
    pub branch_type_id: String,
    pub label: String,
    pub emoji: String,
    pub confidence: f32,
    pub wordnet_grounding_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatReaction {
    pub message_lamport: u64,
    pub emoji: String,
    pub author_did: String,
    pub author_name: Option<String>,
    pub created_at: u64,
}

const DISCOURSE_TYPES: &[(&str, &str, &str, &str, &[&str], &[&str])] = &[
    (
        "comment",
        "Comment",
        "💬",
        "General remark or observation",
        &["comment", "remark", "note", "statement"],
        &["note", "fwiw", "interesting"],
    ),
    (
        "correction",
        "Correction",
        "✏️",
        "Fixes or amends prior content",
        &["correction", "amendment", "rectification"],
        &[
            "actually",
            "incorrect",
            "mistake",
            "fix",
            "wrong",
            "should be",
        ],
    ),
    (
        "inquiry",
        "Inquiry",
        "❓",
        "Question or request for information",
        &["question", "inquiry", "query"],
        &[
            "?",
            "how",
            "why",
            "what",
            "when",
            "where",
            "could you",
            "can you",
        ],
    ),
    (
        "agreement",
        "Agreement",
        "✅",
        "Endorses or confirms prior content",
        &["agreement", "assent", "concurrence"],
        &["yes", "agree", "correct", "exactly", "right", "+1"],
    ),
    (
        "objection",
        "Objection",
        "⚠️",
        "Disagrees or challenges prior content",
        &["objection", "disagreement", "dissent"],
        &["disagree", "no", "however", "but", "not quite", "object"],
    ),
    (
        "clarification",
        "Clarification",
        "🔍",
        "Explains or disambiguates",
        &["clarification", "explanation", "elucidation"],
        &["clarify", "mean", "to be clear", "in other words", "i.e."],
    ),
    (
        "evidence",
        "Evidence",
        "📎",
        "Cites sources or supporting facts",
        &["evidence", "proof", "citation"],
        &[
            "source",
            "cite",
            "according to",
            "study",
            "data shows",
            "provenance",
        ],
    ),
    (
        "suggestion",
        "Suggestion",
        "💡",
        "Proposes an idea or next step",
        &["suggestion", "proposal", "recommendation"],
        &["suggest", "could", "might", "recommend", "try", "idea"],
    ),
    (
        "summary",
        "Summary",
        "📋",
        "Synthesizes or restates thread content",
        &["summary", "synopsis", "recap"],
        &["summary", "in short", "tl;dr", "to summarize", "overall"],
    ),
    (
        "humor",
        "Humor",
        "😄",
        "Light or playful response",
        &["humor", "joke", "wit"],
        &["lol", "haha", "😄", "😂", "jk"],
    ),
];

pub fn local_wordnet_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("QUALIA_WORDNET_DIR") {
        let trimmed = dir.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    PathBuf::from("Local_LIbraries/wordnet")
}

pub fn resolve_wordnet_q42(storage: &Path) -> Option<PathBuf> {
    let index = resource_import::index_dir(storage);
    let local = local_wordnet_dir();
    let candidates = [
        index.join("wordnet.q42"),
        index.join("wordnet-rdf.q42"),
        index.join("english-wordnet.q42"),
        local.join("wordnet.q42"),
        local.join("wordnet.c.q42"),
        local.join("wordnet_compressed.q42"),
        PathBuf::from("docs/playground/wordnet.q42"),
        PathBuf::from("wordnet.q42"),
    ];
    candidates.into_iter().find(|p| p.is_file())
}

pub fn resolve_wordnet_lex(q42_path: &Path) -> Option<PathBuf> {
    if qualia_core_db::q42_volume::is_v2_volume(q42_path).ok() == Some(true) {
        return Some(q42_path.to_path_buf());
    }
    let lex = q42_path.with_extension("q42.lex");
    if lex.is_file() {
        return Some(lex);
    }
    let alt = PathBuf::from(format!("{}.lex", q42_path.display()));
    if alt.is_file() {
        return Some(alt);
    }
    let sibling = q42_path.with_file_name(format!(
        "{}.q42.lex",
        q42_path.file_stem()?.to_string_lossy()
    ));
    if sibling.is_file() {
        return Some(sibling);
    }
    None
}

fn wordnet_lexicon() -> Option<&'static qualia_core_db::q42_lex::Q42Lexicon> {
    WORDNET_LEX
        .get_or_init(|| {
            let state = crate::state::APP_STATE.get()?;
            let storage = state.config.lock().ok()?.storage_path.clone();
            let q42 = resolve_wordnet_q42(Path::new(&storage))?;
            qualia_core_db::q42_lex::Q42Lexicon::load_for_q42(&q42)
                .ok()
                .or_else(|| {
                    resolve_wordnet_lex(&q42)
                        .and_then(|lex_path| qualia_core_db::q42_lex::Q42Lexicon::load(&lex_path).ok())
                })
        })
        .as_ref()
}

pub fn wordnet_available(storage: &Path) -> bool {
    resolve_wordnet_q42(storage).is_some()
}

pub fn list_branch_types(storage: &Path) -> Vec<ChatBranchType> {
    let lex = wordnet_lexicon();
    DISCOURSE_TYPES
        .iter()
        .map(|(id, label, emoji, desc, lemmas, keywords)| {
            let mut branch = ChatBranchType {
                id: (*id).to_string(),
                label: (*label).to_string(),
                emoji: (*emoji).to_string(),
                description: (*desc).to_string(),
                wordnet_lemmas: lemmas.iter().map(|s| (*s).to_string()).collect(),
                keywords: keywords.iter().map(|s| (*s).to_string()).collect(),
                wordnet_grounding_hash: None,
                wordnet_gloss: None,
            };
            if let Some(lex) = lex {
                for lemma in *lemmas {
                    if let Some(hash) = lex.find_literal(lemma) {
                        branch.wordnet_grounding_hash = Some(format!("0x{hash:016x}"));
                        branch.wordnet_gloss = lex.lookup(hash).map(|s| s.to_string());
                        break;
                    }
                }
            }
            let _ = storage;
            branch
        })
        .collect()
}

pub fn classify_branch(
    storage: &Path,
    anchor_text: &str,
    reply_text: &str,
) -> BranchClassification {
    let types = list_branch_types(storage);
    let combined = format!("{anchor_text}\n{reply_text}").to_lowercase();
    let lex = wordnet_lexicon();

    let mut best_id = "comment";
    let mut best_score = 0f32;

    for t in &types {
        let mut score = 0f32;
        for kw in &t.keywords {
            if combined.contains(&kw.to_lowercase()) {
                score += 1.5;
            }
        }
        for lemma in &t.wordnet_lemmas {
            if combined.contains(lemma) {
                score += 1.0;
            }
            if let Some(lex) = lex {
                for (_, gloss) in lex.search_contains(lemma, 3) {
                    if combined.contains(&gloss.to_lowercase()) {
                        score += 0.5;
                    }
                }
            }
        }
        if score > best_score {
            best_score = score;
            best_id = &t.id;
        }
    }

    let chosen = types
        .iter()
        .find(|t| t.id == best_id)
        .or_else(|| types.first());

    if let Some(t) = chosen {
        BranchClassification {
            branch_type_id: t.id.clone(),
            label: t.label.clone(),
            emoji: t.emoji.clone(),
            confidence: (best_score / 3.0).min(1.0),
            wordnet_grounding_hash: t.wordnet_grounding_hash.clone(),
        }
    } else {
        BranchClassification {
            branch_type_id: "comment".to_string(),
            label: "Comment".to_string(),
            emoji: "💬".to_string(),
            confidence: 0.1,
            wordnet_grounding_hash: None,
        }
    }
}

pub fn build_chat_ontology_briefing(storage: &Path) -> String {
    let q42 = resolve_wordnet_q42(storage);
    let types = list_branch_types(storage);
    let mut lines = vec!["[Chat discourse ontology]".to_string()];
    if let Some(path) = q42 {
        lines.push(format!(
            "wordnet_grounding: {} (lexicon-backed branch labels)",
            path.display()
        ));
    } else {
        lines.push(
            "wordnet_grounding: not installed — place artefacts under Local_LIbraries/wordnet/, import via Ontology Hub, or run scripts/fetch_wordnet.sh"
                .to_string(),
        );
    }
    lines.push("branch_types:".to_string());
    for t in types {
        let wn = t.wordnet_grounding_hash.as_deref().unwrap_or("builtin");
        lines.push(format!(
            "  - {} {} ({}) wordnet={}",
            t.emoji, t.label, t.id, wn
        ));
    }
    lines.push(
        "instructions: Label each graph branch with the best-matching branch_type when replying to a fragment."
            .to_string(),
    );
    lines.join("\n")
}

fn reactions_path(storage_root: &Path, session_id: &str) -> PathBuf {
    storage_root
        .join("Chats")
        .join(session_id)
        .join("reactions.jsonl")
}

fn unix_now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

pub fn add_reaction(
    storage_root: &Path,
    session_id: &str,
    message_lamport: u64,
    emoji: &str,
) -> Result<Vec<ChatReaction>, String> {
    if emoji.is_empty() {
        return Err("Emoji required".to_string());
    }
    // Basic validation: allow printable unicode including emoji sequences
    if emoji.chars().count() > 8 {
        return Err("Emoji reaction too long".to_string());
    }

    let profile = crate::user_profile::load_profile();
    let path = reactions_path(storage_root, session_id);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let mut reactions = list_reactions(storage_root, session_id)?;
    if reactions.iter().any(|r| {
        r.message_lamport == message_lamport
            && r.author_did == profile.public_did
            && r.emoji == emoji
    }) {
        return Ok(reactions);
    }

    let reaction = ChatReaction {
        message_lamport,
        emoji: emoji.to_string(),
        author_did: profile.public_did.clone(),
        author_name: profile
            .sharing
            .share_display_name
            .then(|| profile.display_name.clone()),
        created_at: unix_now(),
    };
    reactions.push(reaction);

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    for r in &reactions {
        use std::io::Write;
        writeln!(
            file,
            "{}",
            serde_json::to_string(r).map_err(|e| e.to_string())?
        )
        .map_err(|e| e.to_string())?;
    }

    // WAL quin: chat:hasReaction
    let wal_path = storage_root.join("Chats").join(session_id).join("chat.wal");
    if wal_path.is_file() {
        let subject = q_hash(&format!("chat:session:{session_id}"));
        let predicate = q_hash("chat:hasReaction");
        let object = q_hash(&format!("react:{message_lamport}:{emoji}")) & 0x0FFF_FFFF_FFFF_FFFF;
        let context = q_hash(&profile.public_did);
        let metadata = (message_lamport & 0x1FFF_FFFF) << 32;
        let parity = subject ^ predicate ^ object ^ context ^ metadata;
        let quin = qualia_core_db::QualiaQuin {
            subject,
            predicate,
            object,
            context,
            metadata,
            parity,
        };
        if let Ok(mut wal) = qualia_core_db::wal::WriteAheadLog::open(&wal_path) {
            let _ = wal.append_mutation(&quin);
        }
    }

    Ok(reactions)
}

pub fn toggle_reaction(
    storage_root: &Path,
    session_id: &str,
    message_lamport: u64,
    emoji: &str,
) -> Result<Vec<ChatReaction>, String> {
    let profile = crate::user_profile::load_profile();
    let existing: Vec<_> = list_reactions_for_message(storage_root, session_id, message_lamport)?;
    let already = existing
        .iter()
        .any(|r| r.author_did == profile.public_did && r.emoji == emoji);
    if already {
        remove_reaction(storage_root, session_id, message_lamport, emoji)
    } else {
        add_reaction(storage_root, session_id, message_lamport, emoji)
    }
}

fn remove_reaction(
    storage_root: &Path,
    session_id: &str,
    message_lamport: u64,
    emoji: &str,
) -> Result<Vec<ChatReaction>, String> {
    let profile = crate::user_profile::load_profile();
    let mut reactions = list_reactions(storage_root, session_id)?;
    reactions.retain(|r| {
        !(r.message_lamport == message_lamport
            && r.author_did == profile.public_did
            && r.emoji == emoji)
    });
    let path = reactions_path(storage_root, session_id);
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    for r in &reactions {
        use std::io::Write;
        writeln!(
            file,
            "{}",
            serde_json::to_string(r).map_err(|e| e.to_string())?
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(reactions)
}

pub fn list_reactions(storage_root: &Path, session_id: &str) -> Result<Vec<ChatReaction>, String> {
    let path = reactions_path(storage_root, session_id);
    if !path.is_file() {
        return Ok(Vec::new());
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(r) = serde_json::from_str::<ChatReaction>(line) {
            out.push(r);
        }
    }
    Ok(out)
}

pub fn list_reactions_for_message(
    storage_root: &Path,
    session_id: &str,
    message_lamport: u64,
) -> Result<Vec<ChatReaction>, String> {
    Ok(list_reactions(storage_root, session_id)?
        .into_iter()
        .filter(|r| r.message_lamport == message_lamport)
        .collect())
}
