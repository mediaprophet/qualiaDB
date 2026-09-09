//! One host owner aggregates every cell/role/session/operation charge (E03.2).
//!
//! Extra identities cannot mint a second host budget (E03.6). Leases and ledger
//! charges bind to this owner; copied cell-slot byte fields are not authority.

use crate::governance::webizen::{classify_budget, ArenaBudgetClass};
use crate::net::peer::cells::admit::{
    CellProfile, CellSlot, CellTable, MAX_CELLS, MAX_ORDINARY_CELL,
};
use crate::net::peer::runtime::{
    LeaseTable, ReservationHandle, ReservationLedger, ResourceBudget,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

const HOST_WORK: u64 = 1024;
const HOST_IO: u64 = 1024;
const MAX_TRACKED_IDENTITIES: usize = 8;

/// Aggregate host byte cap: one owner, up to [`MAX_CELLS`] ordinary cells.
/// Per-cell ceiling remains [`MAX_ORDINARY_CELL`]; this is accounting, not RSS.
pub const MAX_HOST_BYTES: u64 = MAX_CELLS as u64 * MAX_ORDINARY_CELL;

/// Aggregate admission owner for one host process.
pub struct HostAdmission {
    ledger: ReservationLedger,
    leases: LeaseTable,
    cells: CellTable,
    host_bytes: u64,
    identities: [StrongDigest; MAX_TRACKED_IDENTITIES],
    identity_count: u8,
}

impl HostAdmission {
    pub fn new(host_bytes: u64) -> Result<Self, QdnfError> {
        if host_bytes == 0 || host_bytes > MAX_HOST_BYTES {
            return Err(QdnfError::Capacity);
        }
        let cap = ResourceBudget {
            bytes: host_bytes,
            work: HOST_WORK,
            io: HOST_IO,
        };
        Ok(Self {
            ledger: ReservationLedger::new(cap, cap, cap, cap, cap),
            leases: LeaseTable::new(),
            cells: CellTable::new(),
            host_bytes,
            identities: [StrongDigest::ZERO; MAX_TRACKED_IDENTITIES],
            identity_count: 0,
        })
    }

    #[inline]
    pub const fn host_bytes(&self) -> u64 {
        self.host_bytes
    }

    #[inline]
    pub fn remaining_host_bytes(&self) -> u64 {
        self.ledger.host_remaining_bytes()
    }

    pub fn profile_for_bytes(bytes: u64) -> Result<CellProfile, QdnfError> {
        if bytes == 0 || bytes > MAX_ORDINARY_CELL {
            return Err(QdnfError::Capacity);
        }
        match classify_budget(bytes) {
            Ok(ArenaBudgetClass::SentinelPass) => Ok(CellProfile::SentinelPass),
            Ok(ArenaBudgetClass::OrdinaryCell) => Ok(CellProfile::Ordinary),
            Ok(ArenaBudgetClass::Rejected) => Err(QdnfError::Capacity),
            Err(_) => Ok(CellProfile::NetworkSmall),
        }
    }

    /// Admit one cell against the shared host ledger.
    pub fn admit_cell(
        &mut self,
        profile: CellProfile,
        bytes: u64,
    ) -> Result<CellSlot, QdnfError> {
        self.cells
            .reserve(&mut self.leases, &mut self.ledger, profile, bytes)
    }

    pub fn release_cell(&mut self, slot: CellSlot) -> Result<(), QdnfError> {
        self.cells
            .release(&mut self.leases, &mut self.ledger, slot)
    }

    #[inline]
    pub fn occupied_cells(&self) -> usize {
        self.cells.occupied()
    }

    /// Charge work for a peer identity. The identity is recorded; it does not
    /// receive an independent host cap.
    pub fn charge(
        &mut self,
        identity: StrongDigest,
        add: ResourceBudget,
        verified_peer: bool,
    ) -> Result<ReservationHandle, QdnfError> {
        if identity.is_zero() {
            return Err(QdnfError::Unauthorized);
        }
        self.remember(identity)?;
        self.ledger.reserve(add, verified_peer)
    }

    pub fn release_charge(&mut self, handle: ReservationHandle) -> Result<(), QdnfError> {
        self.ledger.release(handle)
    }

    fn remember(&mut self, identity: StrongDigest) -> Result<(), QdnfError> {
        let n = self.identity_count as usize;
        let mut i = 0usize;
        while i < n {
            if self.identities[i] == identity {
                return Ok(());
            }
            i += 1;
        }
        if n >= MAX_TRACKED_IDENTITIES {
            return Ok(());
        }
        self.identities[n] = identity;
        self.identity_count = (n as u8).saturating_add(1);
        Ok(())
    }

    #[inline]
    pub const fn tracked_identities(&self) -> u8 {
        self.identity_count
    }
}

/// Extra DIDs/controllers cannot multiply the host byte cap.
#[inline]
pub fn extra_identity_multiplies_host_budget() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(b: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = b;
        d.0[47] = b ^ 0xff;
        d
    }

    #[test]
    fn extra_identity_cannot_multiply_host_budget() {
        let mut host = HostAdmission::new(100).unwrap();
        let add = ResourceBudget {
            bytes: 60,
            work: 1,
            io: 1,
        };
        let a = host.charge(digest(1), add, true).unwrap();
        assert_eq!(host.tracked_identities(), 1);
        assert_eq!(
            host.charge(digest(2), add, true).unwrap_err(),
            QdnfError::BudgetExhausted
        );
        assert_eq!(host.tracked_identities(), 2);
        assert!(!extra_identity_multiplies_host_budget());
        host.release_charge(a).unwrap();
        host.charge(digest(2), add, true).unwrap();
        assert_eq!(host.remaining_host_bytes(), 40);
    }

    #[test]
    fn zero_identity_cannot_charge() {
        let mut host = HostAdmission::new(100).unwrap();
        let add = ResourceBudget {
            bytes: 10,
            work: 1,
            io: 1,
        };
        assert_eq!(
            host.charge(StrongDigest::ZERO, add, true).unwrap_err(),
            QdnfError::Unauthorized
        );
        assert_eq!(host.remaining_host_bytes(), 100);
    }

    #[test]
    fn cell_admission_uses_shared_host_ledger() {
        let mut host = HostAdmission::new(2 * 1024 * 1024).unwrap();
        let profile = CellProfile::NetworkSmall;
        let first = host.admit_cell(profile, 1024 * 1024).unwrap();
        assert_eq!(
            host.admit_cell(profile, 1024 * 1024 + 1).unwrap_err(),
            QdnfError::BudgetExhausted
        );
        host.release_cell(first).unwrap();
        host.admit_cell(profile, 1024 * 1024).unwrap();
    }

    #[test]
    fn host_zero_or_over_aggregate_is_capacity() {
        assert_eq!(HostAdmission::new(0).err(), Some(QdnfError::Capacity));
        assert_eq!(
            HostAdmission::new(MAX_HOST_BYTES + 1).err(),
            Some(QdnfError::Capacity)
        );
        HostAdmission::new(MAX_ORDINARY_CELL + 1).expect("host aggregates more than one cell");
    }
}
