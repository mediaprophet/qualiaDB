//! Retained predecessor exclusions survive overwrite and package identifier aliases.
use super::*;
use qualia_core_db::poet_host::native_authority::enrollment::NativeSigner;

#[test]
fn package_review_predecessor_writer_cannot_grant_or_review_renamed_successor() {
    let root = tempfile::tempdir().unwrap();
    let signer =
        NativeSigner::create_sealed(&root.path().join("authority.sealed"), &[67; 32]).unwrap();
    let mut adopted = snapshot();
    adopted.subjects[0].membership.role = ProjectRole::Steward;
    let mut source_author = adopted.subjects[0].clone();
    source_author.subject = "source-author".into();
    source_author.controller_public_key = ed25519::public_from_seed(&[41; 32]);
    source_author.membership.id = "urn:fixture:membership:c".into();
    source_author.membership.member_did = "urn:fixture:actor:c".into();
    source_author.identifiers[0].value = "urn:fixture:actor:c".into();
    source_author.reviews.clear();
    adopted.subjects.push(source_author);
    let mut source = document();
    source.author_subject = Some("source-author".into());
    let first = PackageRevision {
        source: source.clone(),
        ..package_target()
    };
    let mut successor = first.clone();
    successor.package_digest = [8; 32];
    successor.predecessor_digest = Some([7; 32]);
    successor.producer_subject = "reviewer".into();
    successor.package_id = "urn:fixture:renamed-release".into();
    let grant = PackageReviewGrant {
        id: "urn:fixture:preexisting-exact-grant".into(),
        target: successor.clone(),
        reviewer_subject: "author".into(),
        admission_ref: "urn:fixture:explicit-adoption".into(),
        valid_from_unix: 1,
        valid_until_unix: adopted.valid_until_unix,
        revoked: false,
    };
    adopted.package_reviews.push(grant.clone());
    let anchor = TrustedProject {
        project_id: "project".into(),
        issuer_identifier: "fixture-authority".into(),
        public_key: signer.public_key(),
    };
    let journal = root.path().join("journal");
    std::fs::create_dir(&journal).unwrap();
    let owner = NativeAuthority::open(
        &journal,
        anchor,
        signer.sign_admission(adopted).unwrap(),
        source,
        1024 * 1024,
        256,
    )
    .unwrap();
    let a = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &a, [7; 32]);
    let gen = generation(&owner, &a);
    owner
        .record_package_contribution(
            &a,
            gen,
            &first,
            "urn:fixture:prior-contribution",
            "prior package work",
            3,
            WorkStatus::Unknown,
            |_| Ok(()),
        )
        .unwrap();
    let b = login(&owner, "reviewer", &REVIEWER);
    owner
        .transact_with_package(
            &b,
            Some(&PackageWriteIntent {
                filename: "working.hmc".into(),
                expected_digest: Some([7; 32]),
            }),
            |_| {
                (
                    DocumentTransaction {
                        result: Ok(()),
                        successor: None,
                        reconciliation_required: false,
                    },
                    Some(PackageWriteReceipt {
                        digest: [8; 32],
                        bytes: 123,
                    }),
                )
            },
        )
        .unwrap();
    request(&owner, &b, &successor);
    let a = login(&owner, "author", &AUTHOR);
    let gen = generation(&owner, &a);
    let evidence = owner
        .package_workflow_evidence(&a, &successor, |_| Ok(()))
        .unwrap();
    assert!(evidence.known_destination_lineage_complete);
    assert!(!evidence.can_review);
    assert!(owner
        .record_requested_package_review(
            &a,
            gen,
            &successor,
            "urn:fixture:request",
            "urn:fixture:predecessor-decision",
            ReviewDecision::Accepted,
            "prior producer excluded",
            "urn:fixture:evidence",
            |_| panic!("known predecessor cannot exercise review")
        )
        .is_err());
    let permission = owner.enrollment_permission(&a).unwrap();
    let mut new_grant = grant;
    new_grant.id = "urn:fixture:new-aliased-grant".into();
    // A second declared release alias keeps the saved bytes/source identical,
    // while avoiding duplicate exact-target/reviewer scope in the signed body.
    // That structural duplicate would refuse before testing ancestor exclusion.
    new_grant.target.package_id = "urn:fixture:second-alias-same-successor".into();
    assert_ne!(new_grant.target.package_id, successor.package_id);
    assert_eq!(new_grant.target.package_digest, successor.package_digest);
    assert_eq!(new_grant.target.source, successor.source);
    assert_eq!(
        new_grant.target.producer_subject,
        successor.producer_subject
    );
    assert!(owner
        .grant_package_review(&permission, new_grant.clone(), &signer)
        .unwrap_err()
        .contains("predecessor"));
    let retained = owner.enrollment_snapshot().unwrap().snapshot;
    let mut replacement = retained.clone();
    replacement.sequence += 1;
    replacement.package_reviews.push(new_grant);
    assert_eq!(
        &replacement.package_reviews[..retained.package_reviews.len()],
        retained.package_reviews.as_slice(),
        "every old signed scope must remain unchanged"
    );
    let signed = signer.sign_admission(replacement).unwrap();
    // Prove the signed body passes ordinary signature/project/subject/grant
    // shape admission without the original owner's contributor history.
    let structural_root = tempfile::tempdir().unwrap();
    let structural_owner = NativeAuthority::open(
        structural_root.path(),
        TrustedProject {
            project_id: signed.snapshot.project.id.clone(),
            issuer_identifier: signed.snapshot.issuer_identifier.clone(),
            public_key: signer.public_key(),
        },
        signed.clone(),
        successor.source.clone(),
        1024 * 1024,
        256,
    )
    .expect("replacement must pass signature and ordinary scope-shape admission");
    drop(structural_owner);
    let error = owner.replace_admission(signed).unwrap_err();
    assert!(
        error.contains("predecessor"),
        "expected ancestor denial after valid structural admission: {error}"
    );
}

