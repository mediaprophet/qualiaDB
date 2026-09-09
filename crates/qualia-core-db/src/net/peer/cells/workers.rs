//! Worker start / fail / drain / handover with generation fencing (E10.3).
//!
//! Handover temporarily double-reserves the cell bytes on the shared
//! [`HostAdmission`], then releases the old slot. Cancel during drain returns
//! [`QdnfError::Cancelled`] and leaves the old reservation until
//! [`release_worker`]. Stale generation cannot complete handover.

use super::admit::{CellProfile, CellSlot};
use super::host_owner::HostAdmission;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

/// Exclusive writer fence. Not `Copy`: two live writers are [`QdnfError::Conflict`].
pub struct WorkerFence {
    pub generation: Generation,
    pub draining: bool,
    epoch: Generation,
    slot: Option<CellSlot>,
    cancelled: bool,
    failed: bool,
    writer: bool,
}

impl WorkerFence {
    pub const fn vacant() -> Self {
        Self {
            generation: Generation::ZERO,
            draining: false,
            epoch: Generation::ZERO,
            slot: None,
            cancelled: false,
            failed: false,
            writer: false,
        }
    }

    #[inline]
    pub const fn epoch(&self) -> Generation {
        self.epoch
    }

    #[inline]
    pub const fn cancelled(&self) -> bool {
        self.cancelled
    }

    #[inline]
    pub const fn failed(&self) -> bool {
        self.failed
    }

    #[inline]
    pub const fn is_writer(&self) -> bool {
        self.writer
    }
}

/// Admit a cell and take exclusive writer ownership.
pub fn start_worker(
    host: &mut HostAdmission,
    profile: CellProfile,
    bytes: u64,
) -> Result<WorkerFence, QdnfError> {
    let slot = host.admit_cell(profile, bytes)?;
    let generation = Generation::ZERO.next()?;
    Ok(WorkerFence {
        generation,
        draining: false,
        epoch: generation,
        slot: Some(slot),
        cancelled: false,
        failed: false,
        writer: true,
    })
}

/// Mark failure and begin drain. Vacant fence → [`QdnfError::Incomplete`].
pub fn fail_worker(worker: &mut WorkerFence) -> Result<(), QdnfError> {
    if !worker.writer || worker.slot.is_none() {
        return Err(QdnfError::Incomplete);
    }
    if worker.generation != worker.epoch {
        return Err(QdnfError::StaleGeneration);
    }
    worker.failed = true;
    worker.draining = true;
    worker.epoch = worker.epoch.next()?;
    worker.generation = worker.epoch;
    Ok(())
}

/// Cancel during drain (E03.5). Old reservation remains until [`release_worker`].
pub fn cancel_worker(worker: &mut WorkerFence) -> Result<(), QdnfError> {
    if worker.slot.is_none() {
        return Err(QdnfError::Incomplete);
    }
    if worker.generation != worker.epoch {
        return Err(QdnfError::StaleGeneration);
    }
    worker.cancelled = true;
    worker.draining = true;
    Ok(())
}

/// Release the worker cell on success, error, cancel, or unwind (E03.5).
pub fn release_worker(
    worker: &mut WorkerFence,
    host: &mut HostAdmission,
) -> Result<(), QdnfError> {
    let slot = match worker.slot.take() {
        Some(s) => s,
        None => return Err(QdnfError::DoubleRelease),
    };
    worker.writer = false;
    worker.draining = false;
    host.release_cell(slot)
}

