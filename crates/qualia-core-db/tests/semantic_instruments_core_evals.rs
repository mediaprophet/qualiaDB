//! Contract tests for the 6 Core Semantic Instruments (`WP-CP-02A`).

use qualia_core_db::semantic_instruments::{
    build_reference, core_instrument_catalog, evaluate_contextual_assertions,
    evaluate_contribution_lifecycle, evaluate_identifier_grant_scope,
    evaluate_licence_transition_readiness, evaluate_source_terminology,
    evaluate_wellfair_fairness_rights_review, preflight, run_entry, AssertionKind,
    ContextualAssertionsReview, ContributionLifecycleCompleteness, CoreInstrumentKind,
    DependencyLock, EvaluationOutcome, FairnessReviewDimension, IdentifierStewardshipGrantScope,
    InstalledRelease, LicenceTransitionReadiness, LocalRegistry, RunOutcome, RunRequest,
    SourceTerminologyReconciliation, WellfairFairnessRightsReview,
};

#[test]
fn source_terminology_reconciliation_rejects_forbidden_terms_and_owl_thing() {
    let forbidden_word = concat!("sover", "eignty");
    let bad = SourceTerminologyReconciliation {
        source_text: format!("A NaturalPerson has {forbidden_word} and subclasses owl:Thing"),
        human_classes_subclass_owl_thing: true,
        collapses_human_worth_to_single_score: true,
        asserts_exact_match_without_human_attestation: true,
    };
    let report = evaluate_source_terminology(&bad, 1000);
    assert_eq!(report.outcome, EvaluationOutcome::Failed);
    assert!(report.findings.iter().any(|f| f.code == "si:ForbiddenTerminology"));
    assert!(report.findings.iter().any(|f| f.code == "si:HumanClassSubclassesOwlThing"));
    assert!(report.findings.iter().any(|f| f.code == "si:CollapsedHumanWorthScore"));
    assert!(report.receipt_quin.verify_ecc_parity());

    let clean = SourceTerminologyReconciliation {
        source_text: "A values:NaturalPerson has agency, jurisdiction, and stewardship.".into(),
        ..Default::default()
    };
    let ok_report = evaluate_source_terminology(&clean, 1000);
    assert_eq!(ok_report.outcome, EvaluationOutcome::Satisfied);
    assert!(ok_report.receipt_quin.verify_ecc_parity());
}

#[test]
fn contribution_lifecycle_enforces_activity_is_not_debt_and_recognition_is_not_settlement() {
    let bad = ContributionLifecycleCompleteness {
        is_commissioned: false,
        is_accepted: false,
        has_valuation: false,
        obligation_cents: 25_000,
        settled_verified_cents: 0,
        is_recognition_badge_only: true,
        claims_payable_debt_from_activity_only: true,
        claims_recognition_discharges_debt: true,
    };
    let report = evaluate_contribution_lifecycle(&bad, 1000);
    assert_eq!(report.outcome, EvaluationOutcome::Failed);
    assert!(report.findings.iter().any(|f| f.code == "hro:ActivityIsNotPayableDebt"));
    assert!(report
        .findings
        .iter()
        .any(|f| f.code == "hro:RecognitionDoesNotDischargeDebt"));

    let pending = ContributionLifecycleCompleteness {
        is_commissioned: true,
        is_accepted: true,
        has_valuation: true,
        obligation_cents: 50_000,
        settled_verified_cents: 20_000,
        is_recognition_badge_only: false,
        ..Default::default()
    };
    assert_eq!(
        evaluate_contribution_lifecycle(&pending, 1000).outcome,
        EvaluationOutcome::Held
    );

    let settled = ContributionLifecycleCompleteness {
        settled_verified_cents: 50_000,
        ..pending
    };
    assert_eq!(
        evaluate_contribution_lifecycle(&settled, 1000).outcome,
        EvaluationOutcome::Satisfied
    );
}

#[test]
fn identifier_stewardship_grant_scope_blocks_objections_and_holds_competing_guardians() {
    let objected = IdentifierStewardshipGrantScope {
        subject_did: "did:webizen:person:alice".into(),
        invoking_agent_did: "did:webizen:agent: guardian".into(),
        granted_purpose: "healthcare-booking".into(),
        requested_purpose: "advertising-profiling".into(),
        has_domain_authority: true,
        subject_human_objected: true,
        touches_biometrics_or_mindware: true,
        explicit_biometric_consent: false,
        ..Default::default()
    };
    let report = evaluate_identifier_grant_scope(&objected, 1000);
    assert_eq!(report.outcome, EvaluationOutcome::Failed);
    assert!(report.findings.iter().any(|f| f.code == "hro:SubjectHumanObjected"));
    assert!(report.findings.iter().any(|f| f.code == "hro:OutOfScopePurposeOrProfiling"));
    assert!(report
        .findings
        .iter()
        .any(|f| f.code == "hro:ProtectedBiometricsOrMindwareBoundary"));

    let competing = IdentifierStewardshipGrantScope {
        subject_did: "did:webizen:person:child".into(),
        invoking_agent_did: "did:webizen:person:guardian1".into(),
        granted_purpose: "school-enrolment".into(),
        requested_purpose: "school-enrolment".into(),
        has_domain_authority: true,
        is_guardian_or_proxy: true,
        competing_guardianship_claims: true,
        ..Default::default()
    };
    assert_eq!(
        evaluate_identifier_grant_scope(&competing, 1000).outcome,
        EvaluationOutcome::Held
    );
}

