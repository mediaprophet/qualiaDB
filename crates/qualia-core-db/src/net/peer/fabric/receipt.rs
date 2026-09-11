//! Durable operation receipts. A relay write is not a commit.

use super::intent::PeerId;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiptStatus {
    Queued = 1,
    Received = 2,
    Validated = 3,
    Committed = 4,
    Denied = 5,
    Expired = 6,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OpReceipt {
    pub op_id: u64,
    pub content_digest: [u8; 32],
    pub peer: PeerId,
    pub grant_generation: u32,
    pub status: ReceiptStatus,
    pub expiry_unix: u32,
}

impl OpReceipt {
    pub const fn queued(
        op_id: u64,
        content_digest: [u8; 32],
        peer: PeerId,
        grant_generation: u32,
        expiry_unix: u32,
    ) -> Self {
        Self {
            op_id,
            content_digest,
            peer,
            grant_generation,
            status: ReceiptStatus::Queued,
            expiry_unix,
        }
    }

    pub fn replay(
        &self,
        op_id: u64,
        content_digest: &[u8; 32],
        grant_live: bool,
        grant_generation: u32,
        now_unix: u32,
    ) -> ReceiptStatus {
        if now_unix >= self.expiry_unix {
            return ReceiptStatus::Expired;
        }
        if !grant_live || grant_generation != self.grant_generation {
            return ReceiptStatus::Denied;
        }
        if op_id != self.op_id || content_digest != &self.content_digest {
            return ReceiptStatus::Denied;
        }
        ReceiptStatus::Committed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn revoked_grant_cannot_commit() {
        let r = OpReceipt::queued(9, [3u8; 32], [1u8; 32], 4, 100);
        assert_eq!(
            r.replay(9, &[3u8; 32], false, 4, 10),
            ReceiptStatus::Denied
        );
        assert_eq!(
            r.replay(9, &[3u8; 32], true, 5, 10),
            ReceiptStatus::Denied
        );
        assert_eq!(
            r.replay(9, &[4u8; 32], true, 4, 10),
            ReceiptStatus::Denied
        );
        assert_eq!(
            r.replay(9, &[3u8; 32], true, 4, 10),
            ReceiptStatus::Committed
        );
    }
}
