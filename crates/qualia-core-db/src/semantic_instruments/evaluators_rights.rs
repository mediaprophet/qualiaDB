//! Bounded evaluators for `identifier-stewardship-grant-scope`, `contextual-assertions-review`,
//! `wellfair-fairness-rights-review`, and `licence-transition-readiness` (`QUALIADB-IMPLEMENTATION-ROADMAP.md`).

use serde::{Deserialize, Serialize};

use super::evaluators::{
    finalize_report, CoreInstrumentKind, DiagnosticFinding, EvaluationOutcome,
    InstrumentEvaluationReport,
};
use crate::q_hash;

/// Input contract for `identifier-stewardship-grant-scope`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct IdentifierStewardshipGrantScope {
    pub subject_did: String,
    pub invoking_agent_did: String,
    pub granted_purpose: String,
    pub requested_purpose: String,
    pub has_domain_authority: bool,
    #[serde(default)]
    pub is_guardian_or_proxy: bool,
    #[serde(default)]
    pub competing_guardianship_claims: bool,
    #[serde(default)]
    pub subject_human_objected: bool,
    #[serde(default)]
    pub touches_biometrics_or_mindware: bool,
    #[serde(default)]
    pub explicit_biometric_consent: bool,
}

/// Evaluate `identifier-stewardship-grant-scope`.
pub fn evaluate_identifier_grant_scope(
    input: &IdentifierStewardshipGrantScope,
    now_unix: u32,
) -> InstrumentEvaluationReport {
    let mut findings = Vec::new();
    if input.subject_did.trim().is_empty()
        || input.granted_purpose.trim().is_empty()
        || input.requested_purpose.trim().is_empty()
    {
        findings.push(DiagnosticFinding {
            code: "hro:IncompleteGrantScopeInput".into(),
            outcome: EvaluationOutcome::Held,
            message: "missing subject DID or declared purpose".into(),
        });
    } else {
        if !input.has_domain_authority {
            findings.push(DiagnosticFinding {
                code: "hro:MissingDomainOrStewardshipAuthority".into(),
                outcome: EvaluationOutcome::Failed,
                message: "invoking agent lacks DID/namespace/domain authority".into(),
            });
        }
        if input.subject_human_objected {
            findings.push(DiagnosticFinding {
                code: "hro:SubjectHumanObjected".into(),
                outcome: EvaluationOutcome::Failed,
                message: "affected human objected to disclosure or delegated action".into(),
            });
        }
        if input.is_guardian_or_proxy && input.competing_guardianship_claims {
            findings.push(DiagnosticFinding {
                code: "hro:CompetingGuardianshipClaimsHeld".into(),
                outcome: EvaluationOutcome::Held,
                message: "competing guardianship claims require holding dependent disclosure for competent review".into(),
            });
        }
        if !input
            .granted_purpose
            .trim()
            .eq_ignore_ascii_case(input.requested_purpose.trim())
        {
            findings.push(DiagnosticFinding {
                code: "hro:OutOfScopePurposeOrProfiling".into(),
                outcome: EvaluationOutcome::Failed,
                message: "requested purpose exceeds the scoped identifier grant".into(),
            });
        }
        if input.touches_biometrics_or_mindware && !input.explicit_biometric_consent {
            findings.push(DiagnosticFinding {
                code: "hro:ProtectedBiometricsOrMindwareBoundary".into(),
                outcome: EvaluationOutcome::Failed,
                message: "biometrics and mindware require explicit scoped human consent".into(),
            });
        }
    }
    finalize_report(
        CoreInstrumentKind::IdentifierStewardshipGrantScope,
        q_hash(&input.subject_did),
        findings,
        now_unix,
    )
}

/// Epistemic assertion category for `contextual-assertions-review`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssertionKind {
    #[default]
    Observation,
    Inference,
    Allegation,
    Attestation,
    AdjudicatedFinding,
}

/// Input contract for `contextual-assertions-review`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextualAssertionsReview {
    pub assertion_kind: AssertionKind,
    #[serde(default)]
    pub claimed_target_kind: Option<AssertionKind>,
    pub distinct_root_sources: u32,
    #[serde(default)]
    pub dependent_inference_count: u32,
    #[serde(default)]
    pub has_human_adjudicator_did: bool,
    #[serde(default)]
    pub is_challenged: bool,
    #[serde(default)]
    pub challenge_preserved: bool,
    #[serde(default)]
    pub downstream_marked_for_review: bool,
    pub scope_domain: String,
    #[serde(default)]
    pub collapses_to_universal_reputation_score: bool,
}

