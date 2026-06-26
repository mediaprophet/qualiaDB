//! Illocutionary force + binding-weight conflict resolution (modal-junctures.n3, in code).
//!
//! Answers: when norms conflict on the same (party, action), how does a SOFT directive
//! (Recommends/Urges) resolve against a HARD commissive (Undertakes) or prohibitive
//! (Forbids)? By **binding-weight precedence**, with two refinements drawn from the
//! taxonomy:
//!   * a directive's weight is first scaled by the SPEAKER'S AUTHORITY (a UN treaty
//!     body's "calls upon" outweighs an NGO's — the authority variable);
//!   * an EXEMPTIVE (derogation / waiver — the `q42:unless` defeater) OVERRIDES an
//!     otherwise-active obligation/prohibition regardless of weight;
//!   * equal effective weight is a GENUINE conflict, held PARACONSISTENTLY (both norms
//!     retained for adjudication — the engine does not crash or silently pick one).

/// The engine state a juncture triggers (see modal-junctures.n3 mj:EngineState).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineState {
    EpistemicAbsolute,
    EpistemicWeighted,
    ContextualAnchor,
    ObligateSelf,
    ObligateSelfLiability,
    ObligateTarget,
    Recommend,
    QueueAwait,
    Instantiate,
    DefeatExpire,
    ConceptBinding,
    Permit,
    DefeaterUnless,
    Forbid,
    ForbidTemporal,
    AlignmentPositive,
    AlignmentNegative,
}

/// Deontic binding weight (0 = no binding .. 255 = hardest). Mirrors mj:bindingWeight.
pub const W_NONE: u8 = 0; // assertive / expressive / declarative-performative
pub const W_REQUEST: u8 = 30; // rogative directive (queue/await)
pub const W_RECOMMEND: u8 = 50; // exhortative directive (soft)
pub const W_PERMIT: u8 = 128; // permissive authoritative
pub const W_DIRECTIVE_HARD: u8 = 160; // imperative directive (pre-authority-scaling)
pub const W_OBLIGATE: u8 = 200; // commissive promissive / prohibitive interdictive
pub const W_GUARANTEE: u8 = 220; // commissive guarantive (+liability)
pub const W_EXEMPT: u8 = 250; // permissive exemptive (derogation/waiver → defeater)

/// True if this engine state is an Exemptive defeater (derogation / waiver / `q42:unless`).
#[inline]
pub fn is_exemptive(state: EngineState) -> bool {
    matches!(state, EngineState::DefeaterUnless)
}

