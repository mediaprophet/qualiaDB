//! Known source work survives revisions with no intervening HMC save.
use super::*;
use qualia_core_db::poet_host::native_authority::enrollment::NativeSigner;

#[test]
fn package_review_unsaved_prior_source_contributor_is_excluded_after_source_successor() {
    let root = tempfile::tempdir().unwrap();
    let signer =
        NativeSigner::create_sealed(&root.path().join("authority.sealed"), &[67; 32]).unwrap();
    let mut adopted = snapshot();
    adopted.subjects[0].membership.role = ProjectRole::Steward;
    let mut producer = adopted.subjects[1].clone();
    producer.subject = "third-producer".into();
    producer.controller_public_key = ed25519::public_from_seed(&[41; 32]);
    producer.membership.id = "urn:fixture:membership:third".into();
    producer.membership.member_did = "urn:fixture:actor:third".into();
    producer.identifiers[0].value = "urn:fixture:actor:third".into();
    producer.reviews.clear();
    adopted.subjects.push(producer);
    let mut target = package_target();
    target.source.revision = 2;
    target.source.source_digest = [9; 32];
    target.source.author_subject = Some("reviewer".into());
    target.producer_subject = "third-producer".into();
    target.package_id = "urn:fixture:renamed-source-successor-release".into();
    let grant = PackageReviewGrant {
        id: "urn:fixture:future-package-scope".into(),
        target: target.clone(),
        reviewer_subject: "author".into(),
        admission_ref: "urn:fixture:scope-adoption".into(),
        valid_from_unix: 1,
        valid_until_unix: adopted.valid_until_unix,
        revoked: false,
    };
    adopted.package_reviews.push(grant.clone());
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
    let a = login(&owner, "author", &AUTHOR);
    owner
        .record_contribution(
            &a,
            "urn:fixture:source-v1-contribution",
            "source work before any package save",
            5,
            WorkStatus::Unknown,
        )
        .unwrap();
    assert!(owner.package_write_evidence(&a).unwrap().is_empty());
    let b = login(&owner, "reviewer", &REVIEWER);
    owner
        .transact(&b, |_| DocumentTransaction {
            result: Ok(()),
            successor: Some(target.source.clone()),
            reconciliation_required: false,
        })
        .unwrap();
    let c = login(&owner, "third-producer", &[41; 32]);
    save_audit(&owner, &c, [7; 32]);
    request(&owner, &c, &target);
    let a = login(&owner, "author", &AUTHOR);
    let gen = generation(&owner, &a);
    assert!(
        !owner
            .package_workflow_evidence(&a, &target, |_| Ok(()))
            .unwrap()
            .can_review
    );
    assert!(owner
        .record_requested_package_review(
            &a,
            gen,
            &target,
            "urn:fixture:request",
            "urn:fixture:invalid-source-history-review",
            ReviewDecision::Accepted,
            "known prior source work",
            "urn:fixture:evidence",
            |_| panic!("known source contributor must be excluded")
        )
        .is_err());
    let permission = owner.enrollment_permission(&a).unwrap();
    let mut new_grant = grant;
    new_grant.id = "urn:fixture:aliased-source-history-grant".into();
    assert!(owner
        .grant_package_review(&permission, new_grant, &signer)
        .unwrap_err()
        .contains("predecessor"));
}
