//! Encrypted test-only key files; never opens an OS credential provider.
use super::*;
use qualia_core_db::poet_host::native_authority::enrollment::NativeSigner;

#[test]
fn package_review_explicit_steward_signed_grant_and_revocation_do_not_elevate_source_grants() {
    let root = tempfile::tempdir().unwrap();
    let wrap = [67; 32];
    let signer = NativeSigner::create_sealed(&root.path().join("authority.sealed"), &wrap).unwrap();
    let wrong = NativeSigner::create_sealed(&root.path().join("wrong.sealed"), &wrap).unwrap();
    let mut adopted = snapshot();
    adopted.subjects[0].membership.role = ProjectRole::Steward;
    let until = adopted.valid_until_unix;
    let native_anchor = TrustedProject {
        project_id: "project".into(),
        issuer_identifier: "fixture-authority".into(),
        public_key: signer.public_key(),
    };
    let journal = root.path().join("journal");
    std::fs::create_dir(&journal).unwrap();
    let owner = NativeAuthority::open(
        &journal,
        native_anchor,
        signer.sign_admission(adopted).unwrap(),
        document(),
        1024 * 1024,
        256,
    )
    .unwrap();
    let author = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &author, [7; 32]);
    let target = package_target();
    request(&owner, &author, &target);
    let grant = PackageReviewGrant {
        id: "urn:fixture:explicit-admin-grant".into(),
        target: target.clone(),
        reviewer_subject: "reviewer".into(),
        admission_ref: "urn:fixture:explicit-admin-evidence".into(),
        valid_from_unix: 1,
        valid_until_unix: until,
        revoked: false,
    };
    let permission = owner.enrollment_permission(&author).unwrap();
    let before = std::fs::read_dir(&journal).unwrap().count();
    assert!(owner
        .grant_package_review(&permission, grant.clone(), &wrong)
        .is_err());
    assert_eq!(std::fs::read_dir(&journal).unwrap().count(), before);
    assert!(owner
        .enrollment_snapshot()
        .unwrap()
        .snapshot
        .package_reviews
        .is_empty());
    let signed = owner
        .grant_package_review(&permission, grant.clone(), &signer)
        .unwrap();
    assert_eq!(signed.snapshot.package_reviews, vec![grant]);
    assert!(owner
        .package_workflow_evidence(&author, &target, |_| Ok(()))
        .is_err());
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    assert!(
        owner
            .package_workflow_evidence(&reviewer, &target, |_| Ok(()))
            .unwrap()
            .can_review
    );
    assert!(owner.enrollment_permission(&reviewer).is_err());
    let author = login(&owner, "author", &AUTHOR);
    let permission = owner.enrollment_permission(&author).unwrap();
    owner
        .revoke_package_review(
            &permission,
            "urn:fixture:explicit-admin-grant",
            "urn:fixture:explicit-revoke-evidence",
            &signer,
        )
        .unwrap();
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let evidence = owner
        .package_workflow_evidence(&reviewer, &target, |_| Ok(()))
        .unwrap();
    assert!(!evidence.can_review);
    assert!(
        owner.workflow_evidence(&reviewer).unwrap().can_review,
        "source grants remain a separate scope"
    );
    let retained = owner.enrollment_snapshot().unwrap();
    assert!(retained.snapshot.package_reviews[0].revoked);
}
