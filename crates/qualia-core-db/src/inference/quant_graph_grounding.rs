//! QuantGraph mode — selective grounding / repair against a **NQuin fact graph**.
//!
//! Aggressive quantization can drop nuance; this module is the neuro-symbolic
//! counterpart: after the LLM proposes text, high-stakes fact patterns are checked
//! against an in-process fact graph (NQuin triples + human repair strings).
//!
//! Gated by [`crate::inference_modes::quant_graph_grounding_enabled`].
//! Does not run in Portable or CudaTc modes.
//!
//! # Graph model
//! Each fact is a parity-valid `NQuin`:
//! - subject  = place / entity (`q_hash`)
//! - predicate = `q42:capitalOf` (or other relation)
//! - object   = answer entity hash
//! - context  = `q42:grounding-fact`
//! plus cold-path strings for prompt needles, answer tokens, and repair text.
//!
//! Expand later: load from SPARQL / Wellfair graph / CBOR-LD package.

use std::sync::{Mutex, OnceLock};

use crate::{q_hash, NQuin};

/// Predicate IRI for capital facts (hashed into quins).
pub const P_CAPITAL_OF: &str = "https://ns.webizen.org/q42/capitalOf";
/// Graph context for all grounding facts.
pub const CTX_GROUNDING: &str = "https://ns.webizen.org/q42/grounding-fact";

/// Result of a grounding pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GroundingResult {
    pub text: String,
    pub repaired: bool,
    /// Stable reason id (e.g. `capital_france`).
    pub reason: Option<String>,
    /// Object entity hash when a fact matched (for provenance).
    pub object_hash: Option<u64>,
}

/// Cold-path fact: NQuin + strings for matching and repair.
#[derive(Debug, Clone)]
pub struct GroundingFact {
    pub quin: NQuin,
    /// All must appear in the prompt (lowercase).
    pub prompt_needles: Vec<String>,
    /// If any appears in the answer, treat as grounded.
    pub answer_ok: Vec<String>,
    pub repair: String,
    pub reason: String,
}

fn make_quin(subject_iri: &str, object_iri: &str) -> NQuin {
    let subject = q_hash(subject_iri);
    let predicate = q_hash(P_CAPITAL_OF);
    let object = q_hash(object_iri);
    let context = q_hash(CTX_GROUNDING);
    let metadata = 0u64;
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

fn seed_defaults() -> Vec<GroundingFact> {
    // Prefer bundled TSV if present; else embedded minimal set.
    if let Some(facts) = try_load_bundled_facts() {
        if !facts.is_empty() {
            return facts;
        }
    }
    vec![
        GroundingFact {
            quin: make_quin(
                "https://example.org/place/France",
                "https://example.org/place/Paris",
            ),
            prompt_needles: vec!["capital".into(), "france".into()],
            answer_ok: vec!["paris".into()],
            repair: "The capital of France is Paris.".into(),
            reason: "capital_france".into(),
        },
        GroundingFact {
            quin: make_quin(
                "https://example.org/place/Australia",
                "https://example.org/place/Canberra",
            ),
            prompt_needles: vec!["capital".into(), "australia".into()],
            answer_ok: vec!["canberra".into()],
            repair: "The capital of Australia is Canberra.".into(),
            reason: "capital_australia".into(),
        },
        GroundingFact {
            quin: make_quin(
                "https://example.org/place/Japan",
                "https://example.org/place/Tokyo",
            ),
            prompt_needles: vec!["capital".into(), "japan".into()],
            answer_ok: vec!["tokyo".into()],
            repair: "The capital of Japan is Tokyo.".into(),
            reason: "capital_japan".into(),
        },
    ]
}

/// Parse TSV lines: reason \\t needles; \\t answer_ok; \\t repair \\t place_iri \\t city_iri
pub fn parse_facts_tsv(text: &str) -> Vec<GroundingFact> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        let cols: Vec<&str> = line.split('\t').collect();
        if cols.len() < 6 {
            continue;
        }
        let reason = cols[0].trim().to_string();
        let needles: Vec<String> = cols[1]
            .split(';')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let answer_ok: Vec<String> = cols[2]
            .split(';')
            .map(|s| s.trim().to_ascii_lowercase())
            .filter(|s| !s.is_empty())
            .collect();
        let repair = cols[3].trim().to_string();
        let place_iri = cols[4].trim();
        let city_iri = cols[5].trim();
        if needles.is_empty() || answer_ok.is_empty() || repair.is_empty() {
            continue;
        }
        out.push(GroundingFact {
            quin: make_quin(place_iri, city_iri),
            prompt_needles: needles,
            answer_ok,
            repair,
            reason,
        });
    }
    out
}

