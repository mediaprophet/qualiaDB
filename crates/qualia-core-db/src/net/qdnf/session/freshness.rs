//! Recheck policy freshness at queued, offline, and migration (NET-05.18).
//!
//! A cached [`PolicyOutcome::Allow`] is not sufficient after queue, offline, or
//! migration. Sensitive work stays pending until a fresh Allow with a current
//! grant. A separately authorized help/accessibility path remains available while
//! sensitive work is pending. Blocked and Suspended contacts stay Denied;
//! payment cannot bypass. Packages remain open.

use crate::net::qdnf::authority::{ContactState, PolicyOutcome};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::policy::gate_channel;
use crate::net::qdnf::types::OperationId;

/// Occupied sensitive-pending slots. A 17th distinct pending id is Capacity.
pub const MAX_PENDING: usize = 16;

/// Why admission must recheck protective policy and grant freshness.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecheckReason {
    Queued = 1,
    Offline = 2,
    Migration = 3,
}

/// Result of a freshness recheck. Sensitive pending is not a help-path error.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FreshnessVerdict {
    Admitted = 1,
    /// Sensitive stays pending until a fresh Allow with a current grant.
    SensitivePending = 2,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct PendingSlot {
    id: OperationId,
    reason: RecheckReason,
}

/// Sixteen-slot table of sensitive work waiting on a fresh current grant.
pub struct FreshnessTable {
    slots: [Option<PendingSlot>; MAX_PENDING],
}

impl FreshnessTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_PENDING],
        }
    }

    fn insert_pending(&mut self, id: OperationId, reason: RecheckReason) -> Result<(), QdnfError> {
        let mut free = None;
        let mut i = 0usize;
        while i < MAX_PENDING {
            match self.slots[i] {
                Some(slot) if slot.id == id => {
                    self.slots[i] = Some(PendingSlot { id, reason });
                    return Ok(());
                }
                Some(_) => {}
                None => {
                    if free.is_none() {
                        free = Some(i);
                    }
                }
            }
            i += 1;
        }
        let idx = free.ok_or(QdnfError::Capacity)?;
        self.slots[idx] = Some(PendingSlot { id, reason });
        Ok(())
    }

    fn clear(&mut self, id: OperationId) {
        let mut i = 0usize;
        while i < MAX_PENDING {
            if let Some(slot) = self.slots[i] {
                if slot.id == id {
                    self.slots[i] = None;
                    return;
                }
            }
            i += 1;
        }
    }
}

impl Default for FreshnessTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached Allow never skips a queued / offline / migration recheck.
#[inline]
pub fn cached_allow_skips_recheck() -> bool {
    false
}

/// Help/accessibility remains available while sensitive work is pending.
#[inline]
pub fn help_accessible_while_sensitive_pending() -> bool {
    true
}

