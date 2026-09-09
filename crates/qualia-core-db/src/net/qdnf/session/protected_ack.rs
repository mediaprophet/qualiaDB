//! ACK/loss/credit integration with protected packet numbers (E09.1).
//!
//! Duplicate and reordered authenticated packets must not close the session.
//! Retransmits allocate a fresh packet number. Replay is dropped, not fatal.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::loss::{AckFrame, SentTable, SENT_HISTORY};
use crate::net::qdnf::session::packet_protection::{PacketProtection, OVERHEAD, PN_LEN};
use crate::net::qdnf::types::Generation;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProtectedRecv {
    Delivered = 0,
    Duplicate = 1,
}

pub struct ProtectedAckSession {
    protection: PacketProtection,
    sent: SentTable,
    closed: bool,
}

impl ProtectedAckSession {
    pub fn from_keys(
        send_key: [u8; 32],
        recv_key: [u8; 32],
        generation: Generation,
    ) -> Result<Self, QdnfError> {
        Ok(Self {
            protection: PacketProtection::from_keys(send_key, recv_key, generation)?,
            sent: SentTable::new(),
            closed: false,
        })
    }

    #[inline]
    pub const fn is_closed(&self) -> bool {
        self.closed
    }

    #[inline]
    pub fn in_flight(&self) -> usize {
        self.sent.in_flight_count()
    }

    /// Seal application bytes and record the allocated packet number.
    pub fn seal_tracked(
        &mut self,
        aad: &[u8],
        plaintext: &mut [u8],
        out: &mut [u8],
    ) -> Result<usize, QdnfError> {
        if self.closed {
            return Err(QdnfError::Closed);
        }
        let n = self.protection.seal(aad, plaintext, out)?;
        let pn = packet_number_of(out, n)?;
        self.sent.record_send(pn)?;
        Ok(n)
    }

    /// Open a sealed packet. Replay/duplicates are dropped without closing.
    pub fn open_tracked(
        &mut self,
        aad: &[u8],
        sealed: &[u8],
        out: &mut [u8],
    ) -> Result<(ProtectedRecv, usize), QdnfError> {
        if self.closed {
            return Err(QdnfError::Closed);
        }
        match self.protection.open(aad, sealed, out) {
            Ok(n) => Ok((ProtectedRecv::Delivered, n)),
            Err(QdnfError::Replay) => Ok((ProtectedRecv::Duplicate, 0)),
            Err(e) => Err(e),
        }
    }

    /// Apply authenticated ACK ranges to in-flight ownership. Unknown PNs
    /// that are not in the sent table are ignored per range member that was
    /// never sent; a range that matches nothing is Malformed.
    pub fn apply_ack(&mut self, ack: &AckFrame) -> Result<usize, QdnfError> {
        if self.closed {
            return Err(QdnfError::Closed);
        }
        if ack.range_count() == 0 {
            return Err(QdnfError::Malformed);
        }
        let mut retired = 0usize;
        let mut i = 0usize;
        while i < SENT_HISTORY {
            if let Some(pn) = self.sent.packet_at(i) {
                if ack.contains(pn) {
                    self.sent.on_ack(pn)?;
                    retired = retired.saturating_add(1);
                }
            }
            i += 1;
        }
        if retired == 0 {
            return Err(QdnfError::Malformed);
        }
        Ok(retired)
    }

    /// Retransmit the same application bytes under a fresh packet number.
    pub fn retransmit(
        &mut self,
        aad: &[u8],
        plaintext: &mut [u8],
        out: &mut [u8],
    ) -> Result<usize, QdnfError> {
        self.seal_tracked(aad, plaintext, out)
    }

    pub fn install_update(
        &mut self,
        send_key: [u8; 32],
        recv_key: [u8; 32],
        generation: Generation,
    ) -> Result<(), QdnfError> {
        self.protection.install_update(send_key, recv_key, generation)?;
        self.sent = SentTable::new();
        Ok(())
    }
}

