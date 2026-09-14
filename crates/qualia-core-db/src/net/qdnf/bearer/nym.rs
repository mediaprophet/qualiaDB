//! Nym mixnet transition bearer framing and deterministic loopback carrier.
//!
//! Encapsulates QFrames into Sphinx-compatible datagram envelopes.
//! Bounded, zero-heap in hot path. Labelled transition carrier (not Native Independent).
//! Authoritative QdnfError variants mapped (Closed, NoRoute, WouldBlock, Capacity).

use crate::net::qdnf::bearer::contract::{Bearer, RecvMeta, check_frame_mtu};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::types::{ObservedLocator, ScopeEpoch};

/// 4-byte magic for Nym mixnet envelopes.
pub const NYM_MAGIC: [u8; 4] = *b"QNYM";
/// Current envelope protocol version.
pub const NYM_ENVELOPE_VERSION: u8 = 1;
/// Header length: magic(4) + version(1) + payload_len(2) + sender_len(1) + sender(32) = 40 bytes.
pub const NYM_HEADER_LEN: usize = 40;
/// Sphinx maximum payload size for standard Nym packets (1,024 bytes).
pub const NYM_SPHINX_MTU: u16 = 1024;

/// Encapsulate a QFrame into a Nym Sphinx payload envelope.
///
/// Output layout:
/// `[0..4]`: `b"QNYM"`
/// `[4]`: version `1`
/// `[5..7]`: big-endian payload length
/// `[7]`: sender locator byte length (0..=32)
/// `[8..40]`: sender locator bytes
/// `[40..40+payload_len]`: frame bytes
pub fn encapsulate_nym(
    sender: &ObservedLocator,
    frame: &[u8],
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let total_len = NYM_HEADER_LEN + frame.len();
    if total_len > out.len() {
        return Err(QdnfError::Capacity);
    }
    if frame.len() > u16::MAX as usize {
        return Err(QdnfError::Range);
    }

    out[0..4].copy_from_slice(&NYM_MAGIC);
    out[4] = NYM_ENVELOPE_VERSION;
    let len_be = (frame.len() as u16).to_be_bytes();
    out[5..7].copy_from_slice(&len_be);
    out[7] = sender.len;
    out[8..40].copy_from_slice(&sender.bytes);
    out[40..total_len].copy_from_slice(frame);

    Ok(total_len)
}

/// Decapsulate a Nym Sphinx payload envelope into a QFrame and sender locator.
pub fn decapsulate_nym(
    src: &[u8],
    out_sender: &mut ObservedLocator,
    out_frame: &mut [u8],
) -> Result<usize, QdnfError> {
    if src.len() < NYM_HEADER_LEN {
        return Err(QdnfError::Truncated);
    }
    if src[0..4] != NYM_MAGIC {
        return Err(QdnfError::Malformed);
    }
    if src[4] != NYM_ENVELOPE_VERSION {
        return Err(QdnfError::Unsupported);
    }

    let frame_len = u16::from_be_bytes([src[5], src[6]]) as usize;
    if src.len() < NYM_HEADER_LEN + frame_len {
        return Err(QdnfError::Truncated);
    }
    if frame_len > out_frame.len() {
        return Err(QdnfError::Capacity);
    }

    let sender_len = src[7];
    if sender_len > 32 {
        return Err(QdnfError::Malformed);
    }
    let mut sender_bytes = [0u8; 32];
    sender_bytes.copy_from_slice(&src[8..40]);
    *out_sender = ObservedLocator {
        bytes: sender_bytes,
        len: sender_len,
    };

    out_frame[..frame_len].copy_from_slice(&src[40..40 + frame_len]);
    Ok(frame_len)
}

/// Deterministic, zero-heap in-memory simulated Nym mixnet bearer.
/// Used for pure integration testing of the QSession handshake over Nym.
pub struct NymSimulatedBearer {
    local_locator: ObservedLocator,
    peer_locator: ObservedLocator,
    scope: ScopeEpoch,
    closed: bool,
    inbound_buf: [u8; NYM_SPHINX_MTU as usize],
    inbound_len: usize,
    inbound_src: ObservedLocator,
}

impl NymSimulatedBearer {
    pub fn new(local_locator: ObservedLocator, peer_locator: ObservedLocator) -> Self {
        Self {
            local_locator,
            peer_locator,
            scope: ScopeEpoch { scope: 1, epoch: 1 },
            closed: false,
            inbound_buf: [0u8; NYM_SPHINX_MTU as usize],
            inbound_len: 0,
            inbound_src: ObservedLocator::EMPTY,
        }
    }

