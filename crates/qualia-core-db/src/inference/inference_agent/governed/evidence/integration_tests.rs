//! Independent live-owner, epoch, timing and delivery boundary challenges.
use super::super::*;
use super::test_actor::{actor, bind};
use super::*;
use ed25519_dalek::SigningKey;
use etalon_inference_contract::{Decision, Instrumentation};

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
fn wait_until(deadline: u64) {
    let limit = std::time::Instant::now() + std::time::Duration::from_secs(5);
    while now() < deadline {
        assert!(std::time::Instant::now() < limit, "bounded clock wait");
        std::thread::sleep(std::time::Duration::from_millis(5));
    }
}
fn rules(scope: ActorPolicyScope) -> PolicyRules {
    PolicyRules {
        actors: PolicyList::from_vec(vec![scope]).unwrap(),
        model: DigestScope::Exact([22; 32]),
        request: DigestScope::Exact([11; 32]),
        graph_scopes: PolicyList::from_vec(vec![71]).unwrap(),
        program: PolicyList::from_vec(vec![
            ConstraintInstruction::MatchObject { value: 19 },
            ConstraintInstruction::HaltIfFalse,
        ])
        .unwrap(),
        norms: PolicyList::default(),
        facts: PolicyList::default(),
        epistemic: PolicyList::default(),
        trace: PolicyList::default(),
        required_claim: 0,
        temporal_invariant: 0,
        non_derogable: true,
    }
}
fn observations() -> ObservationRequirements {
    ObservationRequirements {
        allowed_sources: 0x7f,
        allowed_artifact_origins: 7,
        max_payload_bytes: 16384,
        allow_empty_artifact: false,
        score: ScoreRequirement::Optional,
        score_program: PolicyList::default(),
    }
}
fn request() -> InferenceAdmission {
    InferenceAdmission {
        request_digest: [11; 32],
        model_digest: [22; 32],
        model_generation: 4,
        device_id: 5,
        device_generation: 6,
        graph_scope: 71,
        first_position: 20,
        instrumentation: Instrumentation::default(),
    }
}
fn storage(bytes: &mut [u8]) -> EvidenceStorage<'_> {
    let n = bytes.len();
    EvidenceStorage::new(
        bytes,
        EvidenceLimits {
            storage_bytes: n + std::mem::size_of::<EvidenceStorage<'_>>(),
            max_payload_bytes: n,
            max_artifact_events: 2,
        },
    )
    .unwrap()
}
fn host(path: &std::path::Path, key: &SigningKey) -> NativePolicyHost {
    NativePolicyHost::open(
        &path.join("policy"),
        &path.join("anchor"),
        TrustedPolicyIdentity::from_host_configuration(
            key.verifying_key().to_bytes(),
            123,
            INFERENCE_POLICY_AUDIENCE,
            7,
        )
        .unwrap(),
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap()
}
fn install(h: &mut NativePolicyHost, key: &SigningKey, scope: ActorPolicyScope, expiry: u64) {
    h.install_evidence(
        key,
        rules(scope),
        observations(),
        1,
        expiry,
        h.status().generation,
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
}
fn proposal<'a>(
    s: &GovernedEvidenceSession<'_, '_>,
    bytes: &'a [u8],
    sequence: u64,
) -> Proposal<'a> {
    Proposal {
        identity: s.identity(),
        request_digest: [11; 32],
        sequence,
        source: CandidateSource::ModelToken,
        artifact_origin: None,
        payload: Payload::Token {
            token: 27,
            position: 19 + sequence,
            piece: bytes,
        },
        numerical: None,
        observation: crate::NQuin {
            subject: 123,
            predicate: 17,
            object: 19,
            context: 71,
            metadata: 0,
            parity: 123 ^ 17 ^ 19 ^ 71,
        },
        provenance_digest: [33; 32],
    }
}

#[test]
fn pending_ticket_cannot_cross_live_sessions_even_with_identical_payload() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[61; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    install(&mut h, &key, scope, now() + 60);
    let (mut a, mut b) = ([0; 64], [0; 64]);
    let (mut sa, mut sb) = (storage(&mut a), storage(&mut b));
    let mut first = h
        .admit_evidence(
            &key,
            request(),
            POLICY_COLD_BUDGET_BYTES,
            owner.clone(),
            bind(&owner, &handle),
            &mut sa,
        )
        .unwrap();
    let mut second = h
        .admit_evidence(
            &key,
            request(),
            POLICY_COLD_BUDGET_BYTES,
            owner.clone(),
            bind(&owner, &handle),
            &mut sb,
        )
        .unwrap();
    assert_ne!(first.identity(), second.identity());
    let one = first.prepare_token(proposal(&first, b"same", 1)).unwrap();
    let two = second.prepare_token(proposal(&second, b"same", 1)).unwrap();
    assert_eq!(one.decision(), Decision::Allow);
    assert!(second.accept(one).is_err());
    assert!(second.is_terminal());
    assert!(first.accept(two).is_err());
    assert!(first.is_terminal());
}

