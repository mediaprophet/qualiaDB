//! `AuthorityType` — reframed for **supported agency**, not custodial control.
//!
//! (Timothy, 2026-07-03; grounded in the "Agency / Social Book" and "Digital Good Samaritan"
//! works.) This taxonomy describes *how and why one agent relies on another* in a way that
//! **amplifies the principal's personhood** — it is not a warden model that strips capacity.
//! Most of it applies to healthy, well people who simply need structural or specialist support
//! (an accountant, a clinical psychologist, an IT social worker, a work-peer who understands the
//! work, or — when a person is isolated with no better option — a software agent as a declared
//! source of truth). Custodial/parens-patriae intervention is the heavily-restricted, audited
//! **edge case**, never the centre.
//!
//! Two invariants from the source works are load-bearing:
//! - **Non-asymmetry:** expectations, responsibilities, and permissions must never be
//!   asymmetrical; the one taking responsibility is accountable to the one they support, and
//!   vice-versa.
//! - **Selfhood ≠ personhood:** delegation is only ever over *personhood* (socio-legal agency).
//!   *Selfhood* (inherent to the person) is never delegated (see `taxonomy::Sphere`).
//!
//! Modelled as a **faceted** classification on three axes — *Modality of Support* (the why),
//! *Trigger* (the when), and *Accountability & Evidence* (the verification) — as open data in a
//! [`Taxonomy`], so terms/facets extend without code changes. Named relationships (professional
//! delegation, developmental scaffolding, crisis activation, …) are **compositions** of facet
//! terms, provided as presets.

use serde::{Deserialize, Serialize};

use crate::taxonomy::{Taxonomy, TaxonomyTerm, TermId};

/// Stable ids for the seeded facets, terms, and flags (open registry — the well-known ones).
pub mod ids {
    // Facets.
    pub const FACET_MODALITY: &str = "urn:qualia:authority-type:facet:modality";
    pub const FACET_TRIGGER: &str = "urn:qualia:authority-type:facet:trigger";
    pub const FACET_ACCOUNTABILITY: &str = "urn:qualia:authority-type:facet:accountability";
    pub const FACET_FLAG: &str = "urn:qualia:authority-type:facet:flag";

    // Modality of support — the *why* (nature of the reliance).
    pub const MOD_AUGMENTATIVE: &str = "urn:qualia:authority-type:modality:augmentative";
    pub const MOD_DEVELOPMENTAL: &str = "urn:qualia:authority-type:modality:developmental";
    pub const MOD_ADVOCACY: &str = "urn:qualia:authority-type:modality:advocacy";
    pub const MOD_AUTOMATED: &str = "urn:qualia:authority-type:modality:automated";

    // Trigger conditions — the *when*.
    pub const TRIG_PERSISTENT: &str = "urn:qualia:authority-type:trigger:persistent";
    pub const TRIG_DECLARATIVE: &str = "urn:qualia:authority-type:trigger:declarative";
    pub const TRIG_CONTINGENT: &str = "urn:qualia:authority-type:trigger:contingent";

    // Accountability & evidence — the *verification*.
    pub const ACCT_AUDITABLE_FIDUCIARY: &str =
        "urn:qualia:authority-type:accountability:auditable-fiduciary";
    pub const ACCT_MUTUAL_CONSENSUS: &str =
        "urn:qualia:authority-type:accountability:mutual-consensus";
    pub const ACCT_VALUES_BOUND: &str = "urn:qualia:authority-type:accountability:values-bound";

    // Flags — markers that don't fit a facet slot (e.g. the restricted edge case).
    pub const FLAG_PARENS_PATRIAE: &str = "urn:qualia:authority-type:flag:parens-patriae";
}

fn facet(id: &str, label: &str) -> TaxonomyTerm {
    TaxonomyTerm::new(id, label).with_attr("kind", "facet")
}

fn term(id: &str, label: &str, facet: &str, desc: &str) -> TaxonomyTerm {
    TaxonomyTerm::new(id, label)
        .in_category(facet)
        .described(desc)
        .with_attr("facet", facet)
}

