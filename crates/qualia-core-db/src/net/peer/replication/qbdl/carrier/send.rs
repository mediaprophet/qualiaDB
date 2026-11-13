use super::super::wire::{Message, ServiceRecordHeader};
use super::{transcript, CarrierError as E};
use crate::net::qdnf::types::StrongDigest;
/// Per-direction checked message IDs. Construct once from an established
/// session, never per transfer. PendingMessage retains the exact original bytes.
pub(crate) struct MessageSequence {
    transcript: StrongDigest,
    next: u128,
}
pub(crate) struct PendingMessage<'a> {
    transcript: StrongDigest,
    id: u128,
    bytes: &'a [u8],
    offset: usize,
}
impl MessageSequence {
    pub(crate) fn new(value: StrongDigest) -> Result<Self, E> {
        transcript(value)?;
        Ok(Self {
            transcript: value,
            next: 1,
        })
    }
    pub(crate) fn begin<'a>(&mut self, bytes: &'a [u8]) -> Result<PendingMessage<'a>, E> {
        Message::parse(bytes)?;
        let next = self.next.checked_add(1).ok_or(E::Order)?;
        let id = self.next;
        self.next = next;
        Ok(PendingMessage {
            transcript: self.transcript,
            id,
            bytes,
            offset: 0,
        })
    }
}
impl PendingMessage<'_> {
    pub(crate) fn current(&self) -> Option<(ServiceRecordHeader, &[u8])> {
        if self.offset == self.bytes.len() {
            return None;
        }
        let end = (self.offset + 1024).min(self.bytes.len());
        Some((
            ServiceRecordHeader {
                transcript: self.transcript,
                message_id: self.id.to_be_bytes(),
                total: self.bytes.len() as u32,
                offset: self.offset as u32,
                chunk: (end - self.offset) as u16,
                final_chunk: end == self.bytes.len(),
            },
            &self.bytes[self.offset..end],
        ))
    }
    /// Only after the common channel has drained the EXACT sealed frame.
    /// WouldBlock leaves this cursor untouched; do not reseal or advance it.
    pub(crate) fn drained(&mut self) -> Result<(), E> {
        if self.offset == self.bytes.len() {
            return Err(E::Closed);
        }
        self.offset = (self.offset + 1024).min(self.bytes.len());
        Ok(())
    }
}

#[cfg(test)]
mod exhaustion {
    use super::*;
    #[test]
    fn last_id_refuses_without_wrap_or_reuse() {
        let mut seq = MessageSequence {
            transcript: StrongDigest([9; 48]),
            next: u128::MAX,
        };
        let h = super::super::super::wire::Header {
            kind: super::super::super::wire::Kind::Close,
            descriptor: StrongDigest([1; 48]),
            context: StrongDigest([2; 48]),
            publication_generation: 0,
            policy_generation: 1,
            cancellation: 1,
            transfer_id: [3; 16],
            entry: 0,
            leaf: 0,
            count: 0,
            credit_serial: 0,
            credit_bytes: 0,
            deadline: 5,
        };
        let mut bytes = [0; 216];
        super::super::super::wire::encode(&h, &[0; 8], &mut bytes).unwrap();
        assert!(matches!(seq.begin(&bytes), Err(E::Order)));
        assert_eq!(seq.next, u128::MAX);
    }
}
