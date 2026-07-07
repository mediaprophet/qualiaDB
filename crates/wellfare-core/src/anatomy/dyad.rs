//! P2 — the **maternal–fetal dyad** *domain* model (plan of record §3.3, §8), built on the **science**.
//!
//! Gestation modelled as the plan insists: **two coupled bodies, not one body with an organ.** But the
//! rights structure has to follow biology, not a default family shape (Timothy, 2026-07-06):
//!
//! - **The emerging entity has two genetic origins.** Fertilization requires an ovum **and** a sperm — an
//!   ovum alone is not a new life — so there are two [`Progenitor`]s. Either may be **unknown**, a person
//!   who does not know, or a **donor**.
//! - **Genetic ≠ gestational ≠ social ≠ guardian.** Surrogacy and gamete donation make these genuinely
//!   different parties (the ovum source, the sperm source, the gestational carrier, and any intended
//!   parents can all be distinct). Conflating them is the error this model avoids.
//! - **The steward/guardian during gestation is the gestational mother.** She carries; she is the
//!   data-subject; guardianship of the emerging entity is hers ([`MaternalFetalDyad::guardian`]).
//! - **Social/legal personhood accrues at or after *birth*** ([`RightsStage`]) — the exact threshold (at
//!   birth, or after the fourth trimester) is a **deferred values decision** ([`SocialRightsThreshold`],
//!   §9.4). During gestation the emerging entity is *stewarded* in guardianship; it is **not** modelled as a
//!   competing legal person, so the gestational mother's autonomy over her own body and data stays paramount.
//!
//! **What this module is, and isn't.** It builds the rights-and-structure machinery and encodes the
//! invariants in the type system + [`MaternalFetalDyad::validate`]. It does **not** author cross-body
//! physiological *correlation content* (that is clinician/midwife-gated, §9.3), and it does **not** settle
//! the values calls (the social-rights threshold, when guardianship transfers) — those are Timothy's (§9.4).
//! Everything it *does* emit is an [`EpistemicStatus::Hypothesis`] proposal, never a determination.
//!
//! This is the pure **domain/rights** model; the *visual* dyad (mesh placement) is the separate,
//! geometry-gated `anatomy_dyad.rs` in the client layer — they compose (this = *who/what*, that = *where*).
//!
//! [`EpistemicStatus::Hypothesis`]: crate::record::EpistemicStatus::Hypothesis

use serde::{Deserialize, Serialize};

use super::physiology::{PhysiologicalState, ReproductiveState, Trimester};
use super::scorecard::ForumClass;

/// A reference to a rights-bearing **principal** by identifier — a DID, kept as an *identifier* and never
/// treated as an identity (a DID identifies; identity is a probabilistic fabric that must not collapse onto
/// one identifier). Abstract here — the identity/guardianship layer resolves it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PrincipalRef {
    /// A pairwise/contextual DID identifier. Not an identity, not a legal name.
    pub did: String,
}

impl PrincipalRef {
    pub fn new(did: impl Into<String>) -> Self {
        Self { did: did.into() }
    }
    fn is_empty(&self) -> bool {
        self.did.trim().is_empty()
    }
}

/// A genetic **progenitor** — one gamete source. There are two per emerging entity (ovum + sperm). A
/// progenitor may be a known principal, a donor (identity withheld/pseudonymous by arrangement — possibly
/// resolvable later for the child's right to know their origins, a governance policy not asserted here), or
/// simply unknown/unrecorded (a father may not be known, or not know).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Progenitor {
    Known(PrincipalRef),
    Donor,
    Unknown,
}

impl Progenitor {
    /// The known principal, if this progenitor is a known party.
    pub fn known(&self) -> Option<&PrincipalRef> {
        match self {
            Progenitor::Known(p) => Some(p),
            _ => None,
        }
    }
}

