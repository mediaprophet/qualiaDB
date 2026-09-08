//! Caller-owned Webizen arena admission.
//!
//! Admits **reuse** of caller-owned arena storage. This module does not allocate
//! the 42 MiB Sentinel buffer and does not construct [`super::SlgArena`].
//! Exclusive occupancy is tracked through the existing RT-01 [`LeaseTable`].
//!
//! Classification notes (CORE-01, partial):
//! - Canonical Quin parity is five-field (`NQuin::calculate_parity`).
//! - Four-field XOR (`s^p^o^context`, dropping metadata) is classified as
//!   legacy and is **not** auto-repaired.
//! - The 512-entry recent-slot ring is lossy and must not certify
//!   evidence / replay / obligation.
//! - Hash-slot occupancy collisions are always [`CollisionPolicy::MustRecompute`];
//!   absence of a hash hit is not "fact missing" or "complete retained facts".

use crate::net::peer::runtime::{BufferLease, LeaseTable};
use crate::net::qdnf::errors::QdnfError;
use crate::NQuin;

/// `NQuin` ABI width. Must stay 48 bytes.
pub const QUIN_ABI_BYTES: usize = 48;
/// Sentinel **pass** working-set: exact 42 MiB slot backing (not the cell budget).
pub const SENTINEL_PASS_BYTES: usize = 42 * 1024 * 1024;
/// Slots in exact 42 MiB backing: `SENTINEL_PASS_BYTES / QUIN_ABI_BYTES`.
pub const SENTINEL_SLOTS: usize = SENTINEL_PASS_BYTES / QUIN_ABI_BYTES; // 917_504
/// Ordinary cell budget ceiling. 42 MiB is the Sentinel pass, not the cell.
pub const ORDINARY_CELL_BYTES: usize = 512 * 1024 * 1024;
/// Recent-slot ring (512 entries in the arena) is lossy. Never durable.
pub const RECENT_RING_IS_DURABLE: bool = false;
/// Maximum staged + live rules in one admit record.
pub const MAX_RULE_STAGING: u16 = 64;

/// Budget class for a caller-owned backing size.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ArenaBudgetClass {
    /// Exact 42 MiB slot backing for one Sentinel pass.
    SentinelPass = 0,
    /// Strictly larger than 42 MiB and at most 512 MiB.
    OrdinaryCell = 1,
    /// Not an admitted pass or cell (documented for matchers; classify fails closed).
    Rejected = 2,
}

/// Occupied hash-slot collision policy. Absence is never "complete facts".
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CollisionPolicy {
    /// Recompute / continue. Do not infer missing or fully retained facts.
    MustRecompute = 0,
}

/// Stored Quin parity classification. Never dual-accept without classifying.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParityKind {
    /// `subject ^ predicate ^ object ^ context ^ metadata` (canonical).
    FiveFieldCanonical = 0,
    /// `subject ^ predicate ^ object ^ context` (legacy; do not auto-repair).
    FourFieldLegacy = 1,
}

/// Lease/scope wrapper over caller-owned arena backing.
///
/// Does not own or allocate the 42 MiB buffer. `live_rules` / `reserved_rules`
/// are staging counters for this admit record only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArenaAdmit {
    pub scope: u64,
    pub policy_generation: u64,
    pub source_generation: u64,
    pub lease: BufferLease,
    live_rules: u16,
    reserved_rules: u16,
}

impl ArenaAdmit {
    /// Zero `live_rules` (and unused reservations). Keep lease/scope/policy/source.
    ///
    /// Prevents stale staging from leaking between jobs in **this** admit record.
    /// Does not touch `SlgArena`; the caller zeros their own buffer.
    pub fn reset_for_reuse(&mut self) -> Result<(), QdnfError> {
        self.live_rules = 0;
        self.reserved_rules = 0;
        Ok(())
    }

