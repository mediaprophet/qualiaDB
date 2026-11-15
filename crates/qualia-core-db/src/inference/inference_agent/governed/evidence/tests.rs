use super::super::*;
use super::test_actor::{actor, bind};
use super::*;
use ed25519_dalek::SigningKey;
use etalon_inference_contract::{Decision, Instrumentation};
use std::sync::Arc;
#[path = "lifecycle_tests.rs"]
mod lifecycle;

fn rules(scope: ActorPolicyScope) -> PolicyRules {
    PolicyRules {
        actors: PolicyList::from_vec(vec![scope]).unwrap(),
        model: DigestScope::Any,
        request: DigestScope::Any,
        graph_scopes: PolicyList::from_vec(vec![44]).unwrap(),
        program: PolicyList::from_vec(vec![ConstraintInstruction::MatchObject { value: 7 }])
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
        max_payload_bytes: 65536,
        allow_empty_artifact: false,
        score: ScoreRequirement::Optional,
        score_program: PolicyList::default(),
    }
}
fn request() -> InferenceAdmission {
    InferenceAdmission {
        request_digest: [1; 32],
        model_digest: [2; 32],
        model_generation: 1,
        device_id: 1,
        device_generation: 1,
        graph_scope: 44,
        first_position: 10,
        instrumentation: Instrumentation::default(),
    }
}
fn trust(key: &SigningKey) -> TrustedPolicyIdentity {
    TrustedPolicyIdentity::from_host_configuration(
        key.verifying_key().to_bytes(),
        9,
        INFERENCE_POLICY_AUDIENCE,
        1,
    )
    .unwrap()
}
fn storage(bytes: &mut [u8]) -> EvidenceStorage<'_> {
    let n = bytes.len();
    EvidenceStorage::new(
        bytes,
        EvidenceLimits {
            storage_bytes: n + 4096,
            max_payload_bytes: n,
            max_artifact_events: 10,
        },
    )
    .unwrap()
}
fn proposal<'a>(
    session: &GovernedEvidenceSession<'_, '_>,
    piece: &'a [u8],
    sequence: u64,
    value: u64,
) -> Proposal<'a> {
    Proposal {
        identity: session.identity(),
        request_digest: [1; 32],
        sequence,
        source: CandidateSource::ModelToken,
        artifact_origin: None,
        payload: Payload::Token {
            token: 7,
            position: 9 + sequence,
            piece,
        },
        numerical: None,
        observation: crate::NQuin {
            subject: 9,
            predicate: 12,
            object: value,
            context: 44,
            metadata: 0,
            parity: 9 ^ 12 ^ value ^ 44,
        },
        provenance_digest: [3; 32],
    }
}
fn host(dir: &std::path::Path, key: &SigningKey) -> NativePolicyHost {
    NativePolicyHost::open(
        &dir.join("policy"),
        &dir.join("authority"),
        trust(key),
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap()
}
#[test]
fn explicit_version_install_preserves_legacy_until_requested_and_reloads_v3() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[41; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut h = host(dir.path(), &key);
    h.install(
        &key,
        rules(scope),
        1,
        4_000_000_000,
        2,
        0,
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    let path = dir.path().join("policy/sentinel-inference-policy.json");
    let old = std::fs::read(&path).unwrap();
    h.reload(POLICY_COLD_BUDGET_BYTES).unwrap();
    assert_eq!(old, std::fs::read(&path).unwrap());
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&old).unwrap()["version"],
        1
    );
    let mut bytes = [0; 64];
    let mut s = storage(&mut bytes);
    assert!(matches!(
        h.admit_evidence(
            &key,
            request(),
            POLICY_COLD_BUDGET_BYTES,
            Arc::clone(&owner),
            bind(&owner, &handle),
            &mut s
        ),
        Err(PolicyError::PolicyVersionRequired)
    ));
    h.install_evidence(
        &key,
        rules(scope),
        observations(),
        1,
        4_000_000_000,
        1,
        POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
    let installed = std::fs::read(&path).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&installed).unwrap();
    assert_eq!(json["version"], 2);
    assert_eq!(json["active"]["policy_version"], "3");
    assert!(matches!(
        h.admit(
            &key,
            request(),
            POLICY_COLD_BUDGET_BYTES,
            Arc::clone(&owner),
            bind(&owner, &handle)
        ),
        Err(PolicyError::PolicyVersionRequired)
    ));
    let expected = h.status().digest;
    drop(h);
    let mut h = host(dir.path(), &key);
    assert_eq!(expected, h.status().digest);
    assert_eq!(installed, std::fs::read(&path).unwrap());
    std::fs::write(&path, &old).unwrap();
    assert!(h.reload(POLICY_COLD_BUDGET_BYTES).is_err());
}
#[test]
fn token_artifact_share_slot_exact_large_bytes_and_real_vm() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[42; 32]);
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
    let p = proposal(&s, b"tok", 1, 7);
    let ticket = s.prepare_token(p).unwrap();
    assert_eq!(ticket.decision(), Decision::Allow);
    let accepted = s.accept(ticket).unwrap();
    assert_eq!(accepted.bytes(), b"tok");
    accepted.complete(3).unwrap();
    let payload = vec![b'x'; 8192];
    let mut p = proposal(&s, &payload, 1, 7);
    p.source = CandidateSource::TextReplacement;
    p.artifact_origin = Some(ArtifactOrigin::GroundedReplacement);
    p.payload = Payload::Artifact {
        media: ArtifactMedia::Utf8Text,
        bytes: &payload,
    };
    let ticket = s.prepare_artifact(p, None).unwrap();
    let accepted = s.accept(ticket).unwrap();
    assert_eq!(accepted.bytes(), payload);
    assert_eq!(accepted.artifact_media(), Some(ArtifactMedia::Utf8Text));
    accepted.complete(payload.len()).unwrap();
    let ticket = s.prepare_token(proposal(&s, b"deny", 2, 8)).unwrap();
    assert_eq!(ticket.decision(), Decision::Deny);
    assert!(s.is_terminal());
}
#[test]
fn accepted_prefix_drains_after_revoke_and_partial_acknowledgment_stops_session() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[43; 32]);
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
    let ticket = s.prepare_token(proposal(&s, b"abc", 1, 7)).unwrap();
    let accepted = s.accept(ticket).unwrap();
    h.revoke(&key, 1, POLICY_COLD_BUDGET_BYTES).unwrap();
    assert_eq!(accepted.bytes(), b"abc");
    accepted.complete(3).unwrap();
    assert!(s.prepare_token(proposal(&s, b"next", 2, 7)).is_err());
    assert!(s.is_terminal());
}
#[test]
fn pending_slot_refuses_overwrite_and_bulk_boundary_is_before_final_validation() {
    let dir = tempfile::tempdir().unwrap();
    let key = SigningKey::from_bytes(&[44; 32]);
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
    let seen = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let flag = seen.clone();
    s.after_bulk_once(move || flag.store(true, std::sync::atomic::Ordering::SeqCst));
    let first = s.prepare_token(proposal(&s, b"a", 1, 7)).unwrap();
    assert!(seen.load(std::sync::atomic::Ordering::SeqCst));
    assert!(matches!(
        s.prepare_token(proposal(&s, b"b", 2, 7)),
        Err(PolicyError::Capacity)
    ));
    assert!(s.accept(first).is_err());
    assert!(s.is_terminal());
}