#[test]
fn package_review_missing_predecessor_stays_explicitly_scoped_and_unassessed() {
    let root = tempfile::tempdir().unwrap();
    let mut adopted = package_snapshot();
    adopted.package_reviews[0].target.package_digest = [8; 32];
    adopted.package_reviews[0].target.predecessor_digest = Some([99; 32]);
    let target = adopted.package_reviews[0].target.clone();
    let owner = open(root.path(), adopted);
    let a = login(&owner, "author", &AUTHOR);
    owner
        .transact_with_package(
            &a,
            Some(&PackageWriteIntent {
                filename: "working.hmc".into(),
                expected_digest: Some([99; 32]),
            }),
            |_| {
                (
                    DocumentTransaction {
                        result: Ok(()),
                        successor: None,
                        reconciliation_required: false,
                    },
                    Some(PackageWriteReceipt {
                        digest: [8; 32],
                        bytes: 123,
                    }),
                )
            },
        )
        .unwrap();
    request(&owner, &a, &target);
    let b = login(&owner, "reviewer", &REVIEWER);
    let gen = generation(&owner, &b);
    let evidence = owner
        .package_workflow_evidence(&b, &target, |_| Ok(()))
        .unwrap();
    assert!(!evidence.known_destination_lineage_complete);
    assert!(evidence.can_review);
    assert!(!evidence.requests[0].known_destination_lineage_complete);
    let decision = owner
        .record_requested_package_review(
            &b,
            gen,
            &target,
            "urn:fixture:request",
            "urn:fixture:scoped-decision",
            ReviewDecision::ChangesRequested,
            "missing prior destination retained as unknown",
            "urn:fixture:unknown-history",
            |_| Ok(()),
        )
        .unwrap();
    assert!(!decision.known_destination_lineage_complete);
    assert!(decision.contribution_rights_scope.contains("unassessed"));
}
