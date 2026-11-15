use super::*;

#[test]
fn unsolicited_submission_cannot_create_obligation_without_acceptance_and_valuation() {
    let sub = SubmissionOrClaim {
        id: "sub-unsolicited".into(),
        project_id: "proj-1".into(),
        contributor_did: "did:wf:alice".into(),
        agreement_id: None,
        output_id: "out-1".into(),
        claimed_effort_minutes: 120,
        claimed_expense_cents: 0,
        claimed_capital_cents: 0,
        submitted_at_unix: 100,
    };

    // No acceptance, no valuation -> rejected
    assert!(derive_obligation_from_lifecycle("ob-1", &sub, None, None, None, 110).is_err());

    // Rejected acceptance -> rejected
    let rejected = AcceptanceDecision {
        id: "acc-1".into(),
        project_id: "proj-1".into(),
        submission_id: sub.id.clone(),
        agreement_id: None,
        outcome: AcceptanceDecision::Rejected,
        reviewer_did: "did:wf:steward".into(),
        conditions_or_rationale: Some("Out of scope".into()),
        decided_at_unix: 105,
    };
    assert!(
        derive_obligation_from_lifecycle("ob-1", &sub, None, Some(&rejected), None, 110).is_err()
    );

    // Accepted but missing valuation -> rejected
    let accepted = AcceptanceDecision {
        outcome: AcceptanceDecision::Accepted,
        ..rejected
    };
    assert!(
        derive_obligation_from_lifecycle("ob-1", &sub, None, Some(&accepted), None, 110).is_err()
    );

    // Accepted + Valuation -> creates active LifecycleObligation
    let val = Valuation {
        id: "val-1".into(),
        project_id: "proj-1".into(),
        submission_id: sub.id.clone(),
        acceptance_id: accepted.id.clone(),
        valued_effort_cents: 15_000,
        valued_expense_cents: 0,
        valued_capital_cents: 0,
        risk_cap_bps: 10_000,
        approved_by_did: "did:wf:steward".into(),
        valued_at_unix: 108,
    };
    let ob = derive_obligation_from_lifecycle(
        "ob-1",
        &sub,
        None,
        Some(&accepted),
        Some(&val),
        110,
    )
    .expect("accepted + valued creates obligation");
    assert_eq!(ob.principal_cents, 15_000);
    assert_eq!(ob.outstanding_cents, 15_000);
    assert_eq!(ob.status, LifecycleObligation::Active);
}

#[test]
fn commissioned_agreement_with_rejected_deliverable_still_accrues_authorised_time_and_expense() {
    let ag = RequestAndAgreement {
        id: "ag-comm-1".into(),
        project_id: "proj-1".into(),
        contributor_did: "did:wf:bob".into(),
        authorised_by_did: Some("did:wf:steward".into()),
        authority: AgreementAuthority::Commissioned,
        compensation_class: CompensationClass::ContractedService,
        max_authorised_effort_minutes: 120,
        max_authorised_expense_cents: 5_000,
        rate_cents_per_hour: 6_000,
        created_at_unix: 50,
    };
    let sub = SubmissionOrClaim {
        id: "sub-comm-1".into(),
        project_id: "proj-1".into(),
        contributor_did: "did:wf:bob".into(),
        agreement_id: Some(ag.id.clone()),
        output_id: "out-comm-1".into(),
        claimed_effort_minutes: 180, // capped at 120m -> 2h * 6_000 = 12_000
        claimed_expense_cents: 4_000, // within 5_000 cap -> 4_000
        claimed_capital_cents: 0,
        submitted_at_unix: 100,
    };
    let rejected = AcceptanceDecision {
        id: "acc-comm-1".into(),
        project_id: "proj-1".into(),
        submission_id: sub.id.clone(),
        agreement_id: Some(ag.id.clone()),
        outcome: AcceptanceDecision::Rejected,
        reviewer_did: "did:wf:steward".into(),
        conditions_or_rationale: Some("Prototype superseded by architecture change".into()),
        decided_at_unix: 105,
    };
    let ob = derive_obligation_from_lifecycle(
        "ob-comm-1",
        &sub,
        Some(&ag),
        Some(&rejected),
        None,
        110,
    )
    .expect("commissioned work accrues obligation for authorised effort and expenses");
    assert_eq!(ob.principal_cents, 16_000);
    assert_eq!(ob.outstanding_cents, 16_000);
}

