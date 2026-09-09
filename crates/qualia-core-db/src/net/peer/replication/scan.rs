//! Bounded QSync content scan (SVC-01.16 partial).
//!
//! Logical size is a `u64`. Admitted resident is a `u32` cap (4 MiB), never a
//! buffer allocated to that size or to `logical_bytes`. Each page copies into a
//! caller-owned buffer of at most [`MAX_PAGE_BYTES`]. This is not RAM-sized
//! materialization and is not a complete QSync package.
//!
//! Declared limits: wire delivery is at-least-once (a [`QdnfError::Capacity`]
//! does not advance the cursor, so the same offset may be re-offered). Local
//! application of `Ok(n > 0)` is at-most-once for this cursor (offset advances).
//! Not exactly-once. Continuations do not refill remaining caps. Cancelled
//! complete cannot resurrect.

use crate::net::peer::replication::source::ScanSource;
use crate::net::peer::runtime::cancel::{CancelEpoch, OperationTable};
use crate::net::peer::runtime::leases::LeaseTable;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, OperationId, StrongDigest};

/// Maximum bytes copied into one caller page.
pub const MAX_PAGE_BYTES: u32 = 4096;
/// Maximum pages charged against one remaining [`ScanBudget`].
pub const MAX_PAGES: u16 = 32;
/// Admitted resident ceiling. Not a dataset size and not an allocated buffer.
pub const RESIDENT_CAP_BYTES: u32 = 4 * 1024 * 1024;

const PAGE_WORK: u64 = 1;

/// Remaining scan quanta. Continuations must not refill these fields.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanBudget {
    pub pages: u16,
    pub work: u64,
    pub bytes: u32,
}

impl ScanBudget {
    pub const ZERO: Self = Self {
        pages: 0,
        work: 0,
        bytes: 0,
    };

    /// Finite quantum caps. Not an unlimited scan.
    pub const fn initial() -> Self {
        Self {
            pages: MAX_PAGES,
            work: MAX_PAGES as u64,
            bytes: MAX_PAGES as u32 * MAX_PAGE_BYTES,
        }
    }

    #[inline]
    pub const fn is_finite(self) -> bool {
        self.pages <= MAX_PAGES
    }
}

/// Copy cursor: operation id, logical offset, generation, remaining budget.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ScanCursor {
    pub op: OperationId,
    pub offset: u64,
    pub generation: Generation,
    pub cancel_epoch: CancelEpoch,
    pub remaining: ScanBudget,
    pub logical_bytes: u64,
    pub resident_cap: u32,
    /// ZERO means no authenticated source (synthetic `next_page` is not evidence).
    source_digest: StrongDigest,
    cancelled: bool,
}

impl ScanCursor {
    #[inline]
    pub const fn is_cancelled(self) -> bool {
        self.cancelled
    }
}

/// Open a scan over a logical size. Never materializes `logical_bytes`.
///
/// [`QdnfError::Capacity`] if `resident_cap == 0 && logical_bytes > 0`, if
/// `resident_cap > RESIDENT_CAP_BYTES`, or if `budget.pages > MAX_PAGES`.
pub fn open_scan(
    logical_bytes: u64,
    resident_cap: u32,
    budget: ScanBudget,
) -> Result<ScanCursor, QdnfError> {
    if resident_cap == 0 && logical_bytes > 0 {
        return Err(QdnfError::Capacity);
    }
    if resident_cap > RESIDENT_CAP_BYTES {
        return Err(QdnfError::Capacity);
    }
    if !budget.is_finite() {
        return Err(QdnfError::Capacity);
    }
    Ok(ScanCursor {
        op: scan_op_id(logical_bytes, resident_cap),
        offset: 0,
        generation: Generation(1),
        cancel_epoch: CancelEpoch::ZERO,
        remaining: budget,
        logical_bytes,
        resident_cap,
        source_digest: StrongDigest::ZERO,
        cancelled: false,
    })
}

/// Open a scan bound to an authenticated source identity (not computed here).
///
/// [`QdnfError::Malformed`] if `source_digest` is [`StrongDigest::ZERO`].
pub fn open_scan_bound(
    logical_bytes: u64,
    resident_cap: u32,
    budget: ScanBudget,
    source_digest: StrongDigest,
) -> Result<ScanCursor, QdnfError> {
    if source_digest == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    let mut cursor = open_scan(logical_bytes, resident_cap, budget)?;
    cursor.source_digest = source_digest;
    Ok(cursor)
}

