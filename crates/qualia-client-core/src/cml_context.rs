//! **CML context loop** — chat logs carry Context Markup that binds topic-related semantics into the
//! person's *inforg* (their private hypermedia library), which is then reused, permissively, to improve
//! the context given to the local agent.
//!
//! CML (Timothy C. Holborn's Context Markup Language — a working draft) treats *a concept as a context
//! hash*: `q_hash(concept)` addresses the concept's sub-graph. Here a person marks context inline in a
//! message —
//!   `#project:tax-2026  #task:file-return  #topic:deductions  [[capital gains]]`
//! — and each tag becomes a `cml:Proposed` concept stored alongside the turn in the inforg. A later turn
//! that shares those concepts pulls the earlier context back in. Multi-part by construction: one message
//! may carry a general concept, a project, a topic, and a task at once.
//!
//! v1 substrate: the concept identity is the hypermedia store's `fnv60(label)` edge object (so a search
//! for the label matches). The fuller `cml.n3` IRI-hash concept graph compiled into `.q42` for the graph
//! retrieval path is the next increment; this loop already works end-to-end over the inforg. Reuse is
//! **permission-gated**: guardian-flagged (sensitive) entries are never auto-injected.

use crate::wellfair::hypermedia_store::{HypermediaStore, LibraryEntry};
use qualia_core_db::hypermedia::{ingest_with, Descriptors, TextProcessor};
use qualia_core_db::hypermedia::fnv60;
use std::collections::HashSet;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

/// The CML concept tiers a chat message may carry (multi-part).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConceptTier {
    General,
    Project,
    Topic,
    Task,
    Pursuit,
}

impl ConceptTier {
    fn from_key(k: &str) -> Option<Self> {
        match k.to_ascii_lowercase().as_str() {
            "general" => Some(Self::General),
            "project" | "proj" => Some(Self::Project),
            "topic" => Some(Self::Topic),
            "task" => Some(Self::Task),
            "pursuit" | "goal" => Some(Self::Pursuit),
            _ => None,
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::General => "general",
            Self::Project => "project",
            Self::Topic => "topic",
            Self::Task => "task",
            Self::Pursuit => "pursuit",
        }
    }
}

/// One context tag parsed from a message: a `cml:Proposed` concept at a tier.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContextTag {
    pub tier: ConceptTier,
    pub label: String,
}

impl ContextTag {
    /// The CML concept IRI for this tag (`https://ns.webcivics.net/cml/concept/<tier>/<slug>`).
    pub fn iri(&self) -> String {
        format!(
            "https://ns.webcivics.net/cml/concept/{}/{}",
            self.tier.as_str(),
            slug(&self.label)
        )
    }
    /// The concept's context hash — CML's "a concept is a context hash".
    pub fn context_hash(&self) -> u64 {
        fnv60(self.iri().as_bytes())
    }
}

fn slug(s: &str) -> String {
    let lowered: String = s
        .trim()
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { '-' })
        .collect();
    lowered.trim_matches('-').to_string()
}