/// Evaluate `contextual-assertions-review`.
pub fn evaluate_contextual_assertions(
    input: &ContextualAssertionsReview,
    now_unix: u32,
) -> InstrumentEvaluationReport {
    let mut findings = Vec::new();
    let scope = input.scope_domain.trim().to_ascii_lowercase();
    if input.collapses_to_universal_reputation_score
        || scope == "universal"
        || scope == "global_reputation"
        || scope == "social_credit"
    {
        findings.push(DiagnosticFinding {
            code: "hro:UniversalReputationScoreForbidden".into(),
            outcome: EvaluationOutcome::Failed,
            message: "universal human reputation scoring across contexts is forbidden".into(),
        });
    } else if scope.is_empty() || input.distinct_root_sources == 0 {
        findings.push(DiagnosticFinding {
            code: "hro:MissingScopeOrRootProvenance".into(),
            outcome: EvaluationOutcome::Held,
            message: "assertion review requires a bounded scope domain and root provenance source"
                .into(),
        });
    }
    if input.claimed_target_kind == Some(AssertionKind::Observation)
        && input.assertion_kind != AssertionKind::Observation
    {
        findings.push(DiagnosticFinding {
            code: "hro:CannotPromoteToObservation".into(),
            outcome: EvaluationOutcome::Failed,
            message: "inferences and allegations can never be promoted to direct observations"
                .into(),
        });
    }
    if matches!(
        input.assertion_kind,
        AssertionKind::Allegation | AssertionKind::Inference
    ) && input.claimed_target_kind == Some(AssertionKind::AdjudicatedFinding)
    {
        if !input.has_human_adjudicator_did {
            findings.push(DiagnosticFinding {
                code: "hro:AutoPromotionToAdjudicatedFindingForbidden".into(),
                outcome: EvaluationOutcome::Failed,
                message: "allegation or inference cannot auto-promote to AdjudicatedFinding without a human adjudicator DID".into(),
            });
        }
        if input.distinct_root_sources <= 1 && input.dependent_inference_count > 1 {
            findings.push(DiagnosticFinding {
                code: "hro:SingleSourceDependentInferenceSelfCorroboration".into(),
                outcome: EvaluationOutcome::Failed,
                message: "dependent inferences sharing a single root source do not constitute independent corroboration".into(),
            });
        }
    }
    if input.is_challenged {
        if !input.challenge_preserved || !input.downstream_marked_for_review {
            findings.push(DiagnosticFinding {
                code: "hro:ChallengeSuppressionForbidden".into(),
                outcome: EvaluationOutcome::Failed,
                message: "challenges must be preserved alongside the assertion and mark downstream decisions for review".into(),
            });
        } else {
            findings.push(DiagnosticFinding {
                code: "hro:AssertionChallengedPendingReview".into(),
                outcome: EvaluationOutcome::Held,
                message: "assertion is under active challenge with downstream decisions held for review".into(),
            });
        }
    }
    finalize_report(
        CoreInstrumentKind::ContextualAssertionsReview,
        q_hash(&input.scope_domain),
        findings,
        now_unix,
    )
}

/// The 6 normative dimensions of `wellfair-fairness-rights-review` (`FAIRNESS-AND-RIGHTS-REVIEW.md` §2).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FairnessReviewDimension {
    LegalAndAuthority = 1,
    EthicalAndHumanAgency = 2,
    EconomicAndMaterial = 3,
    SocialAndParticipation = 4,
    ResolutionAndRemedy = 5,
    TemporalAndLifecycle = 6,
}

impl FairnessReviewDimension {
    pub const ALL: [Self; 6] = [
        Self::LegalAndAuthority,
        Self::EthicalAndHumanAgency,
        Self::EconomicAndMaterial,
        Self::SocialAndParticipation,
        Self::ResolutionAndRemedy,
        Self::TemporalAndLifecycle,
    ];

    pub const fn code(self) -> &'static str {
        match self {
            Self::LegalAndAuthority => "wellfair:LegalAndAuthority",
            Self::EthicalAndHumanAgency => "wellfair:EthicalAndHumanAgency",
            Self::EconomicAndMaterial => "wellfair:EconomicAndMaterial",
            Self::SocialAndParticipation => "wellfair:SocialAndParticipation",
            Self::ResolutionAndRemedy => "wellfair:ResolutionAndRemedy",
            Self::TemporalAndLifecycle => "wellfair:TemporalAndLifecycle",
        }
    }
}

