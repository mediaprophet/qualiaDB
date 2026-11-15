use super::*;

#[test]
fn retained_copy_survives_caller_rewrite_and_failed_drain_is_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[45; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    h.install_evidence(
        &key,
        rules(scope),
        observations(),
        1,
        4_000_000_000,
        0,
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    for mode in 0..3 {
        let mut bytes = [0; 64];
        let mut storage = storage(&mut bytes);
        let mut s = h
            .admit_evidence(
                &key,
                request(),
                POLICY_COLD_BUDGET_BYTES,
                owner.clone(),
                bind(&owner, &handle),
                &mut storage,
            )
            .unwrap();
        let mut caller = *b"abc";
        let ticket = s.prepare_token(proposal(&s, &caller, 1, 7)).unwrap();
        caller.fill(b'z');
        let accepted = s.accept(ticket).unwrap();
        assert_eq!(accepted.bytes(), b"abc");
        match mode {
            0 => {
                assert!(accepted.complete(2).is_err());
            }
            1 => drop(accepted),
            _ => {
                assert!(
                    std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                        let _accepted = accepted;
                        panic!("delivery interrupted");
                    }))
                    .is_err()
                );
            }
        }
        assert!(s.is_terminal());
        assert!(s.prepare_token(proposal(&s, b"new", 2, 7)).is_err());
    }
}

#[test]
fn signed_missing_score_rules_refuse_before_any_record_and_probabilities_are_not_logits() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[46; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    let mut r = observations();
    r.score = ScoreRequirement::Required(ScoreMeaning::Probability);
    r.score_program = PolicyList::from_vec(vec![ConstraintInstruction::GreaterThan {
        vector: 2,
        value: 0.25,
    }])
    .unwrap();
    h.install_evidence(
        &key,
        rules(scope),
        r,
        1,
        4_000_000_000,
        0,
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    for score in [
        None,
        Some(NumericalObservation {
            meaning: ScoreMeaning::Logit,
            value: 0.5,
        }),
        Some(NumericalObservation {
            meaning: ScoreMeaning::Probability,
            value: 1.1,
        }),
        Some(NumericalObservation {
            meaning: ScoreMeaning::Probability,
            value: 0.5,
        }),
    ] {
        let mut bytes = [0; 64];
        let mut storage = storage(&mut bytes);
        let mut s = h
            .admit_evidence(
                &key,
                request(),
                POLICY_COLD_BUDGET_BYTES,
                owner.clone(),
                bind(&owner, &handle),
                &mut storage,
            )
            .unwrap();
        let mut p = proposal(&s, b"p", 1, 7);
        p.numerical = score;
        if score.is_some_and(|s| s.meaning == ScoreMeaning::Probability && s.value == 0.5) {
            let ticket = s.prepare_token(p).unwrap();
            s.accept(ticket).unwrap().complete(1).unwrap();
        } else {
            assert!(s.prepare_token(p).is_err());
            assert!(s.is_terminal());
        }
    }
}

#[test]
fn budgets_count_full_capacity_and_output_slot_never_authorizes_absent_bytes() {
    let mut bytes = [0; 128];
    let charged = bytes.len() + std::mem::size_of::<EvidenceStorage<'_>>();
    assert!(matches!(
        EvidenceStorage::new(
            &mut bytes,
            EvidenceLimits {
                storage_bytes: charged - 1,
                max_payload_bytes: 1,
                max_artifact_events: 1
            }
        ),
        Err(PolicyError::Budget)
    ));
    let mut storage = EvidenceStorage::new(
        &mut bytes,
        EvidenceLimits {
            storage_bytes: charged,
            max_payload_bytes: 1,
            max_artifact_events: 1,
        },
    )
    .unwrap();
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[47; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    h.install_evidence(
        &key,
        rules(scope),
        observations(),
        1,
        4_000_000_000,
        0,
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    let mut s = h
        .admit_evidence(
            &key,
            request(),
            POLICY_COLD_BUDGET_BYTES,
            owner.clone(),
            bind(&owner, &handle),
            &mut storage,
        )
        .unwrap();
    assert!(matches!(
        s.prepare_token(proposal(&s, b"too long", 1, 7)),
        Err(PolicyError::Budget)
    ));
    assert!(s.is_terminal());
}

#[test]
fn token_and_artifact_real_gate_hot_paths_have_no_allocations_with_positive_control() {
    use crate::specialized_libs::computational_geometry::allocation_counter::{
        assert_zero_alloc, AllocGuard,
    };
    let counter = AllocGuard::begin("evidence positive control", true);
    let allocation = vec![7u8; 1024];
    std::hint::black_box(&allocation);
    assert!(counter.check().is_err());
    drop(allocation);
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[48; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    h.install_evidence(
        &key,
        rules(scope),
        observations(),
        1,
        4_000_000_000,
        0,
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    let mut bytes = [0; 8192];
    let mut storage = storage(&mut bytes);
    let mut s = h
        .admit_evidence(
            &key,
            request(),
            POLICY_COLD_BUDGET_BYTES,
            owner.clone(),
            bind(&owner, &handle),
            &mut storage,
        )
        .unwrap();
    let payload = [b'x'; 8192];
    assert_zero_alloc("actual evidence success and deny", || {
        let ticket = s.prepare_token(proposal(&s, b"x", 1, 7)).unwrap();
        s.accept(ticket).unwrap().complete(1).unwrap();
        let mut p = proposal(&s, &payload, 1, 7);
        p.source = CandidateSource::TextReplacement;
        p.artifact_origin = Some(ArtifactOrigin::RenderedCompletion);
        p.payload = Payload::Artifact {
            media: ArtifactMedia::Utf8Text,
            bytes: &payload,
        };
        let ticket = s.prepare_artifact(p, None).unwrap();
        s.accept(ticket).unwrap().complete(payload.len()).unwrap();
        let denied = s.prepare_token(proposal(&s, b"deny", 2, 8)).unwrap();
        assert_eq!(denied.decision(), Decision::Deny);
        assert!(s.accept(denied).is_err());
    });
}
