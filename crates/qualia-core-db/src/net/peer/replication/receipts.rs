//! Identity, local effect, and durable receipt as distinct stages (SVC-01.04).
//!
//! Operation identity is a SHA-384 [`StrongDigest`]. Binding an id is not a
//! local graph effect. A local effect is not a durable receipt. Transport ACK
//! and consumption never become [`ReceiptClass::DurableReceipt`].
//! [`DeliveryStage::DurableApplied`] is a durable receipt only after the
//! effect ran. [`DeliveryStage::EconomicReceipt`] requires that durable-applied
//! stage already recorded. Packages remain open; this library does not claim
//! exactly-once external work or CORE-03 atomic commit.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::datagrams::DeliveryStage;
use crate::net::qdnf::types::StrongDigest;

/// Occupied identity slots. A 17th distinct bind is Capacity.
pub const MAX_RECEIPTS: usize = 16;

/// Durability class actually achieved. Transport ACK is never this enum's
/// durable variant.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReceiptClass {
    /// Id bound; no local effect.
    Identity = 1,
    /// Local apply; not durable.
    Effect = 2,
    /// Durable applied receipt (or economic receipt after that stage).
    DurableReceipt = 3,
}

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    effect: bool,
    durable_applied: bool,
    id: StrongDigest,
}

/// Sixteen-slot identity / effect / receipt table.
pub struct ReceiptTable {
    slots: [Slot; MAX_RECEIPTS],
}

