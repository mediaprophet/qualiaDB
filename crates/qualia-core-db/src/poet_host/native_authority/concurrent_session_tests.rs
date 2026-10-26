//! Concurrent software-agent proofs exercise isolation, not human acceptance.
use super::*;

pub(super) fn proof(challenge: &SessionChallenge, seed: &[u8; 32]) -> [u8; 64] {
    let mut signature = [0; 64];
    ed25519::sign_with_context(
        seed,
        challenge.message_digest(),
        SESSION_CONTEXT,
        &mut signature,
    )
    .unwrap();
    signature
}

#[test]
fn simultaneous_outstanding_author_and_reviewer_proofs_keep_distinct_roles() {
    let temp = tempfile::tempdir().unwrap();
    let mut adopted = snapshot();
    adopted.subjects[0].reviews.clear();
    adopted.subjects[1].membership.role = ProjectRole::Observer;
    let owner = Arc::new(open(temp.path(), adopted));
    let author_challenge = owner.begin_session("author", 300).unwrap();
    let reviewer_challenge = owner.begin_session("reviewer", 300).unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let workers: Vec<_> = [(author_challenge, AUTHOR), (reviewer_challenge, REVIEWER)]
        .into_iter()
        .map(|(challenge, seed)| {
            let owner = owner.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                let signature = proof(&challenge, &seed);
                barrier.wait();
                owner.complete_session(challenge, &signature).unwrap()
            })
        })
        .collect();
    barrier.wait();
    let mut workers = workers.into_iter();
    let author = workers.next().unwrap().join().unwrap();
    let reviewer = workers.next().unwrap().join().unwrap();
    let gen = generation(&owner, &author);
    assert_eq!(generation(&owner, &reviewer), gen);
    assert!(owner.publication_is_current(&author, gen));
    assert!(owner.publication_is_current(&reviewer, gen));
    assert_eq!(
        owner.workflow_evidence(&author).unwrap().principal_subject,
        "author"
    );
    assert_eq!(
        owner
            .workflow_evidence(&reviewer)
            .unwrap()
            .principal_subject,
        "reviewer"
    );
    owner
        .record_contribution_at(
            &author,
            &document(),
            "effort",
            "fixture work",
            1,
            WorkStatus::Unknown,
        )
        .unwrap();
    assert!(owner
        .record_contribution_at(
            &reviewer,
            &document(),
            "observer-effort",
            "observer cannot author",
            1,
            WorkStatus::Unknown
        )
        .is_err());
    assert!(owner
        .record_review(
            &author,
            "self",
            ReviewDecision::Accepted,
            "self review denied",
            "fixture/self"
        )
        .is_err());
    let review = owner
        .record_review(
            &reviewer,
            "review",
            ReviewDecision::ChangesRequested,
            "fixture inspection",
            "fixture/review",
        )
        .unwrap();
    assert_eq!(review.reviewer_subject, "reviewer");
    assert!(owner.publication_is_current(&author, gen));
}

#[test]
fn invalid_replayed_foreign_and_tampered_proofs_do_not_consume_other_challenges() {
    let temp = tempfile::tempdir().unwrap();
    let other_temp = tempfile::tempdir().unwrap();
    let owner = open(temp.path(), snapshot());
    let other = open(other_temp.path(), snapshot());
    let author = owner.begin_session("author", 300).unwrap();
    let reviewer = owner.begin_session("reviewer", 300).unwrap();
    let foreign = other.begin_session("author", 300).unwrap();
    assert!(owner
        .complete_session(foreign.clone(), &proof(&foreign, &AUTHOR))
        .is_err());
    assert!(owner.cancel_session_challenge(&foreign).is_err());
    let mut tampered = reviewer.clone();
    tampered.digest[0] ^= 1;
    assert!(owner
        .complete_session(tampered.clone(), &proof(&tampered, &REVIEWER))
        .is_err());
    // A valid signature by the wrong admitted controller consumes only author.
    assert!(owner
        .complete_session(author.clone(), &proof(&author, &REVIEWER))
        .is_err());
    assert!(owner
        .complete_session(author.clone(), &proof(&author, &AUTHOR))
        .is_err());
    let reviewer_session = owner
        .complete_session(reviewer.clone(), &proof(&reviewer, &REVIEWER))
        .unwrap();
    assert!(owner
        .complete_session(reviewer.clone(), &proof(&reviewer, &REVIEWER))
        .is_err());
    assert_eq!(
        owner
            .workflow_evidence(&reviewer_session)
            .unwrap()
            .principal_subject,
        "reviewer"
    );
    // Rejected foreign delivery leaves the issuing owner's proof usable too.
    assert!(other
        .complete_session(foreign.clone(), &proof(&foreign, &AUTHOR))
        .is_ok());
}

