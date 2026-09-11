//! PQ handshake fragmentation with admission **before** buffering.
//!
//! Bytes are charged against a per-message cap first. Only then may a fragment
//! be copied into the bounded reassembly slot. Truncated wire is
//! [`QdnfError::Truncated`] and does not copy. Interface loss is
//! [`QdnfError::Closed`]; reconnect bumps generation so old tickets are useless.

use crate::net::qdnf::errors::QdnfError;

use super::mtu::{MAX_QDNF_MTU, MIN_QDNF_MTU, unfragmented_fit};

/// `msg_id(4) || offset(2) || total(2) || frag_len(2)`.
pub const FRAG_HDR_LEN: usize = 10;
/// Hybrid handshake (ML-KEM-768 + X25519 + signatures) fits under this cap.
pub const MAX_HANDSHAKE_BYTES: usize = 8192;
pub const MAX_SESSIONS: usize = 2;
pub const MAX_FRAGMENTS: usize = 8;
pub const MAX_FRAME: usize = MAX_QDNF_MTU as usize;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FragmentHeader {
    pub msg_id: u32,
    pub offset: u16,
    pub total: u16,
    pub frag_len: u16,
}

/// Proof that `charged` bytes were admitted for `msg_id` at `generation`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AdmitTicket {
    generation: u64,
    slot: u8,
    msg_id: u32,
    charged: u16,
}

struct Slot {
    occupied: bool,
    msg_id: u32,
    total: u16,
    admitted: u16,
    received: u16,
    buf: [u8; MAX_HANDSHAKE_BYTES],
    filled: [bool; MAX_HANDSHAKE_BYTES],
}

impl Slot {
    fn empty() -> Self {
        Self {
            occupied: false,
            msg_id: 0,
            total: 0,
            admitted: 0,
            received: 0,
            buf: [0u8; MAX_HANDSHAKE_BYTES],
            filled: [false; MAX_HANDSHAKE_BYTES],
        }
    }

    fn clear(&mut self) {
        *self = Self::empty();
    }
}

pub struct HandshakeReassembler {
    slots: [Slot; MAX_SESSIONS],
    alive: bool,
    generation: u64,
}

impl HandshakeReassembler {
    pub fn new() -> Self {
        Self {
            slots: [Slot::empty(), Slot::empty()],
            alive: true,
            generation: 1,
        }
    }