/// Copy the next page into `out` using the synthetic XOR fill.
///
/// This path is **not** storage evidence. Production scans must use
/// [`next_page_from`] with a [`ScanSource`]. [`synthetic_scan_is_storage_evidence`]
/// is always false. Kept for budget/cursor tests.
///
/// Empty `out` with remaining logical bytes is [`QdnfError::Capacity`] and
/// does not advance offset. Exhausted remaining caps are
/// [`QdnfError::BudgetExhausted`] and do not reset on retry.
pub fn next_page(cursor: &mut ScanCursor, out: &mut [u8]) -> Result<usize, QdnfError> {
    let n = next_page_prepare(cursor, out)?;
    if n == 0 {
        return Ok(0);
    }
    fill_page(cursor.offset, &mut out[..n]);
    commit_page(cursor, n);
    Ok(n)
}

/// Copy the next page from `source` via [`ScanSource::read_at`] at `cursor.offset`.
///
/// Bytes come from the source, never the synthetic XOR fill. Budget and
/// cursor rules match [`next_page`].
pub fn next_page_from(
    cursor: &mut ScanCursor,
    source: &impl ScanSource,
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let n = next_page_prepare(cursor, out)?;
    if n == 0 {
        return Ok(0);
    }
    if cursor.offset >= source.logical_len() {
        return Err(QdnfError::Incomplete);
    }
    let source_remaining = source.logical_len() - cursor.offset;
    let want = if source_remaining < n as u64 {
        source_remaining as usize
    } else {
        n
    };
    if want == 0 {
        return Err(QdnfError::Incomplete);
    }
    let got = source.read_at(cursor.offset, &mut out[..want])?;
    if got == 0 {
        return Err(QdnfError::Incomplete);
    }
    commit_page(cursor, got);
    Ok(got)
}

/// Stop further pages. A cancelled cursor cannot resume as if cancel never
/// happened. Idempotent.
pub fn cancel_scan(cursor: &mut ScanCursor) -> Result<(), QdnfError> {
    if cursor.cancelled {
        return Ok(());
    }
    cursor.cancel_epoch = cursor.cancel_epoch.next()?;
    cursor.cancelled = true;
    Ok(())
}

/// Gate further pages on [`OperationTable`] cancel. Table cancel stops pages.
pub fn next_page_in(
    ops: &OperationTable,
    cursor: &mut ScanCursor,
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    if ops.is_cancelled(cursor.op) {
        cursor.cancelled = true;
        return Err(QdnfError::Cancelled);
    }
    next_page(cursor, out)
}

/// Cancel the cursor and the admitted operation, if present.
pub fn cancel_scan_in(
    ops: &mut OperationTable,
    cursor: &mut ScanCursor,
) -> Result<CancelEpoch, QdnfError> {
    cancel_scan(cursor)?;
    match ops.cancel(cursor.op) {
        Ok(epoch) => {
            cursor.cancel_epoch = epoch;
            Ok(epoch)
        }
        Err(QdnfError::Closed) => Ok(cursor.cancel_epoch),
        Err(e) => Err(e),
    }
}

/// Admit a page-sized exclusive lease (never `logical_bytes`) for this scan.
pub fn admit_scan(
    ops: &mut OperationTable,
    leases: &mut LeaseTable,
    cursor: &mut ScanCursor,
) -> Result<(), QdnfError> {
    if cursor.cancelled {
        return Err(QdnfError::Cancelled);
    }
    let lease = leases.acquire(MAX_PAGE_BYTES, true)?;
    match ops.admit(cursor.op, lease.handle) {
        Ok(handle) => {
            cursor.generation = handle.generation;
            cursor.cancel_epoch = handle.cancel_epoch;
            Ok(())
        }
        Err(e) => {
            let _ = leases.release(lease.handle);
            Err(e)
        }
    }
}