#[test]
fn every_authority_invalidation_rejects_previously_allowed_ticket() {
    for mutation in 0..5 {
        let dir = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[62; 32]);
        let (owner, handle, scope) = actor(&dir.path().join("actor"));
        let mut h = host(dir.path(), &key);
        install(&mut h, &key, scope, now() + 60);
        let mut bytes = [0; 64];
        let mut slot = storage(&mut bytes);
        let mut s = h
            .admit_evidence(
                &key,
                request(),
                POLICY_COLD_BUDGET_BYTES,
                owner.clone(),
                bind(&owner, &handle),
                &mut slot,
            )
            .unwrap();
        let ticket = s.prepare_token(proposal(&s, b"allowed", 1)).unwrap();
        match mutation {
            0 => h.disconnect(),
            1 => h.reload(POLICY_COLD_BUDGET_BYTES).unwrap(),
            2 => {
                install(&mut h, &key, scope, now() + 60);
            }
            3 => {
                h.revoke(&key, h.status().generation, POLICY_COLD_BUDGET_BYTES)
                    .unwrap();
            }
            _ => owner.disconnect(&handle).unwrap(),
        }
        assert!(s.accept(ticket).is_err(), "mutation {mutation}");
        assert!(s.is_terminal());
    }
}

#[test]
fn expiry_after_actual_bulk_work_refuses_prepare_and_accept() {
    for accepting in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let key = SigningKey::from_bytes(&[63; 32]);
        let (owner, handle, scope) = actor(&dir.path().join("actor"));
        let mut h = host(dir.path(), &key);
        let expiry = now() + 2;
        install(&mut h, &key, scope, expiry);
        let mut bytes = [0; 8193];
        let mut slot = storage(&mut bytes);
        let mut s = h
            .admit_evidence(
                &key,
                request(),
                POLICY_COLD_BUDGET_BYTES,
                owner.clone(),
                bind(&owner, &handle),
                &mut slot,
            )
            .unwrap();
        let data = [b'x'; 8193];
        if accepting {
            let ticket = s.prepare_token(proposal(&s, &data, 1)).unwrap();
            s.after_bulk_once(move || wait_until(expiry));
            assert!(s.accept(ticket).is_err());
        } else {
            s.after_bulk_once(move || wait_until(expiry));
            assert!(s.prepare_token(proposal(&s, &data, 1)).is_err());
        }
        assert!(s.is_terminal());
    }
}

#[test]
fn drain_reenters_current_authority_without_holding_acceptance_locks() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[64; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    install(&mut h, &key, scope, now() + 60);
    let mut bytes = [0; 64];
    let mut slot = storage(&mut bytes);
    let mut s = h
        .admit_evidence(
            &key,
            request(),
            POLICY_COLD_BUDGET_BYTES,
            owner.clone(),
            bind(&owner, &handle),
            &mut slot,
        )
        .unwrap();
    let ticket = s
        .prepare_token(proposal(&s, b"accepted prefix", 1))
        .unwrap();
    let accepted = s.accept(ticket).unwrap();
    // Real reentrant operations would deadlock if either acceptance lock escaped.
    owner.disconnect(&handle).unwrap();
    h.revoke(&key, h.status().generation, POLICY_COLD_BUDGET_BYTES)
        .unwrap();
    assert_eq!(accepted.bytes(), b"accepted prefix");
    accepted.complete(15).unwrap();
    assert!(s.prepare_token(proposal(&s, b"later", 2)).is_err());
    assert!(s.is_terminal());
}

#[test]
fn late_payload_tail_binds_exact_key_and_artifact_event_exhaustion_is_terminal() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[65; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    install(&mut h, &key, scope, now() + 60);
    let mut bytes = [0; 8193];
    let mut slot = storage(&mut bytes);
    let mut s = h
        .admit_evidence(
            &key,
            request(),
            POLICY_COLD_BUDGET_BYTES,
            owner.clone(),
            bind(&owner, &handle),
            &mut slot,
        )
        .unwrap();
    let mut data = [b'a'; 8193];
    let mut digests = Vec::new();
    for sequence in 1..=2 {
        data[8192] = sequence as u8;
        let mut p = proposal(&s, &data, sequence);
        p.source = CandidateSource::TextReplacement;
        p.artifact_origin = Some(ArtifactOrigin::GroundedReplacement);
        p.payload = Payload::Artifact {
            media: ArtifactMedia::Utf8Text,
            bytes: &data,
        };
        let ticket = s.prepare_artifact(p, None).unwrap();
        if let AcceptedKey::Artifact(key) = ticket.key() {
            digests.push(key.output);
        } else {
            panic!("wrong domain");
        }
        let accepted = s.accept(ticket).unwrap();
        assert_eq!(accepted.bytes(), data);
        accepted.complete(data.len()).unwrap();
    }
    assert_ne!(digests[0], digests[1]);
    let mut p = proposal(&s, &data, 3);
    p.source = CandidateSource::TextReplacement;
    p.artifact_origin = Some(ArtifactOrigin::GroundedReplacement);
    p.payload = Payload::Artifact {
        media: ArtifactMedia::Utf8Text,
        bytes: &data,
    };
    assert!(s.prepare_artifact(p, None).is_err());
    assert!(s.is_terminal());
}