/// Parse inline CML from a message. Recognises `#project:label`, `#task:label`, `#topic:label`,
/// `#pursuit:label`, `#general:label`, bare `#hashtag` (→ topic), and `[[general concept]]` (may contain
/// spaces). Underscores in a `#key:value` label become spaces. Duplicates (same tier + label) removed.
pub fn parse_cml_tags(text: &str) -> Vec<ContextTag> {
    let mut tags: Vec<ContextTag> = Vec::new();
    let mut push = |tier: ConceptTier, label: String| {
        let label = label.trim().to_string();
        if !label.is_empty()
            && !tags
                .iter()
                .any(|t| t.tier == tier && t.label.eq_ignore_ascii_case(&label))
        {
            tags.push(ContextTag { tier, label });
        }
    };

    // [[general concepts]] (may contain spaces)
    let mut rest = text;
    while let Some(start) = rest.find("[[") {
        let after = &rest[start + 2..];
        if let Some(end) = after.find("]]") {
            push(ConceptTier::General, after[..end].to_string());
            rest = &after[end + 2..];
        } else {
            break;
        }
    }

    // #key:value and bare #hashtag tokens
    for raw in text.split_whitespace() {
        let tok = raw.trim_matches(|c: char| matches!(c, '.' | ',' | '!' | '?' | ';' | ')' | '('));
        if let Some(body) = tok.strip_prefix('#') {
            if body.is_empty() {
                continue;
            }
            if let Some((k, v)) = body.split_once(':') {
                match ConceptTier::from_key(k) {
                    Some(tier) => push(tier, v.replace('_', " ")),
                    None => push(ConceptTier::Topic, body.replace('_', " ")),
                }
            } else {
                push(ConceptTier::Topic, body.replace('_', " "));
            }
        }
    }
    tags
}

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Store a chat turn's CML context into the inforg — **only when the message carries explicit tags**
/// (deliberate, permissive). The turn's text + its concept facets become a searchable library entry.
/// Returns the concepts stored (empty if the message had no tags).
pub fn ingest_turn(storage: &Path, session_id: &str, text: &str) -> Result<Vec<ContextTag>, String> {
    let tags = parse_cml_tags(text);
    if tags.is_empty() {
        return Ok(Vec::new());
    }
    let store = HypermediaStore::open(storage).map_err(|e| e.to_string())?;

    let mut topics: Vec<String> = Vec::new();
    let mut projects: Vec<String> = Vec::new();
    let mut purposes: Vec<String> = Vec::new();
    for t in &tags {
        match t.tier {
            ConceptTier::Project => projects.push(t.label.clone()),
            ConceptTier::Task | ConceptTier::Pursuit => purposes.push(t.label.clone()),
            ConceptTier::Topic | ConceptTier::General => topics.push(t.label.clone()),
        }
    }

    let digest = fnv60(text.as_bytes());
    let uri = format!("urn:qualia:chat:{session_id}:{digest:016x}");
    let r = ingest_with(&TextProcessor::default(), &uri, "text/plain", digest, text.as_bytes());
    let subject = r.container.primary.subject();
    let mut quins = r.quins;
    let desc = Descriptors {
        topics: topics.clone(),
        projects: projects.clone(),
        purposes: purposes.clone(),
        ..Default::default()
    };
    let (dq, _lex) = qualia_core_db::hypermedia::descriptors_to_nquins(subject, &desc);
    quins.extend(dq);

    let entry = LibraryEntry {
        asset_uri: uri,
        primary_subject: subject,
        media_type: "text/plain".to_string(),
        quins,
        topics,
        projects,
        place: None,
        occurred_at: None,
        lat: None,
        lon: None,
        flags: Vec::new(),
        ingested_unix: now_unix(),
        excerpt: text.chars().take(240).collect(),
    };
    store.add(entry).map_err(|e| e.to_string())?;
    Ok(tags)
}

const STOPWORDS: &[&str] = &[
    "the", "and", "for", "with", "that", "this", "what", "how", "why", "who", "you", "your", "are",
    "was", "can", "will", "into", "from", "about", "have", "has", "not", "but", "get", "got",
    "tell", "give", "please", "would", "could", "should", "them", "they", "our", "out",
];

fn salient_terms(prompt: &str) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for w in prompt.split(|c: char| !c.is_alphanumeric()) {
        let w = w.to_lowercase();
        if w.len() > 3 && !STOPWORDS.contains(&w.as_str()) && seen.insert(w.clone()) {
            out.push(w);
        }
    }
    out
}

fn absorb(entries: Vec<LibraryEntry>, chosen: &mut Vec<LibraryEntry>, seen: &mut HashSet<String>) {
    for e in entries {
        // Permission gate: guardian-flagged (sensitive) entries are never auto-injected.
        if e.flags.is_empty() && seen.insert(e.asset_uri.clone()) {
            chosen.push(e);
        }
    }
}

