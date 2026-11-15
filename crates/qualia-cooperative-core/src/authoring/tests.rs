//! Explicit Rust fixture principals. These are not production authority.
use super::*;
use crate::projects::{Project, ProjectMembership, ProjectRole};

fn project() -> Project {
    Project {
        id: "test-project".into(),
        name: "fixture only".into(),
        description: String::new(),
        licensing_ontologies: vec![],
        created_at_unix: 1,
        created_at_instant: None,
    }
}
fn document() -> DocumentRevision {
    DocumentRevision {
        project_id: "test-project".into(),
        document_id: "test-document".into(),
        source_id: "test-source".into(),
        revision: 7,
        source_digest: [1; 32],
        author_subject: Some("test-author-subject".into()),
    }
}
fn member(role: ProjectRole) -> ProjectMembership {
    ProjectMembership {
        id: "test-membership".into(),
        project_id: "test-project".into(),
        member_did: "did:test:author".into(),
        role,
        agreed_at_unix: 1,
        agreed_at_instant: None,
    }
}
fn identifier(value: &str) -> QualifiedIdentifier<'_> {
    QualifiedIdentifier {
        value,
        scheme: "did:test",
        issuer: "test-issuer",
        project_id: "test-project",
        evidence_ref: "test-qualified-association",
        valid_from_unix: 1,
        valid_until_unix: 1000,
        revoked: false,
    }
}
fn principal<'a>(identifiers: &'a [QualifiedIdentifier<'a>]) -> AuthenticatedPrincipal<'a> {
    AuthenticatedPrincipal {
        subject: "test-author-subject",
        kind: PrincipalKind::SoftwareAgent,
        authentication_ref: "test-authentication-only",
        authenticated_until_unix: 1000,
        identifiers,
    }
}
fn admission(membership: &ProjectMembership) -> AdmittedMembership<'_> {
    AdmittedMembership {
        membership,
        member_subject: "test-author-subject",
        admission_ref: "test-admission",
        valid_from_unix: 1,
        valid_until_unix: 1000,
        revoked: false,
    }
}
fn grant() -> ReviewGrant<'static> {
    ReviewGrant {
        id: "test-review-grant",
        project_id: "test-project",
        document_id: "test-document",
        reviewer_subject: "test-author-subject",
        admission_ref: "test-review-admission",
        valid_from_unix: 1,
        valid_until_unix: 1000,
        revoked: false,
    }
}
fn view<'a>(
    p: &'a Project,
    d: &'a DocumentRevision,
    principal: Option<&'a AuthenticatedPrincipal<'a>>,
    members: &'a [AdmittedMembership<'a>],
    grants: &'a [ReviewGrant<'a>],
) -> HostAuthorityView<'a> {
    HostAuthorityView {
        principal,
        project: p,
        memberships: members,
        review_grants: grants,
        current_document: d,
        generation: 3,
        now_unix: 100,
        limits: AuthoringLimits::default(),
    }
}
fn request(d: &DocumentRevision, action: AuthoringAction) -> AuthoringRequest<'_> {
    AuthoringRequest {
        action,
        target: d,
        expected_authority_generation: 3,
    }
}

#[test]
fn unbound_host_and_string_only_claims_deny() {
    let p = project();
    let d = document();
    let m = member(ProjectRole::Contributor);
    let admissions = [admission(&m)];
    assert_eq!(
        authorize(
            &view(&p, &d, None, &admissions, &[]),
            &request(&d, AuthoringAction::ExecuteSource)
        )
        .unwrap_err(),
        AuthoringError::Unauthenticated
    );
    let ids = [identifier("did:test:somebody-else")];
    let actor = principal(&ids);
    assert_eq!(
        authorize(
            &view(&p, &d, Some(&actor), &admissions, &[]),
            &request(&d, AuthoringAction::ExecuteSource)
        )
        .unwrap_err(),
        AuthoringError::NotMember
    );
}