/// The distinct biological + social roles around the emerging child — modelled on the science, not on a
/// presumed family shape. Genetic (the two [`Progenitor`]s), and social (declared [`intended_parents`]).
/// The *gestational* role lives on [`MaternalBody`] (she carries), and *guardianship* during gestation is
/// hers ([`MaternalFetalDyad::guardian`]).
///
/// [`intended_parents`]: Parentage::intended_parents
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Parentage {
    /// The ovum (egg) genetic source. May be the gestational mother, an egg donor, or unknown.
    pub ovum_source: Progenitor,
    /// The sperm genetic source. Required for a new life; may be a donor, unknown, or a person unaware.
    pub sperm_source: Progenitor,
    /// Intended / social parent(s), if declared (e.g. a surrogacy arrangement). Social roles, distinct from
    /// genetics and from who carries.
    #[serde(default)]
    pub intended_parents: Vec<PrincipalRef>,
}

impl Parentage {
    /// The **known genetic** parents — the ovum + sperm sources that are known principals. This is the
    /// *biological* default for guardianship at birth (a donor / unknown contributes no principal).
    pub fn known_genetic_parents(&self) -> Vec<&PrincipalRef> {
        let mut v = Vec::new();
        if let Some(p) = self.ovum_source.known() {
            v.push(p);
        }
        if let Some(p) = self.sperm_source.known() {
            v.push(p);
        }
        v
    }

    /// The **known** adult principals involved (genetic + intended) — the parties an emerging child must
    /// never be collapsed into. Donors/unknowns contribute no principal here.
    fn known_adults(&self) -> Vec<&PrincipalRef> {
        let mut v = self.known_genetic_parents();
        v.extend(self.intended_parents.iter());
        v
    }
}

/// Where the emerging entity is on the path to social/legal personhood.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RightsStage {
    /// Before birth — an entity-in-formation, **stewarded in guardianship** by the gestational mother. Not
    /// yet a full social/legal person (so it is not a competing legal person to the mother).
    EmergingInGestation,
    /// Born — social/legal personhood accrues (the exact threshold is [`SocialRightsThreshold`]).
    Born,
}

/// The **deferred values decision**: when a born child's social/legal rights are recognised as accruing.
/// Timothy flagged uncertainty (at birth, or after the fourth trimester); the model carries the choice, it
/// does not assert one. Whatever the threshold, personhood is **never withheld beyond it** and never
/// applied *during* gestation (which would subordinate the gestational mother's autonomy).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SocialRightsThreshold {
    AtBirth,
    AfterFourthTrimester,
    /// Undecided — Timothy's values call (§9.4).
    Undecided,
}

/// The developing child: an emerging entity indexed on the `t` (gestational-age) axis. Its identifier
/// fabric is **pairwise, uncorrelated, and never collapsed** into any adult's record.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EmergingChild {
    /// **One** pairwise identifier, for this relationship/context only — there is deliberately no single
    /// "the child's identity". Correlation-by-default is the surveillance on-ramp this refuses.
    pub pairwise_did: String,
    /// The science-based origin: two genetic progenitors + any declared social parents.
    pub parentage: Parentage,
    /// The rights stage (in gestation this is [`RightsStage::EmergingInGestation`]).
    pub rights_stage: RightsStage,
}

/// The maternal–fetal interface coupling the two bodies.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InterfaceKind {
    /// The placenta — the HRA reference organ we already compile (`placenta-full-term`).
    Placenta,
}

/// The mother in the dyad: the **gestational** mother — the data-subject who carries, and the emerging
/// child's **guardian during gestation**. Her whole-body model is in a pregnant [`PhysiologicalState`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaternalBody {
    /// The gestational mother's principal reference (she is also the guardian during gestation).
    pub principal: PrincipalRef,
    /// Expected `Reproductive(Pregnant(_))` — [`MaternalFetalDyad::validate`] enforces it.
    pub state: PhysiologicalState,
}