    /// Bind this admit record to `scope` + policy/source generations.
    ///
    /// Cross-scope → [`QdnfError::Unauthorized`]. Generation mismatch →
    /// [`QdnfError::StaleGeneration`].
    pub fn require_scope(
        &self,
        scope: u64,
        policy_generation: u64,
        source_generation: u64,
    ) -> Result<(), QdnfError> {
        if self.scope != scope {
            return Err(QdnfError::Unauthorized);
        }
        if self.policy_generation != policy_generation
            || self.source_generation != source_generation
        {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(())
    }

    /// Reserve a staging slot. Must succeed before incrementing `live_rules`.
    /// Capacity at [`MAX_RULE_STAGING`] (live + reserved).
    pub fn reserve_rule_slot(&mut self) -> Result<(), QdnfError> {
        let occupied = self
            .live_rules
            .checked_add(self.reserved_rules)
            .ok_or(QdnfError::Capacity)?;
        if occupied >= MAX_RULE_STAGING {
            return Err(QdnfError::Capacity);
        }
        self.reserved_rules = self
            .reserved_rules
            .checked_add(1)
            .ok_or(QdnfError::Capacity)?;
        Ok(())
    }

    /// Move one reservation into `live_rules`. Conflict if none reserved.
    pub fn activate_reserved_rule(&mut self) -> Result<(), QdnfError> {
        if self.reserved_rules == 0 {
            return Err(QdnfError::Conflict);
        }
        self.reserved_rules -= 1;
        self.live_rules = self.live_rules.checked_add(1).ok_or(QdnfError::Capacity)?;
        Ok(())
    }

    /// Release the exclusive lease. Subsequent `LeaseTable::release` of a stored
    /// handle copy is [`QdnfError::DoubleRelease`] or [`QdnfError::StaleGeneration`].
    pub fn revoke(self, leases: &mut LeaseTable) -> Result<(), QdnfError> {
        leases.release(self.lease.handle)
    }

    /// Live (activated) rule count for this admit record.
    pub const fn live_rules(&self) -> u16 {
        self.live_rules
    }

    /// Reserved but not yet activated rule count for this admit record.
    pub const fn reserved_rules(&self) -> u16 {
        self.reserved_rules
    }
}

/// Classify a caller backing size. Exact 42 MiB is the Sentinel pass; (42 MiB, 512 MiB]
/// is an ordinary cell. Smaller than 42 MiB is not a documented scratch class here.
/// Over 512 MiB is over the cell budget. Both fail closed as [`QdnfError::Capacity`].
pub fn classify_budget(bytes: u64) -> Result<ArenaBudgetClass, QdnfError> {
    let sentinel = SENTINEL_PASS_BYTES as u64;
    let cell = ORDINARY_CELL_BYTES as u64;
    if bytes == sentinel {
        Ok(ArenaBudgetClass::SentinelPass)
    } else if bytes > sentinel && bytes <= cell {
        Ok(ArenaBudgetClass::OrdinaryCell)
    } else {
        let _ = ArenaBudgetClass::Rejected;
        Err(QdnfError::Capacity)
    }
}

/// Acquire an exclusive RT-01 lease for `backing_bytes` and bind scope + generations.
///
/// `backing_bytes` is cast-checked to `u32`. This does not allocate arena storage.
pub fn admit(
    leases: &mut LeaseTable,
    scope: u64,
    policy_generation: u64,
    source_generation: u64,
    backing_bytes: u64,
) -> Result<ArenaAdmit, QdnfError> {
    match classify_budget(backing_bytes)? {
        ArenaBudgetClass::SentinelPass | ArenaBudgetClass::OrdinaryCell => {}
        ArenaBudgetClass::Rejected => return Err(QdnfError::Capacity),
    }
    let capacity = u32::try_from(backing_bytes).map_err(|_| QdnfError::Capacity)?;
    let lease = leases.acquire(capacity, true)?;
    Ok(ArenaAdmit {
        scope,
        policy_generation,
        source_generation,
        lease,
        live_rules: 0,
        reserved_rules: 0,
    })
}

/// Occupied hash-slot collision: always recompute. A miss is not completeness.
pub fn collision_on_occupied_hash_slot() -> CollisionPolicy {
    CollisionPolicy::MustRecompute
}

/// Classify stored parity. Canonical five-field wins when both XOR folds match
/// (e.g. `metadata == 0`). Garbage is [`QdnfError::Malformed`]. Never dual-accept.
pub fn classify_parity(
    stored: u64,
    s: u64,
    p: u64,
    o: u64,
    ctx: u64,
    metadata: u64,
) -> Result<ParityKind, QdnfError> {
    let five = NQuin::calculate_parity(s, p, o, ctx, metadata);
    if stored == five {
        return Ok(ParityKind::FiveFieldCanonical);
    }
    let four = s ^ p ^ o ^ ctx;
    if stored == four {
        return Ok(ParityKind::FourFieldLegacy);
    }
    Err(QdnfError::Malformed)
}

/// Empty / unoccupied table entries fail closed. They are not an allow.
pub fn empty_entry_is_not_allow(occupied: bool) -> Result<(), QdnfError> {
    if occupied {
        Ok(())
    } else {
        Err(QdnfError::Denied)
    }
}

/// The recent-slot ring is lossy. It must not certify obligation.
pub fn recent_ring_may_certify_obligation() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::LeaseHandle;

    fn admit_sentinel(leases: &mut LeaseTable, scope: u64, policy: u64, source: u64) -> ArenaAdmit {
        admit(leases, scope, policy, source, SENTINEL_PASS_BYTES as u64).expect("admit sentinel")
    }

    #[test]
    fn classify_budget_pass_cell_and_capacity() {
        assert_eq!(
            classify_budget(SENTINEL_PASS_BYTES as u64),
            Ok(ArenaBudgetClass::SentinelPass)
        );
        assert_eq!(
            classify_budget(ORDINARY_CELL_BYTES as u64),
            Ok(ArenaBudgetClass::OrdinaryCell)
        );
        assert_eq!(
            classify_budget(SENTINEL_PASS_BYTES as u64 + 1),
            Ok(ArenaBudgetClass::OrdinaryCell)
        );
        assert_eq!(
            classify_budget(ORDINARY_CELL_BYTES as u64 + 1),
            Err(QdnfError::Capacity)
        );
        assert_eq!(classify_budget(1), Err(QdnfError::Capacity));
    }

