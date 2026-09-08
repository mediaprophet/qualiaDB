//! Native in-process multi-hop simulation for NET-05.15.
//!
//! Nodes 0→1→2 forward authorized streams/datagrams through a published
//! `ForwardingGeneration` and one `FaultPipe` on the middle hop. This is not
//! Ethernet, not OS isolation, not a libp2p replacement of the default daemon,
//! and not package completion. Packages remain open; no frozen COSE object.

use crate::net::qdnf::authority::{evaluate_precedence, ContactState, PolicyOutcome};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::faults::{FaultPipe, FaultSchedule, FRAME_CAP, PIPE_SLOTS};
use crate::net::qdnf::harness::oracle::{independent_hop_limit, independent_magic_ok};
use crate::net::qdnf::route::forwarding::{decrement_hop, ForwardingGeneration};
use crate::net::qdnf::route::spf::{compute_spf, LinkMetric, SpfTable};
use crate::net::qdnf::session::datagrams::{Datagram, DeliveryStage};
use crate::net::qdnf::session::paths::{PathState, PathTable};
use crate::net::qdnf::session::policy::gate_channel;
use crate::net::qdnf::session::streams::{StreamFrame, StreamState};
use crate::net::qdnf::types::OperationId;

pub const NODE_COUNT: usize = 3;
pub const ORIGIN_NODE: u8 = 0;
pub const DEST_NODE: u8 = 2;
const HEADER: usize = 16;
const STREAM_META: usize = 13;
const HOP_OFFSET: usize = 12;
const TYPE_STREAM: u8 = 65;
const TYPE_DGRAM: u8 = 64;
const DEFAULT_HOP: u8 = 3;

#[inline]
#[rustfmt::skip]
pub fn transport_ack_is_durable() -> bool { false }
#[inline]
#[rustfmt::skip]
pub fn os_isolation_evidence() -> bool { false }
#[inline]
#[rustfmt::skip]
pub fn ethernet_demonstrated() -> bool { false }

#[derive(Clone, Copy, Debug)]
pub struct AuthState {
    pub outcome: PolicyOutcome,
    pub grant_current: bool,
    pub contact: ContactState,
    pub revoked: bool,
}

impl AuthState {
    pub const OPEN: Self = Self {
        outcome: PolicyOutcome::Allow,
        grant_current: true,
        contact: ContactState::Active,
        revoked: false,
    };
}

fn admit_application(auth: &AuthState) -> Result<(), QdnfError> {
    evaluate_precedence(
        auth.revoked,
        matches!(auth.contact, ContactState::Blocked),
        !auth.grant_current,
        auth.outcome.admits_service(),
    )?;
    gate_channel(auth.outcome, auth.grant_current, auth.contact)?;
    match auth.contact {
        ContactState::Active | ContactState::Consent => Ok(()),
        _ => Err(QdnfError::Denied),
    }
}

/// Bounded 3-hop authorized path with middle-hop faults and cell handoff.
pub struct MultiHop {
    gens: [ForwardingGeneration; NODE_COUNT],
    middle: FaultPipe,
    stream: StreamState,
    paths: PathTable,
    active_path: u8,
    pub auth: AuthState,
    pub operation: OperationId,
    last_stage: DeliveryStage,
}

impl MultiHop {
    pub fn unpublished() -> Self {
        let mut paths = PathTable::new();
        let active_path = paths.start_race(1).expect("race");
        paths.mark_active(active_path).expect("active");
        Self {
            gens: [
                ForwardingGeneration::empty(1),
                ForwardingGeneration::empty(2),
                ForwardingGeneration::empty(3),
            ],
            middle: FaultPipe::new(),
            stream: StreamState::new(),
            paths,
            active_path,
            auth: AuthState::OPEN,
            operation: OperationId::ZERO,
            last_stage: DeliveryStage::TransportAck,
        }
    }

    pub fn line() -> Self {
        let mut s = Self::unpublished();
        s.publish_line().expect("spf");
        s
    }

    pub fn line_with_schedule(schedule: FaultSchedule) -> Self {
        let mut s = Self::line();
        s.middle = FaultPipe::with_schedule(schedule);
        s
    }

