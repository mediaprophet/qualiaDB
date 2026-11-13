use super::*;
use crate::bundle::stream::{
    descriptor::{EntryRecord, HASH_SCRATCH_BYTES},
    index::{EntryScratch, ExtentLimit, IndexReadLimits, admit_header, admit_index},
    policy::{BoundClosure, CLOSURE_PURPOSE, adopt_local_whole_source, tests::Fixture},
    runtime::{
        ColdPlan, RuntimePoll, RuntimeProfile, SparsePageWorkspace, TransferRequirements,
        TransferRuntime, TransferSpec, TransferTicket,
    },
    selection::select_entry,
};
use crate::crypto::network::authority::PinAuthority;
use crate::net::{
    peer::runtime::{
        FairScheduler, LeaseTable, OperationTable, ReservationLedger, ResourceBudget,
        ResourceGovernor, WorkClass,
    },
    qdnf::{
        authority::AuthorityOwner,
        types::{Generation, OperationId, StrongDigest},
    },
};
use crate::sensing::local_custody::quota::{
    CustodyQuota, QuotaLimits, QuotaSnapshot, QuotaSnapshotAuthority,
};
use crate::storage_driver::MmapDriver;
struct Snapshot;
impl QuotaSnapshotAuthority for Snapshot {
    fn admit_usage(&self, _: &std::path::Path, _: QuotaLimits, s: QuotaSnapshot) -> bool {
        s.id == [1; 32]
    }
}
pub(super) fn fixture(
    run: impl for<'d, 'r> FnOnce(
        NativeSparseStore<'d, 'r>,
        &mut SparsePageWorkspace,
        &mut TransferRuntime<'_>,
        &PinAuthority,
        &BoundClosure<'_>,
        &Fixture,
        &[u8],
        &'d MmapDriver,
        &mut CustodyQuota<'r, 4>,
        SelectedStoreBinding,
    ) -> NativeSparseStore<'d, 'r>,
) {
    fixture_with_payload(b"actual-source-payload", run)
}

pub(super) fn fixture_with_payload(
    data: &[u8],
    run: impl for<'d, 'r> FnOnce(
        NativeSparseStore<'d, 'r>,
        &mut SparsePageWorkspace,
        &mut TransferRuntime<'_>,
        &PinAuthority,
        &BoundClosure<'_>,
        &Fixture,
        &[u8],
        &'d MmapDriver,
        &mut CustodyQuota<'r, 4>,
        SelectedStoreBinding,
    ) -> NativeSparseStore<'d, 'r>,
) {
    let root = tempfile::tempdir().unwrap();
    fixture_at(data, root.path(), [2; 32], 0, run);
}

