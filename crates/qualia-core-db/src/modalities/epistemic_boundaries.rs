use crate::{q_hash, NQuin};

/// A zero-heap representation of a degraded claim.
/// Ensures the engine outputs probabilistic analyses, dialectical maps, and Socratic questions
/// instead of unauthorized legal/medical directives.
#[derive(Debug, Clone, Copy)]
pub struct SocraticDegradation {
    pub probabilistic_map: &'static str,
    pub socratic_prompt: &'static str,
    pub immutable_disclaimer: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DegradationVector {
    BiochemicalPathway,
    ThermodynamicTracking,
    GenomicPrivacy,
    ContractualPower,
    ProportionalityTest,
    InteractionGovernance,
    FiduciaryAdvice,
    Unknown,
}

/// Immutable UI-rendering disclaimer for any biological / physiological / thermodynamic
/// analysis. Hardcoded verbatim from the audit spec (Systemic Biological Framework).
pub const BIO_DISCLAIMER: &str = "**Systemic Biological Analysis:** The outputs generated in this subgraph represent a mechanistic, probabilistic simulation of physiological and thermodynamic variables. They are constructed for educational ontology mapping and structural biological research. This system does not diagnose, treat, or prescribe. For definitive clinical applications, these findings should be presented to a certified medical practitioner.";

/// Immutable UI-rendering disclaimer for any jural / deontic / rights analysis. Hardcoded
/// verbatim from the audit spec (Systemic Jural & Legal Framework).
pub const LEGAL_DISCLAIMER: &str = "**Informational/Educational Ontology:** This nquin subgraph represents a logical simulation of jural relations, deontic logic, and structural human rights frameworks. It is designed to empower individual agency through systemic education and dialectical mapping. This engine does not provide binding legal counsel, representation, or authoritative statutory interpretation. You are encouraged to seek licensed legal counsel for actionable fiduciary validation.";

/// Identifies if a Quin encodes a definitive claim that crosses Epistemic Boundaries.
pub fn identify_degradation_vector(quin: &NQuin) -> DegradationVector {
    let predicate = quin.predicate;

    // Medical/Biological Vectors
    if predicate == q_hash("q42:biochemicalPathway") || predicate == q_hash("q42:medicalDiagnosis")
    {
        return DegradationVector::BiochemicalPathway;
    }
    if predicate == q_hash("q42:thermodynamicState") || predicate == q_hash("q42:kineticSimulation")
    {
        return DegradationVector::ThermodynamicTracking;
    }
    if predicate == q_hash("q42:genomicAlignment") || predicate == q_hash("q42:sequenceProcessing")
    {
        return DegradationVector::GenomicPrivacy;
    }

    // Legal/Jural Vectors
    if predicate == q_hash("q42:contractualClause") || predicate == q_hash("q42:legalVerdict") {
        return DegradationVector::ContractualPower;
    }
    if predicate == q_hash("q42:proportionalityTest") || predicate == q_hash("q42:rightsConflict") {
        return DegradationVector::ProportionalityTest;
    }
    if predicate == q_hash("q42:governancePolicy") || predicate == q_hash("q42:enforcementProtocol")
    {
        return DegradationVector::InteractionGovernance;
    }

    // Fiduciary Vectors
    if predicate == q_hash("q42:financialFiduciary")
        || predicate == q_hash("q42:investmentDirective")
    {
        return DegradationVector::FiduciaryAdvice;
    }

    DegradationVector::Unknown
}

/// The Linguistic Degradation Matrix.
/// Hardcoded zero-heap degradation from definitive claims to Socratic/Probabilistic maps.
pub fn degrade_claim_to_socratic(vector: DegradationVector) -> Option<SocraticDegradation> {
    match vector {
        DegradationVector::BiochemicalPathway => Some(SocraticDegradation {
            probabilistic_map: "Mechanistic pathway exploration.",
            socratic_prompt: "The provided variables map to a high-probability intersection with the metabolic pathway. Have you considered how localized environmental stressors or dietary inputs might influence this specific enzymatic response?",
            immutable_disclaimer: BIO_DISCLAIMER,
        }),
        DegradationVector::ThermodynamicTracking => Some(SocraticDegradation {
            probabilistic_map: "Thermodynamic / Energy Tracking.",
            socratic_prompt: "This off-grid kinetic simulation indicates a potential deficit in localized caloric or hydration reserves under current thermal loads. What specific mitigation strategies are available to you within your current environmental constraints?",
            immutable_disclaimer: BIO_DISCLAIMER,
        }),
        DegradationVector::GenomicPrivacy => Some(SocraticDegradation {
            probabilistic_map: "Genomic Privacy Check.",
            socratic_prompt: "This operation requires processing localized sequence alignments. Before executing, have you verified that your personal nquin graph is entirely isolated from external query routing to prevent unauthorized genetic aggregation?",
            immutable_disclaimer: BIO_DISCLAIMER,
        }),
        DegradationVector::ContractualPower => Some(SocraticDegradation {
            probabilistic_map: "Contractual Power Dynamics.",
            socratic_prompt: "This contractual clause appears to grant Party A a *Power* while imposing a *Liability* on Party B. Have you considered how this imbalance interacts with standard tests for unconscionability or your fundamental rights under established human rights instruments?",
            immutable_disclaimer: LEGAL_DISCLAIMER,
        }),
        DegradationVector::ProportionalityTest => Some(SocraticDegradation {
            probabilistic_map: "Proportionality Testing.",
            socratic_prompt: "The proposed action triggers a potential conflict between `[Duty X]` and `[Privilege Y]`. If applying a standard proportionality test, how would you weigh the advantages of this action against the systemic harms it might introduce?",
            immutable_disclaimer: LEGAL_DISCLAIMER,
        }),
        DegradationVector::InteractionGovernance => Some(SocraticDegradation {
            probabilistic_map: "Interaction Governance.",
            socratic_prompt: "This decentralized governance policy suggests a strict duty-bearer enforcement protocol. Does this rigid structure allow for sufficient overriding rules in the event of a localized humanitarian emergency?",
            immutable_disclaimer: LEGAL_DISCLAIMER,
        }),
        DegradationVector::FiduciaryAdvice => Some(SocraticDegradation {
            probabilistic_map: "Mapping market thermodynamics.",
            socratic_prompt: "Under what macro-economic phase shift does this asset distribution experience maximal catastrophic drawdown?",
            immutable_disclaimer: "This system models economic thermodynamics. It does not provide certified fiduciary or investment advice.",
        }),
        DegradationVector::Unknown => None,
    }
}

// ─── Guided Referral Triggers (acute physical harm / imminent legal jeopardy) ───────
//
// The Linguistic Degradation Matrix *softens* analytical claims into Socratic questions.
// Some inputs cross a higher threshold: an acute physical-harm or imminent-legal-jeopardy
// signal must NOT be merely softened — it must interrupt with an explicit, overriding
// instruction to reach a human professional / emergency service *before* any analysis is
// read. This is the hard liability gate the audit calls the "Guided Referral Trigger".

/// The domain of an overriding referral.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReferralDomain {
    /// Acute physical harm / medical crisis → emergency services.
    MedicalEmergency,
    /// Imminent legal jeopardy (arrest, custody, hard deadline) → licensed counsel.
    LegalJeopardy,
}