    #[inline]
    pub fn local_locator(&self) -> &ObservedLocator {
        &self.local_locator
    }

    /// Simulate delivery of a Sphinx envelope into this bearer's receive buffer.
    pub fn deliver_envelope(&mut self, envelope: &[u8]) -> Result<(), QdnfError> {
        if self.closed {
            return Err(QdnfError::Closed);
        }
        let mut sender = ObservedLocator::EMPTY;
        let mut frame_out = [0u8; NYM_SPHINX_MTU as usize];
        let n = decapsulate_nym(envelope, &mut sender, &mut frame_out)?;
        self.inbound_buf[..n].copy_from_slice(&frame_out[..n]);
        self.inbound_len = n;
        self.inbound_src = sender;
        Ok(())
    }
}

impl Bearer for NymSimulatedBearer {
    fn profile(&self) -> BearerProfile {
        BearerProfile::NymMixnetTransitionV1
    }

    fn mtu(&self) -> u16 {
        NYM_SPHINX_MTU
    }

    fn scope(&self) -> ScopeEpoch {
        self.scope
    }

    fn send(&mut self, dest: &ObservedLocator, frame: &[u8]) -> Result<usize, QdnfError> {
        if self.closed {
            return Err(QdnfError::Closed);
        }
        check_frame_mtu(frame.len(), self.mtu())?;
        if *dest != self.peer_locator && *dest != ObservedLocator::EMPTY {
            return Err(QdnfError::NoRoute);
        }
        Ok(frame.len())
    }

    fn recv(&mut self, out: &mut [u8]) -> Result<(usize, RecvMeta), QdnfError> {
        if self.closed {
            return Err(QdnfError::Closed);
        }
        if self.inbound_len == 0 {
            return Err(QdnfError::WouldBlock);
        }
        let len = self.inbound_len;
        if out.len() < len {
            return Err(QdnfError::Capacity);
        }
        out[..len].copy_from_slice(&self.inbound_buf[..len]);
        let src = self.inbound_src;
        self.inbound_len = 0;
        self.inbound_src = ObservedLocator::EMPTY;

        Ok((
            len,
            RecvMeta {
                observed_source: src,
                scope: self.scope,
                mtu: NYM_SPHINX_MTU,
            },
        ))
    }

    fn shutdown(&mut self) -> Result<(), QdnfError> {
        self.closed = true;
        self.inbound_len = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nym_envelope_roundtrips() {
        let sender = ObservedLocator::from_slice(b"alice-client.sphinx@gateway-1").unwrap();
        let payload = b"hello over nym mixnet";
        let mut envelope = [0u8; 256];
        let enc_len = encapsulate_nym(&sender, payload, &mut envelope).unwrap();

        assert_eq!(&envelope[0..4], &NYM_MAGIC);
        assert_eq!(envelope[4], NYM_ENVELOPE_VERSION);

        let mut out_sender = ObservedLocator::EMPTY;
        let mut out_frame = [0u8; 256];
        let dec_len = decapsulate_nym(&envelope[..enc_len], &mut out_sender, &mut out_frame).unwrap();

        assert_eq!(dec_len, payload.len());
        assert_eq!(&out_frame[..dec_len], payload);
        assert_eq!(out_sender, sender);
    }

    #[test]
    fn simulated_bearer_sends_and_receives() {
        let alice_loc = ObservedLocator::from_slice(b"alice").unwrap();
        let bob_loc = ObservedLocator::from_slice(b"bob").unwrap();
        let mut bob_bearer = NymSimulatedBearer::new(bob_loc, alice_loc);

        assert_eq!(bob_bearer.profile(), BearerProfile::NymMixnetTransitionV1);
        assert!(!bob_bearer.profile().native_independent());
        assert_eq!(bob_bearer.mtu(), NYM_SPHINX_MTU);

        // Prepare an envelope from Alice to Bob
        let payload = b"ConnectRequest(did:qi:alice)";
        let mut envelope = [0u8; 512];
        let enc_len = encapsulate_nym(&alice_loc, payload, &mut envelope).unwrap();

        // Deliver envelope to Bob
        bob_bearer.deliver_envelope(&envelope[..enc_len]).unwrap();

        // Bob reads from bearer
        let mut rx_buf = [0u8; 512];
        let (n, meta) = bob_bearer.recv(&mut rx_buf).unwrap();
        assert_eq!(n, payload.len());
        assert_eq!(&rx_buf[..n], payload);
        assert_eq!(meta.observed_source, alice_loc);
    }
}
