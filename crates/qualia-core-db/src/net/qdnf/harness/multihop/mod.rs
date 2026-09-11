//! Native in-process multi-hop for ROUTE-F / NET-05.15.
//!
//! Production admission is `plan_routes` over nodes 1→2→3. Forwarding is
//! published only after planning succeeds. This is not Ethernet, not OS
//! isolation, not a durable ACK, and not a comparison against other routing.

use crate::net::qdnf::authority::{evaluate_precedence, ContactState, PolicyOutcome};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::harness::faults::FaultPipe;
use crate::net::qdnf::route::hysteresis::{hop_is_held_down, HopHoldDown, HysteresisPolicy};
use crate::net::qdnf::route::{CandidatePath, ForwardingGeneration};
use crate::net::qdnf::session::datagrams::DeliveryStage;
use crate::net::qdnf::session::paths::PathTable;
use crate::net::qdnf::session::policy::gate_channel;
use crate::net::qdnf::session::streams::StreamState;
use crate::net::qdnf::types::OperationId;

mod forward;
mod topology;

pub const NODE_COUNT: usize = 3;
pub const ORIGIN_NODE: u8 = 1;
pub const MIDDLE_NODE: u8 = 2;
pub const DEST_NODE: u8 = 3;
pub(super) const ORIGIN_SLOT: usize = 0;
pub(super) const MIDDLE_SLOT: usize = 1;
pub(super) const DEST_SLOT: usize = 2;

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

/// Bounded 1→2→3 authorized path with middle-hop faults and cell handoff.
pub struct MultiHop {
    gens: [ForwardingGeneration; NODE_COUNT],
    middle: FaultPipe,
    stream: StreamState,
    paths: PathTable,
    active_path: u8,
    planned_hops: u8,
    current_path: CandidatePath,
    last_change_unix: u64,
    held: HopHoldDown,
    policy: HysteresisPolicy,
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
            planned_hops: 0,
            current_path: CandidatePath::EMPTY,
            last_change_unix: 0,
            held: HopHoldDown::EMPTY,
            policy: HysteresisPolicy::STRICT,
            auth: AuthState::OPEN,
            operation: OperationId::ZERO,
            last_stage: DeliveryStage::TransportAck,
        }
    }

    pub fn line() -> Self {
        let mut s = Self::unpublished();
        s.publish_line().expect("plan");
        s
    }

    pub fn line_with_schedule(schedule: crate::net::qdnf::harness::faults::FaultSchedule) -> Self {
        let mut s = Self::line();
        s.middle = FaultPipe::with_schedule(schedule);
        s
    }

    pub fn publish_line(&mut self) -> Result<(), QdnfError> {
        topology::publish_line(self)
    }

    #[rustfmt::skip]
    pub fn lookup_dest(&self) -> Result<u8, QdnfError> {
        self.gens[ORIGIN_SLOT].lookup_next(DEST_NODE)
    }
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
    pub fn planned_hop_count(&self) -> u8 { self.planned_hops }
    #[rustfmt::skip]
    pub fn tick(&mut self) { self.middle.tick(); }

    /// Withdraw the 1→2 next hop and start the hold-down timer.
    pub fn partition_next_hop(&mut self, now_unix: u64) -> Result<(), QdnfError> {
        topology::partition_next_hop(self, now_unix)
    }

    /// Re-advertise 1→2→3. A hop still in hold-down does not rejoin forwarding.
    pub fn heal_next_hop(&mut self, now_unix: u64) -> Result<(), QdnfError> {
        topology::heal_next_hop(self, now_unix)
    }

    #[inline]
    pub fn next_hop_held_down(&self, now_unix: u64) -> bool {
        hop_is_held_down(&self.held, now_unix, &self.policy)
    }

    pub fn handoff(&mut self) -> Result<u8, QdnfError> {
        let next = self.paths.start_race(self.active_path as u64 + 2)?;
        self.paths.mark_active(next)?;
        self.paths.lose_and_cleanup(self.active_path)?;
        self.active_path = next;
        Ok(next)
    }
}

#[cfg(test)]
mod tests;
