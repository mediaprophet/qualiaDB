//! Public native package APIs; signed software fixtures are not human acceptance.
#[path = "native_package_review/fixture.rs"]
mod fixture;
use fixture::*;
#[cfg(feature = "sanctuary-crypto")]
#[path = "native_package_review/admin.rs"]
mod admin;
#[cfg(feature = "sanctuary-crypto")]
#[path = "native_package_review/history.rs"]
mod history;
#[path = "native_package_review/publication.rs"]
mod publication;
#[cfg(feature = "sanctuary-crypto")]
#[path = "native_package_review/source_history.rs"]
mod source_history;
#[path = "native_package_review/target_isolation.rs"]
mod target_isolation;
fn package_target() -> PackageRevision {
    PackageRevision {
        package_id: "urn:fixture:project:release:1".into(),
        filename: "working.hmc".into(),
        package_digest: [7; 32],
        predecessor_digest: None,
        package_bytes: 123,
        source: document(),
        producer_subject: "author".into(),
    }
}
fn package_snapshot() -> AdmissionSnapshot {
    let mut snapshot = snapshot();
    snapshot.package_reviews.push(PackageReviewGrant {
        id: "urn:fixture:grant".into(),
        target: package_target(),
        reviewer_subject: "reviewer".into(),
        admission_ref: "urn:fixture:package-adoption".into(),
        valid_from_unix: 1,
        valid_until_unix: snapshot.valid_until_unix,
        revoked: false,
    });
    snapshot
}
fn save_audit(owner: &NativeAuthority, author: &SessionHandle, digest: [u8; 32]) {
    owner
        .transact_with_package(
            author,
            Some(&PackageWriteIntent {
                filename: "working.hmc".into(),
                expected_digest: None,
            }),
            |_| {
                (
                    DocumentTransaction {
                        result: Ok(()),
                        successor: None,
                        reconciliation_required: false,
                    },
                    Some(PackageWriteReceipt { digest, bytes: 123 }),
                )
            },
        )
        .unwrap();
}
fn request(owner: &NativeAuthority, author: &SessionHandle, target: &PackageRevision) {
    let generation = generation(owner, author);
    owner
        .record_package_contribution(
            author,
            generation,
            target,
            "urn:fixture:contribution",
            "saved native package",
            5,
            WorkStatus::Unknown,
            |_| Ok(()),
        )
        .unwrap();
    owner
        .request_package_review(
            author,
            generation,
            target,
            "urn:fixture:contribution",
            "urn:fixture:request",
            "urn:fixture:request-evidence",
            |_| Ok(()),
        )
        .unwrap();
}

#[test]
fn package_review_old_signed_admission_bytes_remain_exact() {
    // Literal pre-extension v1 body, independent of the new snapshot serializer.
    let golden=br#"{"version":1,"sequence":1,"issuer_identifier":"fixture-authority","project":{"id":"project","name":"legacy fixture","description":"","licensing_ontologies":[],"created_at_unix":1},"valid_from_unix":1,"valid_until_unix":4000000000,"revoked":false,"subjects":[]}"#;
    let snapshot: AdmissionSnapshot = serde_json::from_slice(golden).unwrap();
    assert!(snapshot.package_reviews.is_empty());
    assert_eq!(serde_json::to_vec(&snapshot).unwrap(), golden);
    let digest: [u8; 32] = Sha256::digest(golden).into();
    assert_eq!(admission_digest(&snapshot, 1024 * 1024).unwrap(), digest);
    let mut signature = [0; 64];
    ed25519::sign_with_context(&AUTHORITY, &digest, ADMISSION_CONTEXT, &mut signature).unwrap();
    let legacy = SignedAdmission {
        snapshot,
        signature: signature.to_vec(),
    };
    let root = tempfile::tempdir().unwrap();
    let owner =
        NativeAuthority::open(root.path(), anchor(), legacy, document(), 1024 * 1024, 256).unwrap();
    assert_eq!(
        admission_digest(&owner.enrollment_snapshot().unwrap().snapshot, 1024 * 1024).unwrap(),
        digest
    );
}

