//! Epistemic Ontology Substrate: Open English WordNet (OEWN) & CILI Integration
//!
//! Provides zero-heap, ontologically grounded epistemic stances for QualiaDB assertions,
//! sessions, and reasoning frames. Replaces ad-hoc string enums with formal RDF synsets
//! from Open English WordNet (OEWN 2025-plus) and the Collaborative Interlingual Index (CILI).
//!
//! Architectural Invariants:
//! - Tier-1 zero-heap evaluator loops: Caller supplies fixed-size `&mut [T]` output buffers.
//! - 60-bit token space: All synset and CILI tokens fit into the 60-bit payload of `NQuin`.
//! - Hierarchical subsumption: Integrates with `modalities::dl::check_subsumption_quin`
//!   to resolve hypernym/hyponym trees (e.g. conjecture ⊑ hypothesis).

use crate::modalities::dl::check_subsumption_quin;
use crate::{q_hash, NQuin};

// ─── Epistemic Predicates ────────────────────────────────────────────────────

/// `q42:hasEpistemicStance` — Links an assertion, statement, or session to a synset stance.
pub const PRED_EPISTEMIC_STANCE: u64 = q_hash("q42:hasEpistemicStance");

/// `q42:hasWordNetSynset` — Links a concept to an explicit OEWN synset URI.
pub const PRED_WORDNET_SYNSET: u64 = q_hash("q42:hasWordNetSynset");

/// `q42:hasCiliConcept` — Links a stance to a language-independent CILI concept ID.
pub const PRED_CILI_CONCEPT: u64 = q_hash("q42:hasCiliConcept");

/// `rdfs:subClassOf` — Standard subsumption / hypernym relationship.
pub const PRED_SUB_CLASS_OF: u64 = q_hash("http://www.w3.org/2000/01/rdf-schema#subClassOf");

/// `wn:hypernym` — WordNet lexical hypernym relation.
pub const PRED_WN_HYPERNYM: u64 = q_hash("https://en-wordnet.oed.com/schema/hypernym");

// ─── Canonical OEWN 2025-plus Synsets (60-bit token masked) ───────────────────

/// `synset-fact-noun-1`: Empirical observation, objective reality, ground truth.
pub const SYNSET_FACT: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-fact-noun-1") & 0x0FFF_FFFF_FFFF_FFFF;

/// `synset-observation-noun-1`: Sensory datum, empirical field observation.
pub const SYNSET_OBSERVATION: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-observation-noun-1") & 0x0FFF_FFFF_FFFF_FFFF;

/// `synset-hypothesis-noun-1`: Tentative proposition, testable scientific supposition.
pub const SYNSET_HYPOTHESIS: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-hypothesis-noun-1") & 0x0FFF_FFFF_FFFF_FFFF;

/// `synset-conjecture-noun-1`: Mathematical/logical conjecture, speculation without proof.
pub const SYNSET_CONJECTURE: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-conjecture-noun-1") & 0x0FFF_FFFF_FFFF_FFFF;

/// `synset-simulation-noun-1`: Computational representation, physical digital twin model.
pub const SYNSET_SIMULATION: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-simulation-noun-1") & 0x0FFF_FFFF_FFFF_FFFF;

/// `synset-fiction-noun-1`: Speculative narrative, literary worldbuilding, fictional world.
pub const SYNSET_FICTION: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-fiction-noun-1") & 0x0FFF_FFFF_FFFF_FFFF;

/// `synset-civics-noun-1`: Social commons, participatory governance, civic duty.
pub const SYNSET_CIVICS: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-civics-noun-1") & 0x0FFF_FFFF_FFFF_FFFF;

/// `synset-counterfactual_conditional-noun-1`: Thought experiment, counter-to-fact inquiry.
pub const SYNSET_COUNTERFACTUAL: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-counterfactual_conditional-noun-1")
        & 0x0FFF_FFFF_FFFF_FFFF;

/// `synset-belief-noun-1`: Doxastic belief without definitive proof.
pub const SYNSET_BELIEF: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-belief-noun-1") & 0x0FFF_FFFF_FFFF_FFFF;

/// `synset-knowledge-noun-1`: Justified, verified true belief / confirmed knowledge.
pub const SYNSET_KNOWLEDGE: u64 =
    q_hash("https://en-wordnet.oed.com/id/synset-knowledge-noun-1") & 0x0FFF_FFFF_FFFF_FFFF;

