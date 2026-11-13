// Copyright (c) 2026 Timothy Charles Holborn.
// SPDX-License-Identifier: CC-BY-NC-ND-4.0
//! Real issuer/session signatures and retained native journal tests.
use super::super::graph::*;
use super::*;
use crate::NQuin;
use std::sync::atomic::AtomicBool;

fn signed_graph(
    admission: &AdmissionSnapshot,
    context: u64,
    rights: GraphRights,
) -> SignedGraphGrant {
    let grant = GraphGrant {
        version: 1,
        project_id: "project".into(),
        subject: "author".into(),
        admission_digest: admission_digest(admission, 1024 * 1024).unwrap(),
        context,
        rights,
        max_quins: 8,
        valid_from_unix: 1,
        valid_until_unix: now().unwrap() + 3600,
    };
    let mut signature = [0; 64];
    ed25519::sign_with_context(
        &AUTHORITY,
        &graph_grant_digest(&grant).unwrap(),
        GRAPH_GRANT_CONTEXT,
        &mut signature,
    )
    .unwrap();
    SignedGraphGrant {
        grant,
        signature: signature.to_vec(),
    }
}
fn rights() -> GraphRights {
    GraphRights {
        read: true,
        replace: true,
        subscribe: true,
    }
}
fn quin(context: u64, object: u64) -> NQuin {
    NQuin {
        subject: 1,
        predicate: 2,
        object,
        context,
        metadata: 0xfedcba9876543210,
        parity: NQuin::calculate_parity(1, 2, object, context, 0xfedcba9876543210),
    }
}

#[test]
fn graph_requires_signed_scope_in_addition_to_authenticated_source() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let signed = signed_graph(&admission, 99, rights());
    let mut forged = signed.clone();
    forged.grant.context = 100;
    assert!(matches!(
        owner.bind_graph_scope(&session, &forged),
        Err(GraphError::Signature)
    ));
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    assert!(matches!(
        owner.bind_graph_scope(&reviewer, &signed),
        Err(GraphError::Authentication)
    ));
    let scope = owner.bind_graph_scope(&session, &signed).unwrap();
    owner.disconnect(&session).unwrap();
    assert_eq!(
        owner.read_graph(&scope, &mut []),
        Err(GraphError::Authentication)
    );
}

#[test]
fn graph_write_preserves_all_lanes_reopens_and_rejects_stale_session() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let signed = signed_graph(&admission, 99, rights());
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let scope = owner.bind_graph_scope(&session, &signed).unwrap();
    let data = [quin(99, 3), quin(99, 4)];
    let receipt = owner
        .replace_graph(&scope, 0, &data, &AtomicBool::new(false))
        .unwrap();
    assert_eq!(receipt.revision, 1);
    let mut output = [NQuin::default(); 8];
    assert_eq!(owner.read_graph(&scope, &mut output).unwrap().written, 2);
    assert_eq!(&output[..2], &data);
    drop(owner);
    let owner = open(temp.path(), admission);
    assert!(owner.bind_graph_scope(&session, &signed).is_err());
    assert_eq!(
        owner.read_graph(&scope, &mut output),
        Err(GraphError::StaleScope)
    );
    let session = login(&owner, "author", &AUTHOR);
    let scope = owner.bind_graph_scope(&session, &signed).unwrap();
    assert_eq!(owner.read_graph(&scope, &mut output).unwrap().revision, 1);
    assert_eq!(&output[..2], &data);
}

#[test]
fn graph_context_revision_permissions_and_cancellation_are_enforced() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let scope = owner
        .bind_graph_scope(&session, &signed_graph(&admission, 99, rights()))
        .unwrap();
    let cancelled = AtomicBool::new(false);
    assert_eq!(
        owner.replace_graph(&scope, 0, &[quin(100, 3)], &cancelled),
        Err(GraphError::Capacity)
    );
    assert_eq!(
        owner.replace_graph(&scope, 0, &[quin(99, 3)], &AtomicBool::new(true)),
        Err(GraphError::Cancelled)
    );
    owner
        .replace_graph(&scope, 0, &[quin(99, 3)], &cancelled)
        .unwrap();
    assert_eq!(
        owner.replace_graph(&scope, 0, &[quin(99, 4)], &cancelled),
        Err(GraphError::RevisionConflict)
    );
    let read_only = GraphRights {
        read: true,
        replace: false,
        subscribe: false,
    };
    let scope = owner
        .bind_graph_scope(&session, &signed_graph(&admission, 99, read_only))
        .unwrap();
    assert_eq!(
        owner.replace_graph(&scope, 1, &[], &cancelled),
        Err(GraphError::Permission)
    );
    assert!(matches!(
        owner.subscribe_graph(scope),
        Err(GraphError::Permission)
    ));
}