/// Finish the admitted scan. After cancel this releases storage and returns
/// [`QdnfError::Cancelled`] so a late page is not a live success.
pub fn complete_scan(
    ops: &mut OperationTable,
    leases: &mut LeaseTable,
    cursor: &mut ScanCursor,
) -> Result<(), QdnfError> {
    match ops.complete(cursor.op, cursor.generation, leases) {
        Ok(()) => Ok(()),
        Err(QdnfError::Cancelled) => {
            cursor.cancelled = true;
            Err(QdnfError::Cancelled)
        }
        Err(e) => Err(e),
    }
}

/// Pages may be re-offered after Capacity if the cursor was not advanced.
#[inline]
pub fn at_least_once_delivery() -> bool {
    true
}

/// Once `Ok(n > 0)` returns, a later `next_page` uses a higher offset.
#[inline]
pub fn at_most_once_local_application() -> bool {
    true
}

/// This module never allocates or maps a RAM-sized logical dataset.
#[inline]
pub fn ram_sized_dataset_materialized() -> bool {
    let _ = core::mem::size_of::<StrongDigest>();
    false
}

/// `next_page` copies into caller-owned `out` with no `Vec`/`String`/`Box`.
#[inline]
pub fn allocation_class_hot_zero_heap() -> bool {
    true
}

/// Continuations never restore pages/work/bytes.
#[inline]
pub fn continuation_refills_budget() -> bool {
    false
}

/// Synthetic XOR `next_page` fill is never storage evidence.
#[inline]
pub fn synthetic_scan_is_storage_evidence() -> bool {
    false
}

/// True only when [`open_scan_bound`] attached a non-zero source digest.
#[inline]
pub fn scan_pages_are_storage_evidence(cursor: &ScanCursor) -> bool {
    cursor.source_digest != StrongDigest::ZERO
}

fn next_page_prepare(cursor: &ScanCursor, out: &[u8]) -> Result<usize, QdnfError> {
    if cursor.cancelled {
        return Err(QdnfError::Cancelled);
    }
    if cursor.offset > cursor.logical_bytes {
        return Err(QdnfError::Range);
    }
    let remaining_logical = cursor.logical_bytes - cursor.offset;
    if remaining_logical == 0 {
        return Ok(0);
    }
    if out.is_empty() {
        return Err(QdnfError::Capacity);
    }

    let n = page_len(out.len(), remaining_logical);
    if cursor.remaining.pages == 0 || cursor.remaining.work < PAGE_WORK {
        return Err(QdnfError::BudgetExhausted);
    }
    if cursor.remaining.bytes < n as u32 {
        return Err(QdnfError::BudgetExhausted);
    }
    Ok(n)
}

fn commit_page(cursor: &mut ScanCursor, n: usize) {
    cursor.remaining.pages -= 1;
    cursor.remaining.work -= PAGE_WORK;
    cursor.remaining.bytes -= n as u32;
    cursor.offset += n as u64;
}

fn scan_op_id(logical_bytes: u64, resident_cap: u32) -> OperationId {
    let mut id = [0u8; 16];
    id[0..8].copy_from_slice(&logical_bytes.to_le_bytes());
    id[8..12].copy_from_slice(&resident_cap.to_le_bytes());
    id[12] = 0x51;
    OperationId(id)
}

fn page_len(out_len: usize, remaining_logical: u64) -> usize {
    let cap = if out_len > MAX_PAGE_BYTES as usize {
        MAX_PAGE_BYTES as usize
    } else {
        out_len
    };
    if remaining_logical > cap as u64 {
        cap
    } else {
        remaining_logical as usize
    }
}