#[test]
fn admitted_contributor_covers_every_write_action_and_multiple_identifiers() {
    let p = project();
    let d = document();
    let m = member(ProjectRole::Contributor);
    let ids = [
        identifier("did:test:another-qualified-identifier"),
        identifier("did:test:author"),
    ];
    let actor = principal(&ids);
    let admissions = [admission(&m)];
    let v = view(&p, &d, Some(&actor), &admissions, &[]);
    for action in [
        AuthoringAction::ReadSource,
        AuthoringAction::ExecuteSource,
        AuthoringAction::MaterialEdit,
        AuthoringAction::Undo,
        AuthoringAction::Redo,
        AuthoringAction::Rebuild,
        AuthoringAction::SaveCheckpoint,
        AuthoringAction::ReopenCheckpoint,
        AuthoringAction::ContributionRecord,
    ] {
        let permit = authorize(&v, &request(&d, action)).unwrap();
        assert_eq!(permit.action(), action);
        assert_eq!(permit.target(), &d);
    }
    assert_eq!(
        authorize(&v, &request(&d, AuthoringAction::ReviewDecision)).unwrap_err(),
        AuthoringError::SelfReview
    );
}

#[test]
fn observer_reads_but_cannot_write_even_with_valid_identity() {
    let p = project();
    let d = document();
    let m = member(ProjectRole::Observer);
    let ids = [identifier("did:test:author")];
    let actor = principal(&ids);
    let admissions = [admission(&m)];
    let v = view(&p, &d, Some(&actor), &admissions, &[]);
    authorize(&v, &request(&d, AuthoringAction::ReadSource)).unwrap();
    for action in [
        AuthoringAction::ExecuteSource,
        AuthoringAction::MaterialEdit,
        AuthoringAction::Undo,
        AuthoringAction::Redo,
        AuthoringAction::Rebuild,
        AuthoringAction::SaveCheckpoint,
        AuthoringAction::ReopenCheckpoint,
        AuthoringAction::ContributionRecord,
    ] {
        assert_eq!(
            authorize(&v, &request(&d, action)).unwrap_err(),
            AuthoringError::RoleDenied
        );
    }
}

#[test]
fn revocation_expiry_pending_and_cross_project_membership_deny() {
    let p = project();
    let d = document();
    let mut m = member(ProjectRole::Contributor);
    let ids = [identifier("did:test:author")];
    let actor = principal(&ids);
    for case in 0..5 {
        if case == 4 {
            m.project_id = "another-project".into();
        }
        let mut a = admission(&m);
        match case {
            0 => a.revoked = true,
            1 => a.valid_until_unix = 100,
            2 => a.valid_from_unix = 101,
            3 => a.member_subject = "other-subject",
            _ => {}
        }
        let admissions = [a];
        let v = view(&p, &d, Some(&actor), &admissions, &[]);
        assert_eq!(
            authorize(&v, &request(&d, AuthoringAction::MaterialEdit)).unwrap_err(),
            AuthoringError::NotMember
        );
    }
}

#[test]
fn identifier_association_must_be_current_and_project_qualified() {
    let p = project();
    let d = document();
    let m = member(ProjectRole::Contributor);
    let admissions = [admission(&m)];
    for case in 0..5 {
        let mut id = identifier("did:test:author");
        match case {
            0 => id.revoked = true,
            1 => id.valid_until_unix = 100,
            2 => id.project_id = "other-project",
            3 => id.valid_from_unix = 101,
            _ => id.evidence_ref = "",
        }
        let ids = [id];
        let actor = principal(&ids);
        let v = view(&p, &d, Some(&actor), &admissions, &[]);
        assert!(authorize(&v, &request(&d, AuthoringAction::ExecuteSource)).is_err());
    }
}

#[test]
fn exact_document_revision_digest_source_and_authority_are_required() {
    let p = project();
    let d = document();
    let m = member(ProjectRole::Contributor);
    let ids = [identifier("did:test:author")];
    let actor = principal(&ids);
    let admissions = [admission(&m)];
    let v = view(&p, &d, Some(&actor), &admissions, &[]);
    for case in 0..6 {
        let mut target = d.clone();
        match case {
            0 => target.project_id = "other-project".into(),
            1 => target.document_id = "other-document".into(),
            2 => target.source_id = "other-source".into(),
            3 => target.revision += 1,
            4 => target.source_digest = [2; 32],
            _ => target.author_subject = Some("other-author".into()),
        }
        assert_eq!(
            authorize(&v, &request(&target, AuthoringAction::SaveCheckpoint)).unwrap_err(),
            AuthoringError::StaleDocument
        );
    }
    let mut r = request(&d, AuthoringAction::MaterialEdit);
    r.expected_authority_generation = 2;
    assert_eq!(
        authorize(&v, &r).unwrap_err(),
        AuthoringError::StaleAuthority
    );
}