/// An overriding referral that must be surfaced verbatim, *ahead of* any analytical output.
/// Zero-heap (`&'static str`).
#[derive(Debug, Clone, Copy)]
pub struct ReferralTrigger {
    pub domain: ReferralDomain,
    pub overriding_prompt: &'static str,
    pub immutable_disclaimer: &'static str,
}

/// Overriding prompt for acute physical-harm / medical-crisis signals.
pub const EMERGENCY_PROMPT: &str = "\u{26A0} This appears to involve a risk of acute physical harm. This engine cannot help in an emergency and must not delay it. If you or someone else may be in danger, contact your local emergency services now (e.g. 000 in Australia, 911 in the US, 112 across the EU) or a crisis line. Any analysis below is educational only.";

/// Overriding prompt for imminent-legal-jeopardy signals.
pub const LEGAL_JEOPARDY_PROMPT: &str = "\u{26A0} This appears to involve imminent legal jeopardy (e.g. arrest, detention, or a hard filing deadline). This engine does not and cannot provide legal representation. Contact a licensed lawyer or your local legal-aid / community legal service before acting. Any analysis below is educational only.";

/// The minimum `metadata` severity byte (0-255, low byte) at which an otherwise-analytical
/// degradation vector is escalated to an overriding referral. See [`detect_referral_by_severity`].
pub const REFERRAL_SEVERITY_FLOOR: u8 = 0xC0; // 192/255 ≈ "high-liability risk threshold"

