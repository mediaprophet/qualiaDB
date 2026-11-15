//! Registered Windows actual-authority interruption cases. Execution remains
//! deferred until the implementation stage is complete.
use super::*;
use super::{
    runtime_fixture as f,
    test_actor_fixture::{actor, bind},
};
use crate::inference::inference_agent::governed::{self as g, evidence as e};
use ed25519_dalek::SigningKey;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc, Mutex,
};

fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
fn install_until(
    host: &mut g::NativePolicyHost,
    key: &SigningKey,
    actor: g::ActorPolicyScope,
    request: g::DigestScope,
    until: u64,
) {
    host.install_evidence(
        key,
        g::PolicyRules {
            actors: g::PolicyList::from_vec(vec![actor]).unwrap(),
            model: g::DigestScope::Any,
            request,
            graph_scopes: g::PolicyList::from_vec(vec![44]).unwrap(),
            program: g::PolicyList::from_vec(vec![g::ConstraintInstruction::MatchPredicate {
                value: TOKEN_EVENT,
            }])
            .unwrap(),
            norms: Default::default(),
            facts: Default::default(),
            epistemic: Default::default(),
            trace: Default::default(),
            required_claim: 0,
            temporal_invariant: 0,
            non_derogable: true,
        },
        e::ObservationRequirements {
            allowed_sources: e::CandidateSource::ModelToken.bit(),
            allowed_artifact_origins: 0,
            max_payload_bytes: 32,
            allow_empty_artifact: false,
            score: e::ScoreRequirement::Required(e::ScoreMeaning::Logit),
            score_program: Default::default(),
        },
        1,
        until,
        host.status().generation,
        g::POLICY_COLD_BUDGET_BYTES,
    )
    .unwrap();
}

#[test]
fn actual_policy_or_actor_changes_after_allow_roll_back_all_thirteen_buffers() {
    for action in 0..4 {
        let dir = tempfile::tempdir().unwrap();
        let lease = f::model(&dir.path().join("model.gguf"), false);
        let source = ImmutableModel::Direct(&lease);
        let mut tables = f::Tables::new();
        let mut prompt = f::PromptMemory::new();
        let mut buffers = f::Storage::new();
        let prepared = prepare_governed_cpu(
            &source,
            f::expected_executor(),
            tables.borrow(),
            prompt.borrow(),
            f::input("aa", &[2], 2),
            f::intent(),
            &buffers.borrow(),
            f::evidence(),
            f::limits(),
            &AtomicBool::new(false),
        )
        .unwrap();
        let key = SigningKey::from_bytes(&[93; 32]);
        let (owner, handle, scope) = actor(&dir.path().join("actor"));
        let mut host = f::host(dir.path(), &key);
        let request = prepared.required_request_scope();
        f::install(&mut host, &key, scope, request, false);
        let mut retained = [0; 32];
        let mut storage = CpuEvidenceStorage::new(&mut retained, f::evidence()).unwrap();
        let mut job = prepared
            .admit(
                &mut host,
                &key,
                Arc::clone(&owner),
                bind(&owner, &handle),
                &mut storage,
                buffers.borrow(),
            )
            .unwrap();
        let host = Arc::new(Mutex::new(host));
        let hook_host = Arc::clone(&host);
        let before = f::bits(job.numerical_state());
        let fired = Arc::new(AtomicBool::new(false));
        let hook_fired = Arc::clone(&fired);
        // Runs only in this test build, outside authority locks, after a genuine
        // Allow while the real PendingDense transaction is still uncommitted.
        job.before_accept_once(move || {
            let mut host = hook_host.lock().unwrap();
            match action {
                0 => host.disconnect(),
                1 => {
                    let generation = host.status().generation;
                    host.revoke(&key, generation, g::POLICY_COLD_BUDGET_BYTES)
                        .unwrap();
                }
                2 => f::install(&mut host, &key, scope, request, false),
                _ => owner.disconnect(&handle).unwrap(),
            }
            hook_fired.store(true, Ordering::Release);
        });
        assert!(job.next().is_err());
        assert!(fired.load(Ordering::Acquire));
        assert_eq!(f::bits(job.numerical_state()), before);
        assert_eq!(job.generated_tokens(), 0);
        assert!(job.is_terminal());
        drop(job);
        assert!(buffers.output.iter().all(|&v| v == 99));
        assert!(buffers.lineage.iter().all(Option::is_none));
    }
}

#[test]
fn real_policy_expiry_after_allow_restores_pending_numerics_and_publishes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let lease = f::model(&dir.path().join("model.gguf"), false);
    let source = ImmutableModel::Direct(&lease);
    let mut tables = f::Tables::new();
    let mut prompt = f::PromptMemory::new();
    let mut buffers = f::Storage::new();
    let prepared = prepare_governed_cpu(
        &source,
        f::expected_executor(),
        tables.borrow(),
        prompt.borrow(),
        f::input("aa", &[2], 2),
        f::intent(),
        &buffers.borrow(),
        f::evidence(),
        f::limits(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let key = SigningKey::from_bytes(&[94; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut host = f::host(dir.path(), &key);
    let until = now() + 3;
    install_until(
        &mut host,
        &key,
        scope,
        prepared.required_request_scope(),
        until,
    );
    let mut retained = [0; 32];
    let mut storage = CpuEvidenceStorage::new(&mut retained, f::evidence()).unwrap();
    let mut job = prepared
        .admit(
            &mut host,
            &key,
            Arc::clone(&owner),
            bind(&owner, &handle),
            &mut storage,
            buffers.borrow(),
        )
        .unwrap();
    let before = f::bits(job.numerical_state());
    let fired = Arc::new(AtomicBool::new(false));
    let hook_fired = Arc::clone(&fired);
    job.before_accept_once(move || {
        assert!(
            now() < until,
            "fixture must reach a genuine pre-expiry Allow"
        );
        let bound = std::time::Instant::now() + std::time::Duration::from_secs(5);
        while now() < until {
            assert!(std::time::Instant::now() < bound);
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        hook_fired.store(true, Ordering::Release);
    });
    assert!(job.next().is_err());
    assert!(fired.load(Ordering::Acquire));
    assert_eq!(f::bits(job.numerical_state()), before);
    assert_eq!(job.generated_tokens(), 0);
    assert!(job.is_terminal());
    drop(job);
    assert!(buffers.output.iter().all(|&v| v == 99));
    assert!(buffers.lineage.iter().all(Option::is_none));
    assert!(
        host.status().installed,
        "expiry refusal does not pretend policy was uninstalled"
    );
}