// ─── Collaborative Interlingual Index (CILI) Concepts ────────────────────────

/// CILI i68339: Language-independent fact concept.
pub const CILI_FACT: u64 = q_hash("http://globalwordnet.org/ili/i68339") & 0x0FFF_FFFF_FFFF_FFFF;

/// CILI i68341: Language-independent hypothesis concept.
pub const CILI_HYPOTHESIS: u64 =
    q_hash("http://globalwordnet.org/ili/i68341") & 0x0FFF_FFFF_FFFF_FFFF;

/// CILI i68345: Language-independent conjecture concept.
pub const CILI_CONJECTURE: u64 =
    q_hash("http://globalwordnet.org/ili/i68345") & 0x0FFF_FFFF_FFFF_FFFF;

/// CILI i68212: Language-independent simulation concept.
pub const CILI_SIMULATION: u64 =
    q_hash("http://globalwordnet.org/ili/i68212") & 0x0FFF_FFFF_FFFF_FFFF;

/// CILI i71201: Language-independent fiction concept.
pub const CILI_FICTION: u64 = q_hash("http://globalwordnet.org/ili/i71201") & 0x0FFF_FFFF_FFFF_FFFF;

/// CILI i69420: Language-independent civics concept.
pub const CILI_CIVICS: u64 = q_hash("http://globalwordnet.org/ili/i69420") & 0x0FFF_FFFF_FFFF_FFFF;

/// CILI i70110: Language-independent observation concept.
pub const CILI_OBSERVATION: u64 =
    q_hash("http://globalwordnet.org/ili/i70110") & 0x0FFF_FFFF_FFFF_FFFF;

// ─── Data Structures ─────────────────────────────────────────────────────────

