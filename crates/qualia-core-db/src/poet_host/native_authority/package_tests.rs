use super::*;

#[test]
fn exhausted_package_audit_budget_denies_before_the_file_callback() {
    let root = tempfile::tempdir().unwrap();
    let owner = NativeAuthority::open(
        root.path(),
        anchor(),
        sign(snapshot()),
        document(),
        1024 * 1024,
        1,
    )
    .unwrap();
    let session = login(&owner, "author", &AUTHOR);
    let mut called = false;
    let intent = PackageWriteIntent {
        filename: "working.hmc".into(),
        expected_digest: None,
    };
    let result = owner.transact_with_package(&session, Some(&intent), |_| {
        called = true;
        (
            DocumentTransaction {
                result: Ok(()),
                successor: None,
                reconciliation_required: false,
            },
            Some(PackageWriteReceipt {
                digest: [3; 32],
                bytes: 1,
            }),
        )
    });
    assert!(result.unwrap_err().contains("before package write"));
    assert!(!called);
}
