use super::*;

struct TestAuthority {
    evidence_valid: bool,
    calibration_valid: bool,
}

impl SensingEvidenceAuthenticator for TestAuthority {
    fn authenticates(&self, _evidence: &SensingEvidence) -> bool {
        self.evidence_valid
    }

    fn calibration_verified(&self, _evidence: &SensingEvidence) -> bool {
        self.calibration_valid
    }
}

fn assertion() -> NQuin {
    NQuin {
        subject: 10,
        predicate: 20,
        object: 30,
        context: 40,
        metadata: 50,
        parity: 60,
    }
}

fn evidence(
    citation_id: u64,
    uncertainty: EvidenceUncertainty,
    calibration: CalibrationDeclaration,
) -> SensingEvidence {
    SensingEvidence::new(
        citation_id,
        71,
        81,
        assertion(),
        91,
        101,
        uncertainty,
        calibration,
    )
    .unwrap()
}

fn claim() -> SensingClaim {
    SensingClaim::new(assertion(), 91, 101).unwrap()
}

fn policy(require_calibration: bool) -> GroundingPolicy {
    GroundingPolicy::new(100_000, require_calibration).unwrap()
}

#[test]
fn accepts_only_authenticated_exact_support_within_uncertainty_and_calibration_policy() {
    let item = evidence(
        1,
        EvidenceUncertainty::RelativeMillionths(10_000),
        CalibrationDeclaration::Calibrated(
            super::super::record::CalibrationArtifactReference::new([7; 32]).unwrap(),
        ),
    );
    let decision = evaluate_sensing_grounding(
        claim(),
        &[item],
        policy(true),
        &TestAuthority {
            evidence_valid: true,
            calibration_valid: true,
        },
    );
    assert_eq!(
        decision,
        GroundingDecision::Accepted {
            citation_id: 1,
            source_id: 71,
            sample_id: 81,
            uncertainty_millionths: 10_000,
        }
    );
}

#[test]
fn unsigned_or_unresolved_exact_claim_is_denied() {
    let item = evidence(
        2,
        EvidenceUncertainty::RelativeMillionths(1),
        CalibrationDeclaration::Unknown,
    );
    assert_eq!(
        evaluate_sensing_grounding(
            claim(),
            &[item],
            policy(false),
            &TestAuthority {
                evidence_valid: false,
                calibration_valid: false,
            },
        ),
        GroundingDecision::Denied(DenialReason::UnauthenticatedExactSupport)
    );
}

#[test]
fn unknown_or_excess_uncertainty_routes_to_review() {
    let authority = TestAuthority {
        evidence_valid: true,
        calibration_valid: true,
    };
    let unknown = evidence(3, EvidenceUncertainty::Unknown, CalibrationDeclaration::Unknown);
    assert!(matches!(
        evaluate_sensing_grounding(claim(), &[unknown], policy(false), &authority),
        GroundingDecision::Review { reason: ReviewReason::UnknownUncertainty, .. }
    ));

    let broad = evidence(
        4,
        EvidenceUncertainty::RelativeMillionths(100_001),
        CalibrationDeclaration::Unknown,
    );
    assert!(matches!(
        evaluate_sensing_grounding(claim(), &[broad], policy(false), &authority),
        GroundingDecision::Review { reason: ReviewReason::UncertaintyAbovePolicy, .. }
    ));
}

#[test]
fn calibration_declaration_alone_does_not_pass_required_calibration() {
    let item = evidence(
        5,
        EvidenceUncertainty::RelativeMillionths(1),
        CalibrationDeclaration::Calibrated(
            super::super::record::CalibrationArtifactReference::new([8; 32]).unwrap(),
        ),
    );
    assert!(matches!(
        evaluate_sensing_grounding(
            claim(),
            &[item],
            policy(true),
            &TestAuthority {
                evidence_valid: true,
                calibration_valid: false,
            },
        ),
        GroundingDecision::Review { reason: ReviewReason::CalibrationNotVerified, .. }
    ));
}

#[test]
fn context_quantity_and_unit_must_match_exactly() {
    let mut other = assertion();
    other.context += 1;
    let item = SensingEvidence::new(
        6,
        71,
        81,
        other,
        91,
        101,
        EvidenceUncertainty::RelativeMillionths(1),
        CalibrationDeclaration::Unknown,
    )
    .unwrap();
    assert_eq!(
        evaluate_sensing_grounding(
            claim(),
            &[item],
            policy(false),
            &TestAuthority {
                evidence_valid: true,
                calibration_valid: true,
            },
        ),
        GroundingDecision::Denied(DenialReason::NoExactSupport)
    );

    let mut metadata_mismatch = assertion();
    metadata_mismatch.metadata += 1;
    let changed_measurement = SensingEvidence::new(
        61,
        71,
        81,
        metadata_mismatch,
        91,
        101,
        EvidenceUncertainty::RelativeMillionths(1),
        CalibrationDeclaration::Unknown,
    )
    .unwrap();
    assert_eq!(
        evaluate_sensing_grounding(
            claim(),
            &[changed_measurement],
            policy(false),
            &TestAuthority {
                evidence_valid: true,
                calibration_valid: true,
            },
        ),
        GroundingDecision::Denied(DenialReason::NoExactSupport)
    );

    let wrong_semantics = SensingEvidence::new(
        7,
        71,
        81,
        assertion(),
        92,
        101,
        EvidenceUncertainty::RelativeMillionths(1),
        CalibrationDeclaration::Unknown,
    )
    .unwrap();
    assert_eq!(
        evaluate_sensing_grounding(
            claim(),
            &[wrong_semantics],
            policy(false),
            &TestAuthority {
                evidence_valid: true,
                calibration_valid: true,
            },
        ),
        GroundingDecision::Denied(DenialReason::UnsupportedEvidenceOnly)
    );
}

#[test]
fn empty_duplicate_and_out_of_range_inputs_fail_closed() {
    let authority = TestAuthority {
        evidence_valid: true,
        calibration_valid: true,
    };
    assert_eq!(
        evaluate_sensing_grounding(claim(), &[], policy(false), &authority),
        GroundingDecision::Denied(DenialReason::NoEvidence)
    );
    let duplicate = [
        evidence(8, EvidenceUncertainty::Unknown, CalibrationDeclaration::Unknown),
        evidence(8, EvidenceUncertainty::Unknown, CalibrationDeclaration::Unknown),
    ];
    assert_eq!(
        evaluate_sensing_grounding(claim(), &duplicate, policy(false), &authority),
        GroundingDecision::Denied(DenialReason::DuplicateCitation)
    );
    let invalid = evidence(
        9,
        EvidenceUncertainty::RelativeMillionths(UNCERTAINTY_SCALE + 1),
        CalibrationDeclaration::Unknown,
    );
    assert_eq!(
        evaluate_sensing_grounding(claim(), &[invalid], policy(false), &authority),
        GroundingDecision::Denied(DenialReason::InvalidUncertainty)
    );
}
