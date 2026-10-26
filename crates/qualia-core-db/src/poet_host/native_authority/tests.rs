//! Explicit signed fixtures exercise native cryptography, not human enrollment.
use super::*;
#[path = "package_review/tests.rs"]
mod package_review_tests;
#[path = "package_tests.rs"]
mod package_tests;
use qualia_cooperative_core::{
    authoring::*,
    projects::{Project, ProjectMembership, ProjectRole},
};
use std::sync::Arc;

#[path = "session_test_fixture.rs"]
mod session_test_fixture;
use session_test_fixture::*;

#[test]
fn signed_admission_and_one_use_proof_are_both_required() {
    let temp = tempfile::tempdir().unwrap();
    let mut forged = sign(snapshot());
    forged.snapshot.subjects[0].subject = "impostor".into();
    assert!(
        NativeAuthority::open(temp.path(), anchor(), forged, document(), 1024 * 1024, 256).is_err()
    );
    let owner = open(temp.path(), snapshot());
    assert!(owner.begin_session("current_user", 300).is_err());
    let challenge = owner.begin_session("author", 300).unwrap();
    let replay = challenge.clone();
    let mut signature = [0; 64];
    ed25519::sign_with_context(
        &REVIEWER,
        challenge.message_digest(),
        SESSION_CONTEXT,
        &mut signature,
    )
    .unwrap();
    assert!(owner.complete_session(challenge, &signature).is_err());
    assert!(owner.complete_session(replay, &signature).is_err());
    let session = login(&owner, "author", &AUTHOR);
    let generation = generation(&owner, &session);
    assert!(owner.publication_is_current(&session, generation));
    owner.disconnect(&session).unwrap();
    assert!(!owner.publication_is_current(&session, generation));
}
#[test]
fn current_revocation_expiry_and_membership_identifier_are_enforced() {
    for case in 0..5 {
        let temp = tempfile::tempdir().unwrap();
        let mut adopted = snapshot();
        match case {
            0 => adopted.subjects[0].revoked = true,
            1 => adopted.subjects[0].valid_until_unix = now().unwrap() - 1,
            2 => adopted.subjects[0].identifiers[0].value = "did:test:other".into(),
            3 => adopted.subjects[0].identifiers[0].valid_until_unix = now().unwrap() - 1,
            _ => adopted.subjects[0].membership.project_id = "other-project".into(),
        }
        let opened = NativeAuthority::open(
            temp.path(),
            anchor(),
            sign(adopted),
            document(),
            1024 * 1024,
            256,
        );
        if let Ok(owner) = opened {
            if let Ok(challenge) = owner.begin_session("author", 300) {
                let mut signature = [0; 64];
                ed25519::sign_with_context(
                    &AUTHOR,
                    challenge.message_digest(),
                    SESSION_CONTEXT,
                    &mut signature,
                )
                .unwrap();
                if let Ok(session) = owner.complete_session(challenge, &signature) {
                    assert!(owner
                        .transact(&session, |_| DocumentTransaction {
                            result: Ok(()),
                            successor: None,
                            reconciliation_required: false
                        })
                        .is_err());
                }
            }
        }
    }
}
#[test]
fn signed_replacement_invalidates_sessions_and_cannot_replay_old_admission() {
    let temp = tempfile::tempdir().unwrap();
    let owner = open(temp.path(), snapshot());
    let session = login(&owner, "author", &AUTHOR);
    let generation = generation(&owner, &session);
    let mut revoked = snapshot();
    revoked.sequence = 2;
    revoked.subjects[0].revoked = true;
    owner.replace_admission(sign(revoked)).unwrap();
    assert!(!owner.publication_is_current(&session, generation));
    assert!(owner.replace_admission(sign(snapshot())).is_err());
    assert!(owner.begin_session("author", 300).is_err());
}
#[test]
fn real_signed_review_is_distinct_from_author_and_requires_current_grant() {
    let temp = tempfile::tempdir().unwrap();
    let owner = open(temp.path(), snapshot());
    let author = login(&owner, "author", &AUTHOR);
    assert!(owner
        .record_review(&author, "self", ReviewDecision::Accepted, "self", "fixture")
        .is_err());
    let contribution = owner
        .record_contribution(
            &author,
            "contribution-1",
            "Actual fixture work",
            5,
            WorkStatus::Unpaid {
                evidence_ref: "fixture-effort".into(),
            },
        )
        .unwrap();
    assert_eq!(contribution.contributor_subject, "author");
    assert!(owner
        .record_contribution(
            &author,
            "contribution-1",
            "duplicate",
            5,
            WorkStatus::Unknown
        )
        .is_err());
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let review = owner
        .record_review(
            &reviewer,
            "review-1",
            ReviewDecision::ChangesRequested,
            "Fixture needs work",
            "fixture-decision",
        )
        .unwrap();
    assert_eq!(review.principal_kind, PrincipalKind::SoftwareAgent);
    assert_eq!(review.target.source_digest, [1; 32]);
    let mut revoked = snapshot();
    revoked.sequence = 2;
    revoked.subjects[1].reviews[0].revoked = true;
    owner.replace_admission(sign(revoked)).unwrap();
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    assert!(owner
        .record_review(
            &reviewer,
            "review-2",
            ReviewDecision::Accepted,
            "deny",
            "fixture"
        )
        .is_err());
}
#[test]
fn durable_successors_survive_reopen_but_sessions_do_not() {
    let temp = tempfile::tempdir().unwrap();
    let adopted = snapshot();
    let owner = open(temp.path(), adopted.clone());
    let session = login(&owner, "author", &AUTHOR);
    let mut successor = document();
    successor.revision = 2;
    successor.source_digest = [2; 32];
    owner
        .transact(&session, |_| DocumentTransaction {
            result: Ok(()),
            successor: Some(successor.clone()),
            reconciliation_required: false,
        })
        .unwrap();
    drop(owner);
    let reopened = NativeAuthority::open(
        temp.path(),
        anchor(),
        sign(adopted),
        successor,
        1024 * 1024,
        256,
    )
    .unwrap();
    assert!(!reopened.publication_is_current(&session, 2));
    assert!(reopened
        .transact(&session, |_| DocumentTransaction {
            result: Ok(()),
            successor: None,
            reconciliation_required: false
        })
        .is_err());
}
#[test]
fn transactions_are_serialized_and_busy_publication_fails_closed() {
    let temp = tempfile::tempdir().unwrap();
    let owner = Arc::new(open(temp.path(), snapshot()));
    let session = login(&owner, "author", &AUTHOR);
    let generation = generation(&owner, &session);
    let (inside_tx, inside_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let worker_owner = owner.clone();
    let worker_session = session.clone();
    let worker = std::thread::spawn(move || {
        worker_owner.transact(&worker_session, |view| {
            inside_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            let mut next = view.current_document.clone();
            next.revision += 1;
            next.source_digest = [2; 32];
            DocumentTransaction {
                result: Ok(()),
                successor: Some(next),
                reconciliation_required: false,
            }
        })
    });
    inside_rx.recv().unwrap();
    assert!(!owner.publication_is_current(&session, generation));
    release_tx.send(()).unwrap();
    worker.join().unwrap().unwrap();
    owner
        .transact(&session, |view| {
            assert_eq!(view.current_document.revision, 2);
            DocumentTransaction {
                result: Ok(()),
                successor: None,
                reconciliation_required: false,
            }
        })
        .unwrap();
    assert!(owner.publication_is_current(&session, generation));
}
#[test]
fn failed_post_source_commit_latches_reconciliation() {
    let temp = tempfile::tempdir().unwrap();
    let owner = open(temp.path(), snapshot());
    let session = login(&owner, "author", &AUTHOR);
    let generation = generation(&owner, &session);
    std::fs::write(
        temp.path().join("00000000000000000002.json"),
        b"external collision",
    )
    .unwrap();
    let mut next = document();
    next.revision = 2;
    next.source_digest = [2; 32];
    let error = owner
        .transact(&session, |_| DocumentTransaction {
            result: Ok(()),
            successor: Some(next),
            reconciliation_required: false,
        })
        .unwrap_err();
    assert!(error.contains("may have committed"));
    assert!(!owner.publication_is_current(&session, generation));
}

#[test]
fn foreign_owner_sessions_and_expired_pending_proofs_cannot_admit() {
    let first_dir = tempfile::tempdir().unwrap();
    let second_dir = tempfile::tempdir().unwrap();
    let first = open(first_dir.path(), snapshot());
    let second = open(second_dir.path(), snapshot());
    let session = login(&first, "author", &AUTHOR);
    assert!(second
        .transact(&session, |_| DocumentTransaction {
            result: Ok(()),
            successor: None,
            reconciliation_required: false,
        })
        .is_err());
    let challenge = second.begin_session("author", 300).unwrap();
    let mut signature = [0; 64];
    ed25519::sign_with_context(
        &AUTHOR,
        challenge.message_digest(),
        SESSION_CONTEXT,
        &mut signature,
    )
    .unwrap();
    second
        .state
        .lock()
        .unwrap()
        .pending
        .iter_mut()
        .flatten()
        .find(|p| p.challenge.nonce == challenge.nonce)
        .unwrap()
        .expires = now().unwrap() - 1;
    assert!(second.complete_session(challenge, &signature).is_err());
}

#[test]
fn mismatched_review_document_and_expired_session_reject_current_publication() {
    let temp = tempfile::tempdir().unwrap();
    let mut adopted = snapshot();
    adopted.subjects[1].reviews[0].document_id = "different-document".into();
    let owner = open(temp.path(), adopted);
    let session = login(&owner, "reviewer", &REVIEWER);
    let generation = generation(&owner, &session);
    assert!(owner
        .record_review(
            &session,
            "wrong-scope",
            ReviewDecision::Accepted,
            "must deny",
            "fixture"
        )
        .is_err());
    // Test-only private state change simulates wall-clock passage without sleeps.
    // The production grant's own expiry remains independently checked on admission.
    owner
        .state
        .lock()
        .unwrap()
        .sessions
        .iter_mut()
        .flatten()
        .find(|s| s.handle == session)
        .unwrap()
        .expires = now().unwrap() - 1;
    assert!(!owner.publication_is_current(&session, generation));
}

#[test]
fn journal_has_one_native_owner_before_any_authoring_callback() {
    let temp = tempfile::tempdir().unwrap();
    let adopted = snapshot();
    let first = open(temp.path(), adopted.clone());
    assert!(NativeAuthority::open(
        temp.path(),
        anchor(),
        sign(adopted.clone()),
        document(),
        1024 * 1024,
        256
    )
    .is_err());
    drop(first);
    // The OS releases ownership on drop; old serialized/session state grants no
    // authority. This also exercises canonicalized Windows retained-path reads.
    let reopened = NativeAuthority::open(
        temp.path(),
        anchor(),
        sign(adopted),
        document(),
        1024 * 1024,
        256,
    )
    .unwrap();
    let session = login(&reopened, "author", &AUTHOR);
    assert!(reopened.publication_is_current(&session, generation(&reopened, &session)));
}

#[test]
fn weak_identity_keys_cannot_forge_controller_possession_or_project_adoption() {
    let temp = tempfile::tempdir().unwrap();
    // Compressed Edwards identity is nonzero, but has small order. A native
    // possession claim must reject it even if the project signs its descriptor.
    let mut identity = [0; 32];
    identity[0] = 1;
    let mut adopted = snapshot();
    adopted.subjects[0].controller_public_key = identity;
    assert!(NativeAuthority::open(
        temp.path(),
        anchor(),
        sign(adopted),
        document(),
        1024 * 1024,
        256
    )
    .is_err());
    let mut weak_anchor = anchor();
    weak_anchor.public_key = identity;
    assert!(NativeAuthority::open(
        temp.path(),
        weak_anchor,
        sign(snapshot()),
        document(),
        1024 * 1024,
        256
    )
    .is_err());
}

#[path = "workflow_tests.rs"]
mod workflow_tests;

#[path = "concurrent_session_tests.rs"]
mod concurrent_session_tests;
#[path = "session_lifecycle_tests.rs"]
mod session_lifecycle_tests;