#[test]
fn contextual_assertions_review_blocks_auto_promotion_and_single_source_self_corroboration() {
    let auto_promote = ContextualAssertionsReview {
        assertion_kind: AssertionKind::Allegation,
        claimed_target_kind: Some(AssertionKind::AdjudicatedFinding),
        distinct_root_sources: 1,
        dependent_inference_count: 3,
        has_human_adjudicator_did: false,
        scope_domain: "cooperative-project:alpha".into(),
        ..Default::default()
    };
    let report = evaluate_contextual_assertions(&auto_promote, 1000);
    assert_eq!(report.outcome, EvaluationOutcome::Failed);
    assert!(report
        .findings
        .iter()
        .any(|f| f.code == "hro:AutoPromotionToAdjudicatedFindingForbidden"));
    assert!(report
        .findings
        .iter()
        .any(|f| f.code == "hro:SingleSourceDependentInferenceSelfCorroboration"));

    let challenged = ContextualAssertionsReview {
        assertion_kind: AssertionKind::Observation,
        distinct_root_sources: 2,
        is_challenged: true,
        challenge_preserved: true,
        downstream_marked_for_review: true,
        scope_domain: "cooperative-project:alpha".into(),
        ..Default::default()
    };
    assert_eq!(
        evaluate_contextual_assertions(&challenged, 1000).outcome,
        EvaluationOutcome::Held
    );
}

#[test]
fn wellfair_fairness_rights_review_evaluates_all_six_dimensions_without_scalar_collapse() {
    let collapsed = WellfairFairnessRightsReview {
        review_scope_id: "release:1.0.0".into(),
        dimension_outcomes: FairnessReviewDimension::ALL
            .into_iter()
            .map(|d| (d, EvaluationOutcome::Satisfied))
            .collect(),
        has_independent_challenge_route: true,
        collapses_dimensions_to_single_score: true,
    };
    assert_eq!(
        evaluate_wellfair_fairness_rights_review(&collapsed, 1000).outcome,
        EvaluationOutcome::Failed
    );

    let valid = WellfairFairnessRightsReview {
        collapses_dimensions_to_single_score: false,
        ..collapsed
    };
    let report = evaluate_wellfair_fairness_rights_review(&valid, 1000);
    assert_eq!(report.outcome, EvaluationOutcome::Satisfied);
    assert_eq!(report.findings.len(), 6);
}

#[test]
fn licence_transition_readiness_enforces_pool_funded_is_not_contributor_paid() {
    let pool_only = LicenceTransitionReadiness {
        project_id: "proj:coop-1".into(),
        pool_funded_cents: 500_000,
        required_obligation_cents: 100_000,
        settled_verified_cents: 40_000,
        waived_obligation_cents: 0,
        dependencies_compatible: true,
        unresolved_material_challenges: 0,
        authorised_human_steward_did: Some("did:webizen:person:steward".into()),
    };
    let held_report = evaluate_licence_transition_readiness(&pool_only, 1000);
    assert_eq!(held_report.outcome, EvaluationOutcome::Held);
    assert!(held_report
        .findings
        .iter()
        .any(|f| f.code == "hro:PoolFundedIsNotContributorPaid"));

    let ready = LicenceTransitionReadiness {
        settled_verified_cents: 80_000,
        waived_obligation_cents: 20_000,
        ..pool_only
    };
    let ok_report = evaluate_licence_transition_readiness(&ready, 1000);
    assert_eq!(ok_report.outcome, EvaluationOutcome::Satisfied);
}

#[test]
fn runner_executes_all_six_core_instruments_and_builds_collectables() {
    for seed in core_instrument_catalog() {
        let collectable = build_reference(seed.slug).expect("build core reference collectable");
        assert!(!collectable.is_empty());
    }

    let release_id = CoreInstrumentKind::LicenceTransitionReadiness.release_id();
    let mut registry = LocalRegistry::new();
    registry
        .install(InstalledRelease {
            release_id: release_id.into(),
            content_digest:
                "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            lock: DependencyLock {
                release_id: release_id.into(),
                entries: Vec::new(),
            },
            closed: true,
        })
        .unwrap();

    let payload = serde_json::to_vec(&LicenceTransitionReadiness {
        project_id: "proj:coop-1".into(),
        pool_funded_cents: 100_000,
        required_obligation_cents: 100_000,
        settled_verified_cents: 100_000,
        waived_obligation_cents: 0,
        dependencies_compatible: true,
        unresolved_material_challenges: 0,
        authorised_human_steward_did: Some("did:webizen:person:steward".into()),
    })
    .unwrap();
    let req = RunRequest {
        release_id,
        entry_point: "assess",
        input: &payload,
        now: 1700000000,
        cancelled: false,
        max_input_bytes: 4096,
        max_steps: 16,
        parent_receipt: None,
    };
    let installed = preflight(&registry, &req).unwrap();
    match run_entry(&registry, installed, &req) {
        RunOutcome::Completed { receipt } => assert_eq!(receipt.outcome, "satisfied"),
        _ => panic!("expected Completed outcome"),
    }
}
