//! Software fixtures; no personhood, human review, payment or model acceptance.
use super::{super::*, *};
use crate::projects::{Project, ProjectMembership, ProjectRole};

fn target() -> PackageRevision {
    PackageRevision {
        package_id: "urn:fixture:release:1".into(),
        filename: "working.hmc".into(),
        package_digest: [2; 32],
        predecessor_digest: Some([1; 32]),
        package_bytes: 400,
        source: DocumentRevision {
            project_id: "project".into(),
            document_id: "document".into(),
            source_id: "source".into(),
            revision: 7,
            source_digest: [3; 32],
            author_subject: Some("source-author".into()),
        },
        producer_subject: "producer".into(),
    }
}
fn grant(target: &PackageRevision) -> PackageReviewGrant {
    PackageReviewGrant {
        id: "urn:fixture:package-grant".into(),
        target: target.clone(),
        reviewer_subject: "reviewer".into(),
        admission_ref: "urn:fixture:admission".into(),
        valid_from_unix: 1,
        valid_until_unix: 1000,
        revoked: false,
    }
}
fn source_view<T>(
    subject: &str,
    role: ProjectRole,
    run: impl FnOnce(&HostAuthorityView<'_>) -> T,
) -> T {
    let target = target();
    let project = Project {
        id: "project".into(),
        name: "fixture".into(),
        description: String::new(),
        licensing_ontologies: vec![],
        created_at_unix: 1,
        created_at_instant: None,
    };
    let identifiers = [QualifiedIdentifier {
        value: "urn:fixture:actor",
        scheme: "urn",
        issuer: "fixture",
        project_id: "project",
        evidence_ref: "urn:fixture:association",
        valid_from_unix: 1,
        valid_until_unix: 1000,
        revoked: false,
    }];
    let principal = AuthenticatedPrincipal {
        subject,
        kind: PrincipalKind::SoftwareAgent,
        authentication_ref: "fixture-auth",
        authenticated_until_unix: 1000,
        identifiers: &identifiers,
    };
    let membership = ProjectMembership {
        id: "membership".into(),
        project_id: "project".into(),
        member_did: "urn:fixture:actor".into(),
        role,
        agreed_at_unix: 1,
        agreed_at_instant: None,
    };
    let memberships = [AdmittedMembership {
        membership: &membership,
        member_subject: subject,
        admission_ref: "fixture-member",
        valid_from_unix: 1,
        valid_until_unix: 1000,
        revoked: false,
    }];
    let source_grants = [ReviewGrant {
        id: "source-grant",
        project_id: "project",
        document_id: "document",
        reviewer_subject: subject,
        admission_ref: "source-admission",
        valid_from_unix: 1,
        valid_until_unix: 1000,
        revoked: false,
    }];
    run(&HostAuthorityView {
        principal: Some(&principal),
        project: &project,
        memberships: &memberships,
        review_grants: &source_grants,
        current_document: &target.source,
        generation: 3,
        now_unix: 100,
        limits: AuthoringLimits::default(),
    })
}

#[test]
fn source_grant_never_implies_package_grant() {
    source_view("reviewer", ProjectRole::Contributor, |source| {
        let target = target();
        let view = PackageAuthorityView {
            source,
            known_destination_lineage_complete: true,
            current_package: &target,
            grants: &[],
            contributor_subjects: &[],
        };
        assert!(authorize(
            source,
            &AuthoringRequest {
                action: AuthoringAction::ReviewDecision,
                target: &target.source,
                expected_authority_generation: 3
            }
        )
        .is_ok());
        assert_eq!(
            authorize_package(&view, &target, 3, PackageAction::Decide).unwrap_err(),
            AuthoringError::ReviewDenied
        );
    });
}
#[test]
fn package_grant_requires_exact_release_source_digest_and_lineage() {
    source_view("reviewer", ProjectRole::Contributor, |source| {
        let target = target();
        for case in 0..9 {
            let mut grant = grant(&target);
            match case {
                0 => grant.target.package_id.push('x'),
                1 => grant.target.filename.push('x'),
                2 => grant.target.package_digest = [9; 32],
                3 => grant.target.predecessor_digest = None,
                4 => grant.target.source.revision += 1,
                5 => grant.target.source.source_digest = [8; 32],
                6 => grant.target.source.project_id = "other".into(),
                7 => grant.target.producer_subject = "other".into(),
                _ => grant.target.package_bytes += 1,
            };
            let grants = [grant];
            let view = PackageAuthorityView {
                source,
                known_destination_lineage_complete: true,
                current_package: &target,
                grants: &grants,
                contributor_subjects: &[],
            };
            assert!(
                authorize_package(&view, &target, 3, PackageAction::Decide).is_err(),
                "case {case}"
            );
        }
    });
}
#[test]
fn revoked_expired_duplicate_and_stale_generation_fail_closed() {
    source_view("reviewer", ProjectRole::Contributor, |source| {
        let target = target();
        for case in 0..4 {
            let mut grants = vec![grant(&target)];
            match case {
                0 => grants[0].revoked = true,
                1 => grants[0].valid_until_unix = 100,
                2 => grants.push(grants[0].clone()),
                _ => {}
            };
            let view = PackageAuthorityView {
                source,
                known_destination_lineage_complete: true,
                current_package: &target,
                grants: &grants,
                contributor_subjects: &[],
            };
            assert!(authorize_package(
                &view,
                &target,
                if case == 3 { 2 } else { 3 },
                PackageAction::Decide
            )
            .is_err());
        }
    });
}
#[test]
fn producer_source_author_and_known_contributor_cannot_review() {
    for subject in ["producer", "source-author", "other-contributor"] {
        source_view(subject, ProjectRole::Contributor, |source| {
            let target = target();
            let mut grant = grant(&target);
            grant.reviewer_subject = subject.into();
            let grants = [grant];
            let view = PackageAuthorityView {
                source,
                known_destination_lineage_complete: true,
                current_package: &target,
                grants: &grants,
                contributor_subjects: &["other-contributor"],
            };
            assert_eq!(
                authorize_package(&view, &target, 3, PackageAction::Decide).unwrap_err(),
                AuthoringError::SelfReview
            );
        });
    }
}
#[test]
fn resource_producer_may_differ_from_source_author_and_request_exact_contribution() {
    source_view("producer", ProjectRole::Contributor, |source| {
        let target = target();
        let view = PackageAuthorityView {
            source,
            known_destination_lineage_complete: true,
            current_package: &target,
            grants: &[],
            contributor_subjects: &[],
        };
        let contribution = authorize_package(&view, &target, 3, PackageAction::Contribute)
            .unwrap()
            .contribution_record(
                "urn:fixture:contribution",
                "resource work",
                4,
                WorkStatus::Unknown,
            )
            .unwrap();
        let request = authorize_package(&view, &target, 3, PackageAction::RequestReview)
            .unwrap()
            .request_record("urn:fixture:request", &contribution, "urn:fixture:evidence")
            .unwrap();
        assert_eq!(request.target, contribution.target);
        assert_eq!(contribution.contribution.capital_cents, 0);
        assert_eq!(request.actor.principal_kind, PrincipalKind::SoftwareAgent);
        let mut wrong = contribution.clone();
        wrong.target.package_digest = [8; 32];
        assert!(
            authorize_package(&view, &target, 3, PackageAction::RequestReview)
                .unwrap()
                .request_record("urn:fixture:wrong", &wrong, "urn:fixture:evidence")
                .is_err()
        );
    });
}
#[test]
fn scoped_package_decision_and_record_budget_are_explicit() {
    source_view("reviewer", ProjectRole::Observer, |source| {
        let target = target();
        let grants = [grant(&target)];
        let view = PackageAuthorityView {
            source,
            known_destination_lineage_complete: true,
            current_package: &target,
            grants: &grants,
            contributor_subjects: &[],
        };
        let record = authorize_package(&view, &target, 3, PackageAction::Decide)
            .unwrap()
            .review_record(
                "urn:fixture:decision",
                "urn:fixture:request",
                ReviewDecision::ChangesRequested,
                "reviewed exact bytes",
                "urn:fixture:evidence",
            )
            .unwrap();
        assert_eq!(record.target, target);
        assert_eq!(record.actor.principal_kind, PrincipalKind::SoftwareAgent);
        assert!(authorize_package(&view, &target, 3, PackageAction::Decide)
            .unwrap()
            .review_record(
                "urn:fixture:oversize",
                "urn:fixture:request",
                ReviewDecision::Accepted,
                &"x".repeat(4097),
                "urn:fixture:evidence"
            )
            .is_err());
    });
}
#[test]
fn observers_cannot_produce_and_foreign_producers_cannot_request() {
    for (subject, role) in [
        ("producer", ProjectRole::Observer),
        ("reviewer", ProjectRole::Contributor),
    ] {
        source_view(subject, role, |source| {
            let target = target();
            let view = PackageAuthorityView {
                source,
                known_destination_lineage_complete: true,
                current_package: &target,
                grants: &[],
                contributor_subjects: &[],
            };
            assert!(authorize_package(&view, &target, 3, PackageAction::Contribute).is_err());
            assert!(authorize_package(&view, &target, 3, PackageAction::RequestReview).is_err());
        });
    }
}