/// Load facts from a TSV file path. Returns count added (merge by reason).
pub fn load_facts_from_tsv(path: &std::path::Path) -> Result<usize, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let facts = parse_facts_tsv(&text);
    let n = facts.len();
    for f in facts {
        register_fact(f);
    }
    Ok(n)
}

fn try_load_bundled_facts() -> Option<Vec<GroundingFact>> {
    for candidate in bundled_fact_candidates() {
        if candidate.is_file() {
            if let Ok(text) = std::fs::read_to_string(&candidate) {
                let facts = parse_facts_tsv(&text);
                if !facts.is_empty() {
                    log::info!(
                        "quant_graph|seed|bundled|{}|facts={}",
                        candidate.display(),
                        facts.len()
                    );
                    return Some(facts);
                }
            }
        }
    }
    None
}

fn bundled_fact_candidates() -> Vec<std::path::PathBuf> {
    let mut v = Vec::new();
    if let Ok(p) = std::env::var("QUALIA_GROUNDING_FACTS") {
        v.push(std::path::PathBuf::from(p));
    }
    // Crate-relative and workspace-relative paths.
    let manifest = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    v.push(manifest.join("../../bundled/grounding/facts.tsv"));
    v.push(manifest.join("../../../bundled/grounding/facts.tsv"));
    if let Ok(cwd) = std::env::current_dir() {
        v.push(cwd.join("bundled/grounding/facts.tsv"));
    }
    v
}

/// Re-seed from bundled TSV (or QUALIA_GROUNDING_FACTS), replacing current store.
pub fn seed_facts_from_bundled() -> usize {
    let facts = try_load_bundled_facts().unwrap_or_else(seed_defaults);
    let n = facts.len();
    if let Ok(mut g) = fact_store().lock() {
        *g = facts;
    }
    n
}

fn fact_store() -> &'static Mutex<Vec<GroundingFact>> {
    static STORE: OnceLock<Mutex<Vec<GroundingFact>>> = OnceLock::new();
    STORE.get_or_init(|| Mutex::new(seed_defaults()))
}

/// Number of facts currently registered.
pub fn fact_count() -> usize {
    fact_store().lock().map(|g| g.len()).unwrap_or(0)
}

/// Export all fact quins (for tests / graph dump / later SPARQL seed).
pub fn export_fact_quins(out: &mut [NQuin]) -> usize {
    let Ok(guard) = fact_store().lock() else {
        return 0;
    };
    let n = guard.len().min(out.len());
    for (i, f) in guard.iter().take(n).enumerate() {
        out[i] = f.quin;
    }
    n
}

/// Register (or replace by reason id) a grounding fact. Cold path.
pub fn register_fact(fact: GroundingFact) {
    if let Ok(mut g) = fact_store().lock() {
        if let Some(i) = g.iter().position(|f| f.reason == fact.reason) {
            g[i] = fact;
        } else {
            g.push(fact);
        }
    }
}

/// Convenience: capital-of fact from IRIs + match strings.
pub fn register_capital_fact(
    place_iri: &str,
    city_iri: &str,
    place_needle: &str,
    city_needle: &str,
    repair: &str,
    reason: &str,
) {
    register_fact(GroundingFact {
        quin: make_quin(place_iri, city_iri),
        prompt_needles: vec!["capital".into(), place_needle.to_ascii_lowercase()],
        answer_ok: vec![city_needle.to_ascii_lowercase()],
        repair: repair.to_string(),
        reason: reason.to_string(),
    });
}

/// Reset store to seed defaults (tests) — re-reads bundled TSV when present.
pub fn reset_fact_store_to_defaults() {
    if let Ok(mut g) = fact_store().lock() {
        *g = seed_defaults();
    }
}

/// Apply quant-graph grounding when the mode is active; otherwise identity.
pub fn maybe_ground_generation(prompt: &str, text: &str) -> GroundingResult {
    if !crate::inference_modes::quant_graph_grounding_enabled() {
        return GroundingResult {
            text: text.to_string(),
            repaired: false,
            reason: None,
            object_hash: None,
        };
    }
    ground_generation(prompt, text)
}

