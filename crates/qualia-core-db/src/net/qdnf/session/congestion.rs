//! Byte-based congestion control, ECN validation, and shared-bottleneck caps.
//!
//! Application [`super::credit::CreditTable`] is not a congestion window.
//! TRANSPORT-A: a 16-slot packet-number map retires in-flight bytes once.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::credit::CreditTable;
use crate::net::qdnf::session::paths::{PathHandle, MAX_RACE_CANDIDATES};
use crate::net::qdnf::session::rtt::RttEstimator;

/// Controller segment (not discovered PMTU).
pub const CC_SEGMENT_BYTES: u64 = 1200;
pub const INITIAL_WINDOW_BYTES: u64 = 14_720;
pub const MIN_WINDOW_BYTES: u64 = 2 * CC_SEGMENT_BYTES;
const FLIGHT_SLOTS: usize = 16;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FlightSlot {
    pn: u64,
    bytes: u64,
    retired: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Congestion {
    pub bytes_in_flight: u64,
    pub window_bytes: u64,
    pub ssthresh: u64,
    flight: [Option<FlightSlot>; FLIGHT_SLOTS],
}

impl Congestion {
    pub const fn new() -> Self {
        Self::with_window(INITIAL_WINDOW_BYTES)
    }

    pub const fn with_window(window_bytes: u64) -> Self {
        Self {
            bytes_in_flight: 0,
            window_bytes,
            ssthresh: u64::MAX,
            flight: [None; FLIGHT_SLOTS],
        }
    }
}

impl Default for Congestion {
    fn default() -> Self {
        Self::new()
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct EcnState {
    pub ect: bool,
    pub ce_count: u32,
    pub validated: bool,
}

impl EcnState {
    pub const fn new() -> Self {
        Self {
            ect: false,
            ce_count: 0,
            validated: false,
        }
    }
}

/// Fail-closed path MTU. Unknown is not a silent 1280-byte assumption.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PmtuState {
    mtu: Option<u16>,
}

impl PmtuState {
    pub const fn unknown() -> Self {
        Self { mtu: None }
    }

    pub fn current(self) -> Result<u16, QdnfError> {
        self.mtu.ok_or(QdnfError::Incomplete)
    }

    pub fn discover(&mut self, mtu: u16) -> Result<u16, QdnfError> {
        if mtu == 0 {
            return Err(QdnfError::Range);
        }
        self.mtu = Some(mtu);
        Ok(mtu)
    }

    /// Reduce a known MTU. Unknown is Incomplete; increase is Range.
    pub fn shrink(&mut self, mtu: u16) -> Result<u16, QdnfError> {
        let cur = self.current()?;
        if mtu == 0 || mtu > cur {
            return Err(QdnfError::Range);
        }
        self.mtu = Some(mtu);
        Ok(mtu)
    }

    /// Oversize is Capacity. Unknown is Incomplete, never a silent 1280 clamp.
    pub fn admit_bytes(&self, len: usize) -> Result<(), QdnfError> {
        let mtu = self.current()?;
        if len > mtu as usize {
            Err(QdnfError::Capacity)
        } else {
            Ok(())
        }
    }
}

#[derive(Clone, Copy, Debug)]
struct PathLocalCc {
    handle: PathHandle,
    cc: Congestion,
    rtt: RttEstimator,
}

/// Connection congestion. Extra paths share one window unless independent.
#[derive(Clone, Copy, Debug)]
pub struct PathCcTable {
    pub shared: Congestion,
    pub independent_bottleneck: bool,
    locals: [Option<PathLocalCc>; MAX_RACE_CANDIDATES],
}

impl PathCcTable {
    pub const fn new(window_bytes: u64) -> Self {
        Self {
            shared: Congestion::with_window(window_bytes),
            independent_bottleneck: false,
            locals: [None; MAX_RACE_CANDIDATES],
        }
    }

    pub fn attach(&mut self, handle: PathHandle) -> Result<(), QdnfError> {
        let i = slot_index(handle)?;
        if let Some(existing) = self.locals[i] {
            if existing.handle.generation() != handle.generation() {
                return Err(QdnfError::StaleGeneration);
            }
            return Ok(());
        }
        self.locals[i] = Some(PathLocalCc {
            handle,
            cc: Congestion::with_window(self.shared.window_bytes),
            rtt: RttEstimator::new(),
        });
        Ok(())
    }

    pub fn path_rtt(&self, handle: PathHandle) -> Result<RttEstimator, QdnfError> {
        Ok(self.local(handle)?.rtt)
    }

    /// Window check without consuming a packet number.
    pub fn check_send(&self, handle: PathHandle, bytes: u64) -> Result<(), QdnfError> {
        if self.independent_bottleneck {
            can_send(&self.local(handle)?.cc, bytes)
        } else {
            let _ = self.local(handle)?;
            can_send(&self.shared, bytes)
        }
    }

    pub fn send(&mut self, handle: PathHandle, pn: u64, bytes: u64) -> Result<(), QdnfError> {
        self.check_send(handle, bytes)?;
        let independent = self.independent_bottleneck;
        if independent {
            return on_send(&mut self.local_mut(handle)?.cc, pn, bytes);
        }
        let _ = self.local(handle)?;
        on_send(&mut self.shared, pn, bytes)
    }

    pub fn total_in_flight(&self) -> u64 {
        if self.independent_bottleneck {
            let mut n = 0u64;
            let mut i = 0usize;
            while i < MAX_RACE_CANDIDATES {
                if let Some(local) = self.locals[i] {
                    n = n.saturating_add(local.cc.bytes_in_flight);
                }
                i += 1;
            }
            n
        } else {
            self.shared.bytes_in_flight
        }
    }

    fn local(&self, handle: PathHandle) -> Result<&PathLocalCc, QdnfError> {
        let i = slot_index(handle)?;
        let local = self.locals[i].as_ref().ok_or(QdnfError::Closed)?;
        if local.handle.generation() != handle.generation() {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(local)
    }

    fn local_mut(&mut self, handle: PathHandle) -> Result<&mut PathLocalCc, QdnfError> {
        let i = slot_index(handle)?;
        let local = self.locals[i].as_mut().ok_or(QdnfError::Closed)?;
        if local.handle.generation() != handle.generation() {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(local)
    }
}

fn slot_index(handle: PathHandle) -> Result<usize, QdnfError> {
    let i = handle.slot() as usize;
    if i >= MAX_RACE_CANDIDATES {
        Err(QdnfError::Range)
    } else {
        Ok(i)
    }
}

pub fn can_send(cc: &Congestion, payload: u64) -> Result<(), QdnfError> {
    if payload == 0 {
        return Err(QdnfError::Range);
    }
    let next = cc.bytes_in_flight.saturating_add(payload);
    if next > cc.window_bytes {
        Err(QdnfError::BudgetExhausted)
    } else {
        Ok(())
    }
}

/// Record `bytes` under `pn`. Duplicate live PN is Replay; full map is Capacity.
pub fn on_send(cc: &mut Congestion, pn: u64, bytes: u64) -> Result<(), QdnfError> {
    can_send(cc, bytes)?;
    insert_flight(cc, pn, bytes)?;
    cc.bytes_in_flight = cc.bytes_in_flight.saturating_add(bytes);
    Ok(())
}

pub fn on_acked(cc: &mut Congestion, bytes: u64) {
    cc.bytes_in_flight = cc.bytes_in_flight.saturating_sub(bytes);
    grow_window(cc, bytes);
}

pub fn on_lost(cc: &mut Congestion, bytes: u64) {
    cc.bytes_in_flight = cc.bytes_in_flight.saturating_sub(bytes);
    collapse_window(cc);
}

pub fn on_acked_pn(cc: &mut Congestion, pn: u64) -> Result<(), QdnfError> {
    match take_live(cc, pn)? {
        None => Ok(()),
        Some(bytes) => {
            on_acked(cc, bytes);
            Ok(())
        }
    }
}

pub fn on_lost_pn(cc: &mut Congestion, pn: u64) -> Result<(), QdnfError> {
    match take_live(cc, pn)? {
        None => Ok(()),
        Some(bytes) => {
            on_lost(cc, bytes);
            Ok(())
        }
    }
}

/// Unvalidated CE never increments `ce_count` and never collapses the window.
pub fn on_ecn(ecn: &mut EcnState, ce: bool) {
    if ce && ecn.validated {
        ecn.ce_count = ecn.ce_count.saturating_add(1);
    }
}

pub fn apply_ecn(cc: &mut Congestion, ecn: &EcnState, ce: bool) {
    if ce && ecn.validated {
        collapse_window(cc);
    }
}

/// Congestion first (BudgetExhausted), then application credit (Capacity).
pub fn admit_send(
    cc: &Congestion,
    credit: &CreditTable,
    dir: u8,
    payload: u64,
) -> Result<(), QdnfError> {
    can_send(cc, payload)?;
    credit.admit_application(dir, payload)?;
    Ok(())
}

pub fn application_credit_is_congestion() -> bool {
    false
}

pub fn path_cc_is_connection_authority() -> bool {
    false
}

fn grow_window(cc: &mut Congestion, acked: u64) {
    if acked == 0 {
        return;
    }
    if cc.window_bytes < cc.ssthresh {
        cc.window_bytes = cc.window_bytes.saturating_add(acked);
        return;
    }
    let denom = cc.window_bytes.max(1);
    let inc = CC_SEGMENT_BYTES.saturating_mul(acked) / denom;
    cc.window_bytes = cc.window_bytes.saturating_add(inc.max(1));
}

fn collapse_window(cc: &mut Congestion) {
    let half = (cc.window_bytes / 2).max(MIN_WINDOW_BYTES);
    cc.ssthresh = half;
    cc.window_bytes = half;
}

fn insert_flight(cc: &mut Congestion, pn: u64, bytes: u64) -> Result<(), QdnfError> {
    let mut free = None;
    let mut i = 0usize;
    while i < FLIGHT_SLOTS {
        match cc.flight[i] {
            Some(slot) if slot.pn == pn && !slot.retired => return Err(QdnfError::Replay),
            Some(slot) if slot.pn == pn && slot.retired => {
                cc.flight[i] = Some(FlightSlot {
                    pn,
                    bytes,
                    retired: false,
                });
                return Ok(());
            }
            None if free.is_none() => free = Some(i),
            _ => {}
        }
        i += 1;
    }
    let idx = free.ok_or(QdnfError::Capacity)?;
    cc.flight[idx] = Some(FlightSlot {
        pn,
        bytes,
        retired: false,
    });
    Ok(())
}

fn take_live(cc: &mut Congestion, pn: u64) -> Result<Option<u64>, QdnfError> {
    let mut i = 0usize;
    while i < FLIGHT_SLOTS {
        if let Some(slot) = cc.flight[i] {
            if slot.pn == pn {
                if slot.retired {
                    return Ok(None);
                }
                cc.flight[i] = Some(FlightSlot {
                    pn,
                    bytes: slot.bytes,
                    retired: true,
                });
                return Ok(Some(slot.bytes));
            }
        }
        i += 1;
    }
    Err(QdnfError::Malformed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::session::credit::DIR_CLIENT;
    use crate::net::qdnf::session::paths::{mutate_path, PathState, PathTable};
    use crate::net::qdnf::types::Generation;

    #[test]
    fn transport_a_send_ack_conserves_bytes() {
        let mut cc = Congestion::with_window(10_000);
        on_send(&mut cc, 1, 400).unwrap();
        on_send(&mut cc, 2, 600).unwrap();
        assert_eq!(cc.bytes_in_flight, 1000);
        on_acked_pn(&mut cc, 1).unwrap();
        on_acked_pn(&mut cc, 2).unwrap();
        assert_eq!(cc.bytes_in_flight, 0);
    }

    #[test]
    fn transport_a_loss_does_not_double_count_later_ack() {
        let mut cc = Congestion::with_window(10_000);
        on_send(&mut cc, 7, 250).unwrap();
        assert_eq!(cc.bytes_in_flight, 250);
        on_lost_pn(&mut cc, 7).unwrap();
        assert_eq!(cc.bytes_in_flight, 0);
        on_acked_pn(&mut cc, 7).unwrap();
        assert_eq!(cc.bytes_in_flight, 0);
    }

    #[test]
    fn window_exhausted_is_budget_even_with_stream_credit() {
        let credit = CreditTable::new(10_000);
        let mut cc = Congestion::with_window(100);
        on_send(&mut cc, 1, 100).unwrap();
        assert!(credit.admit_application(DIR_CLIENT, 1).is_ok());
        assert_eq!(can_send(&cc, 1), Err(QdnfError::BudgetExhausted));
        assert_eq!(
            admit_send(&cc, &credit, DIR_CLIENT, 1),
            Err(QdnfError::BudgetExhausted)
        );
        assert!(!application_credit_is_congestion());
    }

    #[test]
    fn stream_credit_exhausted_is_capacity_even_with_cc_window() {
        let credit = CreditTable::new(0);
        let cc = Congestion::with_window(10_000);
        assert!(can_send(&cc, 64).is_ok());
        assert_eq!(
            credit.admit_application(DIR_CLIENT, 1),
            Err(QdnfError::Capacity)
        );
        assert_eq!(
            admit_send(&cc, &credit, DIR_CLIENT, 1),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn unvalidated_ce_does_not_collapse_window() {
        let mut cc = Congestion::with_window(8_000);
        let mut ecn = EcnState {
            ect: true,
            ce_count: 0,
            validated: false,
        };
        let before = cc.window_bytes;
        on_ecn(&mut ecn, true);
        apply_ecn(&mut cc, &ecn, true);
        assert_eq!(ecn.ce_count, 0);
        assert_eq!(cc.window_bytes, before);
        ecn.validated = true;
        on_ecn(&mut ecn, true);
        apply_ecn(&mut cc, &ecn, true);
        assert_eq!(ecn.ce_count, 1);
        assert!(cc.window_bytes < before);
    }

    #[test]
    fn shared_bottleneck_two_paths_one_window() {
        let mut paths = PathTable::new();
        let a = paths.start_race(1).unwrap();
        let b = paths.start_race(1).unwrap();
        paths.mark_active(a).unwrap();
        paths.mark_active(b).unwrap();
        let ha = paths.handle_of(a).unwrap();
        let hb = paths.handle_of(b).unwrap();
        let mut tbl = PathCcTable::new(500);
        assert!(!tbl.independent_bottleneck);
        tbl.attach(ha).unwrap();
        tbl.attach(hb).unwrap();
        tbl.send(ha, 1, 500).unwrap();
        assert_eq!(tbl.send(hb, 2, 1), Err(QdnfError::BudgetExhausted));
        assert!(tbl.total_in_flight() <= 500);
        assert!(!path_cc_is_connection_authority());
    }

    #[test]
    fn pmtu_shrink_rejects_oversize() {
        let mut pmtu = PmtuState::unknown();
        assert_eq!(pmtu.admit_bytes(1), Err(QdnfError::Incomplete));
        assert_eq!(pmtu.shrink(1280), Err(QdnfError::Incomplete));
        pmtu.discover(1500).unwrap();
        pmtu.admit_bytes(1500).unwrap();
        pmtu.shrink(1400).unwrap();
        assert_eq!(pmtu.admit_bytes(1401), Err(QdnfError::Capacity));
        pmtu.admit_bytes(1400).unwrap();
    }

    #[test]
    fn forged_ack_unknown_pn_malformed() {
        let mut cc = Congestion::with_window(10_000);
        on_send(&mut cc, 1, 100).unwrap();
        assert_eq!(on_acked_pn(&mut cc, 99), Err(QdnfError::Malformed));
        assert_eq!(cc.bytes_in_flight, 100);
    }

    #[test]
    fn packet_or_stream_exhaustion_is_capacity() {
        let mut cc = Congestion::with_window(1_000_000);
        let mut pn = 1u64;
        while pn <= 16 {
            on_send(&mut cc, pn, 10).unwrap();
            pn += 1;
        }
        assert_eq!(on_send(&mut cc, 17, 10), Err(QdnfError::Capacity));
        let credit = CreditTable::new(0);
        assert_eq!(
            admit_send(&cc, &credit, DIR_CLIENT, 1),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn stale_path_handle_rejected_by_cc_table() {
        let mut paths = PathTable::new();
        let id = paths.start_race(3).unwrap();
        paths
            .attest_reachability(paths.handle_of(id).unwrap())
            .unwrap();
        let stale = paths.handle_of(id).unwrap();
        let fresh = mutate_path(&mut paths, stale, PathState::Racing).unwrap();
        assert_ne!(stale.generation(), fresh.generation());
        let mut tbl = PathCcTable::new(1200);
        tbl.attach(fresh).unwrap();
        assert_eq!(tbl.send(stale, 1, 10), Err(QdnfError::StaleGeneration));
        assert_eq!(stale.generation(), Generation(3));
    }

    #[test]
    fn unknown_pmtu_fails_closed() {
        let mut p = PmtuState::unknown();
        assert_eq!(p.current(), Err(QdnfError::Incomplete));
        assert_eq!(p.discover(0), Err(QdnfError::Range));
        assert_eq!(p.discover(1500).unwrap(), 1500);
        assert_eq!(p.current(), Ok(1500));
    }
}
