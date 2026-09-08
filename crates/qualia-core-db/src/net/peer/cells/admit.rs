//! Cell-slot reserve/release. Ordinary cells cannot exceed 512 MiB.
//!
//! RT-02 slice (partial, packages remain open):
//! - RT-02.01 partial — reuse RT-01 leases/ledger; no third scheduler.
//! - RT-02.02 partial — reserve before construction; 512 MiB ordinary ceiling; `MAX_CELLS = 4`.
//! - RT-02.03 partial — `NetworkSmall` cannot silently take a 42 MiB Sentinel pass.
//! - RT-02.05 partial — `LlmException` is tagged and isolated (`llm_shares_ordinary_network_reserve` is false);
//!   this slice still refuses > 512 MiB (full LLM profiles are not admitted here).
//!
//! Not in this slice: RT-02.04 measurement split, RT-02.06–02.14 IPC/handoff/platform/throughput.

use crate::governance::webizen::{
    classify_budget, ArenaBudgetClass, ORDINARY_CELL_BYTES, SENTINEL_PASS_BYTES,
};
use crate::net::peer::runtime::{BufferLease, LeaseTable, ReservationLedger, ResourceBudget};
use crate::net::qdnf::errors::QdnfError;

/// Ordinary cell ceiling. Same value as [`ORDINARY_CELL_BYTES`].
pub const MAX_ORDINARY_CELL: u64 = ORDINARY_CELL_BYTES as u64;
/// Occupied cell slots. Tests cover 1/2/4; a 5th remains [`QdnfError::Capacity`].
pub const MAX_CELLS: usize = 4;

/// Local cell admission class. `LlmException` may request the OrdinaryCell size class
/// but stays tagged so it is not mixed with ordinary networking.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CellProfile {
    NetworkSmall = 0,
    SentinelPass = 1,
    Ordinary = 2,
    LlmException = 3,
}

/// One admitted cell. Caller holds the slot token until [`CellTable::release`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellSlot {
    pub id: u8,
    pub profile: CellProfile,
    pub bytes: u64,
    pub lease: BufferLease,
}

/// Four occupancy flags. Does not own a scheduler or arena.
pub struct CellTable {
    occupied: [bool; MAX_CELLS],
}

impl CellTable {
    pub const fn new() -> Self {
        Self {
            occupied: [false; MAX_CELLS],
        }
    }

    /// Reserve **before** construction. Ordinary cells cannot exceed 512 MiB.
    ///
    /// Ledger charge uses [`ReservationLedger::reserve`] / [`ReservationLedger::release`]
    /// with a [`ResourceBudget`] of `bytes` (work/io = 0) and `verified_peer = true`.
    /// Lease acquire is all-or-nothing: a failed [`LeaseTable::acquire`] releases the ledger charge.
    pub fn reserve(
        &mut self,
        leases: &mut LeaseTable,
        ledger: &mut ReservationLedger,
        profile: CellProfile,
        bytes: u64,
    ) -> Result<CellSlot, QdnfError> {
        if bytes == 0 || bytes > MAX_ORDINARY_CELL {
            return Err(QdnfError::Capacity);
        }
        match profile {
            CellProfile::NetworkSmall => {
                // Cannot silently allocate a full Sentinel arena.
                if bytes >= SENTINEL_PASS_BYTES as u64 {
                    return Err(QdnfError::Capacity);
                }
            }
            CellProfile::SentinelPass => match classify_budget(bytes)? {
                ArenaBudgetClass::SentinelPass => {}
                ArenaBudgetClass::OrdinaryCell | ArenaBudgetClass::Rejected => {
                    return Err(QdnfError::Capacity);
                }
            },
            CellProfile::Ordinary => match classify_budget(bytes)? {
                ArenaBudgetClass::OrdinaryCell => {}
                ArenaBudgetClass::SentinelPass | ArenaBudgetClass::Rejected => {
                    return Err(QdnfError::Capacity);
                }
            },
            CellProfile::LlmException => {
                // OrdinaryCell class only; remains tagged LlmException (RT-02.05 partial).
                match classify_budget(bytes)? {
                    ArenaBudgetClass::OrdinaryCell => {}
                    ArenaBudgetClass::SentinelPass | ArenaBudgetClass::Rejected => {
                        return Err(QdnfError::Capacity);
                    }
                }
            }
        }

        let id = match self.free_id() {
            Some(id) => id,
            None => return Err(QdnfError::Capacity),
        };
        let capacity = u32::try_from(bytes).map_err(|_| QdnfError::Capacity)?;
        let budget = cell_bytes_budget(bytes);
        ledger.reserve(budget, true)?;
        let lease = match leases.acquire(capacity, true) {
            Ok(lease) => lease,
            Err(e) => {
                let _ = ledger.release(budget, true);
                return Err(e);
            }
        };
        self.occupied[id as usize] = true;
        Ok(CellSlot {
            id,
            profile,
            bytes,
            lease,
        })
    }

