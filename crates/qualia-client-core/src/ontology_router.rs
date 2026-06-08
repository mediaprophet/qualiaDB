//! Prompt-to-ontology routing for contextual inference.
//!
//! This module chooses which installed ontologies are most relevant for a turn,
//! derives stable namespace hashes for the LLM intent frame, and produces a
//! short routing briefing that can be injected into the augmented prompt.

use std::collections::HashSet;

use qualia_core_db::q_hash;

use crate::chat_session::{ChatEnvironment, OntologyScopeSummary};

const MAX_ROUTED_ONTOLOGIES: usize = 4;
const MAX_CONTEXT_NAMESPACES: usize = 16;

#[derive(Debug, Clone, Default)]
pub struct OntologyRoutingDecision {
    pub ontology_ids: Vec<String>,
    pub context_namespaces: Vec<u64>,
    pub matched_terms: Vec<String>,
    pub routing_brief: String,
}

pub fn route_prompt_to_ontologies(env: &ChatEnvironment, prompt: &str) -> OntologyRoutingDecision {
    if env.ontology_summaries.is_empty() {
        return OntologyRoutingDecision::default();
    }

    let keywords = extract_keywords(prompt);
    let mut scored: Vec<(i32, &OntologyScopeSummary)> = env
        .ontology_summaries
        .iter()
        .map(|summary| (score_summary(summary, &keywords), summary))
        .collect();

    scored.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.id.cmp(&b.1.id)));

    let mut ontology_ids = Vec::new();
    let mut context_namespaces = Vec::new();
    let mut matched_terms = Vec::new();
    let mut seen_terms = HashSet::new();

    for (score, summary) in &scored {
        if *score <= 0 && !ontology_ids.is_empty() {
            break;
        }
        if *score <= 0 && ontology_ids.len() >= MAX_ROUTED_ONTOLOGIES {
            break;
        }
        if ontology_ids.len() >= MAX_ROUTED_ONTOLOGIES {
            break;
        }
        ontology_ids.push(summary.id.clone());
        extend_namespaces(&mut context_namespaces, summary);
        for keyword in &keywords {
            if summary_matches_keyword(summary, keyword) && seen_terms.insert(keyword.clone()) {
                matched_terms.push(keyword.clone());
            }
        }
    }

    if ontology_ids.is_empty() {
        for summary in env.ontology_summaries.iter().take(MAX_ROUTED_ONTOLOGIES) {
            ontology_ids.push(summary.id.clone());
            extend_namespaces(&mut context_namespaces, summary);
        }
    }

    if ontology_ids.iter().all(|id| !id.contains("wordnet")) {
        if let Some(wordnet) = env
            .ontology_summaries
            .iter()
            .find(|o| o.id.contains("wordnet"))
            .filter(|_| ontology_ids.len() < MAX_ROUTED_ONTOLOGIES)
        {
            ontology_ids.push(wordnet.id.clone());
            extend_namespaces(&mut context_namespaces, wordnet);
        }
    }

    context_namespaces.sort_unstable();
    context_namespaces.dedup();
    if context_namespaces.len() > MAX_CONTEXT_NAMESPACES {
        context_namespaces.truncate(MAX_CONTEXT_NAMESPACES);
    }

    let routing_brief = if ontology_ids.is_empty() {
        "[Ontology routing: no installed ontologies selected]".to_string()
    } else if matched_terms.is_empty() {
        format!(
            "[Ontology routing: {} selected for general grounding]",
            ontology_ids.join(", ")
        )
    } else {
        format!(
            "[Ontology routing: {} selected from prompt terms: {}]",
            ontology_ids.join(", "),
            matched_terms.join(", ")
        )
    };

    OntologyRoutingDecision {
        ontology_ids,
        context_namespaces,
        matched_terms,
        routing_brief,
    }
}

fn extract_keywords(prompt: &str) -> Vec<String> {
    prompt
        .split(|c: char| !c.is_alphanumeric())
        .filter(|w| w.len() >= 3)
        .map(|w| w.to_ascii_lowercase())
        .take(24)
        .collect()
}

fn score_summary(summary: &OntologyScopeSummary, keywords: &[String]) -> i32 {
    if keywords.is_empty() {
        return 1;
    }

    let mut score = 0;
    for keyword in keywords {
        if summary_matches_keyword(summary, keyword) {
            score += 4;
        }
        score += domain_bonus(summary, keyword);
    }
    if summary.id.contains("wordnet") {
        score += 1;
    }
    score
}

