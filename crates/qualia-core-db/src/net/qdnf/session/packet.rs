//! Caller-buffered packet numbers, 64-bit replay window, nonce construction,
//! and QUIC-style bounded varints (NET-05.04 partial). Packages remain open.
//!
//! Packet numbers occupy `[0, MAX_PACKET_NUMBER]`. Retransmits use a fresh
//! packet number; ciphertext/nonce reuse is forbidden.

use crate::net::qdnf::errors::QdnfError;

/// Sliding receive window width (one bit per packet number).
pub const REPLAY_WINDOW: u64 = 64;

/// Inclusive send/receive ceiling. The next allocate after this is exhaustion.
pub const MAX_PACKET_NUMBER: u64 = (1u64 << 62) - 1;

/// 12-byte AEAD nonce (4 zero bytes ‖ u64be packet number).
pub const AEAD_NONCE_LEN: usize = 12;

const VARINT_1: u64 = 1 << 6;
const VARINT_2: u64 = 1 << 14;
const VARINT_4: u64 = 1 << 30;

#[repr(transparent)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PacketNumber(pub u64);

/// Per encryption-level packet-number space: next send, largest received,
/// and a 64-bit sliding replay bitmap (bit 0 = largest received).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PacketSpace {
    next_send: u64,
    largest_recv: u64,
    recv_bits: u64,
}

impl PacketSpace {
    pub const fn new() -> Self {
        Self {
            next_send: 0,
            largest_recv: 0,
            recv_bits: 0,
        }
    }

    /// Allocate next send pn. After [`MAX_PACKET_NUMBER`] → [`QdnfError::Expired`].
    pub fn allocate_send(&mut self) -> Result<PacketNumber, QdnfError> {
        if self.next_send > MAX_PACKET_NUMBER {
            return Err(QdnfError::Expired);
        }
        let pn = PacketNumber(self.next_send);
        self.next_send += 1;
        Ok(pn)
    }

    /// Accept a received pn. Duplicate or too old → [`QdnfError::Replay`].
    /// `pn > MAX_PACKET_NUMBER` → [`QdnfError::Range`].
    pub fn accept_receive(&mut self, pn: PacketNumber) -> Result<(), QdnfError> {
        if pn.0 > MAX_PACKET_NUMBER {
            return Err(QdnfError::Range);
        }
        if self.recv_bits == 0 {
            self.largest_recv = pn.0;
            self.recv_bits = 1;
            return Ok(());
        }
        if pn.0 > self.largest_recv {
            let shift = pn.0 - self.largest_recv;
            self.recv_bits = if shift >= REPLAY_WINDOW {
                1
            } else {
                (self.recv_bits << shift) | 1
            };
            self.largest_recv = pn.0;
            return Ok(());
        }
        let delta = self.largest_recv - pn.0;
        if delta >= REPLAY_WINDOW {
            return Err(QdnfError::Replay);
        }
        let bit = 1u64 << delta;
        if self.recv_bits & bit != 0 {
            return Err(QdnfError::Replay);
        }
        self.recv_bits |= bit;
        Ok(())
    }
}

/// 12-byte nonce: 4 zero bytes ‖ u64be(packet number). Never reuse a pn.
pub fn nonce_from_packet_number(pn: PacketNumber, out: &mut [u8; AEAD_NONCE_LEN]) {
    out[..4].fill(0);
    out[4..].copy_from_slice(&pn.0.to_be_bytes());
}

/// Retransmit semantic frames under a fresh packet number, never the same nonce.
pub fn reuse_packet_number_on_retransmit() -> bool {
    false
}

#[inline]
const fn varint_len(value: u64) -> usize {
    if value < VARINT_1 {
        1
    } else if value < VARINT_2 {
        2
    } else if value < VARINT_4 {
        4
    } else {
        8
    }
}

/// QUIC-style bounded varint (1/2/4/8). Short `out` → [`QdnfError::Capacity`].
/// `value > MAX_PACKET_NUMBER` → [`QdnfError::Range`].
pub fn encode_varint(value: u64, out: &mut [u8]) -> Result<usize, QdnfError> {
    if value > MAX_PACKET_NUMBER {
        return Err(QdnfError::Range);
    }
    let len = varint_len(value);
    if out.len() < len {
        return Err(QdnfError::Capacity);
    }
    match len {
        1 => out[0] = value as u8,
        2 => {
            let n = (value as u16) | 0x4000;
            out[..2].copy_from_slice(&n.to_be_bytes());
        }
        4 => {
            let n = (value as u32) | 0x8000_0000;
            out[..4].copy_from_slice(&n.to_be_bytes());
        }
        _ => {
            let n = value | 0xC000_0000_0000_0000;
            out[..8].copy_from_slice(&n.to_be_bytes());
        }
    }
    Ok(len)
}

