//! Declared reliance & judgement provenance as a **DAG** — the evidentiary substrate that keeps a
//! natural person's authorship and responsibility intact while making their toolchain auditable.
//!
//! (Timothy, 2026-07-03; see `docs/plans/adr-authority-attestation-guardianship-model.md` §8/§9/§10.)
//! When a natural agent forms a judgement *for another person*, they must declare which other agents
//! informed it. A `Reliance` records who/what contributed and how; because a consulted agent carries
//! *their own* [`JudgementProvenance`] (nested via [`Reliance::sub_provenance`]), `informed_by` is not a
//! flat list — it is a **directed acyclic graph** of contributions.
//!
//! Load-bearing distinctions:
//! - **Selfhood ≠ personhood, authorship stays with the natural person.** A [`AgentType::SoftwareAgent`]
//!   is *tooling & provenance* — it never bears liability, but its use MUST be disclosed and scoped. The
//!   natural person `responsible_agent` bears authorship + responsibility.
//! - **Dual-timed veracity.** Each contribution carries its veracity *as reasonably assessed at the time*
//!   ([`Reliance::veracity_at_time`], against the [`JudgementProvenance::epistemic_horizon`]) and *as later
//!   determined* by forensic review ([`Reliance::veracity_determined`]). The gap between the two is exactly
//!   where the malice / negligence / honest-error distinction lives (§10).
//! - **Epistemic horizon** is a content-addressed reference (Merkle root / checkpoint hash) to the
//!   information-state available to the agent at decision time — the hindsight-resistance mechanism.
//! - **Standing declaration.** A [`RelianceDeclaration`] lets an agent declare, up front, which tools they
//!   MAY use, so a principal can consent to the toolchain before relying on the agent. An
//!   *undeclared* software agent discovered later is a serious integrity breach — see [`has_undeclared_ai`].
//! - **Disclosure** is two independent axes (subject vs other agents) over a proof modality; the process
//!   subject's right-to-know is the first-class default (§9).

use serde::{Deserialize, Serialize};

/// The kind of agent that contributed to a judgement. Weight and liability differ per kind (§8.1):
/// a natural person bears authorship + responsibility; a software agent is tooling whose use must be
/// disclosed and scoped but which never bears liability; an instrument has calibration/validation state;
/// a dataset/source is provenance for input data. Modelled as an enum here (a closed, well-known set);
/// finer agent-kind facets live in the extensible `taxonomy`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum AgentType {
    /// Bears authorship + responsibility for the judgement.
    #[default]
    NaturalPerson,
    /// AI / automated tooling. Its use MUST be disclosed and scoped; it never bears liability.
    SoftwareAgent,
    /// A body (company, clinic, professional network) that stands behind a contribution.
    Organization,
    /// A measuring/analysis device — carries calibration/validation state (§10).
    Instrument,
    /// A dataset or data source relied upon as input.
    Dataset,
}

/// A reference to a contributing agent. `capacity` is a role/credential term id (e.g. a professional
/// registration URI); `version` pins a software agent or instrument to a specific version/configuration
/// so result variation is attributable (§10).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AgentRef {
    /// Stable identifier for the agent (a DID / URI / registration id). Matched by [`has_undeclared_ai`].
    pub id: String,
    pub agent_type: AgentType,
    /// A role/credential term id evidencing the capacity in which the agent acted, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    /// Version / configuration for a software agent or instrument (result-affecting, §10).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
}

impl AgentRef {
    /// A natural-person agent (bears authorship + responsibility).
    pub fn natural_person(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            agent_type: AgentType::NaturalPerson,
            capacity: None,
            version: None,
        }
    }

    /// A software agent / AI tool (must be disclosed; never bears liability).
    pub fn software_agent(id: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            agent_type: AgentType::SoftwareAgent,
            capacity: None,
            version: None,
        }
    }

    /// Generic constructor for any agent type.
    pub fn new(id: impl Into<String>, agent_type: AgentType) -> Self {
        Self {
            id: id.into(),
            agent_type,
            capacity: None,
            version: None,
        }
    }

    /// Set the capacity/credential term id (fluent).
    pub fn with_capacity(mut self, capacity: impl Into<String>) -> Self {
        self.capacity = Some(capacity.into());
        self
    }

    /// Set the version/configuration (fluent).
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = Some(version.into());
        self
    }
}