/// Input contract for `wellfair-fairness-rights-review`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct WellfairFairnessRightsReview {
    pub review_scope_id: String,
    pub dimension_outcomes: Vec<(FairnessReviewDimension, EvaluationOutcome)>,
    pub has_independent_challenge_route: bool,
    #[serde(default)]
    pub collapses_dimensions_to_single_score: bool,
}

/// Evaluate `wellfair-fairness-rights-review` across all 6 dimensions without collapsing to a scalar.
pub fn evaluate_wellfair_fairness_rights_review(
    input: &WellfairFairnessRightsReview,
    now_unix: u32,
) -> InstrumentEvaluationReport {
    let mut findings = Vec::new();
    if input.collapses_dimensions_to_single_score {
        findings.push(DiagnosticFinding {
            code: "hro:CollapsedSingleFairnessScoreForbidden".into(),
            outcome: EvaluationOutcome::Failed,
            message: "fairness review dimensions must never be collapsed into a single numeric score"
                .into(),
        });
    }
    if input.review_scope_id.trim().is_empty() {
        findings.push(DiagnosticFinding {
            code: "hro:MissingReviewScope".into(),
            outcome: EvaluationOutcome::Held,
            message: "review unit requires a specific scope identifier".into(),
        });
    }
    if !input.has_independent_challenge_route {
        findings.push(DiagnosticFinding {
            code: "hro:MissingIndependentChallengeRoute".into(),
            outcome: EvaluationOutcome::Failed,
            message: "affected humans must have an independent challenge and remedy route".into(),
        });
    }
    for dim in FairnessReviewDimension::ALL {
        match input.dimension_outcomes.iter().find(|(d, _)| *d == dim) {
            Some((_, outcome)) => findings.push(DiagnosticFinding {
                code: dim.code().into(),
                outcome: *outcome,
                message: format!("dimension {:?} evaluated as {}", dim, outcome.as_str()),
            }),
            None => findings.push(DiagnosticFinding {
                code: dim.code().into(),
                outcome: EvaluationOutcome::Held,
                message: format!("dimension {:?} missing evidence; held", dim),
            }),
        }
    }
    finalize_report(
        CoreInstrumentKind::WellfairFairnessRightsReview,
        q_hash(&input.review_scope_id),
        findings,
        now_unix,
    )
}

/// Input contract for `licence-transition-readiness`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenceTransitionReadiness {
    pub project_id: String,
    pub pool_funded_cents: i64,
    pub required_obligation_cents: i64,
    pub settled_verified_cents: i64,
    #[serde(default)]
    pub waived_obligation_cents: i64,
    pub dependencies_compatible: bool,
    pub unresolved_material_challenges: u32,
    pub authorised_human_steward_did: Option<String>,
}

/// Evaluate `licence-transition-readiness` enforcing `Pool Funded != Contributor Paid`.
pub fn evaluate_licence_transition_readiness(
    input: &LicenceTransitionReadiness,
    now_unix: u32,
) -> InstrumentEvaluationReport {
    let mut findings = Vec::new();
    let steward_ok = input
        .authorised_human_steward_did
        .as_deref()
        .map(str::trim)
        .is_some_and(|s| !s.is_empty());
    if !steward_ok {
        findings.push(DiagnosticFinding {
            code: "hro:MissingAuthorisedHumanSteward".into(),
            outcome: EvaluationOutcome::Failed,
            message: "licence transition requires an authorised human release steward DID".into(),
        });
    }
    if !input.dependencies_compatible {
        findings.push(DiagnosticFinding {
            code: "hro:DependencyLicenceIncompatible".into(),
            outcome: EvaluationOutcome::Held,
            message: "dependency licensing authority is unresolved or incompatible".into(),
        });
    }
    if input.unresolved_material_challenges > 0 {
        findings.push(DiagnosticFinding {
            code: "hro:UnresolvedMaterialChallenges".into(),
            outcome: EvaluationOutcome::Held,
            message: "unresolved material challenges block licence transition".into(),
        });
    }
    let discharged = input
        .settled_verified_cents
        .saturating_add(input.waived_obligation_cents);
    if input.required_obligation_cents > 0 && discharged < input.required_obligation_cents {
        findings.push(DiagnosticFinding {
            code: "hro:PoolFundedIsNotContributorPaid".into(),
            outcome: EvaluationOutcome::Held,
            message: format!(
                "pool funded ({}) != contributor paid ({} verified + {} waived < {} required)",
                input.pool_funded_cents,
                input.settled_verified_cents,
                input.waived_obligation_cents,
                input.required_obligation_cents
            ),
        });
    }
    finalize_report(
        CoreInstrumentKind::LicenceTransitionReadiness,
        q_hash(&input.project_id),
        findings,
        now_unix,
    )
}