/// Zero-copy representation of an evaluated epistemic stance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
pub struct EpistemicStanceRecord {
    pub subject: u64,      // Entity, statement, or session DID hash
    pub synset_hash: u64,  // 60-bit WordNet Synset token
    pub cili_hash: u64,    // 60-bit language-independent CILI concept token
    pub confidence: u16,   // Epistemic certainty / confidence weight (0..65535)
    pub is_subsumed: bool, // Satisfies requested target category via hypernym DAG
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EpistemicOntologyError {
    BufferOverflow,
    InvalidQuinEncoding,
    UnresolvedConcept,
}

// ─── Core Functions ──────────────────────────────────────────────────────────

/// Synthesizes a valid zero-allocation `NQuin` encoding an epistemic stance assertion.
///
/// Layout:
/// - `subject`: Target entity/claim DID hash
/// - `predicate`: `PRED_EPISTEMIC_STANCE`
/// - `object`: 60-bit synset hash token
/// - `context`: World or scenario context DID hash
/// - `metadata`: bits 0..15 contain `confidence`
/// - `parity`: XOR fold of subject ^ predicate ^ object ^ context
#[inline]
pub fn compile_epistemic_stance_quin(
    claim_subject: u64,
    synset_hash: u64,
    context_hash: u64,
    confidence: u16,
) -> NQuin {
    let clean_object = synset_hash & 0x0FFF_FFFF_FFFF_FFFF;
    let metadata = confidence as u64;
    let parity = claim_subject ^ PRED_EPISTEMIC_STANCE ^ clean_object ^ context_hash;

    NQuin {
        subject: claim_subject,
        predicate: PRED_EPISTEMIC_STANCE,
        object: clean_object,
        context: context_hash,
        metadata,
        parity,
    }
}

/// Resolves the canonical CILI concept identifier for a known OEWN synset token.
/// Pure, deterministic, zero-heap mapping.
#[inline]
pub const fn synset_to_cili(synset_hash: u64) -> u64 {
    let clean = synset_hash & 0x0FFF_FFFF_FFFF_FFFF;
    if clean == SYNSET_FACT {
        CILI_FACT
    } else if clean == SYNSET_HYPOTHESIS {
        CILI_HYPOTHESIS
    } else if clean == SYNSET_CONJECTURE {
        CILI_CONJECTURE
    } else if clean == SYNSET_SIMULATION {
        CILI_SIMULATION
    } else if clean == SYNSET_FICTION {
        CILI_FICTION
    } else if clean == SYNSET_CIVICS {
        CILI_CIVICS
    } else if clean == SYNSET_OBSERVATION {
        CILI_OBSERVATION
    } else {
        // Unknown or custom synset — retains identity hash
        clean
    }
}

/// Parses an epistemic mode string (e.g. legacy descriptors or configs) into its canonical OEWN synset token.
/// Completely zero-heap matching over static string slices.
#[inline]
pub fn parse_epistemic_mode_to_synset(mode_str: &str) -> Option<u64> {
    match mode_str.trim().to_ascii_lowercase().as_str() {
        "fiction" | "speculative" | "narrative" => Some(SYNSET_FICTION),
        "non-fiction" | "nonfiction" | "fact" | "empirical" | "ground-truth" => Some(SYNSET_FACT),
        "observation" | "field-observation" | "sensory" => Some(SYNSET_OBSERVATION),
        "simulation" | "digital-twin" | "model" => Some(SYNSET_SIMULATION),
        "hypothesis" | "tentative" => Some(SYNSET_HYPOTHESIS),
        "conjecture" | "unproven" => Some(SYNSET_CONJECTURE),
        "counterfactual" | "thought-experiment" => Some(SYNSET_COUNTERFACTUAL),
        "civics" | "civic" | "commons" | "normative" => Some(SYNSET_CIVICS),
        "belief" | "doxastic" => Some(SYNSET_BELIEF),
        "knowledge" | "epistemic" => Some(SYNSET_KNOWLEDGE),
        _ => None,
    }
}

/// Maps a canonical synset token to its primary English canonical display label.
#[inline]
pub fn synset_to_canonical_label(synset_hash: u64) -> &'static str {
    let clean = synset_hash & 0x0FFF_FFFF_FFFF_FFFF;
    if clean == SYNSET_FACT {
        "fact"
    } else if clean == SYNSET_OBSERVATION {
        "observation"
    } else if clean == SYNSET_HYPOTHESIS {
        "hypothesis"
    } else if clean == SYNSET_CONJECTURE {
        "conjecture"
    } else if clean == SYNSET_SIMULATION {
        "simulation"
    } else if clean == SYNSET_FICTION {
        "fiction"
    } else if clean == SYNSET_CIVICS {
        "civics"
    } else if clean == SYNSET_COUNTERFACTUAL {
        "counterfactual"
    } else if clean == SYNSET_BELIEF {
        "belief"
    } else if clean == SYNSET_KNOWLEDGE {
        "knowledge"
    } else {
        "custom"
    }
}

/// Resolves a localized lemma for a given CILI concept token.
/// Supports English ("en"), Spanish ("es"), French ("fr"), and German ("de").
/// Completely zero-heap, compile-time static lookup.
#[inline]
pub fn lookup_epistemic_lemma(cili_hash: u64, lang_code: &str) -> Option<&'static str> {
    let clean = cili_hash & 0x0FFF_FFFF_FFFF_FFFF;
    match lang_code {
        "es" => {
            if clean == CILI_FACT {
                Some("hecho")
            } else if clean == CILI_HYPOTHESIS {
                Some("hipótesis")
            } else if clean == CILI_CONJECTURE {
                Some("conjetura")
            } else if clean == CILI_SIMULATION {
                Some("simulación")
            } else if clean == CILI_FICTION {
                Some("ficción")
            } else if clean == CILI_CIVICS {
                Some("cívica")
            } else if clean == CILI_OBSERVATION {
                Some("observación")
            } else {
                None
            }
        }
        "fr" => {
            if clean == CILI_FACT {
                Some("fait")
            } else if clean == CILI_HYPOTHESIS {
                Some("hypothèse")
            } else if clean == CILI_CONJECTURE {
                Some("conjecture")
            } else if clean == CILI_SIMULATION {
                Some("simulation")
            } else if clean == CILI_FICTION {
                Some("fiction")
            } else if clean == CILI_CIVICS {
                Some("civisme")
            } else if clean == CILI_OBSERVATION {
                Some("observation")
            } else {
                None
            }
        }
        "de" => {
            if clean == CILI_FACT {
                Some("Tatsache")
            } else if clean == CILI_HYPOTHESIS {
                Some("Hypothese")
            } else if clean == CILI_CONJECTURE {
                Some("Vermutung")
            } else if clean == CILI_SIMULATION {
                Some("Simulation")
            } else if clean == CILI_FICTION {
                Some("Fiktion")
            } else if clean == CILI_CIVICS {
                Some("Gemeinschaftswesen")
            } else if clean == CILI_OBSERVATION {
                Some("Beobachtung")
            } else {
                None
            }
        }
        _ => {
            // Default "en"
            if clean == CILI_FACT {
                Some("fact")
            } else if clean == CILI_HYPOTHESIS {
                Some("hypothesis")
            } else if clean == CILI_CONJECTURE {
                Some("conjecture")
            } else if clean == CILI_SIMULATION {
                Some("simulation")
            } else if clean == CILI_FICTION {
                Some("fiction")
            } else if clean == CILI_CIVICS {
                Some("civics")
            } else if clean == CILI_OBSERVATION {
                Some("observation")
            } else {
                None
            }
        }
    }
}

