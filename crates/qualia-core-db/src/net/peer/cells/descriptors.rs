//! Caller-backed cell descriptors (E10.1).
//!
//! Production admission is a 16-slot descriptor table charged against one
//! [`HostAdmission`] owner. Per-cell bytes still cannot exceed 512 MiB.
//! [`super::admit::CellTable`] grew to 16 slots with the same handle-based
//! release; extra `owner_tag` values do not mint a second host budget.

use super::admit::{CellProfile, CellSlot, MAX_CELLS};
use super::host_owner::HostAdmission;
use crate::net::qdnf::errors::QdnfError;

/// Caller descriptor table size. Same as [`MAX_CELLS`].
pub const MAX_DESCRIPTOR_CELLS: usize = 16;

/// One caller-backed cell request. `owner_tag` is recorded identity, not budget.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CellDescriptor {
    pub profile: CellProfile,
    pub bytes: u64,
    pub owner_tag: u64,
}

/// Admit `descs` against the shared host owner.
///
/// A 17th descriptor is [`QdnfError::Capacity`] and admits nothing. Two
/// descriptors totalling more than `host_bytes` yield
/// [`QdnfError::BudgetExhausted`]: the first may occupy `out_slots`, the
/// second fails; there is no silent overcommit.
pub fn admit_descriptors(
    host: &mut HostAdmission,
    descs: &[CellDescriptor],
    out_slots: &mut [Option<CellSlot>],
) -> Result<usize, QdnfError> {
    if descs.len() > MAX_DESCRIPTOR_CELLS || descs.len() > MAX_CELLS {
        return Err(QdnfError::Capacity);
    }
    if out_slots.len() < descs.len() {
        return Err(QdnfError::Capacity);
    }
    let mut n = 0usize;
    let mut i = 0usize;
    while i < descs.len() {
        match host.admit_cell(descs[i].profile, descs[i].bytes) {
            Ok(slot) => {
                let _ = descs[i].owner_tag;
                out_slots[i] = Some(slot);
                n += 1;
            }
            Err(e) => return Err(e),
        }
        i += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::cells::admit::MAX_ORDINARY_CELL;
    use crate::net::qdnf::errors::QdnfError;

    const KIB: u64 = 1024;

    fn desc(bytes: u64, tag: u64) -> CellDescriptor {
        CellDescriptor {
            profile: CellProfile::NetworkSmall,
            bytes,
            owner_tag: tag,
        }
    }

    #[test]
    fn sixteen_descriptors_ok_seventeenth_capacity() {
        const N: usize = MAX_DESCRIPTOR_CELLS;
        let mut host = HostAdmission::new(N as u64 * KIB).unwrap();
        let mut descs = [desc(KIB, 1); N];
        let mut i = 0usize;
        while i < N {
            descs[i].owner_tag = i as u64 + 1;
            i += 1;
        }
        let mut out = [None; N];
        let n = admit_descriptors(&mut host, &descs, &mut out).expect("16 descriptors");
        assert_eq!(n, N);
        assert_eq!(host.occupied_cells(), N);
        assert_eq!(MAX_DESCRIPTOR_CELLS, MAX_CELLS);

        let extra = [desc(KIB, 99); N + 1];
        let mut extra_out = [None; N + 1];
        assert_eq!(
            admit_descriptors(&mut host, &extra, &mut extra_out),
            Err(QdnfError::Capacity)
        );
        assert_eq!(host.occupied_cells(), N);
    }

    #[test]
    fn overcommit_budget_exhausted_first_may_succeed() {
        let mut host = HostAdmission::new(100).unwrap();
        let descs = [desc(60, 1), desc(60, 2)];
        let mut out = [None, None];
        assert_eq!(
            admit_descriptors(&mut host, &descs, &mut out),
            Err(QdnfError::BudgetExhausted)
        );
        assert!(out[0].is_some());
        assert!(out[1].is_none());
        assert_eq!(host.occupied_cells(), 1);
        assert_eq!(host.remaining_host_bytes(), 40);
        host.release_cell(out[0].unwrap()).unwrap();
        assert_eq!(host.remaining_host_bytes(), 100);
    }

    #[test]
    fn owner_tag_does_not_multiply_host_budget() {
        let mut host = HostAdmission::new(100).unwrap();
        let descs = [desc(60, 1), desc(60, 2)];
        let mut out = [None, None];
        assert_eq!(
            admit_descriptors(&mut host, &descs, &mut out),
            Err(QdnfError::BudgetExhausted)
        );
    }

    #[test]
    fn per_cell_cap_still_512_mib() {
        let mut host = HostAdmission::new(MAX_ORDINARY_CELL).unwrap();
        let descs = [CellDescriptor {
            profile: CellProfile::Ordinary,
            bytes: MAX_ORDINARY_CELL + 1,
            owner_tag: 1,
        }];
        let mut out = [None];
        assert_eq!(
            admit_descriptors(&mut host, &descs, &mut out),
            Err(QdnfError::Capacity)
        );
        assert_eq!(host.occupied_cells(), 0);
    }

    #[test]
    fn empty_slice_admits_zero() {
        let mut host = HostAdmission::new(100).unwrap();
        let mut out: [Option<CellSlot>; 0] = [];
        assert_eq!(admit_descriptors(&mut host, &[], &mut out), Ok(0));
    }
}