#[test]
fn graph_subscription_coalesces_reconciles_and_rechecks_revocation() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let grant = signed_graph(&admission, 99, rights());
    let scope = owner.bind_graph_scope(&session, &grant).unwrap();
    let mut subscription = owner
        .subscribe_graph(owner.bind_graph_scope(&session, &grant).unwrap())
        .unwrap();
    let mut output = [NQuin::default(); 8];
    assert_eq!(
        owner
            .poll_graph(&mut subscription, &mut output)
            .unwrap()
            .unwrap()
            .revision,
        0
    );
    assert!(owner
        .poll_graph(&mut subscription, &mut output)
        .unwrap()
        .is_none());
    owner
        .replace_graph(&scope, 0, &[quin(99, 3)], &AtomicBool::new(false))
        .unwrap();
    owner
        .replace_graph(&scope, 1, &[quin(99, 4)], &AtomicBool::new(false))
        .unwrap();
    assert_eq!(
        owner.poll_graph(&mut subscription, &mut []).unwrap_err(),
        GraphError::Capacity
    );
    assert_eq!(
        owner
            .poll_graph(&mut subscription, &mut output)
            .unwrap()
            .unwrap()
            .revision,
        2
    );
    assert_eq!(output[0], quin(99, 4));
    let mut replacement = admission;
    replacement.sequence += 1;
    owner.replace_admission(sign(replacement)).unwrap();
    assert_eq!(
        owner.poll_graph(&mut subscription, &mut output),
        Err(GraphError::StaleScope)
    );
    subscription.close();
    assert_eq!(
        owner.poll_graph(&mut subscription, &mut output),
        Err(GraphError::Cancelled)
    );
}

#[test]
fn graph_journal_exhaustion_withholds_publication_and_latches_reconciliation() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let owner = NativeAuthority::open(
        temp.path(),
        anchor(),
        sign(admission.clone()),
        document(),
        1024 * 1024,
        1,
    )
    .unwrap();
    let session = login(&owner, "author", &AUTHOR);
    let scope = owner
        .bind_graph_scope(&session, &signed_graph(&admission, 99, rights()))
        .unwrap();
    assert_eq!(
        owner.replace_graph(&scope, 0, &[quin(99, 3)], &AtomicBool::new(false)),
        Err(GraphError::Journal(
            "authority journal entry budget exhausted".into()
        ))
    );
    let sentinel = quin(777, 9);
    let mut output = [sentinel];
    assert_eq!(
        owner.read_graph(&scope, &mut output),
        Err(GraphError::Authentication)
    );
    assert_eq!(output, [sentinel]);
    drop(owner);
    let reopened = open(temp.path(), admission.clone());
    let session = login(&reopened, "author", &AUTHOR);
    let scope = reopened
        .bind_graph_scope(&session, &signed_graph(&admission, 99, rights()))
        .unwrap();
    assert_eq!(
        reopened.read_graph(&scope, &mut output).unwrap().revision,
        0
    );
}

#[test]
fn graph_ui_reads_do_not_wait_on_native_authority_transactions() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let scope = owner
        .bind_graph_scope(&session, &signed_graph(&admission, 99, rights()))
        .unwrap();
    owner
        .transact(&session, |_| {
            let mut output = [quin(77, 1)];
            assert_eq!(owner.read_graph(&scope, &mut output), Err(GraphError::Busy));
            assert_eq!(output, [quin(77, 1)]);
            DocumentTransaction {
                result: Ok(()),
                successor: None,
                reconciliation_required: false,
            }
        })
        .unwrap();
    assert_eq!(owner.read_graph(&scope, &mut []).unwrap().revision, 0);
}

#[test]
fn graph_rejects_corrupt_quin_without_mutating_revision() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let scope = owner
        .bind_graph_scope(&session, &signed_graph(&admission, 99, rights()))
        .unwrap();
    let mut bad = quin(99, 3);
    bad.parity ^= 1;
    assert_eq!(
        owner.replace_graph(&scope, 0, &[bad], &AtomicBool::new(false)),
        Err(GraphError::InvalidQuin)
    );
    assert_eq!(owner.read_graph(&scope, &mut []).unwrap().revision, 0);
}

