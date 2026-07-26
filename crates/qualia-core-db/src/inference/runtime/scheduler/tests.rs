use super::*;
use crate::inference::runtime::graph_assist::{PrefixIdentity, PrefixPageSet};
use crate::inference::runtime::kv::paged::{BlockPool, CopyOnWrite};
use crate::inference::runtime::kv::prefix::PrefixKvStore;
use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

#[test]
fn ragged_batch_abi_is_flat_and_uploadable() {
    assert_eq!(core::mem::size_of::<RaggedBatchItem>(), 32);
    assert_eq!(core::mem::size_of::<RaggedBatchOutput>(), 16);
    let item = RaggedBatchItem::default();
    assert_eq!(bytemuck::bytes_of(&item).len(), 32);
}

#[test]
fn graph_prefix_admission_cow_and_cancellation_are_owned_by_scheduler() {
    let mut pool = BlockPool::new(8);
    let page = pool.allocate().unwrap();
    let identity = PrefixIdentity { words: [42; 4] };
    let mut prefixes = PrefixKvStore::<2, 4>::new();
    prefixes
        .publish(
            PrefixPageSet::new(identity, &[page], 16).unwrap(),
            &mut pool,
        )
        .unwrap();
    let mut scheduler = RequestScheduler::<2>::new(4);
    let admission = scheduler
        .admit_with_prefix(100, Some(identity), &prefixes, &mut pool)
        .unwrap();
    assert!(admission.prefix_hit);
    assert_eq!(admission.prefix_tokens, 16);
    assert_eq!(pool.ref_count(page), Some(3)); // producer, registry, request

    let cow = scheduler.ensure_writable(100, 0, &mut pool).unwrap();
    assert!(matches!(
        cow,
        CopyOnWrite::Copy {
            source,
            destination: _
        } if source == page
    ));
    scheduler.finish(100, &mut pool).unwrap();
    assert_eq!(scheduler.active_count(), 0);
    assert_eq!(pool.ref_count(page), Some(2)); // producer + registry
}

#[test]
fn admission_collection_and_finish_do_not_allocate_after_construction() {
    let mut pool = BlockPool::new(4);
    let prefixes = PrefixKvStore::<1, 2>::new();
    let mut scheduler = RequestScheduler::<2>::new(2);
    let mut views = [RequestView {
        request_id: 0,
        slot: 0,
        state: RequestState::Empty,
        token_count: 0,
        prefix_tokens: 0,
    }; 2];
    assert_zero_alloc("request_scheduler_hot_operations", || {
        scheduler
            .admit_with_prefix(7, None, &prefixes, &mut pool)
            .unwrap();
        assert_eq!(scheduler.runnable_into(&mut views).unwrap(), 1);
        scheduler.finish(7, &mut pool).unwrap();
    });
}

#[test]
fn duplicate_and_capacity_fail_closed() {
    let mut pool = BlockPool::new(2);
    let prefixes = PrefixKvStore::<1, 1>::new();
    let mut scheduler = RequestScheduler::<1>::new(1);
    scheduler
        .admit_with_prefix(1, None, &prefixes, &mut pool)
        .unwrap();
    assert_eq!(
        scheduler.admit_with_prefix(1, None, &prefixes, &mut pool),
        Err(SchedulerError::DuplicateRequest)
    );
    assert_eq!(
        scheduler.admit_with_prefix(2, None, &prefixes, &mut pool),
        Err(SchedulerError::Full)
    );
}

struct OneLaunchBackend {
    calls: u32,
    corrupt_identity: bool,
}

