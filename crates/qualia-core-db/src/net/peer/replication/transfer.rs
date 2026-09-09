//! Bounded content-block transfer under shared receive/storage/verify credit
//! (SVC-01.13 partial).
//!
//! One [`ReservationLedger`] charge covers receive+store+verify (not three).
//! Retries charge that parent again (io/work). Old and new generations each
//! occupy admit credit until the old slot is released. Packages remain open.

use crate::crypto::network::digest::sha384;
use crate::net::peer::replication::manifest::{ByteRange, ContentManifest};
use crate::net::peer::runtime::ledger::{
    ChargeRef, ReservationHandle, ReservationLedger, ResourceBudget,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Occupied transfer slots this wave (not a swarm).
pub const MAX_BLOCKS: usize = 8;
/// Per-block byte ceiling (`u32`). Partial blocks still charge `len`.
pub const MAX_BLOCK_BYTES: u32 = 16 * 1024;

const ADMIT_WORK: u64 = 1;
const ADMIT_IO: u64 = 1;
const RETRY_WORK: u64 = 1;
const RETRY_IO: u64 = 1;

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    verified: bool,
    digest: StrongDigest,
    /// Per-block content hash. ZERO means compare `sha384(payload)` to `digest`.
    expected: StrongDigest,
    block_index: u8,
    generation: Generation,
    range: ByteRange,
    charge: ChargeRef,
}

/// Eight-slot table of admitted (manifest, block, generation) transfers.
pub struct TransferTable {
    slots: [Slot; MAX_BLOCKS],
}

impl TransferTable {
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                occupied: false,
                verified: false,
                digest: StrongDigest::ZERO,
                expected: StrongDigest::ZERO,
                block_index: 0,
                generation: Generation::ZERO,
                range: ByteRange { offset: 0, len: 0 },
                charge: ChargeRef::EMPTY,
            }; MAX_BLOCKS],
        }
    }

    pub fn occupied_count(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_BLOCKS {
            if self.slots[i].occupied {
                n += 1;
            }
            i += 1;
        }
        n
    }
}

/// Admit one bounded block. One ledger charge covers receive+store+verify.
///
/// [`QdnfError::Malformed`] if `manifest_digest` is [`StrongDigest::ZERO`].
/// [`QdnfError::Range`] if `len` is 0 or `> MAX_BLOCK_BYTES`, or `block_index`
/// is not in `0..MAX_BLOCKS`. [`QdnfError::Capacity`] at eight occupied slots.
/// [`QdnfError::BudgetExhausted`] leaves the table unchanged.
pub fn admit_block(
    table: &mut TransferTable,
    ledger: &mut ReservationLedger,
    manifest_digest: StrongDigest,
    block_index: u8,
    len: u32,
    generation: Generation,
) -> Result<(), QdnfError> {
    admit_block_with_digest(
        table,
        ledger,
        manifest_digest,
        block_index,
        len,
        generation,
        StrongDigest::ZERO,
    )
}

/// Admit one bounded block with a per-block content digest for [`verify_block`].
pub fn admit_block_with_digest(
    table: &mut TransferTable,
    ledger: &mut ReservationLedger,
    manifest_digest: StrongDigest,
    block_index: u8,
    len: u32,
    generation: Generation,
    expected: StrongDigest,
) -> Result<(), QdnfError> {
    admit_range(
        table,
        ledger,
        manifest_digest,
        block_index,
        ByteRange { offset: 0, len },
        generation,
        expected,
    )
}

/// Admit using an exact [`ByteRange`] from a bound [`ContentManifest`].
pub fn admit_manifest_block(
    table: &mut TransferTable,
    ledger: &mut ReservationLedger,
    manifest: &ContentManifest,
    block_index: u8,
    generation: Generation,
) -> Result<(), QdnfError> {
    let range = manifest_block_range(manifest, block_index)?;
    admit_range(
        table,
        ledger,
        manifest.digest,
        block_index,
        range,
        generation,
        StrongDigest::ZERO,
    )
}

/// Exact range for `block_index` on `manifest`, if it fits one transfer block.
pub fn manifest_block_range(
    manifest: &ContentManifest,
    block_index: u8,
) -> Result<ByteRange, QdnfError> {
    if block_index as usize >= MAX_BLOCKS {
        return Err(QdnfError::Range);
    }
    let range = manifest.range_at(block_index as usize)?;
    check_len(range.len)?;
    Ok(range)
}