/// Decode one bounded varint. Truncated input → [`QdnfError::Truncated`].
/// Overlong / non-canonical encodings → [`QdnfError::Malformed`].
pub fn decode_varint(src: &[u8]) -> Result<(u64, usize), QdnfError> {
    let first = *src.first().ok_or(QdnfError::Truncated)?;
    let len = 1usize << (first >> 6);
    if src.len() < len {
        return Err(QdnfError::Truncated);
    }
    let value = match len {
        1 => first as u64,
        2 => u16::from_be_bytes([src[0], src[1]]) as u64 & 0x3fff,
        4 => u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as u64 & 0x3fff_ffff,
        8 => {
            u64::from_be_bytes([
                src[0], src[1], src[2], src[3], src[4], src[5], src[6], src[7],
            ]) & MAX_PACKET_NUMBER
        }
        _ => return Err(QdnfError::Malformed),
    };
    if varint_len(value) != len {
        return Err(QdnfError::Malformed);
    }
    Ok((value, len))
}

#[cfg(test)]
impl PacketSpace {
    fn set_next_send_for_test(&mut self, next: u64) {
        self.next_send = next;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allocate_zero_then_one_duplicate_receive_is_replay() {
        let mut space = PacketSpace::new();
        assert_eq!(space.allocate_send(), Ok(PacketNumber(0)));
        assert_eq!(space.allocate_send(), Ok(PacketNumber(1)));
        assert_eq!(space.accept_receive(PacketNumber(0)), Ok(()));
        assert_eq!(
            space.accept_receive(PacketNumber(0)),
            Err(QdnfError::Replay)
        );
    }

    #[test]
    fn accept_below_window_is_replay() {
        let mut space = PacketSpace::new();
        assert_eq!(space.accept_receive(PacketNumber(0)), Ok(()));
        assert_eq!(space.accept_receive(PacketNumber(REPLAY_WINDOW)), Ok(()));
        assert_eq!(
            space.accept_receive(PacketNumber(0)),
            Err(QdnfError::Replay)
        );
    }

    #[test]
    fn allocate_after_max_is_expired() {
        let mut space = PacketSpace::new();
        space.set_next_send_for_test(MAX_PACKET_NUMBER);
        assert_eq!(space.allocate_send(), Ok(PacketNumber(MAX_PACKET_NUMBER)));
        assert_eq!(space.allocate_send(), Err(QdnfError::Expired));
        space.set_next_send_for_test(MAX_PACKET_NUMBER + 1);
        assert_eq!(space.allocate_send(), Err(QdnfError::Expired));
    }

    #[test]
    fn nonce_from_packet_number_one_is_be_u64() {
        let mut out = [0xffu8; AEAD_NONCE_LEN];
        nonce_from_packet_number(PacketNumber(1), &mut out);
        assert_eq!(&out[..4], &[0, 0, 0, 0]);
        assert_eq!(&out[4..], &[0, 0, 0, 0, 0, 0, 0, 1]);
    }

    #[test]
    fn reuse_packet_number_on_retransmit_is_false() {
        assert!(!reuse_packet_number_on_retransmit());
    }

    #[test]
    fn decode_varint_overlong_is_malformed() {
        // 0 encoded as 2-byte (0x40 0x00) is non-canonical.
        assert_eq!(decode_varint(&[0x40, 0x00]), Err(QdnfError::Malformed));
        assert_eq!(decode_varint(&[0x00]), Ok((0, 1)));
    }

    #[test]
    fn encode_varint_length_and_short_output() {
        let mut one = [0u8; 1];
        assert_eq!(encode_varint(63, &mut one), Ok(1));
        assert_eq!(one[0], 63);
        let mut two = [0u8; 2];
        assert_eq!(encode_varint(64, &mut two), Ok(2));
        assert_eq!(&two, &[0x40, 0x40]);
        let mut short = [0u8; 1];
        assert_eq!(encode_varint(64, &mut short), Err(QdnfError::Capacity));
        assert_eq!(encode_varint(63, &mut []), Err(QdnfError::Capacity));
    }

    #[test]
    fn accept_above_max_is_range() {
        let mut space = PacketSpace::new();
        assert_eq!(
            space.accept_receive(PacketNumber(MAX_PACKET_NUMBER + 1)),
            Err(QdnfError::Range)
        );
    }

    #[test]
    fn decode_varint_truncated() {
        assert_eq!(decode_varint(&[]), Err(QdnfError::Truncated));
        assert_eq!(decode_varint(&[0x40]), Err(QdnfError::Truncated));
    }
}