    #[test]
    fn admit_require_scope_rejects_wrong_scope_and_stale_source() {
        let mut leases = LeaseTable::new();
        let rec = admit_sentinel(&mut leases, 0xA, 1, 10);
        rec.require_scope(0xA, 1, 10).unwrap();
        assert_eq!(
            rec.require_scope(0xB, 1, 10),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(
            rec.require_scope(0xA, 1, 9),
            Err(QdnfError::StaleGeneration)
        );
        rec.revoke(&mut leases).unwrap();
    }

    #[test]
    fn reset_for_reuse_clears_live_rules_and_keeps_lease() {
        let mut leases = LeaseTable::new();
        let mut rec = admit_sentinel(&mut leases, 7, 2, 3);
        rec.reserve_rule_slot().unwrap();
        rec.activate_reserved_rule().unwrap();
        assert_eq!(rec.live_rules(), 1);
        rec.reset_for_reuse().unwrap();
        assert_eq!(rec.live_rules(), 0);
        assert_eq!(rec.reserved_rules(), 0);
        rec.require_scope(7, 2, 3).unwrap();
        assert!(rec.lease.exclusive);
        assert_eq!(rec.lease.capacity, SENTINEL_PASS_BYTES as u32);
        rec.revoke(&mut leases).unwrap();
    }

    #[test]
    fn reserve_then_activate_conflict_without_reserve_and_sixty_fifth_capacity() {
        let mut leases = LeaseTable::new();
        let mut rec = admit_sentinel(&mut leases, 1, 1, 1);
        rec.reserve_rule_slot().unwrap();
        rec.activate_reserved_rule().unwrap();
        assert_eq!(rec.live_rules(), 1);
        assert_eq!(
            rec.activate_reserved_rule(),
            Err(QdnfError::Conflict)
        );
        rec.reset_for_reuse().unwrap();
        for _ in 0..MAX_RULE_STAGING {
            rec.reserve_rule_slot().unwrap();
        }
        assert_eq!(rec.reserve_rule_slot(), Err(QdnfError::Capacity));
        rec.revoke(&mut leases).unwrap();
    }

    #[test]
    fn revoke_then_release_of_old_handle_is_double_release_or_stale() {
        let mut leases = LeaseTable::new();
        let rec = admit_sentinel(&mut leases, 4, 5, 6);
        let handle: LeaseHandle = rec.lease.handle;
        let _generation_copy = handle.generation;
        rec.revoke(&mut leases).unwrap();
        let second = leases.release(handle);
        assert!(
            second == Err(QdnfError::DoubleRelease)
                || second == Err(QdnfError::StaleGeneration),
            "expected DoubleRelease or StaleGeneration, got {second:?}"
        );
    }

    #[test]
    fn occupied_hash_slot_collision_is_must_recompute() {
        assert_eq!(
            collision_on_occupied_hash_slot(),
            CollisionPolicy::MustRecompute
        );
    }

    #[test]
    fn classify_parity_five_field_four_field_and_malformed() {
        let s = 0x11;
        let p = 0x22;
        let o = 0x33;
        let ctx = 0x44;
        let metadata = 0x55;
        let five = NQuin::calculate_parity(s, p, o, ctx, metadata);
        let four = s ^ p ^ o ^ ctx;
        assert_ne!(five, four, "non-zero metadata must distinguish the two folds");
        assert_eq!(
            classify_parity(five, s, p, o, ctx, metadata),
            Ok(ParityKind::FiveFieldCanonical)
        );
        assert_eq!(
            classify_parity(four, s, p, o, ctx, metadata),
            Ok(ParityKind::FourFieldLegacy)
        );
        assert_eq!(
            classify_parity(0xDEAD_BEEF, s, p, o, ctx, metadata),
            Err(QdnfError::Malformed)
        );
        // Dual-match (metadata == 0) classifies as canonical five-field, not both.
        let zero_meta = 0;
        let both = NQuin::calculate_parity(s, p, o, ctx, zero_meta);
        assert_eq!(both, s ^ p ^ o ^ ctx);
        assert_eq!(
            classify_parity(both, s, p, o, ctx, zero_meta),
            Ok(ParityKind::FiveFieldCanonical)
        );
    }

    #[test]
    fn empty_entry_is_denied_not_allow() {
        assert_eq!(empty_entry_is_not_allow(false), Err(QdnfError::Denied));
        empty_entry_is_not_allow(true).unwrap();
    }

    #[test]
    fn recent_ring_must_not_certify_obligation() {
        assert!(!RECENT_RING_IS_DURABLE);
        assert!(!recent_ring_may_certify_obligation());
    }

    #[test]
    fn quin_abi_and_sentinel_slot_constants() {
        assert_eq!(QUIN_ABI_BYTES, 48);
        assert_eq!(SENTINEL_SLOTS, 917_504);
        assert_eq!(SENTINEL_PASS_BYTES, 42 * 1024 * 1024);
        assert_eq!(SENTINEL_SLOTS, SENTINEL_PASS_BYTES / QUIN_ABI_BYTES);
    }
}