/// The dyad: two coupled principals joined by the interface. **Sanctuary-class, forum-internum** data.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MaternalFetalDyad {
    pub maternal: MaternalBody,
    pub child: EmergingChild,
    /// Gestational age in days. **Convention is the caller's to state**: clinical gestational age is from
    /// the last menstrual period (LMP); the Carnegie/embryology assets are indexed *postfertilization*
    /// (~14 days earlier). Precise stage↔age↔trimester alignment is clinical content (§9), not asserted.
    pub gestational_age_days: u32,
    pub carnegie_stage: Option<u32>,
    pub interface: InterfaceKind,
    /// The (deferred) values setting for when a born child's social rights accrue.
    pub social_rights_threshold: SocialRightsThreshold,
}

/// A reason a dyad is structurally invalid — a **rights** violation or an impossibility.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DyadInvalid {
    /// The maternal state is not a pregnant state — a gestational dyad exists only in pregnancy.
    NotPregnant,
    /// No gestational mother — she is the data-subject and the guardian; she must be present.
    NoGestationalMother,
    /// The child's identifier is empty or **collapsed into an adult** (identical to the gestational
    /// mother, a known genetic progenitor, or an intended parent). The core rights invariant: the child is
    /// its own emerging principal, never a field of an adult's identity.
    ChildIdentityCollapsed,
    /// The child's rights stage is not `EmergingInGestation` while the mother is pregnant (a born child is
    /// not a *gestational* dyad).
    RightsStageMismatch,
    /// The gestational age is outside any plausible range (1..=300 days).
    ImplausibleGestationalAge,
}