/// Drain `old` and hand exclusive ownership to `new`.
///
/// Temporary double-reservation of the cell bytes, then release of `old`.
/// Cancelled drain leaves `old` reserved. Two live writers → Conflict.
pub fn drain_and_handover(
    old: &mut WorkerFence,
    new: &mut WorkerFence,
    host: &mut HostAdmission,
) -> Result<(), QdnfError> {
    if old.generation != old.epoch {
        return Err(QdnfError::StaleGeneration);
    }
    if new.generation != new.epoch {
        return Err(QdnfError::StaleGeneration);
    }
    let old_slot = match old.slot {
        Some(s) if old.writer => s,
        _ => return Err(QdnfError::Incomplete),
    };
    if new.writer || new.slot.is_some() {
        return Err(QdnfError::Conflict);
    }
    old.draining = true;
    if old.cancelled {
        return Err(QdnfError::Cancelled);
    }
    let before = host.remaining_host_bytes();
    let fresh = match host.admit_cell(old_slot.profile, old_slot.bytes) {
        Ok(s) => s,
        Err(e) => return Err(e),
    };
    let after_double = host.remaining_host_bytes();
    if after_double != before.saturating_sub(old_slot.bytes) {
        let _ = host.release_cell(fresh);
        return Err(QdnfError::Incomplete);
    }
    host.release_cell(old_slot)?;
    old.slot = None;
    old.writer = false;
    old.draining = false;
    let generation = old.epoch.next()?;
    *new = WorkerFence {
        generation,
        draining: false,
        epoch: generation,
        slot: Some(fresh),
        cancelled: false,
        failed: false,
        writer: true,
    };
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::cells::admit::CellProfile;
    use crate::net::qdnf::errors::QdnfError;

    const MIB: u64 = 1024 * 1024;

    #[test]
    fn handover_double_reserves_then_releases_old() {
        let mut host = HostAdmission::new(2 * MIB).unwrap();
        let mut old = start_worker(&mut host, CellProfile::NetworkSmall, MIB).unwrap();
        assert_eq!(host.remaining_host_bytes(), MIB);
        assert_eq!(host.occupied_cells(), 1);
        let mut new = WorkerFence::vacant();
        drain_and_handover(&mut old, &mut new, &mut host).expect("handover");
        assert!(!old.is_writer());
        assert!(new.is_writer());
        assert_eq!(host.occupied_cells(), 1);
        assert_eq!(host.remaining_host_bytes(), MIB);
        release_worker(&mut new, &mut host).unwrap();
        assert_eq!(host.occupied_cells(), 0);
    }

    #[test]
    fn cancel_during_drain_keeps_old_reservation() {
        let mut host = HostAdmission::new(2 * MIB).unwrap();
        let mut old = start_worker(&mut host, CellProfile::NetworkSmall, MIB).unwrap();
        cancel_worker(&mut old).unwrap();
        let mut new = WorkerFence::vacant();
        assert_eq!(
            drain_and_handover(&mut old, &mut new, &mut host),
            Err(QdnfError::Cancelled)
        );
        assert!(old.is_writer());
        assert_eq!(host.occupied_cells(), 1);
        assert_eq!(host.remaining_host_bytes(), MIB);
        release_worker(&mut old, &mut host).unwrap();
        assert_eq!(host.occupied_cells(), 0);
    }

    #[test]
    fn stale_generation_cannot_handover() {
        let mut host = HostAdmission::new(2 * MIB).unwrap();
        let mut old = start_worker(&mut host, CellProfile::NetworkSmall, MIB).unwrap();
        old.generation = Generation(99);
        let mut new = WorkerFence::vacant();
        assert_eq!(
            drain_and_handover(&mut old, &mut new, &mut host),
            Err(QdnfError::StaleGeneration)
        );
        old.generation = old.epoch();
        release_worker(&mut old, &mut host).unwrap();
    }

    #[test]
    fn two_writers_conflict() {
        let mut host = HostAdmission::new(2 * MIB).unwrap();
        let mut old = start_worker(&mut host, CellProfile::NetworkSmall, MIB).unwrap();
        let mut other = start_worker(&mut host, CellProfile::NetworkSmall, MIB).unwrap();
        assert_eq!(
            drain_and_handover(&mut old, &mut other, &mut host),
            Err(QdnfError::Conflict)
        );
        release_worker(&mut old, &mut host).unwrap();
        release_worker(&mut other, &mut host).unwrap();
    }

    #[test]
    fn vacant_drain_is_incomplete() {
        let mut host = HostAdmission::new(MIB).unwrap();
        let mut old = WorkerFence::vacant();
        let mut new = WorkerFence::vacant();
        assert_eq!(
            drain_and_handover(&mut old, &mut new, &mut host),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn double_reserve_needs_host_headroom() {
        let mut host = HostAdmission::new(MIB).unwrap();
        let mut old = start_worker(&mut host, CellProfile::NetworkSmall, MIB).unwrap();
        let mut new = WorkerFence::vacant();
        assert_eq!(
            drain_and_handover(&mut old, &mut new, &mut host),
            Err(QdnfError::BudgetExhausted)
        );
        assert_eq!(host.occupied_cells(), 1);
        release_worker(&mut old, &mut host).unwrap();
    }

    #[test]
    fn fail_worker_starts_drain() {
        let mut host = HostAdmission::new(2 * MIB).unwrap();
        let mut old = start_worker(&mut host, CellProfile::NetworkSmall, MIB).unwrap();
        fail_worker(&mut old).unwrap();
        assert!(old.failed());
        assert!(old.draining);
        let mut new = WorkerFence::vacant();
        drain_and_handover(&mut old, &mut new, &mut host).unwrap();
        release_worker(&mut new, &mut host).unwrap();
    }
}