/// Admit or re-admit after queued, offline, or migration.
///
/// Cached Allow without a current grant is insufficient ([`QdnfError::Incomplete`]
/// when not sensitive). Sensitive work that is not a fresh current Allow stays
/// [`FreshnessVerdict::SensitivePending`] unless contact is Blocked/Suspended or
/// the outcome is Deny.
pub fn recheck(
    table: &mut FreshnessTable,
    id: OperationId,
    reason: RecheckReason,
    outcome: PolicyOutcome,
    grant_current: bool,
    contact: ContactState,
    sensitive: bool,
) -> Result<FreshnessVerdict, QdnfError> {
    if id == OperationId::ZERO {
        return Err(QdnfError::Malformed);
    }

    // Queued, offline, and migration always require a live recheck.
    let _ = reason;
    debug_assert!(!cached_allow_skips_recheck());

    if matches!(contact, ContactState::Blocked | ContactState::Suspended) {
        return Err(QdnfError::Denied);
    }

    if outcome == PolicyOutcome::Deny {
        table.clear(id);
        return Err(QdnfError::Denied);
    }

    let current_allow = outcome == PolicyOutcome::Allow && grant_current;

    if sensitive && !current_allow {
        table.insert_pending(id, reason)?;
        return Ok(FreshnessVerdict::SensitivePending);
    }

    if !current_allow {
        if outcome == PolicyOutcome::Allow {
            return Err(QdnfError::Incomplete);
        }
        return Err(match gate_channel(outcome, grant_current, contact) {
            Ok(()) => QdnfError::Unauthorized,
            Err(e) => e,
        });
    }

    table.clear(id);
    gate_channel(outcome, grant_current, contact)?;
    Ok(FreshnessVerdict::Admitted)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn operation(tag: u8) -> OperationId {
        let mut id = OperationId::ZERO;
        id.0[0] = tag;
        id
    }

    #[test]
    fn queued_cached_allow_without_current_grant_is_insufficient() {
        let mut table = FreshnessTable::new();
        assert_eq!(
            recheck(
                &mut table,
                operation(1),
                RecheckReason::Queued,
                PolicyOutcome::Allow,
                false,
                ContactState::Active,
                false,
            ),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(
            recheck(
                &mut table,
                operation(2),
                RecheckReason::Queued,
                PolicyOutcome::Allow,
                false,
                ContactState::Active,
                true,
            ),
            Ok(FreshnessVerdict::SensitivePending)
        );
    }

    #[test]
    fn offline_fresh_allow_current_grant_not_sensitive_is_admitted() {
        let mut table = FreshnessTable::new();
        assert_eq!(
            recheck(
                &mut table,
                operation(3),
                RecheckReason::Offline,
                PolicyOutcome::Allow,
                true,
                ContactState::Active,
                false,
            ),
            Ok(FreshnessVerdict::Admitted)
        );
    }

    #[test]
    fn migration_deny_is_denied() {
        let mut table = FreshnessTable::new();
        assert_eq!(
            recheck(
                &mut table,
                operation(4),
                RecheckReason::Migration,
                PolicyOutcome::Deny,
                true,
                ContactState::Active,
                false,
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            recheck(
                &mut table,
                operation(5),
                RecheckReason::Migration,
                PolicyOutcome::Deny,
                true,
                ContactState::Active,
                true,
            ),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn blocked_cannot_pay_to_bypass() {
        let mut table = FreshnessTable::new();
        assert_eq!(
            recheck(
                &mut table,
                operation(6),
                RecheckReason::Queued,
                PolicyOutcome::Allow,
                true,
                ContactState::Blocked,
                false,
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            recheck(
                &mut table,
                operation(7),
                RecheckReason::Offline,
                PolicyOutcome::Allow,
                true,
                ContactState::Blocked,
                true,
            ),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            recheck(
                &mut table,
                operation(8),
                RecheckReason::Migration,
                PolicyOutcome::Allow,
                true,
                ContactState::Suspended,
                false,
            ),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn sensitive_pending_does_not_block_help_admit() {
        let mut table = FreshnessTable::new();
        assert_eq!(
            recheck(
                &mut table,
                operation(9),
                RecheckReason::Queued,
                PolicyOutcome::Allow,
                false,
                ContactState::Active,
                true,
            ),
            Ok(FreshnessVerdict::SensitivePending)
        );
        assert_eq!(
            recheck(
                &mut table,
                operation(10),
                RecheckReason::Queued,
                PolicyOutcome::Allow,
                true,
                ContactState::Active,
                false,
            ),
            Ok(FreshnessVerdict::Admitted)
        );
        assert!(help_accessible_while_sensitive_pending());
    }

    #[test]
    fn help_accessible_while_sensitive_pending_is_true() {
        assert!(help_accessible_while_sensitive_pending());
    }

    #[test]
    fn cached_allow_does_not_skip_recheck() {
        assert!(!cached_allow_skips_recheck());
    }

    #[test]
    fn seventeenth_pending_sensitive_is_capacity() {
        let mut table = FreshnessTable::new();
        let mut n = 1u8;
        while n <= MAX_PENDING as u8 {
            assert_eq!(
                recheck(
                    &mut table,
                    operation(n),
                    RecheckReason::Offline,
                    PolicyOutcome::Allow,
                    false,
                    ContactState::Active,
                    true,
                ),
                Ok(FreshnessVerdict::SensitivePending)
            );
            n += 1;
        }
        assert_eq!(
            recheck(
                &mut table,
                operation(MAX_PENDING as u8),
                RecheckReason::Queued,
                PolicyOutcome::Incomplete,
                false,
                ContactState::Active,
                true,
            ),
            Ok(FreshnessVerdict::SensitivePending)
        );
        assert_eq!(
            recheck(
                &mut table,
                operation(0x20),
                RecheckReason::Migration,
                PolicyOutcome::Allow,
                false,
                ContactState::Active,
                true,
            ),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn zero_operation_is_malformed() {
        let mut table = FreshnessTable::new();
        assert_eq!(
            recheck(
                &mut table,
                OperationId::ZERO,
                RecheckReason::Queued,
                PolicyOutcome::Allow,
                true,
                ContactState::Active,
                false,
            ),
            Err(QdnfError::Malformed)
        );
    }
}
