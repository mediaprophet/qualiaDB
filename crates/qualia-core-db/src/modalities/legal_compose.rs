//! Composition wires (§17, §19, §26 — legal_logic.md) — wiring existing real primitives into
//! the legal-logic path:
//!   * **§17 ZK-gated eligibility** — an obligation/permission gated on a zero-knowledge proof
//!     (the proof itself is `zk_proofs::ZkProofSystem`, real Groth16); this is the deontic gate
//!     over its verification result + selective disclosure of credential claims.
//!   * **§26 proportionality** — composes the CAS (`specialized_libs::symbolic_algebra`):
//!     differentiate a harm expression, evaluate the marginal harm, and require it strictly
//!     below the advantage (the legal proportionality test).
//!   * **§19 sense-translation gate** — enforces the Curation Directive on cross-cultural
//!     mapping: the machine may propose `skos:closeMatch`; only a human attests `skos:exactMatch`;
//!     an untranslatable concept routes to human review (never force-flattened).

// §26 proportionality composes the CAS, which lives in native-only `specialized_libs`. §17/§19
// below have no such dependency, so only the §26 functions are gated to native.
#[cfg(not(target_arch = "wasm32"))]
use crate::specialized_libs::symbolic_algebra::{differentiate, parse};
use std::collections::HashMap;

// ─── §17 ZK-gated eligibility ─────────────────────────────────────────────────────

/// Whether a ZK-gated obligation/permission is eligible to apply.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Eligibility {
    /// The proof verified — the gated norm applies (the private witness stays hidden).
    Eligible,
    /// The proof did not verify — flagged `policy:claimedIdentityUnverifiable`.
    Unverifiable,
}

/// Gate an obligation on a ZK proof's verification result (`O(p | ZK(...))`). The proof is
/// produced/verified by `zk_proofs::ZkProofSystem` (real Groth16); this maps that boolean to the
/// deontic eligibility, keeping the attribute value itself private.
#[inline]
pub fn zk_eligibility(proof_verified: bool) -> Eligibility {
    if proof_verified {
        Eligibility::Eligible
    } else {
        Eligibility::Unverifiable
    }
}

/// Selective disclosure: reveal only the chosen `reveal` claim ids out of `all_claims`, into
/// `out` (the rest of the credential graph stays undisclosed). Returns the count. Zero-heap.
pub fn selective_disclosure(all_claims: &[u64], reveal: &[u64], out: &mut [u64]) -> usize {
    let mut n = 0usize;
    for &c in all_claims {
        if reveal.contains(&c) {
            if n >= out.len() {
                break;
            }
            out[n] = c;
            n += 1;
        }
    }
    n
}

// ─── §26 Proportionality (composes the CAS) ───────────────────────────────────────

/// The marginal harm `d/d(wrt) harm_expr` evaluated at `at` — parses + differentiates +
/// evaluates via `symbolic_algebra`. `None` if the expression won't parse/evaluate.
#[cfg(not(target_arch = "wasm32"))]
pub fn marginal_harm(harm_expr: &str, wrt: &str, at: f64) -> Option<f64> {
    let expr = parse(harm_expr).ok()?;
    let d = differentiate(&expr, wrt);
    let mut env = HashMap::new();
    env.insert(wrt.to_string(), at);
    d.eval(&env)
}

/// **Proportionality test**: an act is proportionate iff its marginal harm is strictly less
/// than the `advantage` it secures (`∂Harm/∂x < Advantage`). `None` if the harm model is
/// unparseable. The legal proportionality / necessity calculus.
#[cfg(not(target_arch = "wasm32"))]
pub fn proportionality_met(harm_expr: &str, wrt: &str, at: f64, advantage: f64) -> Option<bool> {
    Some(marginal_harm(harm_expr, wrt, at)? < advantage)
}

// ─── §19 Sense-translation gate (Curation Directive) ──────────────────────────────

/// The status of a cross-cultural / cross-lexical mapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchStatus {
    /// Machine-proposed relatedness — auto-assertable (`skos:closeMatch`).
    CloseMatch,
    /// Human-attested strict equivalence (`skos:exactMatch`) — the only authoritative one.
    ExactMatch,
    /// No equivalent (or unattested) — preserved for human review, never force-flattened.
    RequiresHumanReview,
}

/// Enforce the Curation Directive on a sense mapping: a human attestation yields `ExactMatch`;
/// absent that, a machine proposal yields `CloseMatch`; an untranslatable concept (or nothing
/// proposed) yields `RequiresHumanReview`.
pub fn translation_status(machine_proposed: bool, human_attested: bool, translatable: bool) -> MatchStatus {
    if human_attested && translatable {
        MatchStatus::ExactMatch
    } else if !translatable {
        MatchStatus::RequiresHumanReview
    } else if machine_proposed {
        MatchStatus::CloseMatch
    } else {
        MatchStatus::RequiresHumanReview
    }
}