#[test]
fn package_review_contribution_request_decision_and_reopen_keep_exact_lineage() {
    let root = tempfile::tempdir().unwrap();
    let owner = open(root.path(), package_snapshot());
    let author = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &author, [7; 32]);
    let target = owner
        .saved_package_target(&author, "working.hmc", "urn:fixture:project:release:1")
        .unwrap();
    assert_eq!(target, package_target());
    request(&owner, &author, &target);
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let gen = generation(&owner, &reviewer);
    let record = owner
        .record_requested_package_review(
            &reviewer,
            gen,
            &target,
            "urn:fixture:request",
            "urn:fixture:decision",
            ReviewDecision::ChangesRequested,
            "fixture scoped decision",
            "urn:fixture:decision-evidence",
            |current| {
                assert_eq!(current, &target);
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(record.actor.principal_kind, PrincipalKind::SoftwareAgent);
    let retained = owner.enrollment_snapshot().unwrap();
    drop(owner);
    let reopened = NativeAuthority::open(
        root.path(),
        anchor(),
        retained,
        document(),
        1024 * 1024,
        256,
    )
    .unwrap();
    assert!(reopened
        .package_workflow_evidence(&reviewer, &target, |_| Ok(()))
        .is_err());
    let reviewer = login(&reopened, "reviewer", &REVIEWER);
    let evidence = reopened
        .package_workflow_evidence(&reviewer, &target, |_| Ok(()))
        .unwrap();
    assert_eq!(evidence.contributions.len(), 1);
    assert_eq!(evidence.requests.len(), 1);
    assert_eq!(evidence.reviews.len(), 1);
    assert_eq!(
        evidence.requests[0].decision_ids,
        vec!["urn:fixture:decision"]
    );
    assert_eq!(evidence.reviews[0].target, target);
}

#[test]
fn package_review_source_grants_file_mismatch_and_stale_generation_cannot_record() {
    let root = tempfile::tempdir().unwrap();
    let owner = open(root.path(), snapshot());
    let author = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &author, [7; 32]);
    let target = package_target();
    let gen = generation(&owner, &author);
    assert!(owner
        .record_package_contribution(
            &author,
            gen,
            &target,
            "urn:fixture:refused",
            "file differs",
            1,
            WorkStatus::Unknown,
            |_| Err("actual trusted checkpoint hash differs".into())
        )
        .is_err());
    assert!(owner
        .package_workflow_evidence(&author, &target, |_| Ok(()))
        .unwrap()
        .contributions
        .is_empty());
    assert!(owner
        .record_package_contribution(
            &author,
            gen + 1,
            &target,
            "urn:fixture:stale",
            "stale authority",
            1,
            WorkStatus::Unknown,
            |_| panic!("stale authority must refuse before file callback")
        )
        .is_err());
    request(&owner, &author, &target);
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let gen = generation(&owner, &reviewer);
    assert!(owner
        .record_requested_package_review(
            &reviewer,
            gen,
            &target,
            "urn:fixture:request",
            "urn:fixture:source-only",
            ReviewDecision::Accepted,
            "source grant is insufficient",
            "urn:fixture:evidence",
            |_| panic!("ungranted callback")
        )
        .is_err());
}

#[test]
fn package_review_self_review_duplicate_ids_and_changed_saved_digest_refuse() {
    let root = tempfile::tempdir().unwrap();
    let owner = open(root.path(), package_snapshot());
    let author = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &author, [7; 32]);
    let target = package_target();
    request(&owner, &author, &target);
    let gen = generation(&owner, &author);
    assert!(owner
        .record_requested_package_review(
            &author,
            gen,
            &target,
            "urn:fixture:request",
            "urn:fixture:self",
            ReviewDecision::Accepted,
            "self",
            "urn:fixture:self",
            |_| Ok(())
        )
        .is_err());
    assert!(owner
        .request_package_review(
            &author,
            gen,
            &target,
            "urn:fixture:contribution",
            "urn:fixture:request",
            "urn:fixture:duplicate",
            |_| Ok(())
        )
        .is_err());
    save_audit(&owner, &author, [8; 32]);
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let gen = generation(&owner, &reviewer);
    assert!(owner
        .record_requested_package_review(
            &reviewer,
            gen,
            &target,
            "urn:fixture:request",
            "urn:fixture:stale-package",
            ReviewDecision::Accepted,
            "stale package",
            "urn:fixture:stale",
            |_| panic!("changed saved digest callback")
        )
        .is_err());
}

#[test]
fn package_review_revocation_and_unsafe_grant_successors_are_atomic() {
    let root = tempfile::tempdir().unwrap();
    let initial = package_snapshot();
    let owner = open(root.path(), initial.clone());
    let author = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &author, [7; 32]);
    let target = package_target();
    request(&owner, &author, &target);
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    for case in 0..3 {
        let mut replacement = initial.clone();
        replacement.sequence += 1;
        match case {
            0 => replacement.package_reviews.clear(),
            1 => replacement.package_reviews[0].target.package_digest = [9; 32],
            _ => replacement.package_reviews[0].target.package_id.push('x'),
        };
        assert!(owner.replace_admission(sign(replacement)).is_err());
    }
    let mut revoked = initial;
    revoked.sequence += 1;
    revoked.package_reviews[0].revoked = true;
    owner.replace_admission(sign(revoked.clone())).unwrap();
    assert!(owner
        .package_workflow_evidence(&reviewer, &target, |_| Ok(()))
        .is_err());
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    assert!(
        !owner
            .package_workflow_evidence(&reviewer, &target, |_| Ok(()))
            .unwrap()
            .can_review
    );
    revoked.sequence += 1;
    revoked.package_reviews[0].revoked = false;
    assert!(owner.replace_admission(sign(revoked)).is_err());
}

#[test]
fn package_review_source_successor_and_revoked_membership_withhold_old_targets() {
    let root = tempfile::tempdir().unwrap();
    let initial = package_snapshot();
    let owner = open(root.path(), initial.clone());
    let author = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &author, [7; 32]);
    let target = package_target();
    request(&owner, &author, &target);
    owner
        .transact(&author, |view| {
            let mut successor = view.current_document.clone();
            successor.revision += 1;
            successor.source_digest = [9; 32];
            DocumentTransaction {
                result: Ok(()),
                successor: Some(successor),
                reconciliation_required: false,
            }
        })
        .unwrap();
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let gen = generation(&owner, &reviewer);
    assert!(owner
        .record_requested_package_review(
            &reviewer,
            gen,
            &target,
            "urn:fixture:request",
            "urn:fixture:old-source",
            ReviewDecision::Accepted,
            "old source",
            "urn:fixture:old-source",
            |_| panic!("source successor must refuse callback")
        )
        .is_err());
    let mut revoked = initial;
    revoked.sequence += 1;
    revoked.subjects[1].revoked = true;
    owner.replace_admission(sign(revoked)).unwrap();
    assert!(owner
        .package_workflow_evidence(&reviewer, &target, |_| Ok(()))
        .is_err());
    assert!(owner.begin_session("reviewer", 300).is_err());
}
