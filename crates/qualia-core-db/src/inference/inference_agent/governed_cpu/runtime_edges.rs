//! Registered Windows integration cases; execution remains deferred until the
//! implementation stage is complete and the parent opens the validation lane.
use super::*;
use super::{
    runtime_fixture as f,
    test_actor_fixture::{actor, bind},
};
use ed25519_dalek::SigningKey;
use std::sync::{atomic::AtomicBool, Arc};

#[test]
fn denied_eos_and_empty_eos_restore_every_numerical_buffer() {
    for empty in [false, true] {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("model.gguf");
        let lease = if empty {
            f::empty_eos_model(&path)
        } else {
            f::model(&path, true)
        };
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
        let key = SigningKey::from_bytes(&[88; 32]);
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
        assert_eq!(f::bits(job.numerical_state()), before);
        assert_eq!(job.generated_tokens(), 0);
        assert!(job.is_terminal());
        assert!(job.next().is_err());
        drop(job);
        assert!(buffers.output.iter().all(|&n| n == 99));
        assert!(buffers.lineage.iter().all(Option::is_none));
    }
}

#[test]
fn empty_eos_requires_real_acceptance_and_exact_zero_byte_acknowledgment() {
    for delivered in [0, 1] {
        let dir = tempfile::tempdir().unwrap();
        let lease = f::empty_eos_model(&dir.path().join("model.gguf"));
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
        let key = SigningKey::from_bytes(&[89; 32]);
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
        let accepted = job.next().unwrap();
        assert_eq!(accepted.output().token_id, 2);
        assert_eq!(accepted.position(), 1);
        assert!(accepted.bytes().is_empty());
        assert_eq!(accepted.complete(delivered).is_ok(), delivered == 0);
        assert_eq!(job.generated_tokens(), 1);
        assert!(job.is_finished());
        assert_eq!(job.is_terminal(), delivered != 0);
        assert!(job.next().is_err());
        drop(job);
        assert_eq!(buffers.output[0], 2);
        assert!(buffers.lineage[0].is_some());
    }
}

#[test]
fn failed_second_prefill_restores_that_step_but_discards_the_unpublished_job() {
    use etalon_model_io::lease::{LeaseLimits, ModelLease};
    use sha2::{Digest, Sha256};
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("nonfinite.gguf");
    let original = f::model(&path, false);
    assert!(original.bytes().len() <= 32 * 1024);
    let mut bytes = original.bytes().to_vec();
    // This bounded fixture has exactly46 F32 values; token embeddings are first.
    // Verify the known row before replacing only the actual second embedding.
    let data = bytes.len().checked_sub(46 * 4).unwrap();
    assert_eq!(&bytes[data..data + 8], &[0, 0, 128, 63, 0, 0, 0, 0]);
    assert_eq!(&bytes[data + 8..data + 16], &[0, 0, 0, 0, 0, 0, 128, 63]);
    bytes[data + 8..data + 12].copy_from_slice(&f32::NAN.to_le_bytes());
    bytes[data + 12..data + 16].copy_from_slice(&f32::NAN.to_le_bytes());
    drop(original);
    std::fs::write(&path, &bytes).unwrap();
    let lease = ModelLease::open(
        &path,
        Sha256::digest(&bytes).into(),
        LeaseLimits {
            max_source_bytes: 32 * 1024,
            max_mapped_bytes: 32 * 1024,
            max_total_bytes: 64 * 1024,
        },
        &AtomicBool::new(false),
    )
    .unwrap();
    let source = ImmutableModel::Direct(&lease);
    let mut tables = f::Tables::new();
    let mut prompt = f::PromptMemory::new();
    let mut buffers = f::Storage::new();
    // Real first-prefix reference using the same nonfinite model and actual CPU.
    let developer = prepare_developer_cpu(
        StandaloneDeveloper::Explicit,
        &source,
        tables.borrow(),
        prompt.borrow(),
        f::input("aa", &[2], 1),
        &buffers.borrow(),
        f::limits(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let expected = {
        let job = developer.begin(buffers.borrow()).unwrap();
        f::bits(job.numerical_state())
    };
    drop(developer);
    let mut buffers = f::Storage::new();
    let prepared = prepare_governed_cpu(
        &source,
        f::expected_executor(),
        tables.borrow(),
        prompt.borrow(),
        f::input("aba", &[2], 1),
        f::intent(),
        &buffers.borrow(),
        f::evidence(),
        f::limits(),
        &AtomicBool::new(false),
    )
    .unwrap();
    let key = SigningKey::from_bytes(&[90; 32]);
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
    assert!(matches!(
        prepared.admit(
            &mut host,
            &key,
            Arc::clone(&owner),
            bind(&owner, &handle),
            &mut storage,
            buffers.borrow()
        ),
        Err(CpuStepError::Numerical(_))
    ));
    let actual: Vec<Vec<u32>> = buffers
        .state
        .iter()
        .map(|s| s.iter().map(|v| v.to_bits()).collect())
        .collect();
    assert_eq!(actual, expected);
    assert!(buffers.output.iter().all(|&n| n == 99));
    assert!(buffers.lineage.iter().all(Option::is_none));
    // A failed constructor produces no reusable job; the retained prefix is not
    // a whole-job rollback. Reusing nonzero scratch requires explicit reset.
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
}