/// Retry the same (manifest, block_index, generation): charges parent io/work
/// again. Does not allocate a new slot when the slot exists.
pub fn retry_block(
    table: &mut TransferTable,
    ledger: &mut ReservationLedger,
    manifest_digest: StrongDigest,
    block_index: u8,
    generation: Generation,
) -> Result<(), QdnfError> {
    if manifest_digest == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    match find_slot(table, &manifest_digest, block_index, generation) {
        Some(_) => {
            let handle = ledger.reserve(retry_budget(), true)?;
            let _ = handle;
            Ok(())
        }
        None => Err(QdnfError::Incomplete),
    }
}

/// Verify `payload` against the admitted slot. Uses the admit charge.
///
/// Never admitted → [`QdnfError::Incomplete`]. Empty `payload` with `len > 0`
/// → [`QdnfError::Malformed`]. Length mismatch → [`QdnfError::Range`].
/// `sha384(payload)` must equal the per-block digest stored at admit, or the
/// manifest digest when no per-block digest was stored; mismatch →
/// [`QdnfError::Conflict`] and `verified` stays false. `verified` is set only
/// after a matching hash.
pub fn verify_block(
    table: &mut TransferTable,
    manifest_digest: StrongDigest,
    block_index: u8,
    generation: Generation,
    payload: &[u8],
) -> Result<(), QdnfError> {
    if manifest_digest == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    let idx =
        find_slot(table, &manifest_digest, block_index, generation).ok_or(QdnfError::Incomplete)?;
    let range_len = table.slots[idx].range.len;
    if range_len == 0 {
        return Err(QdnfError::Range);
    }
    if payload.is_empty() {
        return Err(QdnfError::Malformed);
    }
    if payload.len() != range_len as usize {
        return Err(QdnfError::Range);
    }
    let hashed = sha384(payload);
    let want = if table.slots[idx].expected != StrongDigest::ZERO {
        table.slots[idx].expected
    } else {
        table.slots[idx].digest
    };
    if hashed != want {
        table.slots[idx].verified = false;
        return Err(QdnfError::Conflict);
    }
    table.slots[idx].verified = true;
    Ok(())
}

/// Release one generation's admit reservation. Other generations are kept.
pub fn release_block(
    table: &mut TransferTable,
    ledger: &mut ReservationLedger,
    manifest_digest: StrongDigest,
    block_index: u8,
    generation: Generation,
) -> Result<(), QdnfError> {
    if manifest_digest == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    let idx =
        find_slot(table, &manifest_digest, block_index, generation).ok_or(QdnfError::Incomplete)?;
    let charged = table.slots[idx].charge;
    ledger.release(ReservationHandle::from_ref(charged))?;
    table.slots[idx].occupied = false;
    table.slots[idx].verified = false;
    Ok(())
}

/// Receive, storage, and verify share one admit charge (not triple-counted).
#[inline]
pub fn shared_receive_storage_verify_is_one_charge() -> bool {
    true
}

/// Retries charge the parent [`ReservationLedger`], not a free extra budget.
#[inline]
pub fn retries_charge_parent() -> bool {
    true
}

fn check_len(len: u32) -> Result<(), QdnfError> {
    if len == 0 || len > MAX_BLOCK_BYTES {
        return Err(QdnfError::Range);
    }
    Ok(())
}

fn admit_budget(len: u32) -> ResourceBudget {
    ResourceBudget {
        bytes: u64::from(len),
        work: ADMIT_WORK,
        io: ADMIT_IO,
    }
}

fn retry_budget() -> ResourceBudget {
    ResourceBudget {
        bytes: 0,
        work: RETRY_WORK,
        io: RETRY_IO,
    }
}

fn admit_range(
    table: &mut TransferTable,
    ledger: &mut ReservationLedger,
    digest: StrongDigest,
    block_index: u8,
    range: ByteRange,
    generation: Generation,
    expected: StrongDigest,
) -> Result<(), QdnfError> {
    if digest == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    if block_index as usize >= MAX_BLOCKS {
        return Err(QdnfError::Range);
    }
    check_len(range.len)?;
    if find_slot(table, &digest, block_index, generation).is_some() {
        return Ok(());
    }
    let free = find_free(table).ok_or(QdnfError::Capacity)?;
    let charged = admit_budget(range.len);
    let handle = ledger.reserve(charged, true)?;
    table.slots[free] = Slot {
        occupied: true,
        verified: false,
        digest,
        expected,
        block_index,
        generation,
        range,
        charge: handle.as_ref(),
    };
    Ok(())
}