    pub fn release(
        &mut self,
        leases: &mut LeaseTable,
        ledger: &mut ReservationLedger,
        slot: CellSlot,
    ) -> Result<(), QdnfError> {
        let idx = slot.id as usize;
        if idx >= MAX_CELLS {
            return Err(QdnfError::Range);
        }
        if !self.occupied[idx] {
            return Err(QdnfError::DoubleRelease);
        }
        leases.release(slot.lease.handle)?;
        ledger.release(cell_bytes_budget(slot.bytes), true)?;
        self.occupied[idx] = false;
        Ok(())
    }

    pub fn occupied(&self) -> usize {
        let mut n = 0;
        let mut i = 0;
        while i < MAX_CELLS {
            if self.occupied[i] {
                n += 1;
            }
            i += 1;
        }
        n
    }

    fn free_id(&self) -> Option<u8> {
        let mut i = 0;
        while i < MAX_CELLS {
            if !self.occupied[i] {
                return Some(i as u8);
            }
            i += 1;
        }
        None
    }
}

impl Default for CellTable {
    fn default() -> Self {
        Self::new()
    }
}

/// LLM/exception profiles do not consume the ordinary networking reserve (RT-02.05 partial).
pub fn llm_shares_ordinary_network_reserve() -> bool {
    false
}

