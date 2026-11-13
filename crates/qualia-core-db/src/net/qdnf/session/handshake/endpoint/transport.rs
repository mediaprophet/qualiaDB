//! One bounded Bearer frame per step; actual existing fragmentation/admission.
use super::{E, EndpointWorkspace, Route};
use crate::net::qdnf::{
    bearer::{
        contract::Bearer,
        fragment::{FRAG_HDR_LEN, MAX_FRAGMENTS, decode_fragment, split_handshake},
    },
    frame::{FrameHeader, decode_frame, encode_frame},
    registries::{FrameType, NextProtocol, flags},
};
#[derive(Default)]
pub(super) struct Cursor {
    tx: usize,
    count: usize,
    kind: u8,
    mtu: u16,
    rx_offset: usize,
    rx_total: usize,
    rx_count: usize,
}
fn frame_type(kind: u8) -> FrameType {
    if kind == 1 {
        FrameType::SessionChallenge
    } else {
        FrameType::SessionProof
    }
}
impl Cursor {
    pub fn queue(
        &mut self,
        w: &mut EndpointWorkspace,
        len: usize,
        mtu: u16,
        kind: u8,
    ) -> Result<(), E> {
        let payload = super::super::hello_fragment_payload_mtu(mtu)?;
        self.count = split_handshake(
            kind as u32,
            &w.io[..len],
            payload,
            &mut w.frames,
            &mut w.lens,
        )?;
        self.tx = 0;
        self.kind = kind;
        self.mtu = mtu;
        Ok(())
    }
    pub fn send<B: Bearer>(
        &mut self,
        w: &mut EndpointWorkspace,
        route: Route,
        b: &mut B,
    ) -> Result<bool, E> {
        route.check(b)?;
        if self.tx >= self.count {
            return Err(E::Replay);
        }
        let mut header = FrameHeader::new(frame_type(self.kind), NextProtocol::QSession);
        header.flags = flags::FRAGMENT;
        header.payload_len = u16::try_from(w.lens[self.tx]).map_err(|_| E::Capacity)?;
        let n = encode_frame(&header, &w.frames[self.tx][..w.lens[self.tx]], &mut w.wire)?;
        if n > self.mtu as usize {
            return Err(E::Capacity);
        }
        let sent = b.send(&route.peer, &w.wire[..n])?;
        if sent != n {
            return Err(E::Truncated);
        }
        self.tx += 1;
        Ok(self.tx == self.count)
    }
    pub fn receive<B: Bearer>(
        &mut self,
        w: &mut EndpointWorkspace,
        route: Route,
        b: &mut B,
        kind: u8,
        mtu: u16,
    ) -> Result<Option<usize>, E> {
        route.check(b)?;
        let (n, meta) = b.recv(&mut w.wire)?;
        if meta.observed_source != route.peer
            || meta.scope != route.scope
            || n > mtu as usize
            || n > meta.mtu as usize
        {
            return Err(E::Unauthorized);
        }
        let wire = w.wire.get(..n).ok_or(E::Capacity)?;
        let (header, offset, len) = decode_frame(wire)?;
        if header.frame_type != frame_type(kind)
            || header.next_protocol != NextProtocol::QSession
            || header.flags != flags::FRAGMENT
            || offset.checked_add(len) != Some(n)
        {
            return Err(E::Malformed);
        }
        let payload = wire.get(offset..offset + len).ok_or(E::Truncated)?;
        let (frag, bytes) = decode_fragment(payload)?;
        let total = frag.total as usize;
        let start = frag.offset as usize;
        let chunk = super::super::hello_fragment_payload_mtu(mtu)? as usize;
        if frag.msg_id != kind as u32
            || start != self.rx_offset
            || payload.len() != FRAG_HDR_LEN + bytes.len()
            || self.rx_count >= MAX_FRAGMENTS
            || total > 8192
            || start >= total
            || bytes.len() != (total - start).min(chunk)
            || (self.rx_count != 0 && total != self.rx_total)
        {
            return Err(E::Malformed);
        }
        let ticket = w.reassembly.admit(frag.msg_id, frag.total, frag.frag_len)?;
        let result = w.reassembly.buffer(ticket, frag.offset, bytes, &mut w.io)?;
        self.rx_total = total;
        self.rx_offset += bytes.len();
        self.rx_count += 1;
        if result.is_some() {
            self.rx_offset = 0;
            self.rx_total = 0;
            self.rx_count = 0;
        }
        Ok(result)
    }
}
