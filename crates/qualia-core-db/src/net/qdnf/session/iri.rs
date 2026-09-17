//! Service-channel IRI binding and delivery-stage receipts (NET-05.06 partial).
//!
//! IRI bytes are already hashed by the caller into [`StrongDigest`]. Two
//! different services claiming the same digest is [`QdnfError::Conflict`];
//! the same digest on the same service is idempotent. Transport ACK and
//! consumption are not durable application or economic receipts. Datagram
//! expiry is independent of this table. Packages remain open.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::datagrams::DeliveryStage;
use crate::net::qdnf::types::{OperationId, StrongDigest};

/// Occupied service-channel slots. A 17th distinct bind is Capacity.
pub const MAX_CHANNELS: usize = 16;

/// Bind a caller-hashed service IRI to a service channel and operation.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ChannelBind {
    pub iri: StrongDigest,
    pub service: StrongDigest,
    pub operation: OperationId,
}

/// Sixteen-slot service-channel table. Empty slots are `None`.
pub struct ChannelTable {
    slots: [Option<ChannelBind>; MAX_CHANNELS],
}

impl ChannelTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_CHANNELS],
        }
    }

    /// Bind `bind` into a free slot.
    ///
    /// Zero IRI or zero service → [`QdnfError::Malformed`]. Same IRI and
    /// service → `Ok` (idempotent). Same IRI, different service →
    /// [`QdnfError::Conflict`]. No free slot for a distinct bind →
    /// [`QdnfError::Capacity`].
    pub fn bind(&mut self, bind: ChannelBind) -> Result<(), QdnfError> {
        if bind.iri == StrongDigest::ZERO || bind.service == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        let mut free = None;
        let mut i = 0usize;
        while i < MAX_CHANNELS {
            match self.slots[i] {
                Some(existing) if existing.iri == bind.iri => {
                    if existing.service == bind.service {
                        return Ok(());
                    }
                    return Err(QdnfError::Conflict);
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
        self.slots[idx] = Some(bind);
        Ok(())
    }
}

impl Default for ChannelTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Durable application and economic receipts only. Transport ACK and
/// consumption do not imply durable applied state.
pub fn stage_implies_durable(stage: DeliveryStage) -> bool {
    matches!(
        stage,
        DeliveryStage::DurableApplied | DeliveryStage::EconomicReceipt
    )
}

/// Transport ACK is never an economic receipt.
pub fn transport_ack_is_economic_receipt() -> bool {
    false
}

/// Consumption is never durable application.
pub fn consumed_is_durable_applied() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::session::datagrams::Datagram;

    fn digest(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d
    }

    fn operation(tag: u8) -> OperationId {
        let mut id = OperationId::ZERO;
        id.0[0] = tag;
        id
    }

    fn bind_pair(iri: u8, service: u8) -> ChannelBind {
        ChannelBind {
            iri: digest(iri),
            service: digest(service),
            operation: operation(1),
        }
    }

    #[test]
    fn transport_ack_is_not_economic_and_consumed_is_not_durable() {
        assert!(!transport_ack_is_economic_receipt());
        assert!(!consumed_is_durable_applied());
    }

    #[test]
    fn stage_implies_durable_separates_ack_from_applied() {
        assert!(!stage_implies_durable(DeliveryStage::TransportAck));
        assert!(stage_implies_durable(DeliveryStage::DurableApplied));
        assert!(!stage_implies_durable(DeliveryStage::Consumed));
        assert!(stage_implies_durable(DeliveryStage::EconomicReceipt));
    }

    #[test]
    fn same_iri_different_service_is_conflict() {
        let mut table = ChannelTable::new();
        assert!(table.bind(bind_pair(1, 2)).is_ok());
        assert_eq!(table.bind(bind_pair(1, 3)), Err(QdnfError::Conflict));
    }

    #[test]
    fn identical_bind_is_idempotent() {
        let mut table = ChannelTable::new();
        let b = bind_pair(4, 5);
        assert!(table.bind(b).is_ok());
        assert!(table.bind(b).is_ok());
    }

    #[test]
    fn sixteen_distinct_ok_seventeenth_capacity() {
        let mut table = ChannelTable::new();
        let mut n = 0u8;
        while n < MAX_CHANNELS as u8 {
            let tag = n + 1;
            assert!(table.bind(bind_pair(tag, tag)).is_ok());
            n += 1;
        }
        assert_eq!(table.bind(bind_pair(0x20, 0x21)), Err(QdnfError::Capacity));
    }

    #[test]
    fn expired_datagram_is_independent_of_channel_table() {
        let mut table = ChannelTable::new();
        let b = bind_pair(7, 8);
        assert!(table.bind(b).is_ok());
        let dg = Datagram {
            operation: operation(1),
            expires_unix: 10,
            len: 4,
        };
        assert_eq!(dg.live_at(10), Err(QdnfError::Expired));
        assert!(dg.live_at(9).is_ok());
        assert!(table.bind(b).is_ok());
        assert!(table.bind(bind_pair(9, 10)).is_ok());
    }

    #[test]
    fn zero_iri_is_malformed() {
        let mut table = ChannelTable::new();
        assert_eq!(
            table.bind(ChannelBind {
                iri: StrongDigest::ZERO,
                service: digest(1),
                operation: operation(1),
            }),
            Err(QdnfError::Malformed)
        );
        assert_eq!(
            table.bind(ChannelBind {
                iri: digest(1),
                service: StrongDigest::ZERO,
                operation: operation(1),
            }),
            Err(QdnfError::Malformed)
        );
    }
}
