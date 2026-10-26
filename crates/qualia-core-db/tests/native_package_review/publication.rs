use super::*;

#[test]
fn package_review_publication_expires_after_same_source_native_package_successor() {
    let root = tempfile::tempdir().unwrap();
    let owner = open(root.path(), package_snapshot());
    let author = login(&owner, "author", &AUTHOR);
    save_audit(&owner, &author, [7; 32]);
    let target = package_target();
    request(&owner, &author, &target);
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let gen = generation(&owner, &reviewer);
    owner
        .record_requested_package_review(
            &reviewer,
            gen,
            &target,
            "urn:fixture:request",
            "urn:fixture:publication-decision",
            ReviewDecision::Accepted,
            "software fixture scoped decision",
            "urn:fixture:evidence",
            |_| Ok(()),
        )
        .unwrap();
    assert!(owner.package_publication_is_current(&reviewer, gen, &target));
    assert!(!owner.package_publication_is_current(&reviewer, gen + 1, &target));
    owner
        .transact_with_package(
            &reviewer,
            Some(&PackageWriteIntent {
                filename: target.filename.clone(),
                expected_digest: Some([7; 32]),
            }),
            |view| {
                assert_eq!(view.current_document, &target.source);
                (
                    DocumentTransaction {
                        result: Ok(()),
                        successor: None,
                        reconciliation_required: false,
                    },
                    Some(PackageWriteReceipt {
                        digest: [8; 32],
                        bytes: 124,
                    }),
                )
            },
        )
        .unwrap();
    assert!(
        owner.publication_is_current(&reviewer, gen),
        "source/session publication context stayed unchanged"
    );
    assert!(
        !owner.package_publication_is_current(&reviewer, gen, &target),
        "old exact saved-package evidence must be withheld"
    );
    let successor = owner
        .saved_package_target(&reviewer, &target.filename, "urn:fixture:project:release:2")
        .unwrap();
    assert!(owner.package_publication_is_current(&reviewer, gen, &successor));
    owner.disconnect(&reviewer).unwrap();
    assert!(!owner.package_publication_is_current(&reviewer, gen, &successor));
}