/// Veracity of a contributed input, over the extended epistemic vocabulary (§10). Used dual-timed:
/// once for the assessment *at the time* (against the epistemic horizon) and again for the later,
/// forensic determination. The default is [`InputVeracity::Uncertain`] — nothing is assumed accurate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum InputVeracity {
    /// Assessed as correct.
    Accurate,
    /// Not yet determined — the honest default before evidence resolves it.
    #[default]
    Uncertain,
    /// Contested; conflicting characterisations held paraconsistently, not collapsed.
    Disputed,
    /// Shown to be false (typically on hindsight / forensic review).
    Refuted,
    /// Deliberately false. If undetectable at the horizon, root cause attaches to the source, not the
    /// relying agent (§10).
    Malicious,
}

/// One contribution to a judgement — an edge in the provenance DAG. Records the contributing agent, the
/// **nature** of reliance (`"diagnostic-support"`, `"drafting"`, `"data-source"`, `"consult"`, …), whether
/// the agent/tool was used **within its validated/insured scope**, dual-timed veracity, and — when the
/// contributor is itself an agent that formed a judgement — that contributor's own nested provenance
/// (`sub_provenance`), which is what makes `informed_by` a DAG rather than a flat list.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Reliance {
    pub agent: AgentRef,
    /// How the agent was relied upon (diagnostic-support | drafting | data-source | consult | …).
    pub nature: String,
    /// Was the tool/agent used inside its validated/insured competence? Use outside declared scope for a
    /// consequential judgement is a red flag (§8.3).
    pub within_validated_scope: bool,
    /// Veracity as reasonably assessed *at the time* of reliance (against the epistemic horizon).
    #[serde(default)]
    pub veracity_at_time: InputVeracity,
    /// Veracity as *later determined* by forensic/hindsight review, if a re-assessment has occurred.
    /// `None` until reviewed. The gap vs `veracity_at_time` distinguishes malice/negligence from honest error.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub veracity_determined: Option<InputVeracity>,
    /// The consulted agent's *own* provenance (their toolchain), nested under this edge → the DAG.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub_provenance: Option<Box<JudgementProvenance>>,
}

impl Reliance {
    /// A new reliance edge. Veracity defaults to `Uncertain`; there is no later determination yet.
    pub fn new(agent: AgentRef, nature: impl Into<String>, within_validated_scope: bool) -> Self {
        Self {
            agent,
            nature: nature.into(),
            within_validated_scope,
            veracity_at_time: InputVeracity::default(),
            veracity_determined: None,
            sub_provenance: None,
        }
    }

    /// Set the veracity assessed at the time of reliance (fluent).
    pub fn assessed_at_time(mut self, veracity: InputVeracity) -> Self {
        self.veracity_at_time = veracity;
        self
    }

    /// Record a later, forensic determination of veracity (fluent).
    pub fn later_determined(mut self, veracity: InputVeracity) -> Self {
        self.veracity_determined = Some(veracity);
        self
    }

    /// Nest the contributor's own judgement provenance under this edge (fluent) — extends the DAG.
    pub fn with_sub_provenance(mut self, jp: JudgementProvenance) -> Self {
        self.sub_provenance = Some(Box::new(jp));
        self
    }
}

/// The provenance of a single judgement: the natural (or other) agent responsible for it, the ordered
/// set of contributions that informed it, an optional epistemic horizon, and free-form procedure notes.
/// Because each [`Reliance`] may carry a nested `sub_provenance`, this type is recursively a DAG node.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JudgementProvenance {
    /// The agent who bears authorship + responsibility for the judgement (`prov:wasAssociatedWith`).
    pub responsible_agent: AgentRef,
    /// The contributions that informed the judgement (`prov:used` / `prov:wasInformedBy`).
    #[serde(default)]
    pub informed_by: Vec<Reliance>,
    /// Content-addressed reference (Merkle root / checkpoint hash) to the information-state available at
    /// decision time — the hindsight-resistance mechanism (§9). `None` until captured.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub epistemic_horizon: Option<String>,
    /// Free-form notes on the procedure/method followed.
    #[serde(default)]
    pub procedure_notes: String,
}