#[test]
fn pool_funded_is_not_contributor_paid_and_badges_never_discharge_debt() {
    let ob = LifecycleObligation {
        id: "ob-1".into(),
        project_id: "proj-1".into(),
        beneficiary_did: "did:wf:alice".into(),
        agreement_id: Some("ag-1".into()),
        acceptance_id: Some("acc-1".into()),
        valuation_id: Some("val-1".into()),
        principal_cents: 50_000,
        settled_cents: 0,
        outstanding_cents: 50_000,
        status: LifecycleObligation::Active,
        waiver_terms: None,
        waived_by_did: None,
        created_at_unix: 100,
    };
    let output_with_badges = OutputOrService {
        id: "out-1".into(),
        project_id: "proj-1".into(),
        contributor_did: "did:wf:alice".into(),
        activity_ids: vec!["act-1".into()],
        artifact_uri: "urn:git:commit:abc".into(),
        non_monetary_badges: vec!["FoundingArchitectBadge".into(), "LeadContributor".into()],
        provenance_attestation_id: Some("urn:attest:prov-1".into()),
        produced_at_unix: 90,
    };

    // Rule 3: Non-monetary badges and provenance attestations never reduce outstanding_cents.
    let after_badges = apply_settlements_and_recognition(&ob, &[], &[output_with_badges]);
    assert_eq!(after_badges.outstanding_cents, 50_000);
    assert_eq!(after_badges.status, LifecycleObligation::Active);

    // Rule 2: Pool has 100_000 cents (>= 50_000 obligation), and an unreceipted allocation exists,
    // but beneficiary_receipt_hash is None -> transition MUST NOT be ready!
    let unreceipted_alloc = SettlementAllocation {
        id: "alloc-1".into(),
        project_id: "proj-1".into(),
        obligation_id: ob.id.clone(),
        beneficiary_did: ob.beneficiary_did.clone(),
        allocated_cents: 50_000,
        currency: "AUD".into(),
        source_pool_id: "pool-1".into(),
        beneficiary_receipt_hash: None,
        settled_at_unix: 120,
    };
    let decision = TransitionDecision {
        id: "td-1".into(),
        project_id: "proj-1".into(),
        target_licence_uri: "https://spdx.org/licenses/MIT.html".into(),
        authorised_steward_did: "did:wf:human-steward".into(),
        steward_is_human: true,
        pool_funded_cents: 100_000,
        total_obligation_cents: 50_000,
        settled_contributor_cents: 0,
        unresolved_material_challenges: 0,
        dependencies_compatible: true,
        decided_at_unix: 130,
    };
    assert!(!is_transition_ready(
        &decision,
        std::slice::from_ref(&ob),
        std::slice::from_ref(&unreceipted_alloc)
    ));

    // Once beneficiary_receipt_hash is present, obligation is Satisfied and transition is Ready.
    let mut receipted_alloc = unreceipted_alloc;
    receipted_alloc.beneficiary_receipt_hash = Some(0xABCD_1234_5678_90EF);
    assert!(is_transition_ready(
        &decision,
        std::slice::from_ref(&ob),
        std::slice::from_ref(&receipted_alloc)
    ));

    // Compile to Quins zero-heap check
    let mut quins = [NQuin::default(); 8];
    let n = ContributionLifecycleStage::SettlementAllocation(receipted_alloc).compile_to_quins(&mut quins);
    assert_eq!(n, 2);
    assert_eq!(quins[1].object, 0xABCD_1234_5678_90EF);
}