    pub fn publish_line(&mut self) -> Result<(), QdnfError> {
        let links = [
            LinkMetric {
                from: 0,
                to: 1,
                cost: 1,
                bidirectional: true,
            },
            LinkMetric {
                from: 1,
                to: 2,
                cost: 1,
                bidirectional: true,
            },
        ];
        let mut i = 0u8;
        while i < NODE_COUNT as u8 {
            let mut table = SpfTable::EMPTY;
            compute_spf(i, &links, &mut table)?;
            self.gens[i as usize].publish(table)?;
            i += 1;
        }
        Ok(())
    }

    #[rustfmt::skip]
    pub fn lookup_dest(&self) -> Result<u8, QdnfError> { self.gens[ORIGIN_NODE as usize].lookup_next(DEST_NODE) }
    #[rustfmt::skip]
    pub fn queued(&self) -> usize { self.middle.queued() }
    #[rustfmt::skip]
    pub fn stream(&self) -> StreamState { self.stream }
    #[rustfmt::skip]
    pub fn paths(&self) -> &PathTable { &self.paths }
    #[rustfmt::skip]
    pub fn paths_mut(&mut self) -> &mut PathTable { &mut self.paths }
    #[rustfmt::skip]
    pub fn last_stage(&self) -> DeliveryStage { self.last_stage }
    #[rustfmt::skip]
    pub fn tick(&mut self) { self.middle.tick(); }

    pub fn handoff(&mut self) -> Result<u8, QdnfError> {
        let next = self.paths.start_race(self.active_path as u64 + 2)?;
        self.paths.mark_active(next)?;
        self.paths.lose_and_cleanup(self.active_path)?;
        self.active_path = next;
        Ok(next)
    }

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
        let _ = self.gens[0].lookup_next(DEST_NODE)?;
        wire[HOP_OFFSET] = decrement_hop(wire[HOP_OFFSET])?;
        let _ = self.gens[1].lookup_next(DEST_NODE)?;
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
    let n = STREAM_META.checked_add(data.len()).ok_or(QdnfError::Capacity)?;
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::harness::faults::Fault;
    use crate::net::qdnf::session::paths::MAX_ACTIVE_PATHS;

    fn sf(offset: u64, len: u16) -> StreamFrame {
        StreamFrame {
            stream_id: 0,
            offset,
            fin: false,
            declared_final: None,
            len,
        }
    }

    fn first(fault: Fault) -> FaultSchedule {
        let mut pattern = [Fault::None; PIPE_SLOTS];
        pattern[0] = fault;
        FaultSchedule::with_pattern(pattern)
    }

    #[test]
    fn three_hop_stream_delivers_in_order_without_fault() {
        let mut m = MultiHop::line();
        m.send_stream(sf(0, 2), &[b'a', b'b']).unwrap();
        let mut out = [0u8; 8];
        let n = m.consume(&mut out).unwrap();
        assert_eq!(&out[..n], &[b'a', b'b']);
        m.send_stream(sf(2, 2), &[b'c', b'd']).unwrap();
        let n = m.consume(&mut out).unwrap();
        assert_eq!(&out[..n], &[b'c', b'd']);
        assert_eq!(m.stream().next_offset, 4);
        assert!(!transport_ack_is_durable());
        assert!(!os_isolation_evidence());
        assert!(!ethernet_demonstrated());
        assert_ne!(DeliveryStage::TransportAck, DeliveryStage::DurableApplied);
    }

    #[test]
    fn reorder_delay_recovers_after_tick() {
        let mut out = [0u8; 8];
        let mut m = MultiHop::line_with_schedule(first(Fault::Reorder));
        m.send_stream(sf(0, 1), &[7]).unwrap();
        assert_eq!(m.consume(&mut out), Err(QdnfError::WouldBlock));
        m.tick();
        let n = m.consume(&mut out).unwrap();
        assert_eq!(&out[..n], &[7]);
        let mut d = MultiHop::line_with_schedule(first(Fault::Delay));
        d.send_stream(sf(0, 1), &[9]).unwrap();
        assert_eq!(d.consume(&mut out), Err(QdnfError::WouldBlock));
        d.tick();
        assert_eq!(d.consume(&mut out).unwrap(), 1);
    }

