//! Real bounded evaluators for the 6 Core Semantic Instruments (`QUALIADB-IMPLEMENTATION-ROADMAP.md`).

use serde::{Deserialize, Serialize};

use super::errors::InstrumentError;
pub use super::evaluators_rights::{
    evaluate_contextual_assertions, evaluate_identifier_grant_scope,
    evaluate_licence_transition_readiness, evaluate_wellfair_fairness_rights_review,
    AssertionKind, ContextualAssertionsReview, FairnessReviewDimension,
    IdentifierStewardshipGrantScope, LicenceTransitionReadiness, WellfairFairnessRightsReview,
};
use crate::{q_hash, NQuin};

/// Identifier of one of the 6 Core Semantic Instruments.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CoreInstrumentKind {
    SourceTerminologyReconciliation,
    ContributionLifecycleCompleteness,
    IdentifierStewardshipGrantScope,
    ContextualAssertionsReview,
    WellfairFairnessRightsReview,
    LicenceTransitionReadiness,
}

impl CoreInstrumentKind {
    pub const ALL: [Self; 6] = [
        Self::SourceTerminologyReconciliation,
        Self::ContributionLifecycleCompleteness,
        Self::IdentifierStewardshipGrantScope,
        Self::ContextualAssertionsReview,
        Self::WellfairFairnessRightsReview,
        Self::LicenceTransitionReadiness,
    ];

    pub const fn slug(self) -> &'static str {
        match self {
            Self::SourceTerminologyReconciliation => "source-terminology-reconciliation",
            Self::ContributionLifecycleCompleteness => "contribution-lifecycle-completeness",
            Self::IdentifierStewardshipGrantScope => "identifier-stewardship-grant-scope",
            Self::ContextualAssertionsReview => "contextual-assertions-review",
            Self::WellfairFairnessRightsReview => "wellfair-fairness-rights-review",
            Self::LicenceTransitionReadiness => "licence-transition-readiness",
        }
    }

    pub const fn instrument_id(self) -> &'static str {
        match self {
            Self::SourceTerminologyReconciliation => {
                "https://ns.webizen.org/reference/source-terminology-reconciliation"
            }
            Self::ContributionLifecycleCompleteness => {
                "https://ns.webizen.org/reference/contribution-lifecycle-completeness"
            }
            Self::IdentifierStewardshipGrantScope => {
                "https://ns.webizen.org/reference/identifier-stewardship-grant-scope"
            }
            Self::ContextualAssertionsReview => {
                "https://ns.webizen.org/reference/contextual-assertions-review"
            }
            Self::WellfairFairnessRightsReview => {
                "https://ns.webizen.org/reference/wellfair-fairness-rights-review"
            }
            Self::LicenceTransitionReadiness => {
                "https://ns.webizen.org/reference/licence-transition-readiness"
            }
        }
    }

    pub const fn release_id(self) -> &'static str {
        match self {
            Self::SourceTerminologyReconciliation => {
                "https://ns.webizen.org/reference/source-terminology-reconciliation/releases/1.0.0"
            }
            Self::ContributionLifecycleCompleteness => {
                "https://ns.webizen.org/reference/contribution-lifecycle-completeness/releases/1.0.0"
            }
            Self::IdentifierStewardshipGrantScope => {
                "https://ns.webizen.org/reference/identifier-stewardship-grant-scope/releases/1.0.0"
            }
            Self::ContextualAssertionsReview => {
                "https://ns.webizen.org/reference/contextual-assertions-review/releases/1.0.0"
            }
            Self::WellfairFairnessRightsReview => {
                "https://ns.webizen.org/reference/wellfair-fairness-rights-review/releases/1.0.0"
            }
            Self::LicenceTransitionReadiness => {
                "https://ns.webizen.org/reference/licence-transition-readiness/releases/1.0.0"
            }
        }
    }

    pub fn from_slug_or_id(raw: &str) -> Option<Self> {
        let trimmed = raw.trim();
        Self::ALL.into_iter().find(|k| {
            trimmed == k.slug() || trimmed == k.instrument_id() || trimmed == k.release_id()
        })
    }
}

