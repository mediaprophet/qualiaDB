use super::*;
use crate::bundle::stream::{
    proof::{leaf_hash, Representation, TreeSpec, LEAF_BYTES},
    runtime::{RuntimeProfile, TransferRuntime},
    storage::{verify_leaf_view, IdentityFields, MarkStatus, StorageClass, StorageIdentity},
};
use crate::net::{
    peer::runtime::{
        FairScheduler, LeaseTable, OperationTable, ReservationLedger, ResourceBudget,
        ResourceGovernor, WorkClass,
    },
    qdnf::{
        errors::QdnfError,
        types::{Generation, StrongDigest},
    },
};

fn identity(length: u64) -> StorageIdentity {
    StorageIdentity::new(IdentityFields {
        descriptor: StrongDigest([1; 48]),
        context: StrongDigest([2; 48]),
        entry: 3,
        generation: 4,
        representation: Representation::Plaintext,
        storage_class: StorageClass::PublicPlaintextCandidate,
        key_epoch: 0,
        logical_length: length,
        root: StrongDigest([5; 48]),
        privacy_partition: StrongDigest([6; 48]),
        policy_generation: 1,
        storage_generation: 8,
    })
    .unwrap()
}
fn one_leaf() -> StorageIdentity {
    let mut fields = *identity(1).fields();
    let tree = TreeSpec::new(
        fields.context,
        fields.entry,
        fields.generation,
        fields.representation,
        fields.key_epoch,
        fields.logical_length,
    )
    .unwrap();
    fields.root = leaf_hash(&tree, 0, &[7]).unwrap();
    StorageIdentity::new(fields).unwrap()
}
fn with_runtime<T>(run: impl FnOnce(&mut TransferRuntime<'_>) -> T) -> T {
    fixture(64 * 1024 * 1024, false, run)
}
fn fixture<T>(bytes: u64, orphan: bool, run: impl FnOnce(&mut TransferRuntime<'_>) -> T) -> T {
    let cap = ResourceBudget {
        bytes,
        work: 10000,
        io: 10000,
    };
    let mut ledger = ReservationLedger::new(cap, cap, cap, cap, cap);
    let mut leases = LeaseTable::new();
    let mut ops = OperationTable::new();
    let mut scheduler = FairScheduler::new();
    let governor = ResourceGovernor::new(4096);
    let mut rt = TransferRuntime::new(
        &mut ledger,
        &mut leases,
        &mut ops,
        &mut scheduler,
        &governor,
        RuntimeProfile::Desktop,
        ResourceBudget {
            bytes: 4096,
            work: 1024,
            io: 64,
        },
    )
    .unwrap();
    let result = run(&mut rt);
    if orphan {
        assert_eq!(rt.close(), Err(QdnfError::WouldBlock));
        drop(rt);
        assert!(ledger.used().host.bytes > 0);
    } else {
        assert_eq!(rt.retained_sparse_pages(), 0);
        rt.close().unwrap();
        drop(rt);
        assert_eq!(ledger.used().host, ResourceBudget::ZERO);
        assert_eq!(leases.occupied_count(), 0);
    }
    result
}

#[test]
fn actual_page_mark_survives_reborrow_and_exact_whole_owner_charge_releases() {
    with_runtime(|rt| {
        let before = rt.parent_used();
        let window = rt.charged_window_bytes();
        let id = one_leaf();
        let leaf = verify_leaf_view(&id, 0, &[7], &[]).unwrap();
        let mut workspace = rt.acquire_sparse_page(id, 0).unwrap();
        let bytes = SparsePageWorkspace::charged_bytes() as u64;
        assert!(bytes > 4096);
        assert_eq!(rt.parent_used().host.bytes, before.host.bytes + bytes);
        assert_eq!(rt.charged_window_bytes(), window + bytes);
        assert_eq!(rt.retained_sparse_pages(), 1);
        assert_eq!(rt.sparse_inflight_ceiling(), 29);
        assert!(!workspace.has_integrity_mark(0).unwrap());
        let ptr = workspace.page.payload().as_ptr();
        workspace.with_candidate_page(|page| {
            assert_eq!(page.payload().as_ptr(), ptr);
            assert_eq!(page.record_integrity(&leaf).unwrap(), MarkStatus::New);
        });
        workspace.with_candidate_page(|page| {
            assert_eq!(page.payload().as_ptr(), ptr);
            assert!(page.has_integrity_mark(0).unwrap());
            assert_eq!(
                page.record_integrity(&leaf).unwrap(),
                MarkStatus::AlreadyMarked
            );
        });
        assert_eq!(rt.close(), Err(QdnfError::WouldBlock));
        rt.release_sparse_page(workspace).unwrap();
        assert_eq!(rt.parent_used(), before);
        assert_eq!(rt.charged_window_bytes(), window);
    });
}
#[test]
fn wrong_runtime_returns_same_marked_owner_for_correct_release() {
    with_runtime(|original| {
        let id = one_leaf();
        let mut workspace = original.acquire_sparse_page(id, 0).unwrap();
        let leaf = verify_leaf_view(&id, 0, &[7], &[]).unwrap();
        workspace.with_candidate_page(|page| {
            page.record_integrity(&leaf).unwrap();
        });
        let error = with_runtime(|other| {
            let before = other.parent_used();
            let error = other.release_sparse_page(workspace).unwrap_err();
            assert_eq!(error.error, QdnfError::StaleGeneration);
            assert_eq!(other.parent_used(), before);
            error
        });
        assert!(error.workspace.has_integrity_mark(0).unwrap());
        assert_eq!(original.retained_sparse_pages(), 1);
        original.release_sparse_page(error.workspace).unwrap();
    });
}
#[test]
fn wrong_generation_refuses_and_slot_reuse_increments_without_wrap() {
    with_runtime(|rt| {
        let id = one_leaf();
        let mut workspace = rt.acquire_sparse_page(id, 0).unwrap();
        let first = workspace.generation;
        let slot = workspace.slot;
        workspace.generation = Generation::ZERO; // private-field fault injection.
        let error = rt.release_sparse_page(workspace).unwrap_err();
        assert_eq!(error.error, QdnfError::StaleGeneration);
        let mut workspace = error.workspace;
        workspace.generation = first;
        rt.release_sparse_page(workspace).unwrap();
        let workspace = rt.acquire_sparse_page(id, 0).unwrap();
        assert_eq!(workspace.slot, slot);
        assert_eq!(workspace.generation, first.next().unwrap());
        rt.release_sparse_page(workspace).unwrap();
        rt.sparse_workspaces[0].generation = Generation(u64::MAX);
        let workspace = rt.acquire_sparse_page(id, 0).unwrap();
        assert_eq!(workspace.slot, 1); // retired slot is skipped, never wrapped.
        rt.release_sparse_page(workspace).unwrap();
    });
}
#[test]
fn bounded_capacity_geometry_duplicates_and_governor_refuse_without_charge() {
    with_runtime(|rt| {
        let initial = rt.parent_used();
        assert!(matches!(
            rt.acquire_sparse_page(identity(0), 0),
            Err(QdnfError::Range)
        ));
        assert!(matches!(
            rt.acquire_sparse_page(identity(1), 1),
            Err(QdnfError::Range)
        ));
        assert!(matches!(
            rt.acquire_sparse_page(identity(1), u64::MAX),
            Err(QdnfError::Range)
        ));
        assert_eq!(rt.parent_used(), initial);
        let id = identity(9 * 32768 * LEAF_BYTES);
        let mut pages: [Option<SparsePageWorkspace>; MAX_SPARSE_WORKSPACES] =
            core::array::from_fn(|_| None);
        for (i, page) in pages.iter_mut().enumerate() {
            *page = Some(rt.acquire_sparse_page(id, i as u64).unwrap());
        }
        let full = rt.parent_used();
        assert_eq!(rt.retained_sparse_pages(), 8);
        assert_eq!(rt.sparse_inflight_ceiling(), 22);
        assert!(matches!(
            rt.acquire_sparse_page(id, 0),
            Err(QdnfError::Conflict)
        ));
        assert!(matches!(
            rt.acquire_sparse_page(id, 8),
            Err(QdnfError::Capacity)
        ));
        assert_eq!(rt.parent_used(), full);
        for workspace in pages.into_iter().flatten() {
            rt.release_sparse_page(workspace).unwrap();
        }
    });
    let cap = core::mem::size_of::<TransferRuntime<'_>>() as u64
        + 8192
        + SparsePageWorkspace::charged_bytes() as u64
        - 1;
    fixture(cap, false, |rt| {
        let before = rt.parent_used();
        assert!(matches!(
            rt.acquire_sparse_page(one_leaf(), 0),
            Err(QdnfError::BudgetExhausted)
        ));
        assert_eq!(rt.retained_sparse_pages(), 0);
        assert_eq!(rt.parent_used(), before);
    });
}
#[test]
fn parent_release_refusal_restores_original_handle_and_returns_retry_owner() {
    with_runtime(|rt| {
        let workspace = rt.acquire_sparse_page(one_leaf(), 0).unwrap();
        let index = usize::from(workspace.slot);
        let before = rt.parent_used();
        let original = rt.sparse_workspaces[index].charge.take().unwrap();
        let alternate = rt
            .governor
            .admit_and_reserve(
                rt.ledger,
                ResourceBudget {
                    bytes: 1,
                    work: 0,
                    io: 0,
                },
                WorkClass::Background,
                true,
            )
            .unwrap();
        rt.sparse_workspaces[index].charge = Some(alternate); // controlled table-corruption fault.
        let at_fault = rt.parent_used();
        let error = rt.release_sparse_page(workspace).unwrap_err();
        assert_eq!(error.error, QdnfError::Conflict);
        assert_eq!(rt.parent_used(), at_fault);
        let restored = rt.sparse_workspaces[index]
            .charge
            .replace(original)
            .unwrap();
        rt.ledger.release(restored).unwrap();
        assert_eq!(rt.parent_used(), before);
        rt.release_sparse_page(error.workspace).unwrap();
    });
}
#[test]
fn dropped_owner_does_not_fake_release_or_allow_runtime_close() {
    fixture(64 * 1024 * 1024, true, |rt| {
        let workspace = rt.acquire_sparse_page(one_leaf(), 0).unwrap();
        let before = rt.parent_used();
        drop(workspace);
        assert_eq!(rt.retained_sparse_pages(), 1);
        assert_eq!(rt.parent_used(), before);
        assert_eq!(rt.close(), Err(QdnfError::WouldBlock));
    });
}
#[test]
fn admission_marks_reborrow_and_release_have_zero_raw_alloc_calls() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    with_runtime(|rt| {
        let id = one_leaf();
        let leaf = verify_leaf_view(&id, 0, &[7], &[]).unwrap();
        let warm = rt.acquire_sparse_page(id, 0).unwrap();
        rt.release_sparse_page(warm).unwrap();
        let guard = AllocGuard::begin("retained-page-owner", true);
        let mut workspace = rt.acquire_sparse_page(id, 0).unwrap();
        workspace.with_candidate_page(|page| {
            page.record_integrity(&leaf).unwrap();
        });
        assert!(workspace.has_integrity_mark(0).unwrap());
        rt.release_sparse_page(workspace).unwrap();
        guard
            .check()
            .expect("workspace owner operations must not allocate");
    });
}

#[test]
fn uncommitted_attempt_or_unwind_clears_marks_without_releasing_charge() {
    with_runtime(|rt| {
        let id = one_leaf();
        let leaf = verify_leaf_view(&id, 0, &[7], &[]).unwrap();
        let mut workspace = rt.acquire_sparse_page(id, 0).unwrap();
        let charged = rt.parent_used();
        workspace.with_candidate_page(|page| {
            page.record_integrity(&leaf).unwrap();
        });
        {
            let mut attempt = workspace.begin_attempt();
            attempt.workspace().with_candidate_page(|page| {
                assert!(page.has_integrity_mark(0).unwrap());
            });
        }
        assert!(!workspace.has_integrity_mark(0).unwrap());
        assert_eq!(rt.parent_used(), charged);
        let panic = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut attempt = workspace.begin_attempt();
            attempt.workspace().with_candidate_page(|page| {
                page.record_integrity(&leaf).unwrap();
            });
            panic!("deliberate outer policy/final-check unwind");
        }));
        assert!(panic.is_err());
        assert!(!workspace.has_integrity_mark(0).unwrap());
        assert_eq!(rt.parent_used(), charged);
        rt.release_sparse_page(workspace).unwrap();
    });
}
#[test]
fn committed_attempt_retains_marks_but_new_facade_reset_removes_them() {
    with_runtime(|rt| {
        let id = one_leaf();
        let leaf = verify_leaf_view(&id, 0, &[7], &[]).unwrap();
        let mut workspace = rt.acquire_sparse_page(id, 0).unwrap();
        let key = workspace.binding_key();
        {
            let mut attempt = workspace.begin_attempt();
            attempt.workspace().with_candidate_page(|page| {
                page.record_integrity(&leaf).unwrap();
            });
            attempt.commit();
        }
        assert!(workspace.has_integrity_mark(0).unwrap());
        let new_key = workspace.reset_for_store().unwrap();
        assert_ne!(new_key, key);
        assert_eq!(
            workspace.check_binding_key(key),
            Err(QdnfError::StaleGeneration)
        );
        workspace.check_binding_key(new_key).unwrap();
        assert!(!workspace.has_integrity_mark(0).unwrap());
        assert_eq!(rt.retained_sparse_pages(), 1);
        rt.release_sparse_page(workspace).unwrap();
    });
}