impl JudgementProvenance {
    /// A judgement attributed to `responsible_agent`, with no contributions declared yet.
    pub fn new(responsible_agent: AgentRef) -> Self {
        Self {
            responsible_agent,
            informed_by: Vec::new(),
            epistemic_horizon: None,
            procedure_notes: String::new(),
        }
    }

    /// Declare one contribution that informed the judgement (fluent, chainable).
    pub fn informed_by(mut self, reliance: Reliance) -> Self {
        self.informed_by.push(reliance);
        self
    }

    /// Pin the epistemic horizon (content-addressed info-state hash) at decision time (fluent).
    pub fn with_horizon(mut self, hash: impl Into<String>) -> Self {
        self.epistemic_horizon = Some(hash.into());
        self
    }

    /// Set the procedure notes (fluent).
    pub fn with_procedure_notes(mut self, notes: impl Into<String>) -> Self {
        self.procedure_notes = notes.into();
        self
    }
}

/// A **standing** declaration (capacity-level) of the agents/tools an agent MAY use, so a principal can
/// evaluate and consent to the toolchain *before* relying on the agent (§8.2). This is not the per-judgement
/// record; it is the up-front permission surface against which [`has_undeclared_ai`] checks actual reliance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RelianceDeclaration {
    /// The agent making the declaration (typically a natural person in a professional capacity).
    pub declaring_agent: AgentRef,
    /// The role/capacity term id this declaration is scoped to, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capacity: Option<String>,
    /// The agents/tools the declaring agent is permitted to rely upon (matched by `AgentRef.id`).
    #[serde(default)]
    pub permitted: Vec<AgentRef>,
    /// When the declaration takes effect (unix seconds). `u32` is intentional (a coarse effective-date,
    /// not a high-precision timestamp).
    pub effective_from_unix: u32,
}

impl RelianceDeclaration {
    /// A new declaration effective from `effective_from_unix`, permitting nothing until tools are added.
    pub fn new(declaring_agent: AgentRef, effective_from_unix: u32) -> Self {
        Self {
            declaring_agent,
            capacity: None,
            permitted: Vec::new(),
            effective_from_unix,
        }
    }

    /// Scope the declaration to a capacity/credential term id (fluent).
    pub fn with_capacity(mut self, capacity: impl Into<String>) -> Self {
        self.capacity = Some(capacity.into());
        self
    }

    /// Permit reliance on an agent/tool (fluent, chainable).
    pub fn permit(mut self, agent: AgentRef) -> Self {
        self.permitted.push(agent);
        self
    }

    /// True if an agent id is covered by this declaration's permitted set.
    pub fn permits_id(&self, id: &str) -> bool {
        self.permitted.iter().any(|a| a.id == id)
    }
}

/// The proof modality by which a provenance node (or the DAG) is disclosed (§9).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum DisclosureModality {
    /// Full reveal of the provenance.
    #[default]
    Full,
    /// Reveal a subset of the DAG's fields/nodes (selective field disclosure).
    SelectiveField,
    /// Prove a property of the provenance *without* revealing the underlying identities/tools — e.g.
    /// "≥2 licensed physicians", "tool within validated scope", "no undeclared AI".
    ///
    /// Where a real zero-knowledge circuit exists for the predicate, this is genuine ZK: real Groth16 over
    /// BLS12-381 lives in `crypto/zk_proofs.rs` (a separate crate). Where **no** circuit exists for the
    /// predicate yet, this modality falls back to a **signed commitment + selective field disclosure** and
    /// MUST be labelled as such — it is NOT to be called ZK until a circuit backs it.
    PropertyProof,
}

/// Per-node disclosure policy over two independent axes (§9): what the **process subject** (the natural
/// person the decision is about) may see, versus what **other agents / institutions** may see. The
/// subject's right-to-know is the first-class default; others get proportionate disclosure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisclosurePolicy {
    /// What the process subject may see. Defaults to [`DisclosureModality::Full`].
    pub subject: DisclosureModality,
    /// What other agents/institutions may see. Defaults to [`DisclosureModality::SelectiveField`].
    pub other_agents: DisclosureModality,
}