/// Unconditional grounding pass (tests / CLI).
pub fn ground_generation(prompt: &str, text: &str) -> GroundingResult {
    let p = prompt.to_ascii_lowercase();
    let a = text.to_ascii_lowercase();
    let Ok(guard) = fact_store().lock() else {
        return GroundingResult {
            text: text.to_string(),
            repaired: false,
            reason: None,
            object_hash: None,
        };
    };
    for fact in guard.iter() {
        if !fact.prompt_needles.iter().all(|s| p.contains(s.as_str())) {
            continue;
        }
        if fact.answer_ok.iter().any(|s| a.contains(s.as_str())) {
            return GroundingResult {
                text: text.to_string(),
                repaired: false,
                reason: Some(fact.reason.clone()),
                object_hash: Some(fact.quin.object),
            };
        }
        log::info!(
            "LLM_MODE|quant-graph|repair|{}|object={:#x}",
            fact.reason,
            fact.quin.object
        );
        return GroundingResult {
            text: fact.repair.clone(),
            repaired: true,
            reason: Some(fact.reason.clone()),
            object_hash: Some(fact.quin.object),
        };
    }
    GroundingResult {
        text: text.to_string(),
        repaired: false,
        reason: None,
        object_hash: None,
    }
}

/// Lookup object hash for a subject entity if a capital fact exists (graph API).
pub fn lookup_capital_object(place_iri: &str) -> Option<u64> {
    let s = q_hash(place_iri);
    let p = q_hash(P_CAPITAL_OF);
    let Ok(guard) = fact_store().lock() else {
        return None;
    };
    guard
        .iter()
        .find(|f| f.quin.subject == s && f.quin.predicate == p)
        .map(|f| f.quin.object)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inference_modes::{set_inference_mode, InferenceMode};

    #[test]
    fn repairs_france_capital_when_wrong() {
        reset_fact_store_to_defaults();
        let g = ground_generation(
            "What is the capital of France?",
            "Question: What is the capital of France? A) Lyon B) Marseille",
        );
        assert!(g.repaired);
        assert!(g.text.to_ascii_lowercase().contains("paris"));
        assert_eq!(g.reason.as_deref(), Some("capital_france"));
        assert!(g.object_hash.is_some());
    }

    #[test]
    fn leaves_correct_answer() {
        reset_fact_store_to_defaults();
        let g = ground_generation(
            "What is the capital of France?",
            "The capital of France is Paris.",
        );
        assert!(!g.repaired);
        assert!(g.text.contains("Paris"));
    }

    #[test]
    fn maybe_ground_respects_mode() {
        if std::env::var("QUALIA_INFERENCE_MODE").is_ok() {
            return;
        }
        reset_fact_store_to_defaults();
        set_inference_mode(InferenceMode::Portable);
        let g = maybe_ground_generation("What is the capital of France?", "I do not know.");
        assert!(!g.repaired);
        set_inference_mode(InferenceMode::QuantGraph);
        let g2 = maybe_ground_generation("What is the capital of France?", "I do not know.");
        assert!(g2.repaired);
        set_inference_mode(InferenceMode::Portable);
    }

    #[test]
    fn fact_quins_have_valid_parity() {
        reset_fact_store_to_defaults();
        let mut buf = [NQuin {
            subject: 0,
            predicate: 0,
            object: 0,
            context: 0,
            metadata: 0,
            parity: 0,
        }; 16];
        let n = export_fact_quins(&mut buf);
        assert!(n >= 3);
        for q in &buf[..n] {
            let fold = q.subject ^ q.predicate ^ q.object ^ q.context ^ q.metadata;
            assert_eq!(q.parity, fold);
            assert_eq!(q.predicate, q_hash(P_CAPITAL_OF));
            assert_eq!(q.context, q_hash(CTX_GROUNDING));
        }
    }

    #[test]
    fn register_and_lookup() {
        reset_fact_store_to_defaults();
        register_capital_fact(
            "https://example.org/place/Italy",
            "https://example.org/place/Rome",
            "italy",
            "rome",
            "The capital of Italy is Rome.",
            "capital_italy",
        );
        assert_eq!(
            lookup_capital_object("https://example.org/place/Italy"),
            Some(q_hash("https://example.org/place/Rome"))
        );
        let g = ground_generation("capital of Italy?", "Milan maybe");
        assert!(g.repaired);
        assert!(g.text.contains("Rome"));
    }
}