#[test]
fn graph_journal_reopen_verifies_retained_grant_signature() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let scope = owner
        .bind_graph_scope(&session, &signed_graph(&admission, 99, rights()))
        .unwrap();
    owner
        .replace_graph(&scope, 0, &[quin(99, 3)], &AtomicBool::new(false))
        .unwrap();
    drop(owner);
    let entry = temp.path().join("00000000000000000002.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&entry).unwrap()).unwrap();
    let byte = json["state"]["graphs"][0]["receipt"]["authorization"]["signature"][0]
        .as_u64()
        .unwrap();
    json["state"]["graphs"][0]["receipt"]["authorization"]["signature"][0] =
        serde_json::json!(byte ^ 1);
    std::fs::write(&entry, serde_json::to_vec(&json).unwrap()).unwrap();
    let result = NativeAuthority::open(
        temp.path(),
        anchor(),
        sign(admission),
        document(),
        1024 * 1024,
        256,
    );
    assert!(matches!(result, Err(error) if error.contains("graph grant signature")));
}

#[test]
fn graph_expired_signed_grant_and_row_bounds_fail_closed() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let mut expired = signed_graph(&admission, 99, rights());
    expired.grant.valid_until_unix = now().unwrap() - 1;
    let mut signature = [0; 64];
    ed25519::sign_with_context(
        &AUTHORITY,
        &graph_grant_digest(&expired.grant).unwrap(),
        GRAPH_GRANT_CONTEXT,
        &mut signature,
    )
    .unwrap();
    expired.signature = signature.to_vec();
    assert!(matches!(
        owner.bind_graph_scope(&session, &expired),
        Err(GraphError::Authentication)
    ));
    let scope = owner
        .bind_graph_scope(&session, &signed_graph(&admission, 99, rights()))
        .unwrap();
    assert_eq!(
        owner.replace_graph(&scope, 0, &[quin(99, 3); 9], &AtomicBool::new(false)),
        Err(GraphError::Capacity)
    );
    assert_eq!(owner.read_graph(&scope, &mut []).unwrap().revision, 0);
}

#[test]
fn graph_history_retains_old_grant_after_signed_admission_replacement() {
    let temp = tempfile::tempdir().unwrap();
    let mut admission = snapshot();
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let old_grant = signed_graph(&admission, 99, rights());
    let scope = owner.bind_graph_scope(&session, &old_grant).unwrap();
    owner
        .replace_graph(&scope, 0, &[quin(99, 3)], &AtomicBool::new(false))
        .unwrap();
    admission.sequence += 1;
    owner.replace_admission(sign(admission.clone())).unwrap();
    drop(owner);
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    assert!(matches!(
        owner.bind_graph_scope(&session, &old_grant),
        Err(GraphError::StaleScope)
    ));
    let scope = owner
        .bind_graph_scope(&session, &signed_graph(&admission, 99, rights()))
        .unwrap();
    let mut output = [NQuin::default(); 8];
    assert_eq!(owner.read_graph(&scope, &mut output).unwrap().revision, 1);
    assert_eq!(output[0], quin(99, 3));
}

#[test]
fn graph_journal_rejects_skipped_revision_even_with_valid_retained_grant() {
    let temp = tempfile::tempdir().unwrap();
    let admission = snapshot();
    let owner = open(temp.path(), admission.clone());
    let session = login(&owner, "author", &AUTHOR);
    let scope = owner
        .bind_graph_scope(&session, &signed_graph(&admission, 99, rights()))
        .unwrap();
    owner
        .replace_graph(&scope, 0, &[quin(99, 3)], &AtomicBool::new(false))
        .unwrap();
    drop(owner);
    let entry = temp.path().join("00000000000000000002.json");
    let mut json: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&entry).unwrap()).unwrap();
    json["state"]["graphs"][0]["receipt"]["previous_revision"] = serde_json::json!(2);
    json["state"]["graphs"][0]["receipt"]["revision"] = serde_json::json!(3);
    std::fs::write(&entry, serde_json::to_vec(&json).unwrap()).unwrap();
    let result = NativeAuthority::open(
        temp.path(),
        anchor(),
        sign(admission),
        document(),
        1024 * 1024,
        256,
    );
    assert!(matches!(result, Err(error) if error.contains("transition mismatch")));
}