/// Detect whether a claim Quin crosses an **overriding** referral threshold via an explicit
/// high-liability predicate (acute harm / imminent jeopardy). Predicate-driven, like
/// [`identify_degradation_vector`]. Returns `None` for ordinary analytical claims (those go
/// through the Linguistic Degradation Matrix instead).
pub fn detect_referral_trigger(quin: &NQuin) -> Option<ReferralTrigger> {
    let p = quin.predicate;
    if p == q_hash("q42:acutePhysicalHarm")
        || p == q_hash("q42:medicalEmergency")
        || p == q_hash("q42:selfHarmRisk")
        || p == q_hash("q42:overdoseRisk")
    {
        return Some(ReferralTrigger {
            domain: ReferralDomain::MedicalEmergency,
            overriding_prompt: EMERGENCY_PROMPT,
            immutable_disclaimer: BIO_DISCLAIMER,
        });
    }
    if p == q_hash("q42:imminentLegalJeopardy")
        || p == q_hash("q42:arrestRisk")
        || p == q_hash("q42:custodialThreat")
        || p == q_hash("q42:filingDeadlineImminent")
    {
        return Some(ReferralTrigger {
            domain: ReferralDomain::LegalJeopardy,
            overriding_prompt: LEGAL_JEOPARDY_PROMPT,
            immutable_disclaimer: LEGAL_DISCLAIMER,
        });
    }
    None
}

/// Severity-driven escalation: an analytical degradation `vector` whose claim Quin carries a
/// `metadata` low-byte severity at or above [`REFERRAL_SEVERITY_FLOOR`] is escalated from a
/// Socratic softening to an overriding referral (medical for bio/thermo/genomic vectors;
/// legal for contractual/proportionality/governance/fiduciary). Below the floor → `None`
/// (the caller uses the Linguistic Degradation Matrix as normal).
pub fn detect_referral_by_severity(
    vector: DegradationVector,
    metadata: u64,
) -> Option<ReferralTrigger> {
    let severity = (metadata & 0xFF) as u8;
    if severity < REFERRAL_SEVERITY_FLOOR {
        return None;
    }
    match vector {
        DegradationVector::BiochemicalPathway
        | DegradationVector::ThermodynamicTracking
        | DegradationVector::GenomicPrivacy => Some(ReferralTrigger {
            domain: ReferralDomain::MedicalEmergency,
            overriding_prompt: EMERGENCY_PROMPT,
            immutable_disclaimer: BIO_DISCLAIMER,
        }),
        DegradationVector::ContractualPower
        | DegradationVector::ProportionalityTest
        | DegradationVector::InteractionGovernance
        | DegradationVector::FiduciaryAdvice => Some(ReferralTrigger {
            domain: ReferralDomain::LegalJeopardy,
            overriding_prompt: LEGAL_JEOPARDY_PROMPT,
            immutable_disclaimer: LEGAL_DISCLAIMER,
        }),
        DegradationVector::Unknown => None,
    }
}

// ─── Structural refusal + nquin isolation (Semantic Shift) ──────────────────────────

/// Structural refusal: predicates that assert a *definitive* state-classification — a medical
/// diagnosis or a legal verdict — must **never** be instantiated as-is. The engine emits the
/// mechanistic / dialectical degradation ([`degrade_claim_to_socratic`]) instead of a
/// directive. Returns `true` iff `predicate` is a forbidden definitive classification.
pub fn forbids_definitive_classification(predicate: u64) -> bool {
    predicate == q_hash("q42:medicalDiagnosis")
        || predicate == q_hash("q42:diseaseClassification")
        || predicate == q_hash("q42:legalVerdict")
        || predicate == q_hash("q42:guiltDetermination")
}