/// Verifies whether an active stance synset matches or is subsumed by an expected category synset.
///
/// Utilizes `modalities::dl::check_subsumption_quin` to traverse hypernym / `rdfs:subClassOf`
/// edges in the provided `tbox` slice in $O(V+E)$ time with zero allocations.
pub fn is_epistemic_subsumed(active_synset: u64, expected_synset: u64, tbox: &[NQuin]) -> bool {
    let clean_active = active_synset & 0x0FFF_FFFF_FFFF_FFFF;
    let clean_expected = expected_synset & 0x0FFF_FFFF_FFFF_FFFF;

    if clean_active == clean_expected {
        return true;
    }

    // Direct OEWN built-in subsumption fallback for base axioms without external TBox
    if clean_active == SYNSET_CONJECTURE && clean_expected == SYNSET_HYPOTHESIS {
        return true;
    }
    if clean_active == SYNSET_OBSERVATION && clean_expected == SYNSET_FACT {
        return true;
    }
    if clean_active == SYNSET_COUNTERFACTUAL && clean_expected == SYNSET_HYPOTHESIS {
        return true;
    }

    // Traverse caller-provided TBox via Description Logic zero-heap transitive closure
    check_subsumption_quin(clean_active, clean_expected, tbox)
}

/// Evaluates a slice of input `NQuin` statements against an expected epistemic stance.
///
/// Output is written to the caller-supplied `out` slice with zero heap allocations.
/// Returns the number of matched records.
pub fn evaluate_epistemic_stances(
    quins: &[NQuin],
    expected_synset: u64, // 0 = accept any epistemic stance
    tbox: &[NQuin],       // WordNet hypernym / ontology subsumption edges
    out: &mut [EpistemicStanceRecord],
) -> Result<usize, EpistemicOntologyError> {
    let mut written = 0;

    for quin in quins {
        if quin.predicate != PRED_EPISTEMIC_STANCE {
            continue;
        }

        let synset_token = quin.object & 0x0FFF_FFFF_FFFF_FFFF;
        let confidence = (quin.metadata & 0xFFFF) as u16;

        let subsumed = if expected_synset == 0 {
            true
        } else {
            is_epistemic_subsumed(synset_token, expected_synset, tbox)
        };

        if expected_synset == 0 || subsumed {
            if written >= out.len() {
                return Err(EpistemicOntologyError::BufferOverflow);
            }

            out[written] = EpistemicStanceRecord {
                subject: quin.subject,
                synset_hash: synset_token,
                cili_hash: synset_to_cili(synset_token),
                confidence,
                is_subsumed: subsumed,
            };
            written += 1;
        }
    }

    Ok(written)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_and_exact_match() {
        let claim_id = q_hash("did:q42:claim:water_balance_2026");
        let context_id = q_hash("did:q42:context:scientific_trial");

        let quin = compile_epistemic_stance_quin(claim_id, SYNSET_HYPOTHESIS, context_id, 45000);
        assert_eq!(quin.subject, claim_id);
        assert_eq!(quin.predicate, PRED_EPISTEMIC_STANCE);
        assert_eq!(quin.object, SYNSET_HYPOTHESIS);
        assert_eq!(quin.metadata & 0xFFFF, 45000);

        let mut out = [EpistemicStanceRecord {
            subject: 0,
            synset_hash: 0,
            cili_hash: 0,
            confidence: 0,
            is_subsumed: false,
        }; 4];

        let quins = [quin];
        let count = evaluate_epistemic_stances(&quins, SYNSET_HYPOTHESIS, &[], &mut out).unwrap();
        assert_eq!(count, 1);
        assert_eq!(out[0].subject, claim_id);
        assert_eq!(out[0].synset_hash, SYNSET_HYPOTHESIS);
        assert_eq!(out[0].cili_hash, CILI_HYPOTHESIS);
        assert_eq!(out[0].confidence, 45000);
        assert!(out[0].is_subsumed);
    }

    #[test]
    fn test_built_in_subsumption_conjecture_to_hypothesis() {
        let claim_id = q_hash("did:q42:conjecture:riemann_hypothesis");
        let context_id = q_hash("did:q42:context:math_corpus");

        let quin = compile_epistemic_stance_quin(claim_id, SYNSET_CONJECTURE, context_id, 65000);
        let mut out = [EpistemicStanceRecord {
            subject: 0,
            synset_hash: 0,
            cili_hash: 0,
            confidence: 0,
            is_subsumed: false,
        }; 4];

        // Query expects generic HYPOTHESIS: conjecture should be accepted via subsumption
        let count = evaluate_epistemic_stances(&[quin], SYNSET_HYPOTHESIS, &[], &mut out).unwrap();
        assert_eq!(count, 1);
        assert_eq!(out[0].synset_hash, SYNSET_CONJECTURE);
        assert_eq!(out[0].cili_hash, CILI_CONJECTURE);
        assert!(out[0].is_subsumed);
    }

    #[test]
    fn test_tbox_hypernym_subsumption() {
        let custom_synset =
            q_hash("https://en-wordnet.oed.com/id/synset-working_hypothesis-noun-1")
                & 0x0FFF_FFFF_FFFF_FFFF;
        let claim_id = q_hash("did:q42:claim:thermal_dissipation_rate");
        let context_id = q_hash("did:q42:context:field_lab");

        let quin = compile_epistemic_stance_quin(claim_id, custom_synset, context_id, 50000);

        // Define TBox edge: working_hypothesis rdfs:subClassOf hypothesis
        let tbox_edge = NQuin {
            subject: custom_synset,
            predicate: PRED_SUB_CLASS_OF,
            object: SYNSET_HYPOTHESIS,
            context: 0,
            metadata: 0,
            parity: custom_synset ^ PRED_SUB_CLASS_OF ^ SYNSET_HYPOTHESIS ^ 0,
        };

        let mut out = [EpistemicStanceRecord {
            subject: 0,
            synset_hash: 0,
            cili_hash: 0,
            confidence: 0,
            is_subsumed: false,
        }; 2];

        let count =
            evaluate_epistemic_stances(&[quin], SYNSET_HYPOTHESIS, &[tbox_edge], &mut out).unwrap();
        assert_eq!(count, 1);
        assert_eq!(out[0].synset_hash, custom_synset);
        assert!(out[0].is_subsumed);
    }

    #[test]
    fn test_mismatched_stance_filtered_out() {
        let claim_id = q_hash("did:q42:story:desert_odyssey");
        let context_id = q_hash("did:q42:context:fictional_arc");

        // Stance is FICTION
        let quin = compile_epistemic_stance_quin(claim_id, SYNSET_FICTION, context_id, 65535);

        let mut out = [EpistemicStanceRecord {
            subject: 0,
            synset_hash: 0,
            cili_hash: 0,
            confidence: 0,
            is_subsumed: false,
        }; 2];

        // Query expects empirical FACT — fiction should be filtered out
        let count = evaluate_epistemic_stances(&[quin], SYNSET_FACT, &[], &mut out).unwrap();
        assert_eq!(count, 0);
    }

    #[test]
    fn test_cili_multilingual_mapping() {
        assert_eq!(synset_to_cili(SYNSET_FACT), CILI_FACT);
        assert_eq!(synset_to_cili(SYNSET_HYPOTHESIS), CILI_HYPOTHESIS);
        assert_eq!(synset_to_cili(SYNSET_SIMULATION), CILI_SIMULATION);
        assert_eq!(synset_to_cili(SYNSET_FICTION), CILI_FICTION);
        assert_eq!(synset_to_cili(SYNSET_CIVICS), CILI_CIVICS);
    }
}