    #[inline]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    pub fn occupied(&self) -> usize {
        self.slots.iter().filter(|s| s.occupied).count()
    }

    pub fn buffered_len(&self, msg_id: u32) -> usize {
        self.slots
            .iter()
            .find(|s| s.occupied && s.msg_id == msg_id)
            .map(|s| s.received as usize)
            .unwrap_or(0)
    }

    /// Charge `frag_len` against the message cap **before** any copy.
    pub fn admit(
        &mut self,
        msg_id: u32,
        total: u16,
        frag_len: u16,
    ) -> Result<AdmitTicket, QdnfError> {
        if !self.alive {
            return Err(QdnfError::Closed);
        }
        if frag_len == 0 || total == 0 {
            return Err(QdnfError::Malformed);
        }
        if total as usize > MAX_HANDSHAKE_BYTES {
            return Err(QdnfError::Capacity);
        }
        let slot_idx = self.find_or_alloc(msg_id, total)?;
        let slot = &mut self.slots[slot_idx];
        if slot.total != total {
            return Err(QdnfError::Conflict);
        }
        let new_admitted = slot
            .admitted
            .checked_add(frag_len)
            .ok_or(QdnfError::Range)?;
        if new_admitted as usize > slot.total as usize {
            return Err(QdnfError::Capacity);
        }
        slot.admitted = new_admitted;
        Ok(AdmitTicket {
            generation: self.generation,
            slot: slot_idx as u8,
            msg_id,
            charged: frag_len,
        })
    }

    /// Copy `src` only after a matching ticket. Short `src` is Truncated.
    pub fn buffer(
        &mut self,
        ticket: AdmitTicket,
        offset: u16,
        src: &[u8],
        complete_out: &mut [u8],
    ) -> Result<Option<usize>, QdnfError> {
        if !self.alive {
            return Err(QdnfError::Closed);
        }
        if ticket.generation != self.generation {
            return Err(QdnfError::StaleGeneration);
        }
        let slot_idx = ticket.slot as usize;
        if slot_idx >= MAX_SESSIONS {
            return Err(QdnfError::Range);
        }
        let slot = &mut self.slots[slot_idx];
        if !slot.occupied || slot.msg_id != ticket.msg_id {
            return Err(QdnfError::Unauthorized);
        }
        if src.len() < ticket.charged as usize {
            return Err(QdnfError::Truncated);
        }
        if src.len() > ticket.charged as usize {
            return Err(QdnfError::Capacity);
        }
        let start = offset as usize;
        let end = start
            .checked_add(ticket.charged as usize)
            .ok_or(QdnfError::Range)?;
        if end > slot.total as usize {
            return Err(QdnfError::Capacity);
        }
        let dest = &mut slot.buf[start..end];
        dest.copy_from_slice(&src[..ticket.charged as usize]);
        for i in start..end {
            if !slot.filled[i] {
                slot.filled[i] = true;
                slot.received = slot.received.saturating_add(1);
            }
        }
        if slot.received == slot.total {
            let n = slot.total as usize;
            if complete_out.len() < n {
                return Err(QdnfError::Capacity);
            }
            complete_out[..n].copy_from_slice(&slot.buf[..n]);
            slot.clear();
            return Ok(Some(n));
        }
        Ok(None)
    }

    pub fn lose_interface(&mut self) {
        self.alive = false;
        let mut i = 0usize;
        while i < MAX_SESSIONS {
            self.slots[i].clear();
            i += 1;
        }
    }

    /// New generation; prior tickets cannot buffer. Requires a prior loss.
    pub fn reconnect(&mut self) -> Result<(), QdnfError> {
        if self.alive {
            return Err(QdnfError::Conflict);
        }
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or(QdnfError::StaleGeneration)?;
        self.alive = true;
        let mut i = 0usize;
        while i < MAX_SESSIONS {
            self.slots[i].clear();
            i += 1;
        }
        Ok(())
    }

    fn find_or_alloc(&mut self, msg_id: u32, total: u16) -> Result<usize, QdnfError> {
        let mut i = 0usize;
        while i < MAX_SESSIONS {
            if self.slots[i].occupied && self.slots[i].msg_id == msg_id {
                return Ok(i);
            }
            i += 1;
        }
        let mut i = 0usize;
        while i < MAX_SESSIONS {
            if !self.slots[i].occupied {
                self.slots[i].occupied = true;
                self.slots[i].msg_id = msg_id;
                self.slots[i].total = total;
                self.slots[i].admitted = 0;
                self.slots[i].received = 0;
                return Ok(i);
            }
            i += 1;
        }
        Err(QdnfError::Capacity)
    }
}

impl Default for HandshakeReassembler {
    fn default() -> Self {
        Self::new()
    }
}

pub fn encode_fragment(
    header: FragmentHeader,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    if payload.len() != header.frag_len as usize {
        return Err(QdnfError::Malformed);
    }
    let total = FRAG_HDR_LEN
        .checked_add(payload.len())
        .ok_or(QdnfError::Range)?;
    if out.len() < total {
        return Err(QdnfError::Capacity);
    }
    out[0..4].copy_from_slice(&header.msg_id.to_be_bytes());
    out[4..6].copy_from_slice(&header.offset.to_be_bytes());
    out[6..8].copy_from_slice(&header.total.to_be_bytes());
    out[8..10].copy_from_slice(&header.frag_len.to_be_bytes());
    out[10..total].copy_from_slice(payload);
    Ok(total)
}

pub fn decode_fragment(src: &[u8]) -> Result<(FragmentHeader, &[u8]), QdnfError> {
    if src.len() < FRAG_HDR_LEN {
        return Err(QdnfError::Truncated);
    }
    let header = FragmentHeader {
        msg_id: u32::from_be_bytes(src[0..4].try_into().unwrap()),
        offset: u16::from_be_bytes([src[4], src[5]]),
        total: u16::from_be_bytes([src[6], src[7]]),
        frag_len: u16::from_be_bytes([src[8], src[9]]),
    };
    let need = FRAG_HDR_LEN
        .checked_add(header.frag_len as usize)
        .ok_or(QdnfError::Range)?;
    if src.len() < need {
        return Err(QdnfError::Truncated);
    }
    Ok((header, &src[FRAG_HDR_LEN..need]))
}