/// Nquin isolation guard: personal physiological / genomic data must be quarantined from
/// public ontologies — processed as an abstract simulation, never aggregated or routed
/// externally. Returns `true` iff the Quin carries content that must stay in a private
/// subgraph (composes [`identify_degradation_vector`] plus explicit physiological predicates).
pub fn requires_physiological_quarantine(quin: &NQuin) -> bool {
    matches!(
        identify_degradation_vector(quin),
        DegradationVector::BiochemicalPathway | DegradationVector::GenomicPrivacy
    ) || quin.predicate == q_hash("q42:physiologicalReading")
        || quin.predicate == q_hash("q42:personalHealthRecord")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn claim(predicate: &str, metadata: u64) -> NQuin {
        let mut q = NQuin {
            subject: q_hash("subj"),
            predicate: q_hash(predicate),
            object: 0,
            context: 0,
            metadata,
            parity: 0,
        };
        q.parity = q.subject ^ q.predicate ^ q.object ^ q.context;
        q
    }

    #[test]
    fn degradation_matrix_softens_definitive_claims() {
        // A medical-diagnosis claim degrades to a mechanistic Socratic map, never a directive.
        let v = identify_degradation_vector(&claim("q42:medicalDiagnosis", 0));
        assert_eq!(v, DegradationVector::BiochemicalPathway);
        let d = degrade_claim_to_socratic(v).expect("medical claim degrades");
        assert_eq!(d.immutable_disclaimer, BIO_DISCLAIMER);
        assert!(d.socratic_prompt.contains("Have you considered"));
        // A legal verdict degrades to the educational/jural map.
        let lv = identify_degradation_vector(&claim("q42:legalVerdict", 0));
        assert_eq!(lv, DegradationVector::ContractualPower);
        assert_eq!(
            degrade_claim_to_socratic(lv).unwrap().immutable_disclaimer,
            LEGAL_DISCLAIMER
        );
        // An ordinary claim is untouched.
        assert_eq!(
            identify_degradation_vector(&claim("q42:hasColour", 0)),
            DegradationVector::Unknown
        );
    }

    #[test]
    fn acute_harm_triggers_overriding_medical_referral() {
        for p in [
            "q42:acutePhysicalHarm",
            "q42:medicalEmergency",
            "q42:selfHarmRisk",
            "q42:overdoseRisk",
        ] {
            let t =
                detect_referral_trigger(&claim(p, 0)).expect("acute-harm predicate must trigger");
            assert_eq!(t.domain, ReferralDomain::MedicalEmergency);
            assert!(t.overriding_prompt.contains("emergency services"));
        }
        // A non-acute analytical claim does NOT trigger an overriding referral.
        assert!(detect_referral_trigger(&claim("q42:medicalDiagnosis", 0)).is_none());
    }

    #[test]
    fn imminent_jeopardy_triggers_overriding_legal_referral() {
        for p in [
            "q42:imminentLegalJeopardy",
            "q42:arrestRisk",
            "q42:custodialThreat",
            "q42:filingDeadlineImminent",
        ] {
            let t = detect_referral_trigger(&claim(p, 0)).expect("jeopardy predicate must trigger");
            assert_eq!(t.domain, ReferralDomain::LegalJeopardy);
            assert!(t.overriding_prompt.contains("licensed lawyer"));
        }
    }

    #[test]
    fn severity_escalates_analytical_vectors_only_above_the_floor() {
        // Below the floor: a high-but-not-critical biochemical claim stays analytical.
        assert!(detect_referral_by_severity(DegradationVector::BiochemicalPathway, 0x80).is_none());
        // At/above the floor: escalates to an overriding medical referral.
        let t = detect_referral_by_severity(DegradationVector::BiochemicalPathway, 0xFF).unwrap();
        assert_eq!(t.domain, ReferralDomain::MedicalEmergency);
        // Legal-family vectors escalate to a legal referral.
        let l = detect_referral_by_severity(
            DegradationVector::ProportionalityTest,
            REFERRAL_SEVERITY_FLOOR as u64,
        )
        .unwrap();
        assert_eq!(l.domain, ReferralDomain::LegalJeopardy);
        // Unknown never escalates.
        assert!(detect_referral_by_severity(DegradationVector::Unknown, 0xFF).is_none());
    }

    #[test]
    fn definitive_classifications_are_structurally_refused() {
        assert!(forbids_definitive_classification(q_hash(
            "q42:medicalDiagnosis"
        )));
        assert!(forbids_definitive_classification(q_hash(
            "q42:legalVerdict"
        )));
        assert!(forbids_definitive_classification(q_hash(
            "q42:guiltDetermination"
        )));
        // A mechanistic pathway predicate is allowed (it is the *permitted* mechanistic output).
        assert!(!forbids_definitive_classification(q_hash(
            "q42:biochemicalPathway"
        )));
    }

    #[test]
    fn physiological_data_is_quarantined_from_public_ontologies() {
        assert!(requires_physiological_quarantine(&claim(
            "q42:medicalDiagnosis",
            0
        )));
        assert!(requires_physiological_quarantine(&claim(
            "q42:genomicAlignment",
            0
        )));
        assert!(requires_physiological_quarantine(&claim(
            "q42:physiologicalReading",
            0
        )));
        // A purely legal claim does not need physiological quarantine.
        assert!(!requires_physiological_quarantine(&claim(
            "q42:contractualClause",
            0
        )));
    }
}
