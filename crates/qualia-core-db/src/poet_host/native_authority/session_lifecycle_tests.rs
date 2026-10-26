use super::concurrent_session_tests::proof;
use super::*;

#[test]
fn session_and_pending_expiry_are_independent_and_reclaim_only_expired_slots() {
    let temp = tempfile::tempdir().unwrap();
    let owner = open(temp.path(), snapshot());
    let short = owner.begin_session("author", 60).unwrap();
    let author = owner
        .complete_session(short.clone(), &proof(&short, &AUTHOR))
        .unwrap();
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let expiring = owner.begin_session("author", 60).unwrap();
    let retained = owner.begin_session("reviewer", 300).unwrap();
    let mut state = owner.state.lock().unwrap();
    let deadline = state.session(&author).unwrap().expires;
    assert!(owner
        .view(&state, &author, deadline - 1, |_| Ok(()))
        .is_ok());
    assert!(owner.view(&state, &author, deadline, |_| Ok(())).is_err());
    assert!(owner.view(&state, &reviewer, deadline, |_| Ok(())).is_ok());
    let session = state.session(&author).unwrap();
    assert!(session
        .grants
        .issue_permit(session.grant, session.binding, deadline, 1)
        .is_err());
    // Make the pending expiry equal to this deterministic simulated time edge.
    state
        .pending
        .iter_mut()
        .flatten()
        .find(|p| p.challenge.nonce == expiring.nonce)
        .unwrap()
        .expires = deadline;
    state.expire_sessions(deadline).unwrap();
    assert!(state.session(&author).is_err());
    assert!(state.session(&reviewer).is_ok());
    assert!(state
        .pending
        .iter()
        .flatten()
        .any(|p| p.challenge.nonce == retained.nonce));
    assert!(!state
        .pending
        .iter()
        .flatten()
        .any(|p| p.challenge.nonce == expiring.nonce));
    drop(state);
    assert!(owner
        .complete_session(expiring.clone(), &proof(&expiring, &AUTHOR))
        .is_err());
    assert!(owner
        .complete_session(retained.clone(), &proof(&retained, &REVIEWER))
        .is_ok());
}

#[test]
fn only_durable_signed_admission_successors_invalidate_every_session_and_proof() {
    let temp = tempfile::tempdir().unwrap();
    let owner = open(temp.path(), snapshot());
    let author = login(&owner, "author", &AUTHOR);
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let pending = owner.begin_session("reviewer", 300).unwrap();
    let gen = generation(&owner, &author);
    let mut replacement = snapshot();
    replacement.sequence = 2;
    replacement.subjects[0].revoked = true;
    let mut forged = sign(replacement.clone());
    forged.signature[0] ^= 1;
    assert!(owner.replace_admission(forged).is_err());
    assert!(owner.publication_is_current(&author, gen));
    assert!(owner.publication_is_current(&reviewer, gen));
    assert!(owner
        .state
        .lock()
        .unwrap()
        .pending
        .iter()
        .flatten()
        .any(|p| p.challenge.nonce == pending.nonce));
    owner.replace_admission(sign(replacement)).unwrap();
    assert!(!owner.publication_is_current(&author, gen));
    assert!(!owner.publication_is_current(&reviewer, gen));
    assert!(owner
        .complete_session(pending.clone(), &proof(&pending, &REVIEWER))
        .is_err());
    assert!(owner.begin_session("author", 300).is_err());
    let fresh = login(&owner, "reviewer", &REVIEWER);
    assert_eq!(generation(&owner, &fresh), gen + 1);
    assert!(!owner.publication_is_current(&fresh, gen));
}