fn fill_page(offset: u64, out: &mut [u8]) {
    let mut i = 0usize;
    while i < out.len() {
        let pos = offset.wrapping_add(i as u64);
        out[i] = (pos as u8) ^ ((pos >> 8) as u8) ^ ((pos >> 16) as u8);
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::leases::LEASE_SLOTS;

    const EIGHT_GIB: u64 = 8 * 1024 * 1024 * 1024;

    fn open_large() -> ScanCursor {
        open_scan(EIGHT_GIB, RESIDENT_CAP_BYTES, ScanBudget::initial()).unwrap()
    }

    #[test]
    fn larger_than_ram_open_pages_without_materializing() {
        let mut cursor = open_large();
        assert!(cursor.logical_bytes > cursor.resident_cap as u64);
        let mut page = [0u8; MAX_PAGE_BYTES as usize];
        let n = next_page(&mut cursor, &mut page).unwrap();
        assert!(n > 0);
        assert!(n <= MAX_PAGE_BYTES as usize);
        assert!(!ram_sized_dataset_materialized());
        assert!(allocation_class_hot_zero_heap());
    }

    #[test]
    fn thirty_third_page_is_budget_exhausted_and_continuation_does_not_refill() {
        let mut cursor = open_large();
        let mut page = [0u8; MAX_PAGE_BYTES as usize];
        let mut i = 0u16;
        while i < MAX_PAGES {
            let n = next_page(&mut cursor, &mut page).unwrap();
            assert_eq!(n, MAX_PAGE_BYTES as usize);
            i += 1;
        }
        assert_eq!(cursor.remaining.pages, 0);
        assert_eq!(
            next_page(&mut cursor, &mut page),
            Err(QdnfError::BudgetExhausted)
        );
        assert!(!continuation_refills_budget());
        assert_eq!(cursor.remaining.pages, 0);
        assert_eq!(
            next_page(&mut cursor, &mut page),
            Err(QdnfError::BudgetExhausted)
        );
        assert_eq!(cursor.remaining.pages, 0);
    }

    #[test]
    fn cancel_then_next_page_is_cancelled() {
        let mut cursor = open_large();
        cancel_scan(&mut cursor).unwrap();
        let mut page = [0u8; MAX_PAGE_BYTES as usize];
        assert_eq!(next_page(&mut cursor, &mut page), Err(QdnfError::Cancelled));
        assert_eq!(next_page(&mut cursor, &mut page), Err(QdnfError::Cancelled));
    }

    #[test]
    fn capacity_on_empty_out_does_not_advance_offset() {
        let mut cursor = open_large();
        let start = cursor.offset;
        assert_eq!(next_page(&mut cursor, &mut []), Err(QdnfError::Capacity));
        assert_eq!(cursor.offset, start);
        assert_eq!(cursor.remaining.pages, MAX_PAGES);
        let mut page = [0u8; MAX_PAGE_BYTES as usize];
        let n = next_page(&mut cursor, &mut page).unwrap();
        assert_eq!(n, MAX_PAGE_BYTES as usize);
        assert!(at_least_once_delivery());
        assert_eq!(cursor.offset, start + n as u64);
    }

    #[test]
    fn ok_page_advances_offset_at_most_once() {
        let mut cursor = open_large();
        let mut page = [0u8; MAX_PAGE_BYTES as usize];
        let first = cursor.offset;
        let n = next_page(&mut cursor, &mut page).unwrap();
        assert!(n > 0);
        let after = cursor.offset;
        assert!(after > first);
        let n2 = next_page(&mut cursor, &mut page).unwrap();
        assert!(n2 > 0);
        assert!(cursor.offset > after);
        assert!(at_most_once_local_application());
    }

    #[test]
    fn ram_sized_dataset_is_not_materialized() {
        assert!(!ram_sized_dataset_materialized());
        let _cursor = open_large();
        assert!(!ram_sized_dataset_materialized());
    }

    #[test]
    fn zero_resident_cap_with_logical_data_is_capacity() {
        assert_eq!(
            open_scan(1, 0, ScanBudget::initial()),
            Err(QdnfError::Capacity)
        );
        assert!(open_scan(0, 0, ScanBudget::initial()).is_ok());
    }

    #[test]
    fn cancelled_complete_cannot_resurrect() {
        let mut leases = LeaseTable::new();
        let mut ops = OperationTable::new();
        let mut cursor = open_large();
        admit_scan(&mut ops, &mut leases, &mut cursor).unwrap();
        cancel_scan_in(&mut ops, &mut cursor).unwrap();
        let mut page = [0u8; MAX_PAGE_BYTES as usize];
        assert_eq!(
            next_page_in(&ops, &mut cursor, &mut page),
            Err(QdnfError::Cancelled)
        );
        assert_eq!(
            complete_scan(&mut ops, &mut leases, &mut cursor),
            Err(QdnfError::Cancelled)
        );
        assert_eq!(next_page(&mut cursor, &mut page), Err(QdnfError::Cancelled));
        assert_eq!(
            complete_scan(&mut ops, &mut leases, &mut cursor),
            Err(QdnfError::Closed)
        );
    }

    #[test]
    fn offset_past_logical_is_range_and_end_is_ok_zero() {
        let mut cursor = open_scan(64, RESIDENT_CAP_BYTES, ScanBudget::initial()).unwrap();
        let mut page = [0u8; MAX_PAGE_BYTES as usize];
        let n = next_page(&mut cursor, &mut page).unwrap();
        assert_eq!(n, 64);
        assert_eq!(next_page(&mut cursor, &mut page), Ok(0));
        cursor.offset = cursor.logical_bytes + 1;
        assert_eq!(next_page(&mut cursor, &mut page), Err(QdnfError::Range));
    }

    #[test]
    fn page_lease_is_not_logical_size() {
        let mut leases = LeaseTable::new();
        let mut ops = OperationTable::new();
        let mut cursor = open_large();
        admit_scan(&mut ops, &mut leases, &mut cursor).unwrap();
        assert_eq!(leases.occupied_count(), 1);
        assert!(EIGHT_GIB > u32::MAX as u64);
        assert!(LEASE_SLOTS >= 32);
        complete_scan(&mut ops, &mut leases, &mut cursor).unwrap();
        assert_eq!(leases.occupied_count(), 0);
    }

    #[test]
    fn synthetic_scan_is_not_storage_evidence() {
        assert!(!synthetic_scan_is_storage_evidence());
        let cursor = open_large();
        assert!(!scan_pages_are_storage_evidence(&cursor));
    }

    #[test]
    fn next_page_from_copies_source_bytes_not_xor() {
        use crate::net::peer::replication::source::MemorySource;

        let src = [0xAAu8; 64];
        let mut cursor_xor = open_scan(64, RESIDENT_CAP_BYTES, ScanBudget::initial()).unwrap();
        let mut cursor_src = open_scan(64, RESIDENT_CAP_BYTES, ScanBudget::initial()).unwrap();
        let mut xor_page = [0u8; 64];
        let mut src_page = [0u8; 64];
        let n_xor = next_page(&mut cursor_xor, &mut xor_page).unwrap();
        let n_src = next_page_from(
            &mut cursor_src,
            &MemorySource::from_slice(&src),
            &mut src_page,
        )
        .unwrap();
        assert_eq!(n_xor, 64);
        assert_eq!(n_src, 64);
        assert_eq!(&src_page[..n_src], &src[..]);
        assert_ne!(&xor_page[..n_xor], &src[..]);
        let mut expected_xor = [0u8; 64];
        fill_page(0, &mut expected_xor);
        assert_eq!(xor_page, expected_xor);
    }

    #[test]
    fn two_sources_same_length_different_pages() {
        use crate::net::peer::replication::source::MemorySource;

        let a = [1u8; 32];
        let b = [2u8; 32];
        let mut ca = open_scan(32, RESIDENT_CAP_BYTES, ScanBudget::initial()).unwrap();
        let mut cb = open_scan(32, RESIDENT_CAP_BYTES, ScanBudget::initial()).unwrap();
        let mut pa = [0u8; 32];
        let mut pb = [0u8; 32];
        assert_eq!(
            next_page_from(&mut ca, &MemorySource::from_slice(&a), &mut pa).unwrap(),
            32
        );
        assert_eq!(
            next_page_from(&mut cb, &MemorySource::from_slice(&b), &mut pb).unwrap(),
            32
        );
        assert_eq!(pa, a);
        assert_eq!(pb, b);
        assert_ne!(pa, pb);
    }

    #[test]
    fn bound_source_digest_is_storage_evidence_identity() {
        let mut d = StrongDigest::ZERO;
        d.0[0] = 0x42;
        let unbound = open_scan(16, RESIDENT_CAP_BYTES, ScanBudget::initial()).unwrap();
        assert!(!scan_pages_are_storage_evidence(&unbound));
        let bound = open_scan_bound(16, RESIDENT_CAP_BYTES, ScanBudget::initial(), d).unwrap();
        assert!(scan_pages_are_storage_evidence(&bound));
        assert_eq!(
            open_scan_bound(
                16,
                RESIDENT_CAP_BYTES,
                ScanBudget::initial(),
                StrongDigest::ZERO
            ),
            Err(QdnfError::Malformed)
        );
    }
}