/// The seeded, supported-agency `AuthorityType` taxonomy. Extend with `insert` / `extend_with`.
pub fn authority_type_taxonomy() -> Taxonomy {
    use ids::*;
    Taxonomy::from_terms([
        facet(FACET_MODALITY, "Modality of support (the why)"),
        facet(FACET_TRIGGER, "Trigger conditions (the when)"),
        facet(FACET_ACCOUNTABILITY, "Accountability & evidence (the verification)"),
        facet(FACET_FLAG, "Flags / markers"),
        // Modality of support.
        term(MOD_AUGMENTATIVE, "Augmentative / Expertise", FACET_MODALITY,
            "Principal retains full capacity and relies on an agent for specific domain expertise; authority is strictly limited to that function."),
        term(MOD_DEVELOPMENTAL, "Developmental / Scaffolding", FACET_MODALITY,
            "Dynamic, time-bound authority that DECREASES as the principal's capacity increases (children; recovery). Explicit goal: transfer full agency."),
        term(MOD_ADVOCACY, "Advocacy / Protective", FACET_MODALITY,
            "Triggered by barriers to expressing agency (language, temporary incapacity, marginalisation). The agent amplifies the principal's KNOWN intents and defends their rights."),
        term(MOD_AUTOMATED, "Automated / Agentic Extension", FACET_MODALITY,
            "A software agent executes pre-declared intents or monitors data streams (Digital Good Samaritan, health monitor) WITHOUT overriding the principal's ultimate authority."),
        // Trigger conditions.
        term(TRIG_PERSISTENT, "Persistent / Baseline", FACET_TRIGGER,
            "Ongoing relationship needing continuous, low-level access or coordination (long-term advisor, specialised educator)."),
        term(TRIG_DECLARATIVE, "Declarative / Consent-driven", FACET_TRIGGER,
            "Actively toggled on/off by the principal for a specific event or transaction (share a record with a new specialist for one consult)."),
        term(TRIG_CONTINGENT, "Contingent / Crisis-activated", FACET_TRIGGER,
            "Dormant until cryptographically verifiable, pre-defined conditions occur (emergency, loss of consciousness, deadman switch)."),
        // Accountability & evidence.
        term(ACCT_AUDITABLE_FIDUCIARY, "Auditable Fiduciary", FACET_ACCOUNTABILITY,
            "Professional / insured / legal standards; an evidence chain distinguishes best-effort mistakes from malpractice."),
        term(ACCT_MUTUAL_CONSENSUS, "Mutual Consensus", FACET_ACCOUNTABILITY,
            "Several trusted agents must form rough consensus (M-of-N) before a critical decision, preventing unilateral abuse."),
        term(ACCT_VALUES_BOUND, "Values-bound", FACET_ACCOUNTABILITY,
            "Governed by explicitly shared value credentials (default: UN Human Rights instruments), giving clear semiotic boundaries for acceptable behaviour."),
        // Flags.
        TaxonomyTerm::new(FLAG_PARENS_PATRIAE, "Parens patriae intervention (edge case)")
            .in_category(FACET_FLAG)
            .described("State-assumed guardianship for unaccompanied minors / wards of the court. Treated as a heavily-restricted, continuously-audited edge-case fallback — NOT a normal relationship.")
            .with_attr("edge_case", "true")
            .with_attr("restricted", "true")
            .with_attr("requires_audit", "true"),
    ])
}

/// A supported-agency authority as a facet vector: one term per axis, plus room to extend
/// (e.g. flags such as parens-patriae, or a jurisdiction-specific facet added later).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthorityProfile {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modality: Option<TermId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trigger: Option<TermId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accountability: Option<TermId>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub flags: Vec<TermId>,
}

impl AuthorityProfile {
    pub fn new(modality: &str, trigger: &str, accountability: &str) -> Self {
        Self {
            modality: Some(modality.into()),
            trigger: Some(trigger.into()),
            accountability: Some(accountability.into()),
            flags: Vec::new(),
        }
    }

    pub fn with_flag(mut self, flag: &str) -> Self {
        self.flags.push(flag.into());
        self
    }

    /// Validate that each set facet term exists and sits in its facet.
    pub fn validate(&self, tax: &Taxonomy) -> Result<(), String> {
        for (field, term, facet) in [
            ("modality", &self.modality, ids::FACET_MODALITY),
            ("trigger", &self.trigger, ids::FACET_TRIGGER),
            (
                "accountability",
                &self.accountability,
                ids::FACET_ACCOUNTABILITY,
            ),
        ] {
            if let Some(t) = term {
                let Some(def) = tax.get(t) else {
                    return Err(format!("{field}: unknown authority-type term '{t}'"));
                };
                if def.category.as_deref() != Some(facet) {
                    return Err(format!("{field}: term '{t}' is not in facet '{facet}'"));
                }
            }
        }
        for f in &self.flags {
            if !tax.contains(f) {
                return Err(format!("flags: unknown authority-type term '{f}'"));
            }
        }
        Ok(())
    }
}