#[test]
fn live_graph_clock_preserves_inclusive_deontic_expiry_threshold() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[66; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    let deadline = now() + 1;
    let mut r = rules(scope);
    let norm_predicate = crate::modalities::logic::deontic::OP_FORBID as u64;
    let performed = crate::q_hash("q42:performed");
    r.norms = PolicyList::from_vec(vec![crate::NQuin {
        subject: 123,
        predicate: norm_predicate,
        object: 19,
        context: 71,
        metadata: deadline,
        parity: 123 ^ norm_predicate ^ 19 ^ 71,
    }])
    .unwrap();
    r.facts = PolicyList::from_vec(vec![crate::NQuin {
        subject: 123,
        predicate: performed,
        object: 19,
        context: 71,
        metadata: 0,
        parity: 123 ^ performed ^ 19 ^ 71,
    }])
    .unwrap();
    h.install_evidence(
        &key,
        r,
        observations(),
        1,
        now() + 60,
        0,
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    let record: serde_json::Value = serde_json::from_slice(
        &std::fs::read(dir.path().join("policy/sentinel-inference-policy.json")).unwrap(),
    )
    .unwrap();
    let signed: SignedEvidencePolicy =
        serde_json::from_value(record["active"]["policy"].clone()).unwrap();
    let trusted = TrustedPolicyIdentity::from_host_configuration(
        key.verifying_key().to_bytes(),
        123,
        AUDIENCE_V3,
        7,
    )
    .unwrap();
    let verified = signed.verify(&trusted, deadline).unwrap();
    let mut bytes = [0; 64];
    let mut slot = storage(&mut bytes);
    let mut s = h
        .admit_evidence(
            &key,
            request(),
            POLICY_COLD_BUDGET_BYTES,
            owner.clone(),
            bind(&owner, &handle),
            &mut slot,
        )
        .unwrap();
    let p = proposal(&s, b"clock", 1);
    let bound = BoundProposal::bind(p, 64).unwrap();
    assert_eq!(
        evaluate_constraints(&verified, &bound, scope, deadline).unwrap(),
        Decision::Deny
    );
    assert_eq!(
        evaluate_constraints(&verified, &bound, scope, deadline + 1).unwrap(),
        Decision::Allow
    );
    wait_until(deadline + 1);
    let ticket = s.prepare_token(p).unwrap();
    assert_eq!(ticket.decision(), Decision::Allow);
    s.accept(ticket).unwrap().complete(5).unwrap();
}

#[test]
fn versioned_store_refuses_ambiguous_tags_missing_rules_and_signature_changes() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[67; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    install(&mut h, &key, scope, now() + 60);
    let path = dir.path().join("policy/sentinel-inference-policy.json");
    for change in 0..7 {
        let mut bytes = [0; 64];
        let mut slot = storage(&mut bytes);
        let mut s = h
            .admit_evidence(
                &key,
                request(),
                POLICY_COLD_BUDGET_BYTES,
                owner.clone(),
                bind(&owner, &handle),
                &mut slot,
            )
            .unwrap();
        let ticket = s.prepare_token(proposal(&s, b"pending", 1)).unwrap();
        // Admission increments the durable session serial; retain that exact anchor pair.
        let current = std::fs::read(&path).unwrap();
        let mut value: serde_json::Value = serde_json::from_slice(&current).unwrap();
        match change {
            0 => {
                value.as_object_mut().unwrap().remove("version");
            }
            1 => value["version"] = 99.into(),
            2 => value["version"] = 1.into(),
            3 => value["active"]["policy_version"] = "2".into(),
            4 => {
                value["active"]["policy"]["document"]
                    .as_object_mut()
                    .unwrap()
                    .remove("observations");
            }
            5 => {
                value["active"]["policy"]["signature"][0][0] = (value["active"]["policy"]
                    ["signature"][0][0]
                    .as_u64()
                    .unwrap()
                    ^ 1)
                .into()
            }
            _ => {
                value["signature"][1][31] = (value["signature"][1][31].as_u64().unwrap() ^ 1).into()
            }
        }
        let corrupt = serde_json::to_vec(&value).unwrap();
        std::fs::write(&path, &corrupt).unwrap();
        assert!(
            h.reload(POLICY_COLD_BUDGET_BYTES).is_err(),
            "change {change}"
        );
        assert_eq!(std::fs::read(&path).unwrap(), corrupt);
        assert!(s.accept(ticket).is_err());
        assert!(s.is_terminal());
        drop(s);
        std::fs::write(&path, &current).unwrap();
        h.reload(POLICY_COLD_BUDGET_BYTES).unwrap();
    }
}