#[test]
fn current_revocation_rechecked_after_prior_success() {
    let p = project();
    let d = document();
    let m = member(ProjectRole::Contributor);
    let ids = [identifier("did:test:author")];
    let actor = principal(&ids);
    let mut admissions = [admission(&m)];
    {
        let v = view(&p, &d, Some(&actor), &admissions, &[]);
        authorize(&v, &request(&d, AuthoringAction::MaterialEdit)).unwrap();
    }
    admissions[0].revoked = true;
    let mut v = view(&p, &d, Some(&actor), &admissions, &[]);
    v.generation += 1;
    let mut r = request(&d, AuthoringAction::MaterialEdit);
    r.expected_authority_generation = 4;
    assert_eq!(authorize(&v, &r).unwrap_err(), AuthoringError::NotMember);
}

#[test]
fn review_requires_separate_current_document_grant_not_steward_role() {
    let p = project();
    let mut d = document();
    d.author_subject = Some("different-test-author".into());
    let m = member(ProjectRole::Steward);
    let ids = [identifier("did:test:author")];
    let actor = principal(&ids);
    let admissions = [admission(&m)];
    let v = view(&p, &d, Some(&actor), &admissions, &[]);
    assert_eq!(
        authorize(&v, &request(&d, AuthoringAction::ReviewDecision)).unwrap_err(),
        AuthoringError::ReviewDenied
    );
    for case in 0..5 {
        let mut g = grant();
        match case {
            0 => g.revoked = true,
            1 => g.valid_until_unix = 100,
            2 => g.project_id = "other-project",
            3 => g.document_id = "other-document",
            _ => g.reviewer_subject = "other-reviewer",
        }
        let grants = [g];
        let v = view(&p, &d, Some(&actor), &admissions, &grants);
        assert_eq!(
            authorize(&v, &request(&d, AuthoringAction::ReviewDecision)).unwrap_err(),
            AuthoringError::ReviewDenied
        );
    }
}

#[test]
fn authorized_review_preserves_revision_actor_capacity_and_actual_decision_evidence() {
    let p = project();
    let mut d = document();
    d.author_subject = Some("different-test-author".into());
    let m = member(ProjectRole::Observer);
    let ids = [identifier("did:test:author")];
    let actor = principal(&ids);
    let admissions = [admission(&m)];
    let grants = [grant()];
    let v = view(&p, &d, Some(&actor), &admissions, &grants);
    let record = authorize(&v, &request(&d, AuthoringAction::ReviewDecision))
        .unwrap()
        .review_record(
            "test-review",
            ReviewDecision::ChangesRequested,
            "test material needs revision",
            "test-decision-act",
        )
        .unwrap();
    assert_eq!(record.target, d);
    assert_eq!(record.principal_kind, PrincipalKind::SoftwareAgent);
    assert_eq!(record.decision, ReviewDecision::ChangesRequested);
    assert_eq!(
        serde_json::from_str::<ReviewRecord>(&serde_json::to_string(&record).unwrap()).unwrap(),
        record
    );
    assert!(authorize(&v, &request(&d, AuthoringAction::ReviewDecision))
        .unwrap()
        .review_record(
            "test-review",
            ReviewDecision::Accepted,
            "",
            "test-decision-act"
        )
        .is_err());
}