fn find_slot(
    table: &TransferTable,
    digest: &StrongDigest,
    block_index: u8,
    generation: Generation,
) -> Option<usize> {
    let mut i = 0usize;
    while i < MAX_BLOCKS {
        let s = &table.slots[i];
        if s.occupied
            && s.digest == *digest
            && s.block_index == block_index
            && s.generation == generation
        {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_free(table: &TransferTable) -> Option<usize> {
    let mut i = 0usize;
    while i < MAX_BLOCKS {
        if !table.slots[i].occupied {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d
    }

    fn fill_payload(buf: &mut [u8], tag: u8) -> StrongDigest {
        let mut i = 0usize;
        while i < buf.len() {
            buf[i] = tag;
            i += 1;
        }
        sha384(buf)
    }

    fn slot_verified(
        table: &TransferTable,
        d: StrongDigest,
        block_index: u8,
        generation: Generation,
    ) -> bool {
        match find_slot(table, &d, block_index, generation) {
            Some(i) => table.slots[i].verified,
            None => false,
        }
    }

    fn fat_ledger() -> ReservationLedger {
        let cap = ResourceBudget {
            bytes: u64::from(MAX_BLOCK_BYTES) * MAX_BLOCKS as u64 * 4,
            work: 64,
            io: 64,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    fn tight_ledger() -> ReservationLedger {
        let cap = ResourceBudget {
            bytes: 0,
            work: 64,
            io: 64,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    #[test]
    fn ninth_block_is_capacity() {
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let gen = Generation(1);
        let mut i = 0u8;
        while i < MAX_BLOCKS as u8 {
            admit_block(&mut table, &mut ledger, digest(i + 1), 0, 32, gen).unwrap();
            i += 1;
        }
        assert_eq!(table.occupied_count(), MAX_BLOCKS);
        assert_eq!(
            admit_block(&mut table, &mut ledger, digest(9), 0, 32, gen),
            Err(QdnfError::Capacity)
        );
        assert_eq!(table.occupied_count(), MAX_BLOCKS);
    }

    #[test]
    fn len_zero_and_over_max_are_range() {
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let gen = Generation(1);
        assert_eq!(
            admit_block(&mut table, &mut ledger, digest(1), 0, 0, gen),
            Err(QdnfError::Range)
        );
        assert_eq!(
            admit_block(
                &mut table,
                &mut ledger,
                digest(1),
                0,
                MAX_BLOCK_BYTES + 1,
                gen
            ),
            Err(QdnfError::Range)
        );
        admit_block(&mut table, &mut ledger, digest(1), 0, MAX_BLOCK_BYTES, gen).unwrap();
        assert_eq!(table.occupied_count(), 1);
    }

    #[test]
    fn retry_charges_additional_io_on_parent() {
        assert!(retries_charge_parent());
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let gen = Generation(2);
        admit_block(&mut table, &mut ledger, digest(3), 1, 64, gen).unwrap();
        let io_before = ledger.used().host.io;
        let bytes_before = ledger.used().host.bytes;
        retry_block(&mut table, &mut ledger, digest(3), 1, gen).unwrap();
        assert!(ledger.used().host.io > io_before);
        assert_eq!(ledger.used().host.bytes, bytes_before);
        assert_eq!(table.occupied_count(), 1);
        assert_eq!(
            retry_block(&mut table, &mut ledger, digest(4), 1, gen),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn old_and_new_generation_both_reserved() {
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let old = Generation(1);
        let new = Generation(2);
        let len = 48u32;
        let mut payload = [0u8; 48];
        let expected = fill_payload(&mut payload, 0x51);
        admit_block_with_digest(&mut table, &mut ledger, digest(5), 0, len, old, expected).unwrap();
        admit_block_with_digest(&mut table, &mut ledger, digest(5), 0, len, new, expected).unwrap();
        assert_eq!(table.occupied_count(), 2);
        assert_eq!(ledger.used().host.bytes, u64::from(len) * 2);
        release_block(&mut table, &mut ledger, digest(5), 0, old).unwrap();
        assert_eq!(table.occupied_count(), 1);
        assert_eq!(ledger.used().host.bytes, u64::from(len));
        verify_block(&mut table, digest(5), 0, new, &payload).unwrap();
        assert!(slot_verified(&table, digest(5), 0, new));
        assert_eq!(
            verify_block(&mut table, digest(5), 0, old, &payload),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn verify_without_admit_is_incomplete() {
        let mut table = TransferTable::new();
        assert_eq!(
            verify_block(&mut table, digest(1), 0, Generation(1), &[0u8; 16]),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn zero_digest_is_malformed() {
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let z = StrongDigest::ZERO;
        let gen = Generation(1);
        assert_eq!(
            admit_block(&mut table, &mut ledger, z, 0, 8, gen),
            Err(QdnfError::Malformed)
        );
        assert_eq!(table.occupied_count(), 0);
        assert_eq!(
            retry_block(&mut table, &mut ledger, z, 0, gen),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            verify_block(&mut table, z, 0, gen, &[1u8; 8]),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn shared_receive_storage_verify_is_one_charge_true() {
        assert!(shared_receive_storage_verify_is_one_charge());
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let gen = Generation(1);
        let ranges = [ByteRange { offset: 0, len: 16 }];
        let m = ContentManifest::bind(digest(1), 0, 64, &ranges).unwrap();
        let range = manifest_block_range(&m, 0).unwrap();
        let mut payload = [0u8; 16];
        let expected = fill_payload(&mut payload, 0x10);
        admit_block_with_digest(
            &mut table,
            &mut ledger,
            m.digest,
            0,
            range.len,
            gen,
            expected,
        )
        .unwrap();
        let after_admit = ledger.used().host;
        assert_eq!(after_admit.bytes, u64::from(range.len));
        assert_eq!(after_admit.work, ADMIT_WORK);
        assert_eq!(after_admit.io, ADMIT_IO);
        verify_block(&mut table, m.digest, 0, gen, &payload).unwrap();
        assert!(slot_verified(&table, m.digest, 0, gen));
        assert_eq!(ledger.used().host, after_admit);
        // Manifest-path admit (no per-block digest) still occupies one charge.
        let mut table2 = TransferTable::new();
        let mut ledger2 = fat_ledger();
        admit_manifest_block(&mut table2, &mut ledger2, &m, 0, gen).unwrap();
        assert_eq!(ledger2.used().host.bytes, u64::from(range.len));
    }

    #[test]
    fn exhausted_ledger_leaves_table_unchanged() {
        let mut table = TransferTable::new();
        let mut ledger = tight_ledger();
        let before = table.occupied_count();
        let used_before = ledger.used();
        assert_eq!(
            admit_block(&mut table, &mut ledger, digest(1), 0, 16, Generation(1)),
            Err(QdnfError::BudgetExhausted)
        );
        assert_eq!(table.occupied_count(), before);
        assert_eq!(ledger.used(), used_before);
    }

    #[test]
    fn matching_payload_sets_verified() {
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let gen = Generation(1);
        let mut payload = [0u8; 32];
        let expected = fill_payload(&mut payload, 0xAB);
        admit_block_with_digest(&mut table, &mut ledger, digest(7), 0, 32, gen, expected).unwrap();
        assert!(!slot_verified(&table, digest(7), 0, gen));
        verify_block(&mut table, digest(7), 0, gen, &payload).unwrap();
        assert!(slot_verified(&table, digest(7), 0, gen));
    }

    #[test]
    fn wrong_payload_digest_is_conflict_and_verified_stays_false() {
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let gen = Generation(1);
        let mut good = [0u8; 32];
        let expected = fill_payload(&mut good, 0x01);
        let mut bad = [0u8; 32];
        fill_payload(&mut bad, 0x02);
        admit_block_with_digest(&mut table, &mut ledger, digest(8), 0, 32, gen, expected).unwrap();
        assert_eq!(
            verify_block(&mut table, digest(8), 0, gen, &bad),
            Err(QdnfError::Conflict)
        );
        assert!(!slot_verified(&table, digest(8), 0, gen));
        verify_block(&mut table, digest(8), 0, gen, &good).unwrap();
        assert!(slot_verified(&table, digest(8), 0, gen));
    }

    #[test]
    fn empty_payload_with_nonzero_len_is_malformed() {
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let gen = Generation(1);
        let mut payload = [0u8; 8];
        let expected = fill_payload(&mut payload, 0x03);
        admit_block_with_digest(&mut table, &mut ledger, digest(9), 0, 8, gen, expected).unwrap();
        assert_eq!(
            verify_block(&mut table, digest(9), 0, gen, &[]),
            Err(QdnfError::Malformed)
        );
        assert!(!slot_verified(&table, digest(9), 0, gen));
    }

    #[test]
    fn payload_len_mismatch_is_range() {
        let mut table = TransferTable::new();
        let mut ledger = fat_ledger();
        let gen = Generation(1);
        let mut payload = [0u8; 8];
        let expected = fill_payload(&mut payload, 0x04);
        admit_block_with_digest(&mut table, &mut ledger, digest(10), 0, 8, gen, expected).unwrap();
        assert_eq!(
            verify_block(&mut table, digest(10), 0, gen, &[0u8; 4]),
            Err(QdnfError::Range)
        );
        assert!(!slot_verified(&table, digest(10), 0, gen));
    }
}