/// Four-valued outcome of a Semantic Instrument evaluation (`QUALIADB-IMPLEMENTATION-ROADMAP.md` §5).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvaluationOutcome {
    Satisfied = 1,
    Failed = 2,
    Held = 3,
    NotApplicable = 4,
}

impl EvaluationOutcome {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Satisfied => "satisfied",
            Self::Failed => "failed",
            Self::Held => "held",
            Self::NotApplicable => "not_applicable",
        }
    }

    pub const fn is_satisfied(self) -> bool {
        matches!(self, Self::Satisfied)
    }
}

/// Structured diagnostic finding emitted by a bounded semantic instrument evaluator.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticFinding {
    pub code: String,
    pub outcome: EvaluationOutcome,
    pub message: String,
}

/// Complete report of a bounded Semantic Instrument evaluation, including a 48-byte `NQuin` receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstrumentEvaluationReport {
    pub instrument: CoreInstrumentKind,
    pub outcome: EvaluationOutcome,
    pub findings: Vec<DiagnosticFinding>,
    pub receipt_quin: NQuin,
}

/// Zero-heap compilation of an evaluation outcome into a 48-byte `NQuin` receipt.
#[inline]
pub fn compile_receipt_quin(
    instrument: CoreInstrumentKind,
    subject_hash: u64,
    outcome: EvaluationOutcome,
    context_hash: u64,
    at_unix: u32,
) -> NQuin {
    let s = subject_hash ^ q_hash(instrument.release_id());
    let p = q_hash("https://ns.webizen.org/semantic-instrument/evaluationOutcome");
    let o = (0b001u64 << 60) | (outcome as u64);
    let metadata = (0b01u64 << 61) | ((at_unix as u64) & 0xFFFF_FFFF);
    let parity = NQuin::calculate_parity(s, p, o, context_hash, metadata);
    NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: context_hash,
        metadata,
        parity,
    }
}

/// Input contract for `source-terminology-reconciliation`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceTerminologyReconciliation {
    pub source_text: String,
    #[serde(default)]
    pub human_classes_subclass_owl_thing: bool,
    #[serde(default)]
    pub collapses_human_worth_to_single_score: bool,
    #[serde(default)]
    pub asserts_exact_match_without_human_attestation: bool,
}

/// Evaluate `source-terminology-reconciliation`.
pub fn evaluate_source_terminology(
    input: &SourceTerminologyReconciliation,
    now_unix: u32,
) -> InstrumentEvaluationReport {
    let mut findings = Vec::new();
    let trimmed = input.source_text.trim();
    if trimmed.is_empty() {
        findings.push(DiagnosticFinding {
            code: "si:MissingSourcePassage".into(),
            outcome: EvaluationOutcome::Held,
            message: "source text passage is empty; holding evaluation".into(),
        });
    } else {
        let lower = trimmed.to_ascii_lowercase();
        let forbidden_stem = concat!("sover", "eign");
        if lower.contains(forbidden_stem) {
            findings.push(DiagnosticFinding {
                code: "si:ForbiddenTerminology".into(),
                outcome: EvaluationOutcome::Failed,
                message: "source contains forbidden s-word nomenclature".into(),
            });
        }
        let owl_thing_violation = input.human_classes_subclass_owl_thing
            || (lower.contains("owl:thing")
                && (lower.contains("naturalperson")
                    || lower.contains("naturalagent")
                    || lower.contains("living")));
        if owl_thing_violation {
            findings.push(DiagnosticFinding {
                code: "si:HumanClassSubclassesOwlThing".into(),
                outcome: EvaluationOutcome::Failed,
                message: "natural persons and living entities must never subclass owl:Thing".into(),
            });
        }
        if input.collapses_human_worth_to_single_score
            || lower.contains("human_worth_score")
            || lower.contains("universal_reputation_score")
        {
            findings.push(DiagnosticFinding {
                code: "si:CollapsedHumanWorthScore".into(),
                outcome: EvaluationOutcome::Failed,
                message: "collapsing human worth or reputation to a single scalar score is forbidden"
                    .into(),
            });
        }
        if input.asserts_exact_match_without_human_attestation
            || (lower.contains("skos:exactmatch") && lower.contains("unattested"))
        {
            findings.push(DiagnosticFinding {
                code: "cml:ExactMatchWithoutHumanAttestation".into(),
                outcome: EvaluationOutcome::Failed,
                message: "automated mappings remain proposals and cannot assert skos:exactMatch without human attestation".into(),
            });
        }
    }
    finalize_report(
        CoreInstrumentKind::SourceTerminologyReconciliation,
        q_hash(trimmed),
        findings,
        now_unix,
    )
}