/// Named relationship archetypes as compositions of facets (Timothy's revised v2 categories).
/// These are the everyday, agency-amplifying relationships — the centre of the model.
pub mod presets {
    use super::ids::*;
    use super::AuthorityProfile;

    /// An accountant / clinical psychologist / lawyer: augmentative expertise, insured, auditable.
    pub fn professional_delegation() -> AuthorityProfile {
        AuthorityProfile::new(MOD_AUGMENTATIVE, TRIG_PERSISTENT, ACCT_AUDITABLE_FIDUCIARY)
    }

    /// Several trusted agents who must reach rough consensus before a critical decision.
    pub fn cooperative_consensus() -> AuthorityProfile {
        AuthorityProfile::new(MOD_AUGMENTATIVE, TRIG_PERSISTENT, ACCT_MUTUAL_CONSENSUS)
    }

    /// Raising a child: scaffolding that decreases as the child matures; UNCRC values-bound.
    pub fn developmental_scaffolding() -> AuthorityProfile {
        AuthorityProfile::new(MOD_DEVELOPMENTAL, TRIG_PERSISTENT, ACCT_VALUES_BOUND)
    }

    /// The Digital Good Samaritan: dormant advocacy activated by a verifiable crisis, with
    /// consensus-gated action on the principal's pre-declared intents.
    pub fn crisis_activated() -> AuthorityProfile {
        AuthorityProfile::new(MOD_ADVOCACY, TRIG_CONTINGENT, ACCT_MUTUAL_CONSENSUS)
    }

    /// A digital will / legacy executor: automated, contingent on death, values-bound.
    pub fn posthumous_legacy() -> AuthorityProfile {
        AuthorityProfile::new(MOD_AUTOMATED, TRIG_CONTINGENT, ACCT_VALUES_BOUND)
    }

    /// Protective oversight to prevent abuse/neglect, with a rigorous evidence chain.
    pub fn protective_custodial() -> AuthorityProfile {
        AuthorityProfile::new(MOD_ADVOCACY, TRIG_PERSISTENT, ACCT_AUDITABLE_FIDUCIARY)
    }

    /// The restricted edge case: state intervention, flagged as such and continuously audited.
    pub fn parens_patriae_fallback() -> AuthorityProfile {
        AuthorityProfile::new(MOD_ADVOCACY, TRIG_CONTINGENT, ACCT_AUDITABLE_FIDUCIARY)
            .with_flag(FLAG_PARENS_PATRIAE)
    }
}

#[cfg(test)]
mod tests {
    use super::ids::*;
    use super::*;

    #[test]
    fn seeded_taxonomy_has_the_three_axes() {
        let tax = authority_type_taxonomy();
        assert_eq!(tax.in_category(FACET_MODALITY).len(), 4);
        assert_eq!(tax.in_category(FACET_TRIGGER).len(), 3);
        assert_eq!(tax.in_category(FACET_ACCOUNTABILITY).len(), 3);
        assert!(tax.contains(MOD_DEVELOPMENTAL));
        // parens patriae is a flagged edge case, not a facet term.
        assert_eq!(
            tax.get(FLAG_PARENS_PATRIAE).unwrap().attr("edge_case"),
            Some("true")
        );
    }

    #[test]
    fn presets_compose_valid_profiles() {
        let tax = authority_type_taxonomy();
        for p in [
            presets::professional_delegation(),
            presets::cooperative_consensus(),
            presets::developmental_scaffolding(),
            presets::crisis_activated(),
            presets::posthumous_legacy(),
            presets::protective_custodial(),
            presets::parens_patriae_fallback(),
        ] {
            assert!(p.validate(&tax).is_ok(), "preset failed validation: {p:?}");
        }
        // The everyday centre uses expertise + fiduciary accountability, not custodial control.
        assert_eq!(
            presets::professional_delegation().modality.as_deref(),
            Some(MOD_AUGMENTATIVE)
        );
        // The edge case carries the restricted flag.
        assert!(presets::parens_patriae_fallback()
            .flags
            .contains(&FLAG_PARENS_PATRIAE.to_string()));
    }

    #[test]
    fn wrong_facet_term_is_rejected() {
        let tax = authority_type_taxonomy();
        let bad = AuthorityProfile {
            modality: Some(TRIG_PERSISTENT.into()), // a trigger term in the modality slot
            ..Default::default()
        };
        assert!(bad.validate(&tax).is_err());
    }
}