#[test]
fn document_successor_stales_exact_read_review_and_package_but_keeps_live_sessions() {
    let temp = tempfile::tempdir().unwrap();
    let owner = open(temp.path(), snapshot());
    let author = login(&owner, "author", &AUTHOR);
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let gen = generation(&owner, &author);
    let before = document();
    owner
        .request_source_review(&author, &before, "request", "fixture/request")
        .unwrap();
    let intent = PackageWriteIntent {
        filename: "working.hmc".into(),
        expected_digest: None,
    };
    owner
        .transact_with_package(&author, Some(&intent), |_| {
            (
                DocumentTransaction {
                    result: Ok(()),
                    successor: None,
                    reconciliation_required: false,
                },
                Some(PackageWriteReceipt {
                    digest: [9; 32],
                    bytes: 10,
                }),
            )
        })
        .unwrap();
    let package = owner
        .saved_package_target(&author, "working.hmc", "fixture/package")
        .unwrap();
    assert!(owner.package_publication_is_current(&author, gen, &package));
    assert!(owner.source_publication_is_current(&reviewer, gen, &before));
    let mut after = before.clone();
    after.revision += 1;
    after.source_digest = [2; 32];
    owner
        .transact(&author, |view| {
            assert_eq!(view.principal.unwrap().subject, "author");
            DocumentTransaction {
                result: Ok(()),
                successor: Some(after.clone()),
                reconciliation_required: false,
            }
        })
        .unwrap();
    assert!(!owner.source_publication_is_current(&reviewer, gen, &before));
    assert!(!owner.package_publication_is_current(&author, gen, &package));
    assert!(owner.source_publication_is_current(&reviewer, gen, &after));
    assert!(owner.publication_is_current(&author, gen));
    assert!(owner.publication_is_current(&reviewer, gen));
    let mut dispatched = false;
    assert!(owner
        .transact(&reviewer, |view| {
            let result = authorize(
                view,
                &AuthoringRequest {
                    action: AuthoringAction::ReadSource,
                    target: &before,
                    expected_authority_generation: gen,
                },
            )
            .map(|_| {
                dispatched = true;
            })
            .map_err(|e| e.to_string());
            DocumentTransaction {
                result,
                successor: None,
                reconciliation_required: false,
            }
        })
        .is_err());
    assert!(!dispatched);
    assert!(owner
        .record_requested_source_review(
            &reviewer,
            &before,
            "request",
            "stale",
            ReviewDecision::Accepted,
            "stale denied",
            "fixture/stale"
        )
        .is_err());
    assert_eq!(owner.workflow_evidence(&reviewer).unwrap().target, after);
    let state = owner.state.lock().unwrap();
    assert_eq!(state.durable.packages[0].actor_subject, "author");
    assert!(state.durable.reviews.is_empty());
    assert!(!state.reconciliation);
}

#[test]
fn wrong_committed_actor_or_source_conflict_latches_reconciliation_for_all_sessions() {
    for wrong_actor in [true, false] {
        let temp = tempfile::tempdir().unwrap();
        let owner = open(temp.path(), snapshot());
        let author = login(&owner, "author", &AUTHOR);
        let reviewer = login(&owner, "reviewer", &REVIEWER);
        let gen = generation(&owner, &author);
        let mut after = document();
        after.revision += 1;
        after.source_digest = [2; 32];
        after.author_subject = Some("reviewer".into());
        assert!(owner
            .transact(&author, |_| DocumentTransaction {
                result: Ok(()),
                successor: wrong_actor.then_some(after),
                reconciliation_required: !wrong_actor,
            })
            .is_err());
        assert!(!owner.publication_is_current(&author, gen));
        assert!(!owner.publication_is_current(&reviewer, gen));
        assert!(owner.workflow_evidence(&reviewer).is_err());
        assert!(owner.begin_session("author", 300).is_err());
        assert_eq!(owner.state.lock().unwrap().durable.document, document());
    }
}

#[test]
fn concurrent_reviewer_waits_for_author_commit_then_refuses_old_revision() {
    let temp = tempfile::tempdir().unwrap();
    let owner = Arc::new(open(temp.path(), snapshot()));
    let author = login(&owner, "author", &AUTHOR);
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    owner
        .request_source_review(&author, &document(), "request", "fixture/request")
        .unwrap();
    let gen = generation(&owner, &reviewer);
    let (entered_tx, entered_rx) = std::sync::mpsc::channel();
    let (release_tx, release_rx) = std::sync::mpsc::channel();
    let writing_owner = owner.clone();
    let writer = std::thread::spawn(move || {
        writing_owner.transact(&author, |view| {
            entered_tx.send(()).unwrap();
            release_rx
                .recv_timeout(std::time::Duration::from_secs(10))
                .unwrap();
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
    entered_rx
        .recv_timeout(std::time::Duration::from_secs(10))
        .unwrap();
    // A busy authority cannot publish either actor's delayed source.
    assert!(!owner.publication_is_current(&reviewer, gen));
    let reading_owner = owner.clone();
    let reading_session = reviewer.clone();
    let reader = std::thread::spawn(move || {
        reading_owner.record_requested_source_review(
            &reading_session,
            &document(),
            "request",
            "stale-review",
            ReviewDecision::Accepted,
            "stale denied",
            "fixture/stale",
        )
    });
    release_tx.send(()).unwrap();
    writer.join().unwrap().unwrap();
    assert!(reader.join().unwrap().is_err());
    let evidence = owner.workflow_evidence(&reviewer).unwrap();
    assert_eq!(evidence.target.revision, 2);
    assert!(evidence.reviews.is_empty());
    assert!(owner.publication_is_current(&reviewer, gen));
}
