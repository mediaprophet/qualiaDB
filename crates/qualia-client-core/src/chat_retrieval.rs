//! Scoped graph retrieval for chat — local `.q42` scan + optional daemon query.

use std::collections::HashSet;
use std::path::Path;

use qualia_core_db::{q42_reader::read_q42_quins, q_hash, QualiaQuin};
use serde::{Deserialize, Serialize};

use crate::chat_session::ChatEnvironment;
use crate::resource_import;

const MAX_RETRIEVAL_TRIPLES: usize = 48;
const OBJECT_HASH_MASK: u64 = 0x0FFF_FFFF_FFFF_FFFF;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GraphCitation {
    pub ontology_id: String,
    pub subject_hash: String,
    pub predicate_hash: String,
    pub object_hash: String,
    pub label: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetrievalBundle {
    pub triple_count: usize,
    pub citations: Vec<GraphCitation>,
    pub provenance_hashes: Vec<u64>,
    pub context_block: String,
    pub daemon_queried: bool,
    pub daemon_match_count: u64,
}

pub fn retrieve_graph_context(
    storage: &Path,
    env: &ChatEnvironment,
    user_prompt: &str,
    routed_ontology_ids: &[String],
) -> RetrievalBundle {
    let keywords = extract_keywords(user_prompt);
    let mut citations = Vec::new();
    let mut provenance_hashes = Vec::new();
    let mut seen: HashSet<(u64, u64, u64)> = HashSet::new();

    let ontology_ids = if routed_ontology_ids.is_empty() {
        &env.ontology_ids
    } else {
        routed_ontology_ids
    };

    for ont_id in ontology_ids {
        let q42_path = resource_import::index_dir(storage).join(format!("{ont_id}.q42"));
        if !q42_path.is_file() {
            continue;
        }
        let Ok(quins) = read_q42_quins(&q42_path) else {
            continue;
        };

        for quin in &quins {
            if citations.len() >= MAX_RETRIEVAL_TRIPLES {
                break;
            }
            if !matches_keywords(quin, &keywords) && !keywords.is_empty() {
                continue;
            }
            let key = (quin.subject, quin.predicate, quin.object);
            if !seen.insert(key) {
                continue;
            }
            let citation_hash = quin.subject ^ quin.predicate ^ quin.object;
            provenance_hashes.push(citation_hash);
            citations.push(GraphCitation {
                ontology_id: ont_id.clone(),
                subject_hash: format!("0x{:016x}", quin.subject),
                predicate_hash: format!("0x{:016x}", quin.predicate),
                object_hash: format!("0x{:016x}", quin.object & OBJECT_HASH_MASK),
                label: format!(
                    "{} → {} → {}",
                    short_hash(quin.subject),
                    short_hash(quin.predicate),
                    short_hash(quin.object)
                ),
            });
        }
    }

    let (daemon_queried, daemon_match_count, daemon_extra) =
        query_daemon_for_prompt(user_prompt, &keywords);

    if daemon_queried && daemon_match_count > 0 {
        provenance_hashes.push(q_hash("qualia:daemon_graph"));
        if citations.is_empty() && !daemon_extra.is_empty() {
            citations.push(GraphCitation {
                ontology_id: "daemon".to_string(),
                subject_hash: "live".to_string(),
                predicate_hash: "graph".to_string(),
                object_hash: format!("{daemon_match_count}"),
                label: daemon_extra,
            });
        }
    }

    provenance_hashes.sort_unstable();
    provenance_hashes.dedup();

    let context_block = format_retrieval_block(&citations, daemon_match_count);

    RetrievalBundle {
        triple_count: citations.len(),
        citations,
        provenance_hashes,
        context_block,
        daemon_queried,
        daemon_match_count,
    }
}

fn extract_keywords(prompt: &str) -> Vec<String> {
    prompt
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 4)
        .map(|w| w.to_lowercase())
        .take(16)
        .collect()
}

fn matches_keywords(quin: &QualiaQuin, keywords: &[String]) -> bool {
    if keywords.is_empty() {
        return true;
    }
    let sub = format!("{:016x}", quin.subject);
    let pred = format!("{:016x}", quin.predicate);
    let obj = format!("{:016x}", quin.object);
    keywords
        .iter()
        .any(|kw| sub.contains(kw) || pred.contains(kw) || obj.contains(kw))
}

fn short_hash(h: u64) -> String {
    format!("{:08x}", (h & 0xFFFF_FFFF) as u32)
}

fn format_retrieval_block(citations: &[GraphCitation], daemon_matches: u64) -> String {
    if citations.is_empty() && daemon_matches == 0 {
        return "[Graph retrieval: no scoped triples matched — rely on environment capabilities only]"
            .to_string();
    }

    let mut lines = vec![format!(
        "[Graph retrieval: {} scoped citation(s), daemon matches: {daemon_matches}]",
        citations.len()
    )];
    for (i, c) in citations.iter().take(24).enumerate() {
        lines.push(format!("  {}. [{}] {}", i + 1, c.ontology_id, c.label));
    }
    lines.join("\n")
}

fn query_daemon_for_prompt(prompt: &str, keywords: &[String]) -> (bool, u64, String) {
    if crate::daemon_status() != "running" {
        return (false, 0, String::new());
    }
    let port = crate::get_active_daemon_port();
    if port == 0 {
        return (false, 0, String::new());
    }

    let client = match reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(4))
        .build()
    {
        Ok(c) => c,
        Err(_) => return (false, 0, String::new()),
    };

    let token = crate::issue_qapp_session_token("Chat").unwrap_or_default();
    let url = format!("http://127.0.0.1:{port}/query");

    let query = if keywords.is_empty() {
        "?subject ?predicate ?object .".to_string()
    } else {
        format!("# Scoped chat retrieval for: {}", keywords.join(", "))
    };

    let response = client
        .post(&url)
        .header("X-Qualia-Token", token)
        .header("Accept", "application/ld+json")
        .json(&serde_json::json!({
            "query": query,
            "format": "json-ld",
            "prompt_hint": prompt.chars().take(120).collect::<String>()
        }))
        .send();

    let Ok(resp) = response else {
        return (true, 0, String::new());
    };
    if !resp.status().is_success() {
        return (true, 0, String::new());
    }

    let Ok(body) = resp.json::<serde_json::Value>() else {
        return (true, 0, String::new());
    };

    let match_count = body
        .get("match_count")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);

    let summary = if match_count > 0 {
        format!("Daemon graph returned {match_count} quin(s) for scoped query")
    } else {
        String::new()
    };

    (true, match_count, summary)
}