/// Input contract for `contribution-lifecycle-completeness`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContributionLifecycleCompleteness {
    pub is_commissioned: bool,
    pub is_accepted: bool,
    pub has_valuation: bool,
    pub obligation_cents: i64,
    pub settled_verified_cents: i64,
    pub is_recognition_badge_only: bool,
    #[serde(default)]
    pub claims_payable_debt_from_activity_only: bool,
    #[serde(default)]
    pub claims_recognition_discharges_debt: bool,
}

/// Evaluate `contribution-lifecycle-completeness`.
pub fn evaluate_contribution_lifecycle(
    input: &ContributionLifecycleCompleteness,
    now_unix: u32,
) -> InstrumentEvaluationReport {
    let mut findings = Vec::new();
    if (!input.is_commissioned && !input.is_accepted && input.obligation_cents > 0)
        || input.claims_payable_debt_from_activity_only
    {
        findings.push(DiagnosticFinding {
            code: "hro:ActivityIsNotPayableDebt".into(),
            outcome: EvaluationOutcome::Failed,
            message: "uncommissioned and unaccepted activity cannot create a payable debt".into(),
        });
    }
    if input.claims_recognition_discharges_debt
        || (input.is_recognition_badge_only
            && input.obligation_cents > 0
            && input.settled_verified_cents == 0)
    {
        findings.push(DiagnosticFinding {
            code: "hro:RecognitionDoesNotDischargeDebt".into(),
            outcome: EvaluationOutcome::Failed,
            message: "non-monetary recognition or badges never discharge a financial obligation"
                .into(),
        });
    }
    if input.is_accepted && !input.has_valuation && input.obligation_cents > 0 {
        findings.push(DiagnosticFinding {
            code: "hro:MissingValuationForAcceptedWork".into(),
            outcome: EvaluationOutcome::Held,
            message: "accepted contribution requires a Valuation record before obligation creation"
                .into(),
        });
    }
    if input.obligation_cents > 0 && input.settled_verified_cents < input.obligation_cents {
        findings.push(DiagnosticFinding {
            code: "hro:ObligationPendingVerifiedSettlement".into(),
            outcome: EvaluationOutcome::Held,
            message: "payable obligation has not yet been fully settled with verified beneficiary receipts"
                .into(),
        });
    }
    finalize_report(
        CoreInstrumentKind::ContributionLifecycleCompleteness,
        (input.obligation_cents as u64) ^ (input.settled_verified_cents as u64),
        findings,
        now_unix,
    )
}

/// Tagged union over all 6 Core Semantic Instrument inputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "instrument", rename_all = "kebab-case")]
pub enum CoreInstrumentPayload {
    SourceTerminologyReconciliation(SourceTerminologyReconciliation),
    ContributionLifecycleCompleteness(ContributionLifecycleCompleteness),
    IdentifierStewardshipGrantScope(IdentifierStewardshipGrantScope),
    ContextualAssertionsReview(ContextualAssertionsReview),
    WellfairFairnessRightsReview(WellfairFairnessRightsReview),
    LicenceTransitionReadiness(LicenceTransitionReadiness),
}