pub fn packet_number_of(sealed: &[u8], len: usize) -> Result<u64, QdnfError> {
    if len < OVERHEAD || sealed.len() < PN_LEN {
        return Err(QdnfError::Truncated);
    }
    let bytes: [u8; 8] = sealed[..PN_LEN]
        .try_into()
        .map_err(|_| QdnfError::Range)?;
    Ok(u64::from_be_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> (ProtectedAckSession, ProtectedAckSession) {
        let a = ProtectedAckSession::from_keys([1u8; 32], [2u8; 32], Generation(1)).unwrap();
        let b = ProtectedAckSession::from_keys([2u8; 32], [1u8; 32], Generation(1)).unwrap();
        (a, b)
    }

    #[test]
    fn reorder_and_duplicate_do_not_close() {
        let (mut a, mut b) = pair();
        let mut sealed = [[0u8; 64]; 3];
        let mut lens = [0usize; 3];
        let mut i = 0usize;
        while i < 3 {
            let mut pt = *b"pkt-body";
            lens[i] = a.seal_tracked(b"aad", &mut pt, &mut sealed[i]).unwrap();
            i += 1;
        }
        assert_eq!(a.in_flight(), 3);
        let order = [2usize, 0, 1];
        let mut delivered = 0u8;
        let mut j = 0usize;
        while j < 3 {
            let idx = order[j];
            let mut out = [0u8; 16];
            let (kind, n) = b
                .open_tracked(b"aad", &sealed[idx][..lens[idx]], &mut out)
                .unwrap();
            assert_eq!(kind, ProtectedRecv::Delivered);
            assert_eq!(n, 8);
            delivered = delivered.saturating_add(1);
            j += 1;
        }
        let mut out = [0u8; 16];
        let (dup, n) = b
            .open_tracked(b"aad", &sealed[0][..lens[0]], &mut out)
            .unwrap();
        assert_eq!(dup, ProtectedRecv::Duplicate);
        assert_eq!(n, 0);
        assert!(!a.is_closed());
        assert!(!b.is_closed());
        assert_eq!(delivered, 3);
    }

    #[test]
    fn ack_retires_in_flight_and_unknown_ack_is_malformed() {
        let (mut a, mut b) = pair();
        let mut pt = *b"hello-qpr";
        let mut sealed = [0u8; 64];
        let n = a.seal_tracked(b"aad", &mut pt, &mut sealed).unwrap();
        let pn = packet_number_of(&sealed, n).unwrap();
        let mut out = [0u8; 16];
        b.open_tracked(b"aad", &sealed[..n], &mut out).unwrap();
        let mut ack = AckFrame::new();
        ack.insert_range(pn, pn).unwrap();
        assert_eq!(a.apply_ack(&ack).unwrap(), 1);
        assert_eq!(a.in_flight(), 0);
        let mut bogus = AckFrame::new();
        bogus.insert_range(99, 99).unwrap();
        assert_eq!(a.apply_ack(&bogus), Err(QdnfError::Malformed));
        assert!(!a.is_closed());
    }

    #[test]
    fn retransmit_uses_fresh_packet_number() {
        let (mut a, _b) = pair();
        let mut pt = *b"hello-qpr";
        let mut first = [0u8; 64];
        let n1 = a.seal_tracked(b"aad", &mut pt, &mut first).unwrap();
        let pn1 = packet_number_of(&first, n1).unwrap();
        let mut pt2 = *b"hello-qpr";
        let mut second = [0u8; 64];
        let n2 = a.retransmit(b"aad", &mut pt2, &mut second).unwrap();
        let pn2 = packet_number_of(&second, n2).unwrap();
        assert_ne!(pn1, pn2);
        assert_eq!(a.in_flight(), 2);
        assert!(!crate::net::qdnf::session::loss::reuse_packet_number_on_retransmit());
    }
}