fn cell_bytes_budget(bytes: u64) -> ResourceBudget {
    ResourceBudget {
        bytes,
        work: 0,
        io: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cell_ledger() -> ReservationLedger {
        let cap = ResourceBudget {
            bytes: MAX_ORDINARY_CELL * MAX_CELLS as u64,
            work: 64,
            io: 64,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    const MIB: u64 = 1024 * 1024;
    const ORDINARY_64: u64 = 64 * MIB;

    #[test]
    fn network_small_one_mib_ok_forty_two_mib_capacity() {
        let mut table = CellTable::new();
        let mut leases = LeaseTable::new();
        let mut ledger = cell_ledger();
        let slot = table
            .reserve(&mut leases, &mut ledger, CellProfile::NetworkSmall, MIB)
            .expect("1 MiB NetworkSmall");
        assert_eq!(slot.profile, CellProfile::NetworkSmall);
        assert_eq!(slot.bytes, MIB);
        assert_eq!(table.occupied(), 1);
        assert_eq!(
            table.reserve(
                &mut leases,
                &mut ledger,
                CellProfile::NetworkSmall,
                SENTINEL_PASS_BYTES as u64
            ),
            Err(QdnfError::Capacity)
        );
        assert_eq!(table.occupied(), 1);
    }

    #[test]
    fn ordinary_over_512_mib_is_capacity() {
        let mut table = CellTable::new();
        let mut leases = LeaseTable::new();
        let mut ledger = cell_ledger();
        assert_eq!(
            table.reserve(
                &mut leases,
                &mut ledger,
                CellProfile::Ordinary,
                MAX_ORDINARY_CELL + 1
            ),
            Err(QdnfError::Capacity)
        );
        let slot = table
            .reserve(
                &mut leases,
                &mut ledger,
                CellProfile::Ordinary,
                MAX_ORDINARY_CELL,
            )
            .expect("512 MiB Ordinary is the ceiling");
        assert_eq!(slot.profile, CellProfile::Ordinary);
        assert_eq!(table.occupied(), 1);
    }

    #[test]
    fn one_two_four_cells_ok_fifth_capacity() {
        let mut table = CellTable::new();
        let mut leases = LeaseTable::new();
        let mut ledger = cell_ledger();
        let a = table
            .reserve(&mut leases, &mut ledger, CellProfile::Ordinary, ORDINARY_64)
            .expect("cell 1");
        assert_eq!(table.occupied(), 1);
        let b = table
            .reserve(&mut leases, &mut ledger, CellProfile::Ordinary, ORDINARY_64)
            .expect("cell 2");
        assert_eq!(table.occupied(), 2);
        let c = table
            .reserve(&mut leases, &mut ledger, CellProfile::Ordinary, ORDINARY_64)
            .expect("cell 3");
        let d = table
            .reserve(&mut leases, &mut ledger, CellProfile::Ordinary, ORDINARY_64)
            .expect("cell 4");
        assert_eq!(table.occupied(), 4);
        assert_eq!(leases.occupied_count(), 4);
        assert_eq!(
            table.reserve(&mut leases, &mut ledger, CellProfile::Ordinary, ORDINARY_64,),
            Err(QdnfError::Capacity)
        );
        assert_eq!(table.occupied(), 4);
        assert_eq!(ledger.used().cell.bytes, ORDINARY_64 * 4);
        let _ = (a, b, c, d);
    }

    #[test]
    fn llm_does_not_share_ordinary_network_reserve() {
        assert!(!llm_shares_ordinary_network_reserve());
        let mut table = CellTable::new();
        let mut leases = LeaseTable::new();
        let mut ledger = cell_ledger();
        let slot = table
            .reserve(
                &mut leases,
                &mut ledger,
                CellProfile::LlmException,
                ORDINARY_64,
            )
            .expect("LlmException may request OrdinaryCell class");
        assert_eq!(slot.profile, CellProfile::LlmException);
        assert_ne!(slot.profile, CellProfile::Ordinary);
        assert_eq!(
            table.reserve(
                &mut leases,
                &mut ledger,
                CellProfile::LlmException,
                MAX_ORDINARY_CELL + 1
            ),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn release_restores_occupancy_and_lease() {
        let mut table = CellTable::new();
        let mut leases = LeaseTable::new();
        let mut ledger = cell_ledger();
        let slot = table
            .reserve(&mut leases, &mut ledger, CellProfile::Ordinary, ORDINARY_64)
            .expect("reserve");
        assert_eq!(table.occupied(), 1);
        assert_eq!(leases.occupied_count(), 1);
        table
            .release(&mut leases, &mut ledger, slot)
            .expect("release");
        assert_eq!(table.occupied(), 0);
        assert_eq!(leases.occupied_count(), 0);
        assert_eq!(ledger.used().cell.bytes, 0);
        let again = table
            .reserve(&mut leases, &mut ledger, CellProfile::Ordinary, ORDINARY_64)
            .expect("reserve after release");
        assert_eq!(table.occupied(), 1);
        assert_eq!(again.id, 0);
        table
            .release(&mut leases, &mut ledger, again)
            .expect("second release");
        assert_eq!(
            table.release(&mut leases, &mut ledger, again),
            Err(QdnfError::DoubleRelease)
        );
    }

    #[test]
    fn sentinel_pass_exact_42_mib_ok_41_mib_capacity() {
        let mut table = CellTable::new();
        let mut leases = LeaseTable::new();
        let mut ledger = cell_ledger();
        let slot = table
            .reserve(
                &mut leases,
                &mut ledger,
                CellProfile::SentinelPass,
                SENTINEL_PASS_BYTES as u64,
            )
            .expect("exact 42 MiB SentinelPass");
        assert_eq!(slot.profile, CellProfile::SentinelPass);
        assert_eq!(slot.bytes, SENTINEL_PASS_BYTES as u64);
        assert_eq!(
            classify_budget(slot.bytes),
            Ok(ArenaBudgetClass::SentinelPass)
        );
        assert_eq!(
            table.reserve(
                &mut leases,
                &mut ledger,
                CellProfile::SentinelPass,
                SENTINEL_PASS_BYTES as u64 - MIB
            ),
            Err(QdnfError::Capacity)
        );
        assert_eq!(table.occupied(), 1);
    }
}