// ─── §1 Human-rights-instrument binding of compositions ────────────────────────────

/// A composition (and its proportionality test) is **valid** only when it is anchored to an
/// established human-rights `instrument` (a non-zero instrument hash) AND is proportionate. This
/// is the structural bind: legal reasoning may not float free of a cited instrument, and a
/// restriction must pass proportionality. (`proportionate` is the `proportionality_met` verdict.)
pub fn composition_valid(instrument: u64, proportionate: bool) -> bool {
    instrument != 0 && proportionate
}

/// Is a legal composition anchored to a cited instrument at all? (A composition citing `0` is
/// ungrounded and must be routed to human review rather than asserted.)
#[inline]
pub fn anchored_to_instrument(instrument: u64) -> bool {
    instrument != 0
}

// ─── §2 Translation matrix: natural language → machine logic ───────────────────────

/// The result of translating a natural-language term to a machine-logic construct.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Translation {
    /// A human-attested mapping yielded the machine construct (its hash).
    Mapped(u64),
    /// No attested mapping exists — routed to human review (never machine-flattened).
    RequiresHumanReview,
}

/// Translate a natural-language term to its machine-logic construct via a `matrix` of
/// `(nl_term_hash, machine_construct_hash)` rows, **gated by the Curation Directive**: a mapping
/// is only used if `human_attested` (the machine may propose, but only a human ratifies a
/// definitive NL→logic equivalence). An unmapped or unattested term routes to human review.
pub fn translate_via_matrix(nl_term: u64, matrix: &[(u64, u64)], human_attested: bool) -> Translation {
    if !human_attested {
        return Translation::RequiresHumanReview;
    }
    match matrix.iter().find(|(nl, _)| *nl == nl_term) {
        Some(&(_, construct)) => Translation::Mapped(construct),
        None => Translation::RequiresHumanReview,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    #[test]
    fn composition_binds_to_instrument_and_proportionality() {
        let iccpr = q_hash("instrument:iccpr");
        assert!(composition_valid(iccpr, true));
        assert!(!composition_valid(iccpr, false), "must be proportionate");
        assert!(!composition_valid(0, true), "must cite an instrument");
        assert!(anchored_to_instrument(iccpr) && !anchored_to_instrument(0));
    }

    #[test]
    fn translation_matrix_honours_the_curation_directive() {
        let nl = q_hash("nl:unconscionable");
        let logic = q_hash("logic:UnconscionabilityTest");
        let matrix = [(nl, logic)];
        // Human-attested + present → mapped.
        assert_eq!(translate_via_matrix(nl, &matrix, true), Translation::Mapped(logic));
        // Not attested → human review (machine doesn't get to flatten meaning).
        assert_eq!(translate_via_matrix(nl, &matrix, false), Translation::RequiresHumanReview);
        // Unmapped term → human review.
        assert_eq!(translate_via_matrix(q_hash("nl:unknown"), &matrix, true), Translation::RequiresHumanReview);
    }

    #[test]
    fn zk_gate_and_selective_disclosure() {
        assert_eq!(zk_eligibility(true), Eligibility::Eligible);
        assert_eq!(zk_eligibility(false), Eligibility::Unverifiable);
        let claims = [q_hash("age"), q_hash("name"), q_hash("address")];
        let reveal = [q_hash("age")];
        let mut out = [0u64; 4];
        let n = selective_disclosure(&claims, &reveal, &mut out);
        assert_eq!(n, 1);
        assert_eq!(out[0], q_hash("age"), "only the chosen claim is disclosed");
    }

    #[test]
    fn proportionality_composes_the_cas() {
        // Linear harm 3*x → marginal harm = 3 everywhere.
        assert_eq!(marginal_harm("3*x", "x", 0.0), Some(3.0));
        // 3 < 5 → proportionate; 3 < 2 → not.
        assert_eq!(proportionality_met("3*x", "x", 0.0, 5.0), Some(true));
        assert_eq!(proportionality_met("3*x", "x", 0.0, 2.0), Some(false));
        // Unparseable harm model → None (refuse rather than guess).
        assert_eq!(proportionality_met("@@@", "x", 0.0, 1.0), None);
    }

    #[test]
    fn sense_translation_honours_the_curation_directive() {
        // Machine may propose closeMatch.
        assert_eq!(translation_status(true, false, true), MatchStatus::CloseMatch);
        // Only a human attests exactMatch.
        assert_eq!(translation_status(true, true, true), MatchStatus::ExactMatch);
        // Untranslatable concept → preserved for review, never flattened.
        assert_eq!(translation_status(true, true, false), MatchStatus::RequiresHumanReview);
        // Nothing proposed → review.
        assert_eq!(translation_status(false, false, true), MatchStatus::RequiresHumanReview);
    }
}