mod bridge;

#[test]
fn every_store_reset_retires_old_key_and_rollover_preserves_marks_and_charge() {
    with_runtime(|rt| {
        let id = one_leaf();
        let leaf = verify_leaf_view(&id, 0, &[7], &[]).unwrap();
        let mut workspace = rt.acquire_sparse_page(id, 0).unwrap();
        let first = workspace.reset_for_store().unwrap();
        workspace.with_candidate_page(|page| {
            page.record_integrity(&leaf).unwrap();
        });
        let second = workspace.reset_for_store().unwrap();
        assert_ne!(first, second);
        assert_eq!(
            workspace.check_binding_key(first),
            Err(QdnfError::StaleGeneration)
        );
        workspace.check_binding_key(second).unwrap();
        assert!(!workspace.has_integrity_mark(0).unwrap());
        workspace.with_candidate_page(|page| {
            page.record_integrity(&leaf).unwrap();
        });
        workspace.store_incarnation = Generation(u64::MAX); // private rollover fault.
        let before_key = workspace.binding_key();
        let before_charge = rt.parent_used();
        assert_eq!(workspace.reset_for_store(), Err(QdnfError::StaleGeneration));
        assert_eq!(workspace.binding_key(), before_key);
        assert!(workspace.has_integrity_mark(0).unwrap());
        assert_eq!(rt.parent_used(), before_charge);
        rt.release_sparse_page(workspace).unwrap();
    });
}