#[test]
fn contribution_preserves_canonical_identity_private_status_chain_and_exact_revision() {
    let p = project();
    let d = document();
    let m = member(ProjectRole::Contributor);
    let ids = [identifier("did:test:author")];
    let actor = principal(&ids);
    let admissions = [admission(&m)];
    let v = view(&p, &d, Some(&actor), &admissions, &[]);
    let first = authorize(&v, &request(&d, AuthoringAction::ContributionRecord))
        .unwrap()
        .contribution_record(
            "test-c1",
            "test editing work",
            12,
            WorkStatus::Unknown,
            None,
        )
        .unwrap();
    let next = authorize(&v, &request(&d, AuthoringAction::ContributionRecord))
        .unwrap()
        .contribution_record(
            "test-c2",
            "test later work",
            3,
            WorkStatus::Unpaid {
                evidence_ref: "test-self-report".into(),
            },
            Some(&first),
        )
        .unwrap();
    assert_eq!(next.target, d);
    assert_eq!(next.contribution.predecessor_id.as_deref(), Some("test-c1"));
    assert_eq!(next.contribution.capital_cents, 0);
    assert_eq!(next.contribution.effort_minutes, 3);
    assert_eq!(next.principal_kind, PrincipalKind::SoftwareAgent);
    assert_eq!(
        serde_json::from_str::<ContributionRecord>(&serde_json::to_string(&next).unwrap()).unwrap(),
        next
    );
    assert_eq!(
        authorize(&v, &request(&d, AuthoringAction::ReadSource))
            .unwrap()
            .contribution_record("test-c3", "test", 0, WorkStatus::Unknown, None)
            .unwrap_err(),
        AuthoringError::WrongAction
    );
}

#[test]
fn bounds_authentication_expiry_ambiguous_membership_and_empty_digest_fail_closed() {
    let p = project();
    let d = document();
    let m = member(ProjectRole::Contributor);
    let ids = [identifier("did:test:author")];
    let mut actor = principal(&ids);
    let admissions = [admission(&m)];
    actor.authenticated_until_unix = 100;
    assert_eq!(
        authorize(
            &view(&p, &d, Some(&actor), &admissions, &[]),
            &request(&d, AuthoringAction::ReadSource)
        )
        .unwrap_err(),
        AuthoringError::Unauthenticated
    );
    actor.authenticated_until_unix = 1000;
    let mut v = view(&p, &d, Some(&actor), &admissions, &[]);
    v.limits.max_identifiers = 0;
    assert_eq!(
        authorize(&v, &request(&d, AuthoringAction::ReadSource)).unwrap_err(),
        AuthoringError::BudgetExceeded
    );
    let duplicates = [admission(&m), admission(&m)];
    let v = view(&p, &d, Some(&actor), &duplicates, &[]);
    assert_eq!(
        authorize(&v, &request(&d, AuthoringAction::ReadSource)).unwrap_err(),
        AuthoringError::NotMember
    );
    let mut bad = d.clone();
    bad.source_digest = [0; 32];
    let v = view(&p, &bad, Some(&actor), &admissions, &[]);
    assert_eq!(
        authorize(&v, &request(&bad, AuthoringAction::ReadSource)).unwrap_err(),
        AuthoringError::InvalidBinding
    );
    let mut v = view(&p, &d, Some(&actor), &admissions, &[]);
    v.limits.max_record_bytes = 1;
    assert_eq!(
        authorize(&v, &request(&d, AuthoringAction::ContributionRecord))
            .unwrap()
            .contribution_record("test", "test", 0, WorkStatus::Unknown, None)
            .unwrap_err(),
        AuthoringError::BudgetExceeded
    );
}

#[test]
fn conflicting_identifier_associations_cannot_hide_revocation() {
    let p = project();
    let d = document();
    let m = member(ProjectRole::Contributor);
    let admissions = [admission(&m)];
    for reverse in [false, true] {
        let mut revoked = identifier("did:test:author");
        revoked.revoked = true;
        let active = identifier("did:test:author");
        let ids = if reverse {
            [revoked, active]
        } else {
            [active, revoked]
        };
        let actor = principal(&ids);
        let v = view(&p, &d, Some(&actor), &admissions, &[]);
        assert_eq!(
            authorize(&v, &request(&d, AuthoringAction::ReadSource)).unwrap_err(),
            AuthoringError::NotMember
        );
    }
}

#[test]
fn canonical_high_resolution_membership_time_cannot_be_hidden_by_legacy_coarse_field() {
    let p = project();
    let d = document();
    let mut m = member(ProjectRole::Contributor);
    m.agreed_at_instant = Some(
        crate::projects::ProjectMembership::new(
            "fixture",
            "fixture",
            ProjectRole::Contributor,
            101,
        )
        .agreed_at(),
    );
    let admissions = [admission(&m)];
    let ids = [identifier("did:test:author")];
    let actor = principal(&ids);
    let v = view(&p, &d, Some(&actor), &admissions, &[]);
    assert_eq!(
        authorize(&v, &request(&d, AuthoringAction::ReadSource)).unwrap_err(),
        AuthoringError::NotMember
    );
}
