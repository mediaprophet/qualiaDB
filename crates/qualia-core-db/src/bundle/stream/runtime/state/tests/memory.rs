use super::*;
use crate::bundle::stream::{
    proof::{leaf_hash, Representation, TreeSpec},
    storage::{verify_leaf_view, IdentityFields, StorageClass, StorageIdentity},
};
fn identity() -> StorageIdentity {
    use crate::net::qdnf::types::StrongDigest as D;
    let spec = TreeSpec::new(D([2; 48]), 3, 4, Representation::Plaintext, 0, 1).unwrap();
    StorageIdentity::new(IdentityFields {
        descriptor: D([1; 48]),
        context: D([2; 48]),
        entry: 3,
        generation: 4,
        representation: Representation::Plaintext,
        storage_class: StorageClass::PublicPlaintextCandidate,
        key_epoch: 0,
        logical_length: 1,
        root: leaf_hash(&spec, 0, &[7]).unwrap(),
        privacy_partition: D([6; 48]),
        policy_generation: 1,
        storage_generation: 8,
    })
    .unwrap()
}
#[test]
fn actual_workspace_integrity_memory_is_not_copied_or_recharged_between_polls() {
    with_parents(false, |parents| {
        let mut state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        let id = identity();
        let before = state.charged_window_bytes().unwrap();
        let mut page = parents
            .try_with_runtime(&mut state, |mut rt| rt.acquire_sparse_page(id, 0))
            .unwrap();
        let proof = verify_leaf_view(&id, 0, &[7], &[]).unwrap();
        let ptr = page.with_candidate_page(|p| p.payload().as_ptr());
        page.with_candidate_page(|p| {
            p.record_integrity(&proof).unwrap();
        });
        let count = state.charged_window_bytes().unwrap();
        assert!(count > before);
        for _ in 0..32 {
            parents.try_with_runtime(&mut state, |_| Ok(())).unwrap();
            assert!(page.has_integrity_mark(0).unwrap());
            assert_eq!(page.with_candidate_page(|p| p.payload().as_ptr()), ptr);
            assert_eq!(state.charged_window_bytes().unwrap(), count);
        }
        parents
            .try_with_runtime(&mut state, |mut rt| {
                rt.release_sparse_page(page).map_err(|e| e.error)
            })
            .unwrap();
        assert_eq!(state.charged_window_bytes().unwrap(), before);
        state.try_close().unwrap();
    });
}
#[test]
fn raw_allocation_counter_and_bounded_move_footprint_measure_actual_scopes() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    with_parents(false, |parents| {
        let mut state = parents.admit(RuntimeProfile::Desktop, control()).unwrap();
        let rows = [spec(4)];
        let plan = ColdPlan::compile(&rows).unwrap();
        let mut ticket = parents
            .try_with_runtime(&mut state, |mut rt| rt.admit(&plan, 0, 0, Generation(1)))
            .unwrap();
        parents
            .try_with_runtime(&mut state, |rt| {
                assert_eq!(rt.state(&ticket)?, TransferState::Queued);
                Ok(())
            })
            .unwrap();
        let started = std::time::Instant::now();
        let measure = AllocGuard::begin("actual runtime move/restore scopes", true);
        for _ in 0..256 {
            parents
                .try_with_runtime(&mut state, |rt| {
                    std::hint::black_box(rt.state(&ticket)?);
                    Ok(())
                })
                .unwrap();
        }
        measure.check().unwrap();
        let elapsed = started.elapsed();
        println!(
            "256 scoped polls: {:?}; logical state moves/poll={}B; admitted static={}B",
            elapsed,
            RetainedRuntime::move_bytes_per_poll(),
            state.data.as_ref().unwrap().static_charge_bytes
        );
        let positive = AllocGuard::begin("state scope positive", true);
        std::hint::black_box(vec![1u8; 8]);
        assert!(positive.check().is_err());
        ready(parents, &mut state, &ticket);
        finish(parents, &mut state, &mut ticket);
        state.try_close().unwrap();
    });
}
