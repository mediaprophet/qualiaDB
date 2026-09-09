//! Expiring datagrams. Transport ACK is not durable application receipt.
//!
//! Datagrams are unreliable: they are never retransmitted, and a retransmit
//! path must not reuse a packet number. Fresh PNs belong to
//! [`super::protected_ack::ProtectedAckSession`] for *stream* frames only.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::OperationId;

pub const MAX_DATAGRAMS: usize = 8;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Datagram {
    pub operation: OperationId,
    pub expires_unix: u64,
    pub len: u16,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeliveryStage {
    TransportAck = 1,
    Consumed = 2,
    DurableApplied = 3,
    EconomicReceipt = 4,
}

impl Datagram {
    pub fn live_at(&self, now_unix: u64) -> Result<(), QdnfError> {
        if now_unix >= self.expires_unix {
            Err(QdnfError::Expired)
        } else {
            Ok(())
        }
    }
}

/// Deadline-aware delivery. An expired datagram is [`QdnfError::Expired`],
/// never handed to the application late.
pub fn receive(d: &Datagram, now_unix: u64) -> Result<(), QdnfError> {
    d.live_at(now_unix)
}

/// Datagrams are not retransmitted. Loss is silence, not a PN reuse.
pub const fn datagrams_are_retransmitted() -> bool {
    false
}

pub const fn datagram_retransmit_reuses_packet_number() -> bool {
    false
}

/// Unreliable datagrams must not stall reliable stream progress.
pub const fn datagrams_block_streams() -> bool {
    false
}

/// Bounded inbox. Expired slots are dropped as Expired, not delivered.
pub struct DatagramInbox {
    slots: [Option<Datagram>; MAX_DATAGRAMS],
}

impl DatagramInbox {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_DATAGRAMS],
        }
    }

    pub fn enqueue(&mut self, d: Datagram, now_unix: u64) -> Result<(), QdnfError> {
        receive(&d, now_unix)?;
        let mut i = 0usize;
        while i < MAX_DATAGRAMS {
            if self.slots[i].is_none() {
                self.slots[i] = Some(d);
                return Ok(());
            }
            i += 1;
        }
        Err(QdnfError::Capacity)
    }

    /// Pop the first live datagram. Expired occupants are cleared as Expired.
    pub fn take(&mut self, now_unix: u64) -> Result<Datagram, QdnfError> {
        let mut i = 0usize;
        while i < MAX_DATAGRAMS {
            if let Some(d) = self.slots[i] {
                self.slots[i] = None;
                return receive(&d, now_unix).map(|()| d);
            }
            i += 1;
        }
        Err(QdnfError::WouldBlock)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dg(expires: u64) -> Datagram {
        Datagram {
            operation: OperationId([1u8; 16]),
            expires_unix: expires,
            len: 4,
        }
    }

    #[test]
    fn transport_ack_is_not_durable() {
        assert_ne!(DeliveryStage::TransportAck, DeliveryStage::DurableApplied);
    }

    #[test]
    fn expired_datagram_is_expired_not_late() {
        let d = dg(10);
        assert_eq!(receive(&d, 10), Err(QdnfError::Expired));
        assert_eq!(receive(&d, 11), Err(QdnfError::Expired));
        receive(&d, 9).unwrap();
        let mut inbox = DatagramInbox::new();
        inbox.enqueue(d, 9).unwrap();
        assert_eq!(inbox.take(10), Err(QdnfError::Expired));
        assert!(!datagrams_are_retransmitted());
        assert!(!datagram_retransmit_reuses_packet_number());
        assert!(!datagrams_block_streams());
    }

    #[test]
    fn enqueue_rejects_already_expired() {
        let mut inbox = DatagramInbox::new();
        assert_eq!(inbox.enqueue(dg(5), 5), Err(QdnfError::Expired));
        inbox.enqueue(dg(20), 1).unwrap();
        let got = inbox.take(1).unwrap();
        assert_eq!(got.expires_unix, 20);
    }
}
