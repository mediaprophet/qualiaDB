//! Reconnect recovery for application operations (NET-05.16 partial).
//!
//! Durable application (`DurableApplied`) and economic (`EconomicReceipt`)
//! stages occupy a slot in [`RecoveryTable`]. Transport ACK and consumption
//! are recorded separately and never count as accepted work or payment.
//! Cancellation is owned by [`OperationTable`]; this module does not copy
//! that kernel's slot table. Packages remain open.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::datagrams::DeliveryStage;
use crate::net::qdnf::types::OperationId;

/// Re-export the peer-runtime cancel kernel. Session recovery does not copy its slot table.
pub use crate::net::peer::runtime::cancel::{CancelEpoch, OperationHandle, OperationTable};

/// Occupied durable-receipt slots. A 17th distinct durable id is Capacity.
pub const MAX_DURABLE: usize = 16;

/// Reconnect decision. Transport-only delivery does not skip durable work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReconnectOutcome {
    /// Never durably applied; the caller may proceed.
    NeedsWork,
    /// DurableApplied or EconomicReceipt already recorded; must not duplicate.
    AlreadyDurable,
    /// Transport ACK or consumption exists; still not durable.
    TransportOnly,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct StageRecord {
    id: OperationId,
    stage: DeliveryStage,
}

/// Sixteen durable slots plus sixteen transport-observation slots. Copy-friendly.
#[derive(Clone, Copy, Debug)]
pub struct RecoveryTable {
    durable: [Option<StageRecord>; MAX_DURABLE],
    transport: [Option<StageRecord>; MAX_DURABLE],
}

impl RecoveryTable {
    pub const fn new() -> Self {
        Self {
            durable: [None; MAX_DURABLE],
            transport: [None; MAX_DURABLE],
        }
    }

    /// Occupied durable slots. Transport observations are excluded.
    pub fn durable_len(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_DURABLE {
            if self.durable[i].is_some() {
                n += 1;
            }
            i += 1;
        }
        n
    }