impl RaggedDecodeBackend for OneLaunchBackend {
    fn execute_ragged(
        &mut self,
        items: &[RaggedBatchItem],
        block_tables: &[u32],
        out: &mut [RaggedBatchOutput],
    ) -> Result<RaggedBatchReceipt, RaggedBackendError> {
        if out.len() < items.len() {
            return Err(RaggedBackendError::OutputTooSmall);
        }
        self.calls += 1;
        for (index, item) in items.iter().enumerate() {
            let table_end = item.block_table_offset as usize + item.logical_pages as usize;
            assert!(table_end <= block_tables.len());
            out[index] = RaggedBatchOutput {
                request_id: item.request_id,
                slot: item.slot,
                next_token_id: item.token_id + 1,
            };
        }
        if self.corrupt_identity && !items.is_empty() {
            out[0].request_id ^= 1;
        }
        Ok(RaggedBatchReceipt {
            batch_size: items.len() as u32,
            backend_launches: 1,
            device_to_host_bytes: (items.len() * core::mem::size_of::<u32>()) as u64,
        })
    }
}

#[test]
fn decode_round_lowers_ragged_tables_and_calls_backend_once_without_allocation() {
    let mut pool = BlockPool::new(8);
    let prefixes = PrefixKvStore::<1, 1>::new();
    let mut scheduler = RequestScheduler::<3>::new(4);
    for request_id in [11, 22] {
        scheduler
            .admit_with_prefix(request_id, None, &prefixes, &mut pool)
            .unwrap();
        scheduler.mark_prefill_complete(request_id).unwrap();
        scheduler
            .seed_decode_token(request_id, request_id as u32)
            .unwrap();
    }
    scheduler.ensure_writable(11, 0, &mut pool).unwrap();
    scheduler.ensure_writable(22, 0, &mut pool).unwrap();

    let mut backend = OneLaunchBackend {
        calls: 0,
        corrupt_identity: false,
    };
    let mut items = [RaggedBatchItem::default(); 3];
    let mut tables = [0u32; 12];
    let mut outputs = [RaggedBatchOutput::default(); 3];
    assert_zero_alloc("ragged_scheduler_decode_round", || {
        let receipt = scheduler
            .execute_decode_round(&mut backend, &mut items, &mut tables, &mut outputs)
            .unwrap();
        assert_eq!(receipt.batch_size, 2);
        assert_eq!(receipt.backend_launches, 1);
    });
    assert_eq!(backend.calls, 1);
    assert_eq!(items[0].request_id, 11);
    assert_eq!(items[0].block_table_offset, 0);
    assert_eq!(items[1].request_id, 22);
    assert_eq!(items[1].block_table_offset, 4);

    let mut views = [RequestView {
        request_id: 0,
        slot: 0,
        state: RequestState::Empty,
        token_count: 0,
        prefix_tokens: 0,
    }; 3];
    assert_eq!(scheduler.runnable_into(&mut views).unwrap(), 2);
    assert_eq!(views[0].token_count, 1);
    assert_eq!(views[1].token_count, 1);
}

#[test]
fn decode_round_rejects_mismatched_outputs_before_mutating_requests() {
    let mut pool = BlockPool::new(2);
    let prefixes = PrefixKvStore::<1, 1>::new();
    let mut scheduler = RequestScheduler::<1>::new(1);
    scheduler
        .admit_with_prefix(7, None, &prefixes, &mut pool)
        .unwrap();
    scheduler.mark_prefill_complete(7).unwrap();
    scheduler.seed_decode_token(7, 41).unwrap();
    let mut backend = OneLaunchBackend {
        calls: 0,
        corrupt_identity: true,
    };
    let mut items = [RaggedBatchItem::default(); 1];
    let mut tables = [0u32; 1];
    let mut outputs = [RaggedBatchOutput::default(); 1];
    assert_eq!(
        scheduler.execute_decode_round(&mut backend, &mut items, &mut tables, &mut outputs),
        Err(DecodeRoundError::OutputIdentityMismatch)
    );
    let mut views = [RequestView {
        request_id: 0,
        slot: 0,
        state: RequestState::Empty,
        token_count: 0,
        prefix_tokens: 0,
    }; 1];
    scheduler.runnable_into(&mut views).unwrap();
    assert_eq!(views[0].token_count, 0);
}
