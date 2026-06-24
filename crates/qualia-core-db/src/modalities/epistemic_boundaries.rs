use crate::{NQuin, q_hash};

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

/// Identifies if a Quin encodes a definitive claim that crosses Epistemic Boundaries.
pub fn identify_degradation_vector(quin: &NQuin) -> DegradationVector {
    let predicate = quin.predicate;
    
    // Medical/Biological Vectors
    if predicate == q_hash("q42:biochemicalPathway") || predicate == q_hash("q42:medicalDiagnosis") {
        return DegradationVector::BiochemicalPathway;
    }
    if predicate == q_hash("q42:thermodynamicState") || predicate == q_hash("q42:kineticSimulation") {
        return DegradationVector::ThermodynamicTracking;
    }
    if predicate == q_hash("q42:genomicAlignment") || predicate == q_hash("q42:sequenceProcessing") {
        return DegradationVector::GenomicPrivacy;
    }
    
    // Legal/Jural Vectors
    if predicate == q_hash("q42:contractualClause") || predicate == q_hash("q42:legalVerdict") {
        return DegradationVector::ContractualPower;
    }
    if predicate == q_hash("q42:proportionalityTest") || predicate == q_hash("q42:rightsConflict") {
        return DegradationVector::ProportionalityTest;
    }
    if predicate == q_hash("q42:governancePolicy") || predicate == q_hash("q42:enforcementProtocol") {
        return DegradationVector::InteractionGovernance;
    }
    
    // Fiduciary Vectors
    if predicate == q_hash("q42:financialFiduciary") || predicate == q_hash("q42:investmentDirective") {
        return DegradationVector::FiduciaryAdvice;
    }
    
    DegradationVector::Unknown
}

/// The Linguistic Degradation Matrix.
/// Hardcoded zero-heap degradation from definitive claims to Socratic/Probabilistic maps.
pub fn degrade_claim_to_socratic(vector: DegradationVector) -> Option<SocraticDegradation> {
    const BIO_DISCLAIMER: &str = "**Systemic Biological Analysis:** The outputs generated in this subgraph represent a mechanistic, probabilistic simulation of physiological and thermodynamic variables. They are constructed for educational ontology mapping and structural biological research. This system does not diagnose, treat, or prescribe. For definitive clinical applications, these findings should be presented to a certified medical practitioner.";
    const LEGAL_DISCLAIMER: &str = "**Informational/Educational Ontology:** This nquin subgraph represents a logical simulation of jural relations, deontic logic, and structural human rights frameworks. It is designed to empower individual agency through systemic education and dialectical mapping. This engine does not provide binding legal counsel, representation, or authoritative statutory interpretation. You are encouraged to seek licensed legal counsel for actionable fiduciary validation.";
    
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