/// A cross-body **proposal** about the dyad — always [`EpistemicStatus::Hypothesis`], never a
/// determination. P2 emits only *structural / rights / science* considerations; medical correlations are
/// deferred curation (§9.3).
///
/// [`EpistemicStatus::Hypothesis`]: crate::record::EpistemicStatus::Hypothesis
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DyadConsideration {
    pub kind: ConsiderationKind,
    /// Plain, non-diagnostic wording.
    pub note: String,
    pub epistemic_status: crate::record::EpistemicStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ConsiderationKind {
    /// A statement about the rights structure of the dyad.
    Rights,
    /// A statement grounded in reproductive biology (the two-gamete origin; the distinct roles).
    Science,
    /// A pointer that content (medical correlations, values thresholds) is deferred to curation.
    CurationDeferred,
}

impl MaternalFetalDyad {
    /// A dyad's data is always **forum internum** (a person's — and an emerging person's — inward domain).
    pub const fn forum_class(&self) -> ForumClass {
        ForumClass::Internum
    }

    /// Always the most-restrictive sensitivity class (`Sanctuary`), returned as the ladder's canonical name
    /// so the storage layer maps it without this pure crate depending on the sensitivity enum. The
    /// high-water-mark rule makes anything derived from a dyad inherit this.
    pub const fn sensitivity_class(&self) -> &'static str {
        "Sanctuary"
    }

    /// The **guardian during gestation** — the gestational mother. Guardianship is a scoped, revocable
    /// *role*, not ownership; whether/when it transfers post-birth (e.g. to intended parents in surrogacy)
    /// is a deferred values decision (§9.4).
    pub fn guardian(&self) -> &PrincipalRef {
        &self.maternal.principal
    }

    /// The maternal trimester, if the mother is in a pregnant state.
    pub fn trimester(&self) -> Option<Trimester> {
        match self.maternal.state {
            PhysiologicalState::Reproductive(ReproductiveState::Pregnant(t)) => Some(t),
            _ => None,
        }
    }

    /// Enforce the non-negotiable rights + possibility invariants.
    pub fn validate(&self) -> Result<(), DyadInvalid> {
        if self.trimester().is_none() {
            return Err(DyadInvalid::NotPregnant);
        }
        if self.maternal.principal.is_empty() {
            return Err(DyadInvalid::NoGestationalMother);
        }
        let child = self.child.pairwise_did.trim();
        if child.is_empty() {
            return Err(DyadInvalid::ChildIdentityCollapsed);
        }
        // The child is a distinct principal from the gestational mother AND every known adult (genetic or
        // intended). Never a field of an adult's identity.
        if child == self.maternal.principal.did.trim()
            || self
                .child
                .parentage
                .known_adults()
                .iter()
                .any(|a| a.did.trim() == child)
        {
            return Err(DyadInvalid::ChildIdentityCollapsed);
        }
        if self.child.rights_stage != RightsStage::EmergingInGestation {
            return Err(DyadInvalid::RightsStageMismatch);
        }
        if self.gestational_age_days == 0 || self.gestational_age_days > 300 {
            return Err(DyadInvalid::ImplausibleGestationalAge);
        }
        Ok(())
    }

    /// The structural / rights / science considerations for a **valid** dyad — proposals, never
    /// determinations. These are the honest, non-medical statements the model *can* make; the cross-body
    /// physiological correlations are curation-grade and pointedly **not** seeded (their absence is
    /// deliberate, not a gap).
    pub fn considerations(&self) -> Vec<DyadConsideration> {
        use crate::record::EpistemicStatus::Hypothesis;
        let c = |kind, note: &str| DyadConsideration {
            kind,
            note: note.to_string(),
            epistemic_status: Hypothesis,
        };
        vec![
            c(
                ConsiderationKind::Science,
                "The emerging entity arises from the union of an ovum and a sperm (two genetic \
                 progenitors). Genetic, gestational, and social/intended-parent roles are modelled \
                 distinctly — they may be different parties (donation, surrogacy), and a progenitor may be \
                 unknown or unaware.",
            ),
            c(
                ConsiderationKind::Rights,
                "The developing child is a distinct emerging principal — held in guardianship by the \
                 gestational mother, never collapsed into any adult's record, never correlated by default.",
            ),
            c(
                ConsiderationKind::Rights,
                "Social/legal personhood is modelled as accruing at or after birth; during gestation the \
                 entity is stewarded, not a competing legal person, so the gestational mother's autonomy \
                 over her own body and data remains paramount. The exact threshold (birth vs the fourth \
                 trimester) and when guardianship transfers are deferred values decisions.",
            ),
            c(
                ConsiderationKind::CurationDeferred,
                "Cross-body physiological correlations (what the maternal state may mean for development, \
                 and vice-versa) are proposals for discussion with a clinician/midwife — none are asserted \
                 here; that content is curation-grade.",
            ),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::record::EpistemicStatus;

    fn dyad() -> MaternalFetalDyad {
        MaternalFetalDyad {
            maternal: MaternalBody {
                principal: PrincipalRef::new("did:wf:mother"),
                state: PhysiologicalState::Reproductive(ReproductiveState::Pregnant(Trimester::Second)),
            },
            child: EmergingChild {
                pairwise_did: "did:wf:child-pairwise-1".into(),
                parentage: Parentage {
                    ovum_source: Progenitor::Known(PrincipalRef::new("did:wf:mother")),
                    sperm_source: Progenitor::Unknown,
                    intended_parents: vec![],
                },
                rights_stage: RightsStage::EmergingInGestation,
            },
            gestational_age_days: 20 * 7,
            carnegie_stage: None,
            interface: InterfaceKind::Placenta,
            social_rights_threshold: SocialRightsThreshold::Undecided,
        }
    }

    #[test]
    fn a_valid_dyad_is_two_coupled_principals_and_guardian_is_the_gestational_mother() {
        let d = dyad();
        assert_eq!(d.validate(), Ok(()));
        assert_eq!(d.trimester(), Some(Trimester::Second));
        // The steward/guardian during gestation is the gestational mother (Timothy's direction).
        assert_eq!(d.guardian().did, "did:wf:mother");
    }

    #[test]
    fn the_science_a_missing_father_is_representable_not_an_error() {
        // A sperm source that is unknown / a person who does not know is a first-class, valid case.
        let d = dyad();
        assert_eq!(d.child.parentage.sperm_source, Progenitor::Unknown);
        assert_eq!(d.validate(), Ok(()));
    }

    #[test]
    fn surrogacy_and_donors_are_distinct_roles() {
        // Egg donor + sperm donor + a gestational carrier who is neither + two intended parents.
        let mut d = dyad();
        d.maternal.principal = PrincipalRef::new("did:wf:surrogate");
        d.child.parentage = Parentage {
            ovum_source: Progenitor::Donor,
            sperm_source: Progenitor::Donor,
            intended_parents: vec![PrincipalRef::new("did:wf:parent-a"), PrincipalRef::new("did:wf:parent-b")],
        };
        assert_eq!(d.validate(), Ok(()));
        // Guardian during gestation is the one carrying — the surrogate — not the intended parents.
        assert_eq!(d.guardian().did, "did:wf:surrogate");
    }

    #[test]
    fn the_child_is_never_collapsed_into_any_adult() {
        // Into the gestational mother:
        let mut d = dyad();
        d.child.pairwise_did = "did:wf:mother".into();
        assert_eq!(d.validate(), Err(DyadInvalid::ChildIdentityCollapsed));

        // Into a known genetic progenitor:
        let mut d = dyad();
        d.child.parentage.sperm_source = Progenitor::Known(PrincipalRef::new("did:wf:father"));
        d.child.pairwise_did = "did:wf:father".into();
        assert_eq!(d.validate(), Err(DyadInvalid::ChildIdentityCollapsed));

        // Into an intended parent:
        let mut d = dyad();
        d.child.parentage.intended_parents = vec![PrincipalRef::new("did:wf:intended")];
        d.child.pairwise_did = "did:wf:intended".into();
        assert_eq!(d.validate(), Err(DyadInvalid::ChildIdentityCollapsed));

        // Empty:
        let mut d = dyad();
        d.child.pairwise_did = "  ".into();
        assert_eq!(d.validate(), Err(DyadInvalid::ChildIdentityCollapsed));
    }

    #[test]
    fn a_dyad_only_exists_in_a_pregnant_state_and_in_gestation_stage() {
        let mut d = dyad();
        d.maternal.state = PhysiologicalState::Baseline;
        assert_eq!(d.validate(), Err(DyadInvalid::NotPregnant));

        let mut d = dyad();
        d.child.rights_stage = RightsStage::Born; // a born child is not a gestational dyad
        assert_eq!(d.validate(), Err(DyadInvalid::RightsStageMismatch));
    }

    #[test]
    fn missing_gestational_mother_or_implausible_age_is_rejected() {
        let mut d = dyad();
        d.maternal.principal = PrincipalRef::new("");
        assert_eq!(d.validate(), Err(DyadInvalid::NoGestationalMother));

        let mut d = dyad();
        d.gestational_age_days = 400;
        assert_eq!(d.validate(), Err(DyadInvalid::ImplausibleGestationalAge));
    }

    #[test]
    fn social_rights_threshold_is_a_carried_deferred_value_not_asserted() {
        let d = dyad();
        // The model carries the (undecided) values choice; it does not decide it.
        assert_eq!(d.social_rights_threshold, SocialRightsThreshold::Undecided);
        // During gestation the child is emerging, not yet a born social person.
        assert_eq!(d.child.rights_stage, RightsStage::EmergingInGestation);
    }

    #[test]
    fn dyad_data_is_sanctuary_class_forum_internum() {
        let d = dyad();
        assert_eq!(d.forum_class(), ForumClass::Internum);
        assert_eq!(d.sensitivity_class(), "Sanctuary");
    }

    #[test]
    fn considerations_are_structural_science_and_rights_proposals_never_determinations() {
        let d = dyad();
        let cons = d.considerations();
        assert!(cons.iter().all(|c| c.epistemic_status == EpistemicStatus::Hypothesis));
        assert!(cons.iter().any(|c| c.kind == ConsiderationKind::Science));
        assert!(cons.iter().any(|c| c.kind == ConsiderationKind::Rights));
        assert!(cons.iter().any(|c| c.kind == ConsiderationKind::CurationDeferred));
    }

    #[test]
    fn serde_round_trips() {
        let d = dyad();
        let json = serde_json::to_string(&d).unwrap();
        let back: MaternalFetalDyad = serde_json::from_str(&json).unwrap();
        assert_eq!(d, back);
    }
}