/// Decode, admit, then copy. Truncated or oversize wire never copies.
///
/// Admission is charged before [`HandshakeReassembler::buffer`]. A missing
/// ticket cannot be skipped: there is no whole-datagram fallback here.
pub fn admit_and_buffer(
    reassembler: &mut HandshakeReassembler,
    src: &[u8],
    complete_out: &mut [u8],
) -> Result<Option<usize>, QdnfError> {
    let (header, body) = decode_fragment(src)?;
    let ticket = reassembler.admit(header.msg_id, header.total, header.frag_len)?;
    reassembler.buffer(ticket, header.offset, body, complete_out)
}

pub fn fragment_payload_mtu(mtu: u16) -> Result<u16, QdnfError> {
    if mtu < MIN_QDNF_MTU {
        return Err(QdnfError::Range);
    }
    let payload = (mtu as usize).saturating_sub(FRAG_HDR_LEN);
    if payload == 0 {
        return Err(QdnfError::Range);
    }
    Ok(payload as u16)
}

/// Split a PQ handshake into bounded frames. `payload_mtu` is bytes after the
/// fragment header (use [`fragment_payload_mtu`]).
pub fn split_handshake(
    msg_id: u32,
    payload: &[u8],
    payload_mtu: u16,
    out_frames: &mut [[u8; MAX_FRAME]; MAX_FRAGMENTS],
    out_lens: &mut [usize; MAX_FRAGMENTS],
) -> Result<usize, QdnfError> {
    if payload.is_empty() {
        return Err(QdnfError::Malformed);
    }
    if payload.len() > MAX_HANDSHAKE_BYTES {
        return Err(QdnfError::Capacity);
    }
    if payload_mtu == 0 {
        return Err(QdnfError::Range);
    }
    if payload.len() <= payload_mtu as usize {
        unfragmented_fit(payload.len() + FRAG_HDR_LEN, MAX_QDNF_MTU)?;
    }
    let total = u16::try_from(payload.len()).map_err(|_| QdnfError::Capacity)?;
    let mut offset = 0usize;
    let mut n = 0usize;
    while offset < payload.len() {
        if n >= MAX_FRAGMENTS {
            return Err(QdnfError::Capacity);
        }
        let remaining = payload.len() - offset;
        let take = remaining.min(payload_mtu as usize);
        let header = FragmentHeader {
            msg_id,
            offset: offset as u16,
            total,
            frag_len: take as u16,
        };
        let written = encode_fragment(header, &payload[offset..offset + take], &mut out_frames[n])?;
        out_lens[n] = written;
        offset += take;
        n += 1;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admit_before_buffer_rejects_without_copy() {
        let mut r = HandshakeReassembler::new();
        assert_eq!(r.admit(1, 9000, 100), Err(QdnfError::Capacity));
        assert_eq!(r.occupied(), 0);
        assert_eq!(r.buffered_len(1), 0);

        let ticket = r.admit(2, 8, 4).unwrap();
        assert_eq!(r.occupied(), 1);
        assert_eq!(r.buffered_len(2), 0);

        let mut out = [0u8; 16];
        assert_eq!(
            r.buffer(ticket, 0, &[1, 2], &mut out),
            Err(QdnfError::Truncated)
        );
        assert_eq!(r.buffered_len(2), 0);

        r.buffer(ticket, 0, &[9, 9, 9, 9], &mut out).unwrap();
        assert_eq!(r.buffered_len(2), 4);
    }

    #[test]
    fn truncated_header_is_truncated() {
        assert_eq!(decode_fragment(&[0u8; 9]), Err(QdnfError::Truncated));
        let header = FragmentHeader {
            msg_id: 1,
            offset: 0,
            total: 4,
            frag_len: 4,
        };
        let mut wire = [0u8; 14];
        let n = encode_fragment(header, &[1, 2, 3, 4], &mut wire).unwrap();
        assert_eq!(decode_fragment(&wire[..n - 1]), Err(QdnfError::Truncated));
    }

    #[test]
    fn oversize_unfragmented_is_capacity() {
        assert_eq!(
            unfragmented_fit(MAX_HANDSHAKE_BYTES, MIN_QDNF_MTU),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn split_and_reassemble_pq_share() {
        let mut payload = [0u8; 1216];
        payload[0] = 0x11;
        payload[1215] = 0x22;
        let mtu = fragment_payload_mtu(MIN_QDNF_MTU).unwrap();
        let mut frames = [[0u8; MAX_FRAME]; MAX_FRAGMENTS];
        let mut lens = [0usize; MAX_FRAGMENTS];
        let n = split_handshake(7, &payload, mtu, &mut frames, &mut lens).unwrap();
        assert!(n >= 1);
        let mut r = HandshakeReassembler::new();
        let mut out = [0u8; 2048];
        let mut complete = None;
        for i in 0..n {
            let (hdr, body) = decode_fragment(&frames[i][..lens[i]]).unwrap();
            let ticket = r.admit(hdr.msg_id, hdr.total, hdr.frag_len).unwrap();
            complete = r.buffer(ticket, hdr.offset, body, &mut out).unwrap();
        }
        assert_eq!(complete, Some(1216));
        assert_eq!(&out[..1216], &payload);
        assert_eq!(r.occupied(), 0);
    }

    #[test]
    fn interface_loss_is_closed_and_reconnect_needs_new_admission() {
        let mut r = HandshakeReassembler::new();
        let ticket = r.admit(1, 4, 4).unwrap();
        r.lose_interface();
        assert_eq!(r.admit(1, 4, 4), Err(QdnfError::Closed));
        let mut out = [0u8; 4];
        assert_eq!(
            r.buffer(ticket, 0, &[1, 2, 3, 4], &mut out),
            Err(QdnfError::Closed)
        );
        assert_eq!(r.buffered_len(1), 0);
        r.reconnect().unwrap();
        assert_eq!(
            r.buffer(ticket, 0, &[1, 2, 3, 4], &mut out),
            Err(QdnfError::StaleGeneration)
        );
        let ticket2 = r.admit(1, 4, 4).unwrap();
        assert_eq!(
            r.buffer(ticket2, 0, &[1, 2, 3, 4], &mut out).unwrap(),
            Some(4)
        );
        assert_eq!(&out[..4], &[1, 2, 3, 4]);
    }

    #[test]
    fn third_concurrent_session_is_capacity() {
        let mut r = HandshakeReassembler::new();
        r.admit(1, 8, 4).unwrap();
        r.admit(2, 8, 4).unwrap();
        assert_eq!(r.admit(3, 8, 4), Err(QdnfError::Capacity));
        assert_eq!(r.occupied(), 2);
        assert_eq!(r.buffered_len(3), 0);
    }

    #[test]
    fn admit_and_buffer_completes_and_oversize_total_is_capacity() {
        let mut payload = [0u8; 24];
        payload[0] = 0xA1;
        payload[23] = 0xA2;
        let mut frames = [[0u8; MAX_FRAME]; MAX_FRAGMENTS];
        let mut lens = [0usize; MAX_FRAGMENTS];
        let n = split_handshake(3, &payload, 16, &mut frames, &mut lens).unwrap();
        assert!(n >= 2);
        let mut r = HandshakeReassembler::new();
        let mut out = [0u8; 32];
        let mut complete = None;
        for i in 0..n {
            complete = admit_and_buffer(&mut r, &frames[i][..lens[i]], &mut out).unwrap();
        }
        assert_eq!(complete, Some(24));
        assert_eq!(&out[..24], &payload);
        assert_eq!(r.occupied(), 0);

        let mut closed = HandshakeReassembler::new();
        let too_big = (MAX_HANDSHAKE_BYTES as u16).saturating_add(1);
        assert_eq!(closed.admit(9, too_big, 8), Err(QdnfError::Capacity));
        assert_eq!(closed.occupied(), 0);
        assert_eq!(closed.buffered_len(9), 0);
        assert_eq!(
            admit_and_buffer(&mut closed, &[0u8; 4], &mut out),
            Err(QdnfError::Truncated)
        );
        assert_eq!(closed.buffered_len(9), 0);
    }
}