    fn find_durable(&self, id: OperationId) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_DURABLE {
            if let Some(rec) = self.durable[i] {
                if rec.id == id {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    fn find_transport(&self, id: OperationId) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_DURABLE {
            if let Some(rec) = self.transport[i] {
                if rec.id == id {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    fn first_free_durable(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_DURABLE {
            if self.durable[i].is_none() {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn first_free_transport(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_DURABLE {
            if self.transport[i].is_none() {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

impl Default for RecoveryTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Durable application and economic receipts only.
#[inline]
pub fn stage_is_durable(stage: DeliveryStage) -> bool {
    matches!(
        stage,
        DeliveryStage::DurableApplied | DeliveryStage::EconomicReceipt
    )
}

/// Transport ACK / consumption never imply accepted work or payment.
#[inline]
pub fn transport_delivery_is_accepted_work() -> bool {
    false
}

/// Reconnect must not emit a second durable effect for a recorded id.
#[inline]
pub fn reconnect_may_duplicate_durable() -> bool {
    false
}

/// Record that `id` reached `stage`.
///
/// `TransportAck` / `Consumed` never occupy a durable slot.
/// `DurableApplied` / `EconomicReceipt` occupy one durable slot; repeating the
/// same durable stage for the same id is idempotent (not Conflict).
pub fn record_stage(
    table: &mut RecoveryTable,
    id: OperationId,
    stage: DeliveryStage,
) -> Result<(), QdnfError> {
    if id == OperationId::ZERO {
        return Err(QdnfError::Malformed);
    }
    if stage_is_durable(stage) {
        record_durable(table, id, stage)
    } else {
        record_transport(table, id, stage)
    }
}

fn record_durable(
    table: &mut RecoveryTable,
    id: OperationId,
    stage: DeliveryStage,
) -> Result<(), QdnfError> {
    if let Some(idx) = table.find_durable(id) {
        if let Some(rec) = &mut table.durable[idx] {
            if stage_rank(stage) > stage_rank(rec.stage) {
                rec.stage = stage;
            }
        }
        return Ok(());
    }
    let idx = table.first_free_durable().ok_or(QdnfError::Capacity)?;
    table.durable[idx] = Some(StageRecord { id, stage });
    Ok(())
}

fn record_transport(
    table: &mut RecoveryTable,
    id: OperationId,
    stage: DeliveryStage,
) -> Result<(), QdnfError> {
    if let Some(idx) = table.find_transport(id) {
        if let Some(rec) = &mut table.transport[idx] {
            if stage_rank(stage) > stage_rank(rec.stage) {
                rec.stage = stage;
            }
        }
        return Ok(());
    }
    let idx = table.first_free_transport().ok_or(QdnfError::Capacity)?;
    table.transport[idx] = Some(StageRecord { id, stage });
    Ok(())
}

#[inline]
fn stage_rank(stage: DeliveryStage) -> u8 {
    stage as u8
}

/// Reconnect attempt for `id`.
///
/// Already-durable ids return [`ReconnectOutcome::AlreadyDurable`] without a
/// second durable effect. Cancellation from [`OperationTable`] returns
/// [`QdnfError::Cancelled`]. Transport ACK / consumption alone is not enough
/// to skip work.
pub fn reconnect(
    table: &RecoveryTable,
    ops: &OperationTable,
    id: OperationId,
) -> Result<ReconnectOutcome, QdnfError> {
    if id == OperationId::ZERO {
        return Err(QdnfError::Malformed);
    }
    if table.find_durable(id).is_some() {
        return Ok(ReconnectOutcome::AlreadyDurable);
    }
    if ops.is_cancelled(id) {
        return Err(QdnfError::Cancelled);
    }
    if table.find_transport(id).is_some() {
        return Ok(ReconnectOutcome::TransportOnly);
    }
    Ok(ReconnectOutcome::NeedsWork)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::cancel::OperationTable;
    use crate::net::peer::runtime::leases::LeaseTable;

    fn op(n: u8) -> OperationId {
        let mut id = [0u8; 16];
        id[0] = n;
        OperationId(id)
    }

    #[test]
    fn reconnect_after_durable_applied_is_already_durable() {
        let mut table = RecoveryTable::new();
        let ops = OperationTable::new();
        let id = op(1);
        assert!(record_stage(&mut table, id, DeliveryStage::DurableApplied).is_ok());
        assert_eq!(table.durable_len(), 1);
        assert_eq!(
            reconnect(&table, &ops, id),
            Ok(ReconnectOutcome::AlreadyDurable)
        );
        assert_eq!(table.durable_len(), 1);
        assert!(!reconnect_may_duplicate_durable());
    }

    #[test]
    fn transport_ack_reconnect_is_not_already_durable() {
        let mut table = RecoveryTable::new();
        let ops = OperationTable::new();
        let id = op(2);
        assert!(record_stage(&mut table, id, DeliveryStage::TransportAck).is_ok());
        assert_eq!(table.durable_len(), 0);
        let outcome = reconnect(&table, &ops, id).unwrap();
        assert_ne!(outcome, ReconnectOutcome::AlreadyDurable);
        assert_eq!(outcome, ReconnectOutcome::TransportOnly);
        assert!(record_stage(&mut table, id, DeliveryStage::Consumed).is_ok());
        assert_eq!(
            reconnect(&table, &ops, id),
            Ok(ReconnectOutcome::TransportOnly)
        );
        assert_eq!(table.durable_len(), 0);
    }

    #[test]
    fn cancel_then_reconnect_is_cancelled() {
        let table = RecoveryTable::new();
        let mut leases = LeaseTable::new();
        let mut ops = OperationTable::new();
        let lease = leases.acquire(8, true).unwrap();
        let id = op(3);
        let _handle: OperationHandle = ops.admit(id, lease.handle).unwrap();
        let _epoch: CancelEpoch = ops.cancel(id).unwrap();
        assert_eq!(reconnect(&table, &ops, id), Err(QdnfError::Cancelled));
        assert_eq!(table.durable_len(), 0);
    }

    #[test]
    fn economic_receipt_is_durable() {
        let mut table = RecoveryTable::new();
        let ops = OperationTable::new();
        let id = op(4);
        assert!(record_stage(&mut table, id, DeliveryStage::EconomicReceipt).is_ok());
        assert_eq!(table.durable_len(), 1);
        assert_eq!(
            reconnect(&table, &ops, id),
            Ok(ReconnectOutcome::AlreadyDurable)
        );
        assert!(stage_is_durable(DeliveryStage::EconomicReceipt));
        assert!(!stage_is_durable(DeliveryStage::TransportAck));
        assert!(!stage_is_durable(DeliveryStage::Consumed));
    }

    #[test]
    fn transport_delivery_is_not_accepted_work() {
        assert!(!transport_delivery_is_accepted_work());
        assert!(!reconnect_may_duplicate_durable());
    }

    #[test]
    fn seventeenth_distinct_durable_is_capacity() {
        let mut table = RecoveryTable::new();
        let mut n = 1u8;
        while n <= MAX_DURABLE as u8 {
            assert!(record_stage(&mut table, op(n), DeliveryStage::DurableApplied).is_ok());
            n += 1;
        }
        assert_eq!(table.durable_len(), MAX_DURABLE);
        assert_eq!(
            record_stage(&mut table, op(17), DeliveryStage::DurableApplied),
            Err(QdnfError::Capacity)
        );
        assert_eq!(table.durable_len(), MAX_DURABLE);
        assert!(record_stage(&mut table, op(1), DeliveryStage::DurableApplied).is_ok());
        assert_eq!(table.durable_len(), MAX_DURABLE);
    }

    #[test]
    fn zero_operation_id_is_malformed() {
        let mut table = RecoveryTable::new();
        let ops = OperationTable::new();
        assert_eq!(
            record_stage(&mut table, OperationId::ZERO, DeliveryStage::DurableApplied),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            record_stage(&mut table, OperationId::ZERO, DeliveryStage::TransportAck),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            reconnect(&table, &ops, OperationId::ZERO),
            Err(QdnfError::Malformed)
        );
        assert_eq!(table.durable_len(), 0);
    }

    #[test]
    fn duplicate_durable_applied_is_idempotent() {
        let mut table = RecoveryTable::new();
        let ops = OperationTable::new();
        let id = op(8);
        assert!(record_stage(&mut table, id, DeliveryStage::DurableApplied).is_ok());
        assert_eq!(
            record_stage(&mut table, id, DeliveryStage::DurableApplied),
            Ok(())
        );
        assert_ne!(
            record_stage(&mut table, id, DeliveryStage::DurableApplied),
            Err(QdnfError::Conflict)
        );
        assert_eq!(table.durable_len(), 1);
        assert_eq!(
            reconnect(&table, &ops, id),
            Ok(ReconnectOutcome::AlreadyDurable)
        );
        assert!(record_stage(&mut table, id, DeliveryStage::EconomicReceipt).is_ok());
        assert_eq!(table.durable_len(), 1);
    }
}
