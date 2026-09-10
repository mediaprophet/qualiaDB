//! In-process 1..=16 cell scaling under one host budget (E10.5).
//!
//! Admission is fair-share: a greedy request is capped so remaining cohort
//! cells keep an equal floor of leftover bytes. RSS and NUMA live in
//! [`super::hardware`]. Idle joules stay unmeasured.
//!
//! Tests charge KiB-scale [`CellProfile::NetworkSmall`] slots. The 42 MiB
//! Sentinel figure stays in [`super::pass_budget`] as accounting.

use crate::net::peer::cells::admit::{CellProfile, CellSlot, MAX_CELLS};
use crate::net::peer::cells::host_owner::HostAdmission;
use crate::net::qdnf::errors::QdnfError;

/// Production occupancy. A 17th cell is [`QdnfError::Capacity`].
pub const MAX_SCALE_CELLS: usize = MAX_CELLS;

/// Hardware counters this slice does not collect. Never treat as zero.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HardwareObservation {
    Unsupported = 0,
    Unmeasured = 1,
}

/// Equal-share cap for one request among `remaining_cells` still to admit.
///
/// `remaining_cells` includes the caller. Others keep at least
/// `floor(remaining_bytes / remaining_cells)` each. Remainder may go to this
/// request. 0 or more than [`MAX_SCALE_CELLS`] is [`QdnfError::Capacity`].
pub fn fair_grant(
    remaining_bytes: u64,
    remaining_cells: usize,
    requested: u64,
) -> Result<u64, QdnfError> {
    if remaining_cells == 0 || remaining_cells > MAX_SCALE_CELLS {
        return Err(QdnfError::Capacity);
    }
    if requested == 0 {
        return Err(QdnfError::Capacity);
    }
    let n = remaining_cells as u64;
    let floor = remaining_bytes / n;
    if floor == 0 {
        return Err(QdnfError::BudgetExhausted);
    }
    let others = n - 1;
    let reserved = floor.checked_mul(others).ok_or(QdnfError::Capacity)?;
    let max_this = remaining_bytes.saturating_sub(reserved);
    Ok(requested.min(max_this))
}

/// One host owner plus fair cell admission (1..=16).
pub struct ScaleHost {
    host: HostAdmission,
}

impl ScaleHost {
    pub fn new(host_bytes: u64) -> Result<Self, QdnfError> {
        Ok(Self {
            host: HostAdmission::new(host_bytes)?,
        })
    }

    #[inline]
    pub fn occupied(&self) -> usize {
        self.host.occupied_cells()
    }

    #[inline]
    pub fn remaining_bytes(&self) -> u64 {
        self.host.remaining_host_bytes()
    }

    #[inline]
    pub fn remaining_slots(&self) -> usize {
        MAX_SCALE_CELLS.saturating_sub(self.occupied())
    }

    /// Admit `requested` bytes, reserving equal floors for the rest of the
    /// cohort (`cohort_remaining` includes this cell).
    pub fn admit_fair(
        &mut self,
        requested: u64,
        cohort_remaining: usize,
    ) -> Result<CellSlot, QdnfError> {
        if self.remaining_slots() == 0 {
            return Err(QdnfError::Capacity);
        }
        if cohort_remaining > self.remaining_slots() {
            return Err(QdnfError::Capacity);
        }
        let grant = fair_grant(self.remaining_bytes(), cohort_remaining, requested)?;
        let profile = HostAdmission::profile_for_bytes(grant)?;
        self.host.admit_cell(profile, grant)
    }

    /// Admit `n` equal cells (1..=16) of `per_cell` bytes.
    pub fn admit_equal(
        &mut self,
        n: usize,
        per_cell: u64,
        out: &mut [Option<CellSlot>; MAX_SCALE_CELLS],
    ) -> Result<usize, QdnfError> {
        if n == 0 || n > MAX_SCALE_CELLS {
            return Err(QdnfError::Capacity);
        }
        let mut i = 0usize;
        while i < n {
            out[i] = Some(self.admit_fair(per_cell, n - i)?);
            i += 1;
        }
        Ok(n)
    }