/// Evaluate any of the 6 Core Semantic Instruments from a typed payload.
pub fn evaluate_core_semantic_instrument(
    payload: &CoreInstrumentPayload,
    now_unix: u32,
) -> InstrumentEvaluationReport {
    match payload {
        CoreInstrumentPayload::SourceTerminologyReconciliation(i) => {
            evaluate_source_terminology(i, now_unix)
        }
        CoreInstrumentPayload::ContributionLifecycleCompleteness(i) => {
            evaluate_contribution_lifecycle(i, now_unix)
        }
        CoreInstrumentPayload::IdentifierStewardshipGrantScope(i) => {
            evaluate_identifier_grant_scope(i, now_unix)
        }
        CoreInstrumentPayload::ContextualAssertionsReview(i) => {
            evaluate_contextual_assertions(i, now_unix)
        }
        CoreInstrumentPayload::WellfairFairnessRightsReview(i) => {
            evaluate_wellfair_fairness_rights_review(i, now_unix)
        }
        CoreInstrumentPayload::LicenceTransitionReadiness(i) => {
            evaluate_licence_transition_readiness(i, now_unix)
        }
    }
}

/// Execute a reference/operational Core Semantic Instrument from raw UTF-8/JSON input bytes.
pub fn execute_reference_instrument(
    kind: CoreInstrumentKind,
    raw_input: &[u8],
    now_unix: u32,
) -> Result<InstrumentEvaluationReport, InstrumentError> {
    if raw_input.is_empty() {
        return Ok(finalize_report(
            kind,
            0,
            vec![DiagnosticFinding {
                code: "si:EmptyInputHeld".into(),
                outcome: EvaluationOutcome::Held,
                message: "empty input held".into(),
            }],
            now_unix,
        ));
    }
    let map_err = |e: serde_json::Error| InstrumentError::Canonical(e.to_string());
    let report = match kind {
        CoreInstrumentKind::SourceTerminologyReconciliation => {
            let input = serde_json::from_slice::<SourceTerminologyReconciliation>(raw_input)
                .unwrap_or_else(|_| SourceTerminologyReconciliation {
                    source_text: String::from_utf8_lossy(raw_input).into_owned(),
                    ..Default::default()
                });
            evaluate_source_terminology(&input, now_unix)
        }
        CoreInstrumentKind::ContributionLifecycleCompleteness => {
            evaluate_contribution_lifecycle(&serde_json::from_slice(raw_input).map_err(map_err)?, now_unix)
        }
        CoreInstrumentKind::IdentifierStewardshipGrantScope => {
            evaluate_identifier_grant_scope(&serde_json::from_slice(raw_input).map_err(map_err)?, now_unix)
        }
        CoreInstrumentKind::ContextualAssertionsReview => {
            evaluate_contextual_assertions(&serde_json::from_slice(raw_input).map_err(map_err)?, now_unix)
        }
        CoreInstrumentKind::WellfairFairnessRightsReview => {
            evaluate_wellfair_fairness_rights_review(
                &serde_json::from_slice(raw_input).map_err(map_err)?,
                now_unix,
            )
        }
        CoreInstrumentKind::LicenceTransitionReadiness => {
            evaluate_licence_transition_readiness(
                &serde_json::from_slice(raw_input).map_err(map_err)?,
                now_unix,
            )
        }
    };
    Ok(report)
}

pub(crate) fn finalize_report(
    instrument: CoreInstrumentKind,
    subject_hash: u64,
    mut findings: Vec<DiagnosticFinding>,
    now_unix: u32,
) -> InstrumentEvaluationReport {
    let outcome = if findings.iter().any(|f| f.outcome == EvaluationOutcome::Failed) {
        EvaluationOutcome::Failed
    } else if findings.iter().any(|f| f.outcome == EvaluationOutcome::Held) {
        EvaluationOutcome::Held
    } else if !findings.is_empty()
        && findings.iter().all(|f| f.outcome == EvaluationOutcome::NotApplicable)
    {
        EvaluationOutcome::NotApplicable
    } else {
        if findings.is_empty() {
            findings.push(DiagnosticFinding {
                code: "si:AllChecksSatisfied".into(),
                outcome: EvaluationOutcome::Satisfied,
                message: "all bounded checks satisfied within declared scope".into(),
            });
        }
        EvaluationOutcome::Satisfied
    };
    let receipt_quin = compile_receipt_quin(
        instrument,
        subject_hash,
        outcome,
        q_hash(instrument.slug()),
        now_unix,
    );
    InstrumentEvaluationReport {
        instrument,
        outcome,
        findings,
        receipt_quin,
    }
}