pub(super) fn fixture_at(
    data: &[u8],
    root: &std::path::Path,
    attempt: [u8; 32],
    prior_usage: u64,
    run: impl for<'d, 'r> FnOnce(
        NativeSparseStore<'d, 'r>,
        &mut SparsePageWorkspace,
        &mut TransferRuntime<'_>,
        &PinAuthority,
        &BoundClosure<'_>,
        &Fixture,
        &[u8],
        &'d MmapDriver,
        &mut CustodyQuota<'r, 4>,
        SelectedStoreBinding,
    ) -> NativeSparseStore<'d, 'r>,
) {
    let f = Fixture::new_with_payload(data);
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let source = f.source();
    let limits = IndexReadLimits::default();
    let header = admit_header(
        &source[..64],
        source.len() as u64,
        ExtentLimit::default(),
        limits,
    )
    .unwrap();
    let mut index_scratch = [EntryScratch::EMPTY; 1];
    let index = admit_index(
        &header,
        &source[header.index_offset() as usize..],
        limits,
        &mut index_scratch,
    )
    .unwrap();
    let mut rows = [EntryRecord::EMPTY; 128];
    let mut framing = vec![0; HASH_SCRATCH_BYTES];
    let entry = select_entry(
        &bound,
        f.body(),
        &source[..64],
        &index,
        f.entry_page(),
        0,
        &mut rows,
        &mut framing,
    )
    .unwrap();
    let payload =
        &source[entry.offset() as usize..(entry.offset() + entry.logical_length()) as usize];
    let binding = SelectedStoreBinding::from_selected(
        &entry,
        StorePartition {
            privacy: StrongDigest([8; 48]),
            policy_generation: Generation(1),
            storage_generation: 1,
        },
    )
    .unwrap();
    let layout = StoreLayout::for_binding(&binding).unwrap();
    let allowance = layout.aggregate_allowance().unwrap();
    let driver = MmapDriver::new(root);
    let mut quota = CustodyQuota::<4>::from_authenticated_usage(
        root,
        QuotaLimits {
            encoded_bytes: allowance * 4,
            entries: 4,
        },
        QuotaSnapshot {
            id: [1; 32],
            encoded_bytes: allowance.checked_mul(prior_usage).unwrap(),
            entries: prior_usage,
        },
        &Snapshot,
    )
    .unwrap();
    let session = driver
        .create_range_staging(
            quota.admit_staging(attempt, allowance).unwrap(),
            layout.range_limits(),
        )
        .unwrap();
    let cap = ResourceBudget {
        bytes: 64 * 1024 * 1024,
        work: 100000,
        io: 100000,
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
    let mut workspace = rt.acquire_sparse_page(*binding.identity(), 0).unwrap();
    let store = NativeSparseStore::new(session, binding, &mut workspace).unwrap();
    let store = run(
        store,
        &mut workspace,
        &mut rt,
        &pins,
        &bound,
        &f,
        payload,
        &driver,
        &mut quota,
        binding,
    );
    store.close().unwrap();
    assert_eq!(
        quota.charged_usage().1,
        if quota.charged_usage().0 == allowance {
            1
        } else {
            2
        }
    );
    rt.release_sparse_page(workspace).unwrap();
    rt.close().unwrap();
    drop(rt);
    assert_eq!(ledger.used().host, ResourceBudget::ZERO);
    assert_eq!(leases.occupied_count(), 0);
}
pub(super) fn ticket(rt: &mut TransferRuntime<'_>, bytes: usize, n: u8) -> TransferTicket {
    fn part(bytes: u64) -> ResourceBudget {
        ResourceBudget {
            bytes,
            work: 1,
            io: 1,
        }
    }
    let rows = [TransferSpec {
        operation: OperationId([n; 16]),
        principal: 1,
        policy_generation: Generation(1),
        deadline: 100,
        class: WorkClass::Background,
        cost: 1,
        credit_bytes: bytes.max(1) as u32,
        verified_peer: true,
        requirements: TransferRequirements {
            network: part(bytes.max(1) as u64),
            proof: part(PROOF_SLOT_BYTES as u64),
            storage: part((READ_SCRATCH_BYTES + 32 * 48) as u64),
            consumer: part(bytes.max(1) as u64),
            scratch: part(4096),
        },
    }];
    let plan = ColdPlan::compile(&rows).unwrap();
    let ticket = rt.admit(&plan, 0, 20, Generation(1)).unwrap();
    assert!(matches!(rt.poll(20).unwrap(), RuntimePoll::Ready(_)));
    let _credit = rt.issue_credit(&ticket, 20, Generation(1)).unwrap();
    ticket
}
pub(super) fn finish(rt: &mut TransferRuntime<'_>, ticket: &mut TransferTicket) {
    let key = ticket.completion_key();
    rt.complete_io(ticket, key, 20).unwrap();
}
pub(super) fn write(
    store: &mut NativeSparseStore<'_, '_>,
    workspace: &mut SparsePageWorkspace,
    rt: &mut TransferRuntime<'_>,
    pins: &PinAuthority,
    bound: &BoundClosure<'_>,
    f: &Fixture,
    payload: &[u8],
    n: u8,
) -> Result<LeafBarrierObservation, StoreError> {
    let mut op = ticket(rt, payload.len(), n);
    let mut scratch = vec![0; READ_SCRATCH_BYTES];
    let mut nodes = [StrongDigest::ZERO; 32];
    let mut owner = AuthorityOwner::new();
    let result = {
        let mut grant = adopt_local_whole_source(&mut owner, pins, bound, f.request()).unwrap();
        let mut stage = rt
            .storage_stage(&op, payload, &[0], &mut scratch, 20, Generation(1))
            .unwrap();
        store.write_leaf(
            &mut stage,
            &mut grant,
            pins,
            workspace,
            0,
            &mut nodes,
            &|| 20,
            &|| Generation(1),
        )
    };
    finish(rt, &mut op);
    result
}
