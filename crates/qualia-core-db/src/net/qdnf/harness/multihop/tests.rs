use super::forward::{HEADER, TYPE_STREAM};
use super::topology::{
    plan_unrestricted, plan_with, profile_constraint, realm_constraint, shortcut_index,
};
use super::*;
use crate::net::qdnf::authority::ContactState;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::faults::{Fault, FaultSchedule, PIPE_SLOTS};
use crate::net::qdnf::harness::oracle::{independent_hop_limit, independent_magic_ok};
use crate::net::qdnf::route::forwarding::decrement_hop;
use crate::net::qdnf::session::datagrams::DeliveryStage;
use crate::net::qdnf::session::paths::MAX_ACTIVE_PATHS;
use crate::net::qdnf::session::streams::StreamFrame;
use crate::net::qdnf::types::OperationId;

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
fn route_f_protected_multihop_converges() {
    let mut m = MultiHop::line();
    assert_eq!(m.planned_hop_count(), 2);
    assert_eq!(m.lookup_dest(), Ok(MIDDLE_NODE));
    m.send_stream(sf(0, 2), &[b'q', b'r']).unwrap();
    let mut out = [0u8; 8];
    let n = m.consume(&mut out).unwrap();
    assert_eq!(&out[..n], &[b'q', b'r']);
    assert_eq!(m.stream().next_offset, 2);
    assert!(!transport_ack_is_durable());
    assert!(!os_isolation_evidence());
    assert!(!ethernet_demonstrated());
}

#[test]
fn route_f_forbidden_realm_cannot_bypass() {
    let index = shortcut_index().unwrap();
    let cheap = plan_unrestricted(&index).unwrap();
    assert_eq!(cheap.first_hop, DEST_NODE);
    assert_eq!(cheap.hop_len, 1);
    assert_eq!(cheap.metrics.cost, 1);

    let realm = plan_with(&index, &realm_constraint()).unwrap();
    assert_eq!(realm.first_hop, MIDDLE_NODE);
    assert_eq!(realm.hop_len, 2);
    assert!(realm.metrics.cost > cheap.metrics.cost);

    let profile = plan_with(&index, &profile_constraint()).unwrap();
    assert_eq!(profile.first_hop, MIDDLE_NODE);
    assert_eq!(profile.hop_len, 2);
    assert!(profile.metrics.cost > cheap.metrics.cost);
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

#[test]
fn flapping_next_hop_held_down_does_not_immediately_rejoin() {
    let mut m = MultiHop::line();
    assert_eq!(m.lookup_dest(), Ok(MIDDLE_NODE));
    m.partition_next_hop(10).unwrap();
    assert!(m.next_hop_held_down(10));
    assert_eq!(m.lookup_dest(), Err(QdnfError::Incomplete));
    assert_eq!(m.send_stream(sf(0, 1), &[1]), Err(QdnfError::Incomplete));
    m.heal_next_hop(11).unwrap();
    assert!(m.next_hop_held_down(11));
    assert_ne!(m.lookup_dest(), Ok(MIDDLE_NODE));
    assert_eq!(m.lookup_dest(), Err(QdnfError::Incomplete));
    assert!(!ethernet_demonstrated());
    m.heal_next_hop(70).unwrap();
    assert!(!m.next_hop_held_down(70));
    assert_eq!(m.lookup_dest(), Ok(MIDDLE_NODE));
    m.send_stream(sf(0, 1), &[1]).unwrap();
    let mut out = [0u8; 8];
    assert_eq!(m.consume(&mut out).unwrap(), 1);
    assert!(!ethernet_demonstrated());
    assert!(!os_isolation_evidence());
}