    #[test]
    fn drop_then_retransmit_delivers_once() {
        let mut m = MultiHop::line_with_schedule(first(Fault::Drop));
        let frame = sf(0, 1);
        m.send_stream(frame, &[1]).unwrap();
        assert_eq!(m.queued(), 0);
        let mut out = [0u8; 8];
        assert_eq!(m.consume(&mut out), Err(QdnfError::WouldBlock));
        m.send_stream(frame, &[1]).unwrap();
        assert_eq!(m.consume(&mut out).unwrap(), 1);
        assert_eq!(m.stream().next_offset, 1);
        m.send_stream(frame, &[1]).unwrap();
        assert_eq!(m.consume(&mut out), Err(QdnfError::Overlap));
        assert_eq!(m.stream().next_offset, 1);
    }

    #[test]
    fn revoked_or_blocked_refuses_further_frames() {
        let mut m = MultiHop::line();
        m.send_stream(sf(0, 1), &[1]).unwrap();
        let mut out = [0u8; 8];
        assert_eq!(m.consume(&mut out).unwrap(), 1);
        m.auth.revoked = true;
        assert_eq!(m.send_stream(sf(1, 1), &[2]), Err(QdnfError::Revoked));
        m.auth.revoked = false;
        m.auth.contact = ContactState::Blocked;
        assert_eq!(m.send_stream(sf(1, 1), &[2]), Err(QdnfError::Denied));
        m.auth.contact = ContactState::Suspended;
        assert_eq!(m.send_stream(sf(1, 1), &[2]), Err(QdnfError::Denied));
        assert_eq!(m.stream().next_offset, 1);
        m.auth.contact = ContactState::Active;
        m.send_stream(sf(1, 1), &[2]).unwrap();
        m.auth.revoked = true;
        assert_eq!(m.consume(&mut out), Err(QdnfError::Revoked));
        assert_eq!(m.stream().next_offset, 1);
    }

    #[test]
    fn hop_limit_zero_and_unpublished_fail_closed() {
        assert_eq!(decrement_hop(0), Err(QdnfError::HopLimit));
        assert_eq!(
            MultiHop::unpublished().lookup_dest(),
            Err(QdnfError::Incomplete)
        );
        let mut m = MultiHop::line();
        assert_eq!(
            m.send_stream_with_hop(sf(0, 1), &[1], 0),
            Err(QdnfError::HopLimit)
        );
        assert_eq!(m.queued(), 0);
    }

    #[test]
    fn fourth_active_path_is_capacity() {
        let mut m = MultiHop::line();
        let b = m.paths_mut().start_race(2).unwrap();
        m.paths_mut().mark_active(b).unwrap();
        let c = m.paths_mut().start_race(3).unwrap();
        m.paths_mut().mark_active(c).unwrap();
        assert_eq!(m.paths().active_count(), MAX_ACTIVE_PATHS);
        let d = m.paths_mut().start_race(4).unwrap();
        assert_eq!(m.paths_mut().mark_active(d), Err(QdnfError::Capacity));
        let mut h = MultiHop::line();
        h.send_stream(sf(0, 1), &[3]).unwrap();
        h.handoff().unwrap();
        let mut out = [0u8; 8];
        assert_eq!(h.consume(&mut out), Err(QdnfError::Closed));
        assert_eq!(h.stream().next_offset, 0);
        h.send_stream(sf(0, 1), &[3]).unwrap();
        assert_eq!(h.consume(&mut out).unwrap(), 1);
        assert_eq!(h.operation, OperationId::ZERO);
    }

    #[test]
    fn stalled_consumer_pipe_slots_capacity() {
        let mut m = MultiHop::line();
        let mut i = 0usize;
        while i < PIPE_SLOTS {
            m.send_stream(sf(0, 1), &[1]).unwrap();
            i += 1;
        }
        assert_eq!(m.queued(), PIPE_SLOTS);
        assert_eq!(m.send_stream(sf(0, 1), &[1]), Err(QdnfError::Capacity));
        assert_eq!(m.stream().next_offset, 0);
    }

    #[test]
    fn oracle_hop_limit_independent_of_product_decrement() {
        let mut src = [0u8; HEADER];
        src[0] = b'Q';
        src[1] = b'D';
        src[2] = b'N';
        src[3] = b'F';
        src[5] = TYPE_STREAM;
        src[12] = 7;
        assert!(independent_magic_ok(&src));
        let product = decrement_hop(7).unwrap();
        assert_eq!(independent_hop_limit(&src), Ok(7));
        assert_eq!(product, 6);
        assert_ne!(product, independent_hop_limit(&src).unwrap());
    }
}