/// A directive's EFFECTIVE weight scales with the speaker's structural authority (0..255);
/// non-authority-scaled junctures keep their base weight. So an NGO (low authority) that
/// "demands" carries far less force than a treaty body that does.
#[inline]
pub fn effective_weight(base: u8, authority_scaled: bool, speaker_authority: u8) -> u8 {
    if authority_scaled {
        ((base as u16 * speaker_authority as u16) / 255) as u8
    } else {
        base
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Resolution {
    /// The first norm governs.
    AGoverns,
    /// The second norm governs.
    BGoverns,
    /// Equal binding force — a genuine conflict, retained paraconsistently for adjudication.
    GenuineConflict,
}

/// A norm participating in a conflict: effective binding weight, whether it is an
/// Exemptive (derogation/waiver), and whether it is IMMUNE — non-derogable, a Hohfeldian
/// immunity (e.g. ICCPR Art. 4(2): no derogation from the right to life, freedom from
/// torture, etc.).
#[derive(Debug, Clone, Copy)]
pub struct Norm {
    pub weight: u8,
    pub exemptive: bool,
    pub immune: bool,
}

impl Norm {
    /// An ordinary norm of the given binding weight.
    pub fn new(weight: u8) -> Self {
        Self { weight, exemptive: false, immune: false }
    }
    /// An Exemptive (derogation / waiver → the q42:unless defeater).
    pub fn exemptive(weight: u8) -> Self {
        Self { weight, exemptive: true, immune: false }
    }
    /// A non-derogable norm (Hohfeldian immunity) — cannot be defeated by an Exemptive.
    pub fn immune(weight: u8) -> Self {
        Self { weight, exemptive: false, immune: true }
    }
}

/// Resolve two CONFLICTING norms (same party + action): the Exemptive override (BOUNDED
/// by immunity), then effective binding weight.
pub fn resolve_conflict(a: Norm, b: Norm) -> Resolution {
    // An Exemptive defeats the other norm regardless of weight — UNLESS that norm is
    // non-derogable (immune), in which case the derogation is invalid and the immune norm
    // STANDS (ICCPR Art. 4(2): the right to life etc. cannot be derogated).
    match (a.exemptive, b.exemptive) {
        (true, false) => {
            return if b.immune { Resolution::BGoverns } else { Resolution::AGoverns };
        }
        (false, true) => {
            return if a.immune { Resolution::AGoverns } else { Resolution::BGoverns };
        }
        _ => {}
    }
    use std::cmp::Ordering::*;
    match a.weight.cmp(&b.weight) {
        Greater => Resolution::AGoverns,
        Less => Resolution::BGoverns,
        Equal => Resolution::GenuineConflict,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_directive_never_overrides_a_hard_commissive() {
        // "Recommends X" vs "Undertakes not-X" on the same party+action.
        let recommend = Norm::new(W_RECOMMEND);
        let undertake = Norm::new(W_OBLIGATE);
        assert_eq!(resolve_conflict(undertake, recommend), Resolution::AGoverns);
        assert_eq!(resolve_conflict(recommend, undertake), Resolution::BGoverns);
    }

    #[test]
    fn exemptive_overrides_a_derogable_obligation() {
        // A derogation/waiver (Exemptive → q42:unless) defeats a DEROGABLE obligation.
        let derogation = Norm::exemptive(W_EXEMPT);
        let obligation = Norm::new(W_OBLIGATE);
        assert_eq!(resolve_conflict(derogation, obligation), Resolution::AGoverns);
        assert_eq!(resolve_conflict(obligation, derogation), Resolution::BGoverns);
    }

    #[test]
    fn exemptive_cannot_defeat_a_non_derogable_norm() {
        // ICCPR Art. 4 derogation (Exemptive) vs Art. 6 right to life — non-derogable per
        // Art. 4(2). The immune norm STANDS; the derogation is invalid against it.
        let derogation = Norm::exemptive(W_EXEMPT);
        let right_to_life = Norm::immune(W_OBLIGATE);
        assert_eq!(resolve_conflict(derogation, right_to_life), Resolution::BGoverns);
        assert_eq!(resolve_conflict(right_to_life, derogation), Resolution::AGoverns);
    }

    #[test]
    fn equal_force_is_a_genuine_paraconsistent_conflict() {
        // e.g. an Obligate and a Forbid of equal weight on the same act — held, not crashed.
        assert_eq!(
            resolve_conflict(Norm::new(W_OBLIGATE), Norm::new(W_OBLIGATE)),
            Resolution::GenuineConflict
        );
    }

    #[test]
    fn directive_weight_scales_with_speaker_authority() {
        // The SAME imperative "demands" carries different force by who says it.
        let ngo = effective_weight(W_DIRECTIVE_HARD, true, 30); // low authority
        let treaty_body = effective_weight(W_DIRECTIVE_HARD, true, 255); // high authority
        assert!(ngo < treaty_body);
        // An NGO's demand loses to a hard commitment; the treaty body's competes/wins.
        assert_eq!(resolve_conflict(Norm::new(ngo), Norm::new(W_OBLIGATE)), Resolution::BGoverns);
        assert_eq!(
            resolve_conflict(Norm::new(treaty_body), Norm::new(W_RECOMMEND)),
            Resolution::AGoverns
        );
    }
}