#[test]
fn logout_and_challenge_cancellation_are_selective_even_for_same_subject() {
    let temp = tempfile::tempdir().unwrap();
    let owner = open(temp.path(), snapshot());
    let first = login(&owner, "author", &AUTHOR);
    let second = login(&owner, "author", &AUTHOR);
    let reviewer = login(&owner, "reviewer", &REVIEWER);
    let gen = generation(&owner, &first);
    let cancelled = owner.begin_session("author", 300).unwrap();
    let pending = owner.begin_session("reviewer", 300).unwrap();
    owner.cancel_session_challenge(&cancelled).unwrap();
    assert!(owner.cancel_session_challenge(&cancelled).is_err());
    assert!(owner
        .complete_session(cancelled.clone(), &proof(&cancelled, &AUTHOR))
        .is_err());
    owner.disconnect(&first).unwrap();
    assert!(owner.disconnect(&first).is_err());
    assert!(!owner.publication_is_current(&first, gen));
    assert!(owner.publication_is_current(&second, gen));
    assert!(owner.publication_is_current(&reviewer, gen));
    assert!(owner
        .complete_session(pending.clone(), &proof(&pending, &REVIEWER))
        .is_ok());
    assert_eq!(generation(&owner, &second), gen);
}

#[test]
fn pending_and_live_capacity_fail_without_evicting_any_active_actor() {
    let temp = tempfile::tempdir().unwrap();
    let owner = open(temp.path(), snapshot());
    let first = login(&owner, "author", &AUTHOR);
    let gen = generation(&owner, &first);
    let mut pending = Vec::new();
    for _ in 0..MAX_PENDING_SESSION_CHALLENGES {
        pending.push(owner.begin_session("reviewer", 300).unwrap());
    }
    assert!(owner
        .begin_session("author", 300)
        .unwrap_err()
        .contains("capacity"));
    assert!(owner.publication_is_current(&first, gen));
    let challenge = pending.remove(0);
    owner.cancel_session_challenge(&challenge).unwrap();
    let replacement = owner.begin_session("author", 300).unwrap();
    owner.cancel_session_challenge(&replacement).unwrap();
    for challenge in &pending {
        owner.cancel_session_challenge(challenge).unwrap();
    }
    let mut sessions = vec![first.clone()];
    for _ in 1..MAX_NATIVE_SESSIONS {
        sessions.push(login(&owner, "reviewer", &REVIEWER));
    }
    let overflow = owner.begin_session("author", 300).unwrap();
    assert!(owner
        .complete_session(overflow.clone(), &proof(&overflow, &AUTHOR))
        .unwrap_err()
        .contains("capacity"));
    for session in &sessions {
        assert!(owner.publication_is_current(session, gen));
    }
    owner.disconnect(&sessions[1]).unwrap();
    // Capacity refusal consumes its proof; explicit fresh proof is required.
    assert!(owner
        .complete_session(overflow.clone(), &proof(&overflow, &AUTHOR))
        .is_err());
    let replacement = login(&owner, "author", &AUTHOR);
    assert!(owner.publication_is_current(&replacement, gen));
    assert!(owner.publication_is_current(&first, gen));
    // Reusing a slot repeatedly never exhausts QDNF's retained revoked records.
    owner.disconnect(&replacement).unwrap();
    for _ in 0..20 {
        let transient = login(&owner, "reviewer", &REVIEWER);
        owner.disconnect(&transient).unwrap();
        assert!(owner.publication_is_current(&first, gen));
    }
}
