//! Wire pack/unpack and FaultPipe delivery of authorised stream/datagram frames.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::faults::FRAME_CAP;
use crate::net::qdnf::harness::oracle::independent_magic_ok;
use crate::net::qdnf::route::forwarding::decrement_hop;
use crate::net::qdnf::session::datagrams::{Datagram, DeliveryStage};
use crate::net::qdnf::session::paths::PathState;
use crate::net::qdnf::session::streams::StreamFrame;

use super::{admit_application, MultiHop, DEST_NODE, MIDDLE_NODE, MIDDLE_SLOT, ORIGIN_SLOT};

pub(super) const HEADER: usize = 16;
pub(super) const STREAM_META: usize = 13;
const HOP_OFFSET: usize = 12;
pub(super) const TYPE_STREAM: u8 = 65;
const TYPE_DGRAM: u8 = 64;
const DEFAULT_HOP: u8 = 3;

impl MultiHop {
    pub fn send_stream(&mut self, frame: StreamFrame, payload: &[u8]) -> Result<(), QdnfError> {
        self.send_stream_with_hop(frame, payload, DEFAULT_HOP)
    }

    pub fn send_stream_with_hop(
        &mut self,
        frame: StreamFrame,
        payload: &[u8],
        hop_limit: u8,
    ) -> Result<(), QdnfError> {
        self.preflight()?;
        let mut meta = [0u8; STREAM_META + 64];
        if payload.len() > 64 {
            return Err(QdnfError::Capacity);
        }
        let mn = pack_stream_meta(frame, payload, &mut meta)?;
        let mut wire = [0u8; FRAME_CAP];
        let n = pack_header(
            TYPE_STREAM,
            hop_limit,
            self.active_path,
            &meta[..mn],
            &mut wire,
        )?;
        self.inject(&mut wire[..n])
    }

    pub fn send_datagram(&mut self, dgram: Datagram, payload: &[u8]) -> Result<(), QdnfError> {
        self.preflight()?;
        dgram.live_at(0)?;
        if payload.len() != dgram.len as usize {
            return Err(QdnfError::Malformed);
        }
        let mut body = [0u8; 80];
        if payload.len() + 16 > body.len() {
            return Err(QdnfError::Capacity);
        }
        body[..16].copy_from_slice(&dgram.operation.0);
        body[16..16 + payload.len()].copy_from_slice(payload);
        let mut wire = [0u8; FRAME_CAP];
        let n = pack_header(
            TYPE_DGRAM,
            DEFAULT_HOP,
            self.active_path,
            &body[..16 + payload.len()],
            &mut wire,
        )?;
        self.inject(&mut wire[..n])
    }

    pub fn consume(&mut self, out: &mut [u8]) -> Result<usize, QdnfError> {
        let mut scratch = [0u8; FRAME_CAP];
        let n = self.middle.pop(&mut scratch)?;
        if n <= HEADER || !independent_magic_ok(&scratch[..n]) {
            return Err(if n <= HEADER {
                QdnfError::Truncated
            } else {
                QdnfError::Malformed
            });
        }
        match self.paths.get(scratch[13]) {
            Some(slot) if slot.state == PathState::Active => {}
            _ => return Err(QdnfError::Closed),
        }
        admit_application(&self.auth)?;
        let payload = &scratch[HEADER..n];
        if scratch[5] == TYPE_DGRAM {
            if payload.len() < 16 {
                return Err(QdnfError::Truncated);
            }
            let data = &payload[16..];
            if out.len() < data.len() {
                return Err(QdnfError::Capacity);
            }
            if !data.is_empty() {
                out[..data.len()].copy_from_slice(data);
            }
            self.last_stage = DeliveryStage::TransportAck;
            return Ok(data.len());
        }
        let frame = unpack_stream(payload)?;
        let data = &payload[STREAM_META..STREAM_META + frame.len as usize];
        if out.len() < data.len() {
            return Err(QdnfError::Capacity);
        }
        self.stream.accept(frame)?;
        if !data.is_empty() {
            out[..data.len()].copy_from_slice(data);
        }
        self.last_stage = DeliveryStage::Consumed;
        Ok(data.len())
    }

    fn preflight(&self) -> Result<(), QdnfError> {
        admit_application(&self.auth)?;
        match self.paths.get(self.active_path) {
            Some(slot) if slot.state == PathState::Active => Ok(()),
            _ => Err(QdnfError::Closed),
        }
    }

    fn inject(&mut self, wire: &mut [u8]) -> Result<(), QdnfError> {
        if wire.len() <= HOP_OFFSET {
            return Err(QdnfError::Truncated);
        }
        let next0 = self.gens[ORIGIN_SLOT].lookup_next(DEST_NODE)?;
        if next0 != MIDDLE_NODE {
            return Err(QdnfError::NoRoute);
        }
        wire[HOP_OFFSET] = decrement_hop(wire[HOP_OFFSET])?;
        let next1 = self.gens[MIDDLE_SLOT].lookup_next(DEST_NODE)?;
        if next1 != DEST_NODE {
            return Err(QdnfError::NoRoute);
        }
        wire[HOP_OFFSET] = decrement_hop(wire[HOP_OFFSET])?;
        self.middle.push(wire)
    }
}

#[rustfmt::skip]
fn pack_header(frame_type: u8, hop: u8, path: u8, payload: &[u8], out: &mut [u8]) -> Result<usize, QdnfError> {
    let total = HEADER.checked_add(payload.len()).ok_or(QdnfError::Capacity)?;
    if total > FRAME_CAP || out.len() < total {
        return Err(QdnfError::Capacity);
    }
    out[..4].copy_from_slice(b"QDNF");
    out[4] = 1;
    out[5] = frame_type;
    out[6] = 0;
    out[7] = 0;
    out[8] = 0;
    out[9] = HEADER as u8;
    let pl = payload.len() as u16;
    out[10] = (pl >> 8) as u8;
    out[11] = pl as u8;
    out[12] = hop;
    out[13] = path;
    out[14] = 0;
    out[15] = 0;
    if !payload.is_empty() {
        out[HEADER..total].copy_from_slice(payload);
    }
    Ok(total)
}

fn pack_stream_meta(frame: StreamFrame, data: &[u8], out: &mut [u8]) -> Result<usize, QdnfError> {
    if data.len() != frame.len as usize {
        return Err(QdnfError::Malformed);
    }
    let n = STREAM_META
        .checked_add(data.len())
        .ok_or(QdnfError::Capacity)?;
    if out.len() < n {
        return Err(QdnfError::Capacity);
    }
    out[0..2].copy_from_slice(&frame.stream_id.to_be_bytes());
    out[2..10].copy_from_slice(&frame.offset.to_be_bytes());
    out[10] = u8::from(frame.fin);
    out[11..13].copy_from_slice(&frame.len.to_be_bytes());
    if !data.is_empty() {
        out[13..n].copy_from_slice(data);
    }
    Ok(n)
}

fn unpack_stream(payload: &[u8]) -> Result<StreamFrame, QdnfError> {
    if payload.len() < STREAM_META {
        return Err(QdnfError::Truncated);
    }
    let mut off = [0u8; 8];
    off.copy_from_slice(&payload[2..10]);
    let len = u16::from_be_bytes([payload[11], payload[12]]);
    if payload.len() < STREAM_META + len as usize {
        return Err(QdnfError::Truncated);
    }
    Ok(StreamFrame {
        stream_id: u16::from_be_bytes([payload[0], payload[1]]),
        offset: u64::from_be_bytes(off),
        fin: payload[10] != 0,
        declared_final: None,
        len,
    })
}
