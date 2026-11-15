//! Signed software fixtures. Saved audit callbacks here are not actual HMC IO.
//! Real checkpoint-byte integration belongs to the native Poet owner tests.
use super::super::package_review::*;
use super::*;
#[cfg(feature = "sanctuary-crypto")]
#[path = "admin_tests.rs"]
mod admin_tests;

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
    assert_eq!(legacy.verify(&anchor(), 1024 * 1024).unwrap(), digest);
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
fn package_review_evidence_counts_refuse_before_any_append() {
    let root = tempfile::tempdir().unwrap();
    let owner = open(root.path(), snapshot());
    let author = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &author, [7; 32]);
    let target = package_target();
    let gen = generation(&owner, &author);
    let record = owner
        .record_package_contribution(
            &author,
            gen,
            &target,
            "urn:fixture:c",
            "work",
            1,
            WorkStatus::Unknown,
            |_| Ok(()),
        )
        .unwrap();
    let mut state = owner.state.lock().unwrap();
    let before = std::fs::read_dir(root.path()).unwrap().count();
    let mut next = state.durable.clone();
    next.package_contributions = vec![record; 257];
    assert!(state.journal.append(&next).is_err());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), before);
    assert_eq!(state.durable.package_contributions.len(), 1);
}

#[test]
fn package_review_entry_byte_budget_refuses_before_creating_a_file() {
    let root = tempfile::tempdir().unwrap();
    let owner = open(root.path(), snapshot());
    let author = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &author, [7; 32]);
    let target = package_target();
    let gen = generation(&owner, &author);
    let record = owner
        .record_package_contribution(
            &author,
            gen,
            &target,
            "urn:fixture:c",
            "work",
            1,
            WorkStatus::Unknown,
            |_| Ok(()),
        )
        .unwrap();
    let mut state = owner.state.lock().unwrap();
    let before = std::fs::read_dir(root.path()).unwrap().count();
    let mut next = state.durable.clone();
    next.package_contributions.clear();
    for index in 0..200 {
        let mut item = record.clone();
        item.contribution.id = format!("urn:fixture:{index}:{}", "x".repeat(4000));
        item.contribution.description = "d".repeat(4000);
        next.package_contributions.push(item);
    }
    assert!(state
        .journal
        .append(&next)
        .unwrap_err()
        .contains("byte budget"));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), before);
    assert_eq!(state.durable.package_contributions.len(), 1);
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

#[test]
fn package_review_generation_overflow_never_admits_signed_successor() {
    let root = tempfile::tempdir().unwrap();
    let initial = package_snapshot();
    let owner = open(root.path(), initial.clone());
    let before = std::fs::read_dir(root.path()).unwrap().count();
    {
        let mut state = owner.state.lock().unwrap();
        state.generation = u64::MAX;
    }
    let mut replacement = initial.clone();
    replacement.sequence += 1;
    replacement.package_reviews[0].revoked = true;
    assert!(owner
        .replace_admission(sign(replacement))
        .unwrap_err()
        .contains("generation"));
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), before);
    let state = owner.state.lock().unwrap();
    assert_eq!(state.durable.admission.snapshot.sequence, initial.sequence);
    assert!(!state.durable.admission.snapshot.package_reviews[0].revoked);
}