impl Default for DisclosurePolicy {
    fn default() -> Self {
        // The subject's right-to-know is first-class; others get proportionate/selective disclosure.
        Self {
            subject: DisclosureModality::Full,
            other_agents: DisclosureModality::SelectiveField,
        }
    }
}

/// The integrity-breach detector (§8.3): returns `true` if any [`AgentType::SoftwareAgent`] appearing in
/// `jp.informed_by` (recursively, through nested `sub_provenance`) is **not** covered by `declaration`'s
/// permitted set (matched by `AgentRef.id`). An undeclared AI reliance discovered after the fact is a
/// serious integrity breach — the omission is itself evidence.
///
/// Only software agents are checked; other agent types are not subject to the standing-declaration gate.
pub fn has_undeclared_ai(jp: &JudgementProvenance, declaration: &RelianceDeclaration) -> bool {
    for reliance in &jp.informed_by {
        if reliance.agent.agent_type == AgentType::SoftwareAgent
            && !declaration.permits_id(&reliance.agent.id)
        {
            return true;
        }
        if let Some(sub) = &reliance.sub_provenance {
            if has_undeclared_ai(sub, declaration) {
                return true;
            }
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a 2-level DAG: agent A's judgement is informed by B (a consult), and B's contribution
    /// carries B's *own* provenance, which is in turn informed by a software agent tool.
    fn two_level_dag(tool: AgentRef) -> JudgementProvenance {
        // Level 2: agent B's own provenance, using an AI tool for drafting.
        let b_provenance = JudgementProvenance::new(AgentRef::natural_person("did:example:b"))
            .with_horizon("merkle:b-info-state")
            .with_procedure_notes("B drafted using the tool, within scope.")
            .informed_by(
                Reliance::new(tool, "drafting", true).assessed_at_time(InputVeracity::Accurate),
            );

        // Level 1: agent A's judgement, informed by a consult with B (B's provenance nested).
        JudgementProvenance::new(AgentRef::natural_person("did:example:a"))
            .with_horizon("merkle:a-info-state")
            .with_procedure_notes("A consulted B before forming the judgement.")
            .informed_by(
                Reliance::new(
                    AgentRef::natural_person("did:example:b").with_capacity("urn:cap:psychologist"),
                    "consult",
                    true,
                )
                .with_sub_provenance(b_provenance),
            )
    }

    #[test]
    fn builds_two_level_dag() {
        let jp = two_level_dag(AgentRef::software_agent("did:tool:llm-x"));
        assert_eq!(jp.responsible_agent.id, "did:example:a");
        assert_eq!(jp.informed_by.len(), 1);

        // A is informed by B (a consult).
        let consult = &jp.informed_by[0];
        assert_eq!(consult.nature, "consult");
        assert_eq!(consult.agent.id, "did:example:b");

        // B carries its own nested provenance (the DAG's second level).
        let b_prov = consult
            .sub_provenance
            .as_ref()
            .expect("B's sub-provenance should be present");
        assert_eq!(b_prov.responsible_agent.id, "did:example:b");
        assert_eq!(b_prov.informed_by.len(), 1);
        assert_eq!(
            b_prov.informed_by[0].agent.agent_type,
            AgentType::SoftwareAgent
        );
        assert_eq!(
            b_prov.epistemic_horizon.as_deref(),
            Some("merkle:b-info-state")
        );
    }

    #[test]
    fn default_disclosure_gives_subject_full_others_selective() {
        let policy = DisclosurePolicy::default();
        assert_eq!(policy.subject, DisclosureModality::Full);
        assert_eq!(policy.other_agents, DisclosureModality::SelectiveField);
    }

    #[test]
    fn input_veracity_defaults_to_uncertain() {
        assert_eq!(InputVeracity::default(), InputVeracity::Uncertain);
        // A fresh reliance inherits the uncertain default and has no later determination yet.
        let r = Reliance::new(AgentRef::natural_person("did:x"), "data-source", true);
        assert_eq!(r.veracity_at_time, InputVeracity::Uncertain);
        assert_eq!(r.veracity_determined, None);
    }

    #[test]
    fn agent_type_default_is_natural_person() {
        assert_eq!(AgentType::default(), AgentType::NaturalPerson);
        // A generically-constructed reference to a person bears authorship.
        assert_eq!(
            AgentRef::natural_person("did:p").agent_type,
            AgentType::NaturalPerson
        );
    }

    #[test]
    fn detects_undeclared_ai_deep_in_the_dag() {
        // The AI tool used two levels down is NOT covered by the declaration → integrity breach.
        let jp = two_level_dag(AgentRef::software_agent("did:tool:secret-llm"));
        let declaration =
            RelianceDeclaration::new(AgentRef::natural_person("did:example:a"), 1_700_000_000)
                .with_capacity("urn:cap:psychologist")
                .permit(AgentRef::software_agent("did:tool:approved-llm"));

        assert!(
            has_undeclared_ai(&jp, &declaration),
            "an undeclared AI nested in the DAG must be detected"
        );
    }

    #[test]
    fn returns_false_when_ai_is_declared() {
        // Same DAG, but now the tool used deep in the DAG IS permitted → no breach.
        let jp = two_level_dag(AgentRef::software_agent("did:tool:llm-x"));
        let declaration =
            RelianceDeclaration::new(AgentRef::natural_person("did:example:a"), 1_700_000_000)
                .permit(AgentRef::software_agent("did:tool:llm-x"));

        assert!(
            !has_undeclared_ai(&jp, &declaration),
            "a declared AI must not be flagged as undeclared"
        );
        assert!(declaration.permits_id("did:tool:llm-x"));
    }

    #[test]
    fn non_software_agents_are_not_gated_by_declaration() {
        // A dataset/instrument contribution is not a SoftwareAgent, so an empty declaration is fine.
        let jp = JudgementProvenance::new(AgentRef::natural_person("did:example:a"))
            .informed_by(Reliance::new(
                AgentRef::new("did:dataset:registry", AgentType::Dataset),
                "data-source",
                true,
            ))
            .informed_by(Reliance::new(
                AgentRef::new("did:instr:analyzer", AgentType::Instrument).with_version("v2.1"),
                "measurement",
                true,
            ));
        let declaration =
            RelianceDeclaration::new(AgentRef::natural_person("did:example:a"), 1_700_000_000);
        assert!(!has_undeclared_ai(&jp, &declaration));
    }

    #[test]
    fn dual_timed_veracity_round_trips_through_serde() {
        // A reliance judged accurate at the time but later refuted (the malice/negligence-detecting gap).
        let reliance = Reliance::new(
            AgentRef::software_agent("did:tool:llm-x").with_version("2026.01"),
            "diagnostic-support",
            true,
        )
        .assessed_at_time(InputVeracity::Accurate)
        .later_determined(InputVeracity::Refuted);

        let json = serde_json::to_string(&reliance).expect("serialize");
        // snake_case serde on the enum.
        assert!(json.contains("\"veracity_at_time\":\"accurate\""));
        assert!(json.contains("\"veracity_determined\":\"refuted\""));

        let back: Reliance = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, reliance);
        assert_eq!(back.veracity_at_time, InputVeracity::Accurate);
        assert_eq!(back.veracity_determined, Some(InputVeracity::Refuted));
    }

    #[test]
    fn whole_provenance_round_trips_through_serde() {
        let jp = two_level_dag(AgentRef::software_agent("did:tool:llm-x"));
        let json = serde_json::to_string(&jp).expect("serialize");
        let back: JudgementProvenance = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, jp);
    }

    #[test]
    fn out_of_scope_reliance_is_recorded() {
        // Using a tool outside its validated scope is a red flag the record must preserve faithfully.
        let r = Reliance::new(
            AgentRef::software_agent("did:tool:llm-x"),
            "diagnostic-support",
            false,
        );
        assert!(!r.within_validated_scope);
    }

    #[test]
    fn property_proof_modality_is_available() {
        // The disclosure modality enum offers the property-proof option (labelled honestly in docs).
        let policy = DisclosurePolicy {
            subject: DisclosureModality::Full,
            other_agents: DisclosureModality::PropertyProof,
        };
        assert_eq!(policy.other_agents, DisclosureModality::PropertyProof);
    }
}