/// Retrieve inforg context relevant to `prompt` and format it as a prompt block (empty if nothing).
/// Matches the prompt's explicit tags first (reliable), then a few salient terms (best-effort), over
/// the person's library. Permission-gated (flagged entries excluded).
pub fn retrieve_context(storage: &Path, prompt: &str, max_snippets: usize) -> String {
    let store = match HypermediaStore::open(storage) {
        Ok(s) => s,
        Err(_) => return String::new(),
    };
    let mut chosen: Vec<LibraryEntry> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for tag in parse_cml_tags(prompt) {
        let facet = match tag.tier {
            ConceptTier::Project => "project",
            ConceptTier::Task | ConceptTier::Pursuit => "purpose",
            _ => "topic",
        };
        if let Ok(entries) = store.search(facet, &tag.label) {
            absorb(entries, &mut chosen, &mut seen);
        }
    }
    for term in salient_terms(prompt).into_iter().take(6) {
        if chosen.len() >= max_snippets {
            break;
        }
        if let Ok(entries) = store.search("topic", &term) {
            absorb(entries, &mut chosen, &mut seen);
        }
    }

    if chosen.is_empty() {
        return String::new();
    }
    let mut lines = Vec::new();
    for e in chosen.into_iter().take(max_snippets) {
        let mut facets: Vec<String> = Vec::new();
        facets.extend(e.projects.iter().cloned());
        facets.extend(e.topics.iter().cloned());
        let tag = if facets.is_empty() {
            String::new()
        } else {
            format!(" [{}]", facets.join(", "))
        };
        lines.push(format!("- {}{}", e.excerpt.trim(), tag));
    }
    format!(
        "Relevant context from your library (inforg) — use for grounding, cite if used:\n{}",
        lines.join("\n")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_multi_part_context() {
        let tags = parse_cml_tags("note #project:tax_2026 [[capital gains]] #deductions #task:file-return");
        assert!(tags.contains(&ContextTag { tier: ConceptTier::Project, label: "tax 2026".into() }));
        assert!(tags.contains(&ContextTag { tier: ConceptTier::General, label: "capital gains".into() }));
        assert!(tags.contains(&ContextTag { tier: ConceptTier::Topic, label: "deductions".into() }));
        assert!(tags.contains(&ContextTag { tier: ConceptTier::Task, label: "file-return".into() }));
    }

    #[test]
    fn untagged_message_stores_nothing() {
        let dir = tempfile::tempdir().unwrap();
        let stored = ingest_turn(dir.path(), "s1", "just a plain message with no markup").unwrap();
        assert!(stored.is_empty());
        assert!(retrieve_context(dir.path(), "anything", 4).is_empty());
    }

    #[test]
    fn tagged_turn_is_stored_and_reused() {
        let dir = tempfile::tempdir().unwrap();
        let stored = ingest_turn(
            dir.path(),
            "s1",
            "The liver secretes bile. #project:hep-notes #topic:anatomy",
        )
        .unwrap();
        assert_eq!(stored.len(), 2);
        // A later turn sharing the project pulls the earlier context back in.
        let ctx = retrieve_context(dir.path(), "remind me about #project:hep-notes", 4);
        assert!(ctx.contains("liver"), "expected inforg recall, got: {ctx}");
        // An unrelated project matches nothing.
        assert!(retrieve_context(dir.path(), "unrelated #project:nothing-here", 4).is_empty());
    }

    #[test]
    fn concept_is_a_context_hash() {
        let t = ContextTag { tier: ConceptTier::Topic, label: "Capital Gains".into() };
        assert_eq!(t.iri(), "https://ns.webcivics.net/cml/concept/topic/capital-gains");
        assert_ne!(t.context_hash(), 0);
    }
}
