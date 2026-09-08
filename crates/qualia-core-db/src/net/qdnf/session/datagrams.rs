//! Expiring datagrams. Transport ACK is not durable application receipt.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::OperationId;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_ack_is_not_durable() {
        assert_ne!(
            DeliveryStage::TransportAck,
            DeliveryStage::DurableApplied
        );
    }
}