    pub fn release(&mut self, slot: CellSlot) -> Result<(), QdnfError> {
        self.host.release_cell(slot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::cells::pass_budget::SENTINEL_PASS_TOTAL;
    use crate::net::qdnf::errors::QdnfError;

    const CELL: u64 = 64;

    #[test]
    fn sixteen_cells_admitted_seventeenth_rejected() {
        assert!(16 * CELL < SENTINEL_PASS_TOTAL);
        let mut host = ScaleHost::new(MAX_SCALE_CELLS as u64 * CELL).unwrap();
        let mut out = [None; MAX_SCALE_CELLS];
        let n = host
            .admit_equal(MAX_SCALE_CELLS, CELL, &mut out)
            .expect("16 cells");
        assert_eq!(n, MAX_SCALE_CELLS);
        assert_eq!(host.occupied(), MAX_SCALE_CELLS);
        assert_eq!(host.remaining_slots(), 0);
        assert_eq!(
            host.admit_fair(CELL, 1),
            Err(QdnfError::Capacity)
        );
        assert_eq!(host.occupied(), MAX_SCALE_CELLS);
        let mut i = 0usize;
        while i < MAX_SCALE_CELLS {
            assert_eq!(out[i].unwrap().bytes, CELL);
            assert_eq!(out[i].unwrap().profile, CellProfile::NetworkSmall);
            i += 1;
        }
    }

    #[test]
    fn greedy_cell_cannot_starve_remaining_budget() {
        let budget = MAX_SCALE_CELLS as u64 * CELL;
        let mut host = ScaleHost::new(budget).unwrap();
        let greedy = host
            .admit_fair(budget, MAX_SCALE_CELLS)
            .expect("greedy is capped, not rejected");
        assert_eq!(greedy.bytes, CELL);
        assert!(greedy.bytes < budget);
        assert_eq!(host.remaining_bytes(), 15 * CELL);
        let mut i = 0usize;
        while i < 15 {
            let slot = host
                .admit_fair(CELL, 15 - i)
                .expect("remaining cohort keeps a floor share");
            assert_eq!(slot.bytes, CELL);
            i += 1;
        }
        assert_eq!(host.occupied(), MAX_SCALE_CELLS);
        assert_eq!(host.remaining_bytes(), 0);
    }

    #[test]
    fn scale_one_two_four_eight_sixteen() {
        let counts = [1usize, 2, 4, 8, 16];
        let mut c = 0usize;
        while c < counts.len() {
            let n = counts[c];
            let mut host = ScaleHost::new(n as u64 * CELL).unwrap();
            let mut out = [None; MAX_SCALE_CELLS];
            assert_eq!(host.admit_equal(n, CELL, &mut out).unwrap(), n);
            assert_eq!(host.occupied(), n);
            c += 1;
        }
    }

    #[test]
    fn fair_grant_rejects_seventeenth_and_zero() {
        assert_eq!(fair_grant(CELL, 0, CELL), Err(QdnfError::Capacity));
        assert_eq!(
            fair_grant(CELL, MAX_SCALE_CELLS + 1, CELL),
            Err(QdnfError::Capacity)
        );
        assert_eq!(fair_grant(CELL, 1, 0), Err(QdnfError::Capacity));
        assert_eq!(
            fair_grant(MAX_SCALE_CELLS as u64 - 1, MAX_SCALE_CELLS, 1),
            Err(QdnfError::BudgetExhausted)
        );
    }

    #[test]
    fn remainder_goes_to_current_without_starving_floor() {
        let budget = 16 * CELL + 15;
        assert_eq!(fair_grant(budget, 16, budget).unwrap(), CELL + 15);
        let mut host = ScaleHost::new(budget).unwrap();
        let greedy = host.admit_fair(budget, 16).unwrap();
        assert_eq!(greedy.bytes, CELL + 15);
        assert_eq!(host.remaining_bytes(), 15 * CELL);
        let mut out = [None; MAX_SCALE_CELLS];
        assert_eq!(host.admit_equal(15, CELL, &mut out).unwrap(), 15);
        assert_eq!(host.occupied(), 16);
    }
}