fn summary_matches_keyword(summary: &OntologyScopeSummary, keyword: &str) -> bool {
    contains_term(&summary.id, keyword)
        || contains_term(&summary.name, keyword)
        || summary
            .domain
            .as_deref()
            .map(|d| contains_term(d, keyword))
            .unwrap_or(false)
        || summary
            .tags
            .iter()
            .flatten()
            .any(|tag| contains_term(tag, keyword))
}

fn contains_term(text: &str, keyword: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains(keyword)
}

fn domain_bonus(summary: &OntologyScopeSummary, keyword: &str) -> i32 {
    let health = [
        "health", "medical", "medicine", "clinical", "patient", "drug", "fhir", "loinc", "snomed",
        "anatomy", "body", "heart", "fever", "symptom",
    ];
    let legal = [
        "legal",
        "law",
        "contract",
        "rights",
        "guardian",
        "guardianship",
        "consent",
        "policy",
        "agreement",
        "duty",
    ];
    let civic = ["commons", "community", "governance", "public", "care"];

    let domain = summary
        .domain
        .as_deref()
        .unwrap_or_default()
        .to_ascii_lowercase();
    let tags = summary
        .tags
        .as_ref()
        .map(|v| v.join(" ").to_ascii_lowercase())
        .unwrap_or_default();
    let haystack = format!("{} {}", domain, tags);

    if health.contains(&keyword) && haystack.contains("health") {
        return 6;
    }
    if legal.contains(&keyword)
        && (haystack.contains("legal") || haystack.contains("rights") || haystack.contains("guard"))
    {
        return 6;
    }
    if civic.contains(&keyword) && haystack.contains("commons") {
        return 4;
    }
    0
}

fn extend_namespaces(out: &mut Vec<u64>, summary: &OntologyScopeSummary) {
    out.push(q_hash(&summary.id));
    out.push(q_hash(&format!("ont:{}", summary.id)));
    out.push(q_hash(&summary.name));
    if let Some(domain) = &summary.domain {
        out.push(q_hash(domain));
        out.push(q_hash(&format!("domain:{domain}")));
    }
    if let Some(tags) = &summary.tags {
        for tag in tags.iter().take(4) {
            out.push(q_hash(tag));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat_session::{ChatEnvironment, OntologyScopeSummary};

    fn env() -> ChatEnvironment {
        ChatEnvironment {
            session_id: "s".into(),
            active_model_profile_id: 0,
            ontology_ids: vec!["snomed".into(), "legal-commons".into(), "wordnet".into()],
            prior_session_ids: vec![],
            graph_scope_hashes: vec![],
            lexicon_prefixes: vec![],
            capability_briefing: String::new(),
            model_id: None,
            model_modality: "text".into(),
            context_window: 4096,
            engine_capabilities: vec![],
            installed_qapps: vec![],
            ontology_summaries: vec![
                OntologyScopeSummary {
                    id: "snomed".into(),
                    name: "SNOMED Clinical Terms".into(),
                    quin_count: 10,
                    q42_path: "snomed.q42".into(),
                    domain: Some("health".into()),
                    tags: Some(vec!["medical".into(), "clinical".into()]),
                    source: None,
                },
                OntologyScopeSummary {
                    id: "legal-commons".into(),
                    name: "Legal Commons".into(),
                    quin_count: 10,
                    q42_path: "legal.q42".into(),
                    domain: Some("legal".into()),
                    tags: Some(vec!["contract".into(), "rights".into()]),
                    source: None,
                },
                OntologyScopeSummary {
                    id: "wordnet".into(),
                    name: "WordNet".into(),
                    quin_count: 10,
                    q42_path: "wordnet.q42".into(),
                    domain: Some("lexical".into()),
                    tags: Some(vec!["language".into()]),
                    source: None,
                },
            ],
            daemon_reachable: false,
            session_kind: crate::chat_session::SessionKind::Solo,
            participants: vec![],
            graph_mutation: false,
            axiom_bounds: crate::context_binding::AxiomBounds::default(),
        }
    }

    #[test]
    fn routes_medical_prompt_to_health_ontology() {
        let decision = route_prompt_to_ontologies(&env(), "What does this patient fever indicate?");
        assert!(decision.ontology_ids.iter().any(|id| id == "snomed"));
        assert!(decision.context_namespaces.contains(&q_hash("health")));
    }

    #[test]
    fn routes_guardianship_prompt_to_legal_ontology() {
        let decision =
            route_prompt_to_ontologies(&env(), "Draft a guardianship consent agreement.");
        assert!(decision.ontology_ids.iter().any(|id| id == "legal-commons"));
        assert!(decision.context_namespaces.contains(&q_hash("legal")));
    }
}
