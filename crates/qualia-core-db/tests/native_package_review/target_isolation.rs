use super::*;

#[test]
fn package_review_evidence_contains_only_the_exact_selected_target() {
    let root = tempfile::tempdir().unwrap();
    let owner = open(root.path(), package_snapshot());
    let a = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &a, [7; 32]);
    let first = package_target();
    request(&owner, &a, &first);
    let b = login(&owner, "reviewer", &REVIEWER);
    let gen = generation(&owner, &b);
    owner
        .record_requested_package_review(
            &b,
            gen,
            &first,
            "urn:fixture:request",
            "urn:fixture:prior-decision",
            ReviewDecision::Accepted,
            "scoped software fixture decision",
            "urn:fixture:evidence",
            |_| Ok(()),
        )
        .unwrap();
    let first_evidence = owner
        .package_workflow_evidence(&b, &first, |_| Ok(()))
        .unwrap();
    assert_eq!(first_evidence.contributions.len(), 1);
    assert_eq!(first_evidence.requests.len(), 1);
    assert_eq!(first_evidence.reviews.len(), 1);
    let a = login(&owner, "author", &AUTHOR);
    owner
        .transact_with_package(
            &a,
            Some(&PackageWriteIntent {
                filename: "second.hmc".into(),
                expected_digest: None,
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
    let second = owner
        .saved_package_target(&a, "second.hmc", "urn:fixture:project:release:2")
        .unwrap();
    let second_evidence = owner
        .package_workflow_evidence(&a, &second, |_| Ok(()))
        .unwrap();
    assert_eq!(second_evidence.target, second);
    assert!(second_evidence.contributions.is_empty());
    assert_eq!(second_evidence.prior_contributions.len(), 1);
    assert_eq!(second_evidence.prior_contributions[0].target, first);
    assert_eq!(
        second_evidence.prior_contributions[0].actor.subject,
        "author"
    );
    assert_eq!(
        second_evidence.prior_contributions[0].work_status,
        WorkStatus::Unknown
    );
    assert!(second_evidence.requests.is_empty());
    assert!(second_evidence.reviews.is_empty());
    let mut renamed = first.clone();
    renamed.package_id = "urn:fixture:project:renamed-release:1".into();
    let renamed_evidence = owner
        .package_workflow_evidence(&a, &renamed, |_| Ok(()))
        .unwrap();
    assert!(renamed_evidence.contributions.is_empty());
    assert_eq!(renamed_evidence.prior_contributions.len(), 1);
    assert_eq!(renamed_evidence.prior_contributions[0].target, first);
    assert!(renamed_evidence.requests.is_empty());
    assert!(renamed_evidence.reviews.is_empty());
}