impl ReceiptTable {
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                occupied: false,
                effect: false,
                durable_applied: false,
                id: StrongDigest::ZERO,
            }; MAX_RECEIPTS],
        }
    }

    fn find_id(&self, id: &StrongDigest) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_RECEIPTS {
            if self.slots[i].occupied && self.slots[i].id == *id {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    fn find_free(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_RECEIPTS {
            if !self.slots[i].occupied {
                return Some(i);
            }
            i += 1;
        }
        None
    }
}

impl Default for ReceiptTable {
    fn default() -> Self {
        Self::new()
    }
}

fn reject_zero(id: StrongDigest) -> Result<(), QdnfError> {
    if id == StrongDigest::ZERO {
        Err(QdnfError::Malformed)
    } else {
        Ok(())
    }
}

/// Bind `id` as operation identity. Zero digest is malformed. A duplicate bind
/// of the same id is idempotent. Distinct ids beyond [`MAX_RECEIPTS`] are
/// capacity.
pub fn bind_identity(table: &mut ReceiptTable, id: StrongDigest) -> Result<(), QdnfError> {
    reject_zero(id)?;
    if table.find_id(&id).is_some() {
        return Ok(());
    }
    let idx = table.find_free().ok_or(QdnfError::Capacity)?;
    table.slots[idx] = Slot {
        occupied: true,
        effect: false,
        durable_applied: false,
        id,
    };
    Ok(())
}

/// Record a local graph effect. Requires a bound identity. Unknown id is
/// unauthorized. Idempotent once the effect is applied.
pub fn apply_effect(table: &mut ReceiptTable, id: StrongDigest) -> Result<(), QdnfError> {
    reject_zero(id)?;
    let idx = table.find_id(&id).ok_or(QdnfError::Unauthorized)?;
    table.slots[idx].effect = true;
    Ok(())
}

/// Map a delivery stage onto the durability class actually achieved.
///
/// Transport ACK and consumption are never [`ReceiptClass::DurableReceipt`].
/// Durable-applied requires the local effect. Economic receipt requires that
/// durable-applied was already recorded.
pub fn acknowledge(
    table: &mut ReceiptTable,
    id: StrongDigest,
    stage: DeliveryStage,
) -> Result<ReceiptClass, QdnfError> {
    reject_zero(id)?;
    let idx = table.find_id(&id).ok_or(QdnfError::Unauthorized)?;
    let slot = &mut table.slots[idx];
    match stage {
        DeliveryStage::TransportAck | DeliveryStage::Consumed => {
            if slot.effect {
                Ok(ReceiptClass::Effect)
            } else {
                Ok(ReceiptClass::Identity)
            }
        }
        DeliveryStage::DurableApplied => {
            if !slot.effect {
                return Err(QdnfError::Incomplete);
            }
            slot.durable_applied = true;
            Ok(ReceiptClass::DurableReceipt)
        }
        DeliveryStage::EconomicReceipt => {
            if !slot.durable_applied {
                return Err(QdnfError::Incomplete);
            }
            Ok(ReceiptClass::DurableReceipt)
        }
    }
}

/// Transport ACK is not a durable receipt class.
pub fn transport_ack_is_receipt_class() -> bool {
    false
}

/// Identity, local effect, and durable receipt are three distinct stages.
pub fn identity_effect_receipt_are_distinct() -> bool {
    ReceiptClass::Identity as u8 != ReceiptClass::Effect as u8
        && ReceiptClass::Effect as u8 != ReceiptClass::DurableReceipt as u8
        && ReceiptClass::Identity as u8 != ReceiptClass::DurableReceipt as u8
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: u8) -> StrongDigest {
        StrongDigest([tag; 48])
    }

    #[test]
    fn bind_then_transport_ack_is_identity_not_durable() {
        let mut table = ReceiptTable::new();
        let id = digest(1);
        bind_identity(&mut table, id).unwrap();
        let class = acknowledge(&mut table, id, DeliveryStage::TransportAck).unwrap();
        assert_eq!(class, ReceiptClass::Identity);
        assert_ne!(class, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn apply_effect_then_transport_ack_is_not_durable() {
        let mut table = ReceiptTable::new();
        let id = digest(2);
        bind_identity(&mut table, id).unwrap();
        apply_effect(&mut table, id).unwrap();
        let class = acknowledge(&mut table, id, DeliveryStage::TransportAck).unwrap();
        assert_eq!(class, ReceiptClass::Effect);
        assert_ne!(class, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn apply_effect_then_durable_applied_is_durable_receipt() {
        let mut table = ReceiptTable::new();
        let id = digest(3);
        bind_identity(&mut table, id).unwrap();
        apply_effect(&mut table, id).unwrap();
        let class = acknowledge(&mut table, id, DeliveryStage::DurableApplied).unwrap();
        assert_eq!(class, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn durable_applied_without_effect_is_incomplete() {
        let mut table = ReceiptTable::new();
        let id = digest(4);
        bind_identity(&mut table, id).unwrap();
        assert_eq!(
            acknowledge(&mut table, id, DeliveryStage::DurableApplied),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn economic_receipt_without_durable_applied_is_incomplete() {
        let mut table = ReceiptTable::new();
        let id = digest(5);
        bind_identity(&mut table, id).unwrap();
        apply_effect(&mut table, id).unwrap();
        assert_eq!(
            acknowledge(&mut table, id, DeliveryStage::EconomicReceipt),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn unknown_id_is_unauthorized() {
        let mut table = ReceiptTable::new();
        let id = digest(6);
        assert_eq!(
            acknowledge(&mut table, id, DeliveryStage::TransportAck),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(apply_effect(&mut table, id), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn zero_digest_is_malformed() {
        let mut table = ReceiptTable::new();
        assert_eq!(
            bind_identity(&mut table, StrongDigest::ZERO),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            apply_effect(&mut table, StrongDigest::ZERO),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            acknowledge(&mut table, StrongDigest::ZERO, DeliveryStage::TransportAck),
            Err(QdnfError::Malformed)
        );
    }

    #[test]
    fn seventeenth_identity_is_capacity() {
        let mut table = ReceiptTable::new();
        let mut n = 1u8;
        while n <= MAX_RECEIPTS as u8 {
            bind_identity(&mut table, digest(n)).unwrap();
            n += 1;
        }
        assert_eq!(
            bind_identity(&mut table, digest(0x20)),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn transport_ack_is_not_a_receipt_class() {
        assert!(!transport_ack_is_receipt_class());
        assert_ne!(DeliveryStage::TransportAck, DeliveryStage::DurableApplied);
    }

    #[test]
    fn identity_effect_and_receipt_are_distinct() {
        assert!(identity_effect_receipt_are_distinct());
        assert_ne!(ReceiptClass::Identity, ReceiptClass::Effect);
        assert_ne!(ReceiptClass::Effect, ReceiptClass::DurableReceipt);
        assert_ne!(ReceiptClass::Identity, ReceiptClass::DurableReceipt);
    }

    #[test]
    fn double_bind_identity_is_idempotent() {
        let mut table = ReceiptTable::new();
        let id = digest(7);
        bind_identity(&mut table, id).unwrap();
        bind_identity(&mut table, id).unwrap();
        let class = acknowledge(&mut table, id, DeliveryStage::TransportAck).unwrap();
        assert_eq!(class, ReceiptClass::Identity);
    }
}
