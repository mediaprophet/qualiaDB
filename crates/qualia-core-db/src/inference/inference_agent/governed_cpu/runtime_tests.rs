//! Draft actual-library integration cases. Source-only until the root build lane
//! permits same-crate compilation; none of these cases has been executed yet.
use super::*;
use super::{
    runtime_fixture as f,
    test_actor_fixture::{actor, bind},
};
use crate::inference::inference_agent::governed::{self as g, evidence as e};
use ed25519_dalek::SigningKey;
use std::sync::{atomic::AtomicBool, Arc};

#[test]
fn genuine_signed_allow_and_explicit_developer_match_all_numerical_buffers() {
    use crate::specialized_libs::computational_geometry::allocation_counter::{
        assert_zero_alloc, AllocGuard,
    };
    let positive = AllocGuard::begin("CPU positive control", true);
    let allocation = vec![7u8; 1024];
    std::hint::black_box(&allocation);
    assert!(positive.check().is_err());
    drop(allocation);
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
    assert_eq!(prepared.review().first_position, 1);
    assert_eq!(
        prepared.review().tokenizer_semantics_version,
        etalon_model::tokenizer::TOKENIZER_SEMANTICS_VERSION
    );
    let key = SigningKey::from_bytes(&[81; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut host = f::host(dir.path(), &key);
    f::install(
        &mut host,
        &key,
        scope,
        prepared.required_request_scope(),
        false,
    );
    let mut retained = [0; 32];
    let mut storage = CpuEvidenceStorage::new(&mut retained, f::evidence()).unwrap();
    let mut normal = prepared
        .admit(
            &mut host,
            &key,
            Arc::clone(&owner),
            bind(&owner, &handle),
            &mut storage,
            buffers.borrow(),
        )
        .unwrap();
    let mut dev_tables = f::Tables::new();
    let mut dev_prompt = f::PromptMemory::new();
    let mut dev_buffers = f::Storage::new();
    let developer = prepare_developer_cpu(
        StandaloneDeveloper::Explicit,
        &source,
        dev_tables.borrow(),
        dev_prompt.borrow(),
        f::input("aa", &[2], 2),
        &dev_buffers.borrow(),
        f::limits(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let mut standalone = developer.begin(dev_buffers.borrow()).unwrap();
    assert_eq!(
        f::bits(normal.numerical_state()),
        f::bits(standalone.numerical_state())
    );
    for index in 0..2 {
        let mut actual = (0, 0, 0);
        assert_zero_alloc(
            "actual governed CPU numerical/tokenizer/VM/commit/drain",
            || {
                let record = normal.next().unwrap();
                assert_eq!(record.bytes(), b"a");
                actual = (
                    record.output().token_id,
                    record.output().max_logit.to_bits(),
                    record.position(),
                );
                assert_eq!(actual.0, 0);
                assert_eq!(actual.2, 1 + index);
                let delivered = record.bytes().len();
                record.complete(delivered).unwrap();
            },
        );
        let record = standalone.next().unwrap();
        assert_eq!(record.bytes(), b"a");
        assert_eq!(
            (
                record.output().token_id,
                record.output().max_logit.to_bits(),
                record.position() as u64
            ),
            actual
        );
        let delivered = record.bytes().len();
        record.complete(delivered).unwrap();
        assert_eq!(
            f::bits(normal.numerical_state()),
            f::bits(standalone.numerical_state())
        );
    }
    assert!(normal.is_finished());
    assert!(standalone.is_finished());
    assert_eq!(normal.generated_tokens(), 2);
}

#[test]
fn real_denial_restores_all_buffers_and_never_writes_output_or_lineage() {
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
    let key = SigningKey::from_bytes(&[82; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut host = f::host(dir.path(), &key);
    f::install(
        &mut host,
        &key,
        scope,
        prepared.required_request_scope(),
        true,
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
    assert!(job.next().is_err());
    assert!(job.is_terminal());
    assert_eq!(job.generated_tokens(), 0);
    assert_eq!(f::bits(job.numerical_state()), before);
    assert!(job.next().is_err());
    assert_eq!(f::bits(job.numerical_state()), before);
    drop(job);
    assert!(buffers.output.iter().all(|&n| n == 99));
    assert!(buffers.lineage.iter().all(Option::is_none));
}

#[test]
fn accepted_prefix_survives_controller_loss_but_no_later_token_commits() {
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
    let key = SigningKey::from_bytes(&[83; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut host = f::host(dir.path(), &key);
    f::install(
        &mut host,
        &key,
        scope,
        prepared.required_request_scope(),
        false,
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
    let record = job.next().unwrap();
    assert_eq!(record.bytes(), b"a");
    // Callback-equivalent host reentry while a committed receipt is held. There
    // must be no owner/policy gate held by the returned drainer.
    host.disconnect();
    record.complete(1).unwrap();
    let accepted = f::bits(job.numerical_state());
    assert!(job.next().is_err());
    assert_eq!(f::bits(job.numerical_state()), accepted);
    assert_eq!(job.generated_tokens(), 1);
    assert!(job.is_terminal());
    drop(job);
    assert_eq!(buffers.output[0], 0);
    assert_eq!(buffers.output[1], 99);
    assert!(buffers.lineage[0].is_some());
    assert!(buffers.lineage[1].is_none());
}

#[test]
fn partial_drain_and_unwind_terminally_discard_the_committed_job() {
    for unwind in [false, true] {
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
            f::input("a", &[2], 2),
            f::intent(),
            &buffers.borrow(),
            f::evidence(),
            f::limits(),
            &AtomicBool::new(false),
        )
        .unwrap();
        let key = SigningKey::from_bytes(&[84; 32]);
        let (owner, handle, scope) = actor(&dir.path().join("actor"));
        let mut host = f::host(dir.path(), &key);
        f::install(
            &mut host,
            &key,
            scope,
            prepared.required_request_scope(),
            false,
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
        if unwind {
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                let record = job.next().unwrap();
                assert_eq!(record.bytes(), b"a");
                panic!("injected delivery unwind");
            }));
            assert!(result.is_err());
        } else {
            assert!(job.next().unwrap().complete(0).is_err());
        }
        let committed = f::bits(job.numerical_state());
        assert_eq!(job.generated_tokens(), 1);
        assert!(job.is_terminal());
        assert!(job.next().is_err());
        assert_eq!(f::bits(job.numerical_state()), committed);
    }
}

#[test]
fn actual_policy_host_loss_during_prefill_refuses_before_any_proposal() {
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
    let review = prepared.review();
    let key = SigningKey::from_bytes(&[85; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut host = f::host(dir.path(), &key);
    f::install(
        &mut host,
        &key,
        scope,
        prepared.required_request_scope(),
        false,
    );
    let mut retained = [0; 32];
    let mut storage = e::EvidenceStorage::new(&mut retained, f::evidence()).unwrap();
    let authority = host
        .admit_evidence(
            &key,
            g::InferenceAdmission {
                request_digest: review.request_digest,
                model_digest: review.model_digest,
                model_generation: 1,
                device_id: 1,
                device_generation: 1,
                graph_scope: 44,
                first_position: review.first_position,
                instrumentation: Default::default(),
            },
            g::POLICY_COLD_BUDGET_BYTES,
            Arc::clone(&owner),
            bind(&owner, &handle),
            &mut storage,
        )
        .unwrap();
    let _hook = test_hooks::install_after_prefill(move || host.disconnect());
    assert!(prepared
        .start_admitted(authority, buffers.borrow())
        .is_err());
    assert!(buffers.output.iter().all(|&n| n == 99));
    assert!(buffers.lineage.iter().all(Option::is_none));
}

#[test]
fn eos_and_last_context_position_are_accepted_before_stop_and_overflow_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let lease = f::model(&dir.path().join("model.gguf"), true);
    let source = ImmutableModel::Direct(&lease);
    let mut tables = f::Tables::new();
    let mut prompt = f::PromptMemory::new();
    let mut buffers = f::Storage::new();
    let prepared = prepare_governed_cpu(
        &source,
        f::expected_executor(),
        tables.borrow(),
        prompt.borrow(),
        f::input("aaaaaaaa", &[2], 1),
        f::intent(),
        &buffers.borrow(),
        f::evidence(),
        f::limits(),
        &AtomicBool::new(false),
    )
    .unwrap();
    assert_eq!(prepared.review().first_position, 7);
    let key = SigningKey::from_bytes(&[86; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut host = f::host(dir.path(), &key);
    f::install(
        &mut host,
        &key,
        scope,
        prepared.required_request_scope(),
        false,
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
    let record = job.next().unwrap();
    assert_eq!(record.output().token_id, 2);
    assert_eq!(record.position(), 7);
    assert_eq!(record.bytes(), b"<eos>");
    record.complete(5).unwrap();
    assert!(job.is_finished());
    drop(job);
    drop(prepared);
    let mut refused = f::Storage::new();
    assert!(prepare_governed_cpu(
        &source,
        f::expected_executor(),
        tables.borrow(),
        prompt.borrow(),
        f::input("aaaaaaaa", &[2], 2),
        f::intent(),
        &refused.borrow(),
        f::evidence(),
        f::limits(),
        &AtomicBool::new(false)
    )
    .is_err());
}

#[test]
fn aggregate_minus_one_and_any_request_policy_do_not_start_execution() {
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
    let key = SigningKey::from_bytes(&[87; 32]);
    let (owner, handle, scope) = actor(&dir.path().join("actor"));
    let mut host = f::host(dir.path(), &key);
    f::install(&mut host, &key, scope, g::DigestScope::Any, false);
    let mut retained = [0; 32];
    let mut storage = CpuEvidenceStorage::new(&mut retained, f::evidence()).unwrap();
    assert!(prepared
        .admit(
            &mut host,
            &key,
            Arc::clone(&owner),
            bind(&owner, &handle),
            &mut storage,
            buffers.borrow()
        )
        .is_err());
    assert!(buffers
        .state
        .iter()
        .all(|s| s.iter().all(|v| v.to_bits() == 0)));
    let needed = prepared.budget().admitted_bytes();
    drop(prepared);
    let mut low = f::limits();
    low.aggregate_bytes = needed - 1;
    assert!(prepare_governed_cpu(
        &source,
        f::expected_executor(),
        tables.borrow(),
        prompt.borrow(),
        f::input("aa", &[2], 2),
        f::intent(),
        &buffers.borrow(),
        f::evidence(),
        low,
        &AtomicBool::new(false)
    )
    .is_err());
    assert!(buffers.output.iter().all(|&n| n == 99));
    assert!(buffers.lineage.iter().all(Option::is_none));
}
