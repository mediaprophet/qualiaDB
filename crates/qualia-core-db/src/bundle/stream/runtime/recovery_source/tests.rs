use super::*;
use crate::bundle::stream::runtime::{RuntimeProfile, TransferRuntime};
use crate::net::{
    peer::runtime::{
        FairScheduler, LeaseTable, OperationTable, ReservationLedger, ResourceBudget,
        ResourceGovernor,
    },
    qdnf::{errors::QdnfError, types::Generation},
};
use crate::storage_driver::{
    ranges::recovery::tests::{limits, locator, material, quota},
    MmapDriver,
};
fn runtime<T>(orphan: bool, run: impl FnOnce(&mut TransferRuntime<'_>) -> T) -> T {
    let cap = ResourceBudget {
        bytes: 64 * 1024 * 1024,
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
    let out = run(&mut rt);
    if orphan {
        assert_eq!(rt.close(), Err(QdnfError::WouldBlock))
    } else {
        assert_eq!(rt.retained_recovery_sources(), 0);
        rt.close().unwrap()
    }
    drop(rt);
    if !orphan {
        assert_eq!(ledger.used().host, ResourceBudget::ZERO)
    }
    out
}
#[test]
fn actual_same_ledger_source_charge_wrong_owner_retry_and_generation_reuse() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = quota(root.path());
    drop(quota.admit_staging([2; 32], 24).unwrap());
    let _run = material(root.path(), [2; 32], true, false);
    let scope = quota.recovery_root_scope().unwrap();
    runtime(false, |rt| {
        let before = rt.parent_used();
        let item = locator(&driver, &scope, [2; 32]);
        let source = rt
            .acquire_recovery_source(&driver, &scope, item, limits())
            .unwrap();
        let first = source.generation;
        assert_eq!(
            rt.parent_used().host.bytes,
            before.host.bytes + RetainedRecoverySource::charged_bytes() as u64
        );
        assert_eq!(rt.parent_used().host.io, before.host.io + 4);
        assert_eq!(rt.recovery_inflight_ceiling(), 29);
        assert_eq!(rt.close(), Err(QdnfError::WouldBlock));
        let error = runtime(false, |other| {
            other.release_recovery_source(source).unwrap_err()
        });
        assert_eq!(error.error, QdnfError::StaleGeneration);
        rt.release_recovery_source(error.source).unwrap();
        assert_eq!(rt.parent_used(), before);
        let source = rt
            .acquire_recovery_source(&driver, &scope, locator(&driver, &scope, [2; 32]), limits())
            .unwrap();
        assert_eq!(source.generation, first.next().unwrap());
        rt.release_recovery_source(source).unwrap();
    });
}
#[test]
fn four_retained_slots_then_reject_before_open_without_budget_leak() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = quota(root.path());
    let mut runs = Vec::new();
    for n in 2..7 {
        drop(quota.admit_staging([n; 32], 24).unwrap());
        runs.push(material(root.path(), [n; 32], true, false));
    }
    let scope = quota.recovery_root_scope().unwrap();
    runtime(false, |rt| {
        let mut sources: [Option<RetainedRecoverySource<'_>>; 4] = core::array::from_fn(|_| None);
        for (i, source) in sources.iter_mut().enumerate() {
            *source = Some(
                rt.acquire_recovery_source(
                    &driver,
                    &scope,
                    locator(&driver, &scope, [i as u8 + 2; 32]),
                    limits(),
                )
                .unwrap(),
            )
        }
        let before = rt.parent_used();
        assert_eq!(rt.retained_recovery_sources(), 4);
        assert_eq!(rt.recovery_inflight_ceiling(), 26);
        assert!(matches!(
            rt.acquire_recovery_source(
                &driver,
                &scope,
                locator(&driver, &scope, [6; 32]),
                limits()
            ),
            Err(SourceError::Runtime(QdnfError::Capacity))
        ));
        assert_eq!(rt.parent_used(), before);
        for source in sources.into_iter().flatten() {
            rt.release_recovery_source(source).unwrap()
        }
    });
}
#[test]
fn open_failure_rolls_back_ram_only_and_source_drop_never_releases_charge() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = quota(root.path());
    drop(quota.admit_staging([2; 32], 24).unwrap());
    let run = material(root.path(), [2; 32], false, false);
    let scope = quota.recovery_root_scope().unwrap();
    runtime(false, |rt| {
        let before = rt.parent_used();
        assert!(matches!(
            rt.acquire_recovery_source(
                &driver,
                &scope,
                locator(&driver, &scope, [2; 32]),
                limits()
            ),
            Err(SourceError::Backend(_))
        ));
        assert_eq!(rt.parent_used(), before);
        assert_eq!(rt.retained_recovery_sources(), 0);
    });
    std::fs::write(run.path().join("proof"), [3, 4]).unwrap();
    runtime(true, |rt| {
        let source = rt
            .acquire_recovery_source(&driver, &scope, locator(&driver, &scope, [2; 32]), limits())
            .unwrap();
        let before = rt.parent_used();
        drop(source);
        assert_eq!(rt.parent_used(), before);
        assert_eq!(rt.retained_recovery_sources(), 1);
        assert_eq!(rt.close(), Err(QdnfError::WouldBlock));
    });
    assert_eq!(quota.charged_usage(), (24, 1));
}
#[test]
fn release_reject_returns_same_owner_and_handles_remain_live_until_correct_retry() {
    let root = tempfile::tempdir().unwrap();
    let driver = MmapDriver::new(root.path());
    let mut quota = quota(root.path());
    let _run = material(root.path(), [2; 32], true, false);
    let scope = quota.recovery_root_scope().unwrap();
    runtime(false, |rt| {
        let mut source = rt
            .acquire_recovery_source(&driver, &scope, locator(&driver, &scope, [2; 32]), limits())
            .unwrap();
        let generation = source.generation;
        source.generation = Generation::ZERO;
        let error = rt.release_recovery_source(source).unwrap_err();
        assert_eq!(error.error, QdnfError::StaleGeneration);
        let mut source = error.source;
        source.generation = generation;
        source.with_read_view(|mut read| {
            let mut data = [0; 2];
            read.read_exact_at(
                crate::storage_driver::ranges::RangeFile::Payload,
                0,
                &mut data,
            )
            .unwrap();
            assert_eq!(data, [1, 2])
        });
        rt.release_recovery_source(source).unwrap();
    });
}
