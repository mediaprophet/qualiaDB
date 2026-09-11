//! Scheduler: race permitted paths. Relay is eligible immediately.

use super::candidates::{gather_permitted, AddrFamily, CandidateKind, CandidateTables};
use super::policy::PathPolicy;
use super::state::{ConnError, ConnSlot, ConnState};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scheduled {
    None,
    CheckRelay,
    CheckDirectV6,
    CheckDirectV4,
    Defer,
    Close,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Schedule {
    pub now_ms: u64,
    pub first_v6_ms: Option<u64>,
    pub relay_started: bool,
    pub v6_started: bool,
    pub v4_started: bool,
    pub selected: Scheduled,
}

impl Schedule {
    pub const fn new() -> Self {
        Self {
            now_ms: 0,
            first_v6_ms: None,
            relay_started: false,
            v6_started: false,
            v4_started: false,
            selected: Scheduled::None,
        }
    }

    pub fn ipv4_eligible(&self, policy: PathPolicy) -> bool {
        match self.first_v6_ms {
            None => true,
            Some(t) => self.now_ms.saturating_sub(t) >= u64::from(policy.ipv4_prompt_ms),
        }
    }

    pub fn setup_expired(&self, policy: PathPolicy) -> bool {
        self.now_ms >= u64::from(policy.setup_deadline_ms)
    }
}

/// Advance the slot and return the next permitted action. Concurrent relay + direct.
pub fn tick(
    slot: &mut ConnSlot,
    schedule: &mut Schedule,
    policy: PathPolicy,
    now_ms: u64,
    tables: &CandidateTables,
) -> Result<Scheduled, ConnError> {
    schedule.now_ms = now_ms;
    if slot.state == ConnState::Closed {
        return Ok(Scheduled::Close);
    }
    if slot.state == ConnState::Admitted {
        if !policy.allows_relay() && !policy.allows_direct_probes() {
            slot.transition(slot.generation, ConnState::Deferred)?;
            schedule.selected = Scheduled::Defer;
            return Ok(Scheduled::Defer);
        }
        slot.transition(slot.generation, ConnState::Establishing)?;
    }
    if schedule.setup_expired(policy) && slot.state == ConnState::Establishing {
        slot.transition(slot.generation, ConnState::Deferred)?;
        schedule.selected = Scheduled::Defer;
        return Ok(Scheduled::Defer);
    }
    if policy.allows_relay() && policy.relay_immediate && !schedule.relay_started {
        schedule.relay_started = true;
        schedule.selected = Scheduled::CheckRelay;
        return Ok(Scheduled::CheckRelay);
    }
    let has_v6_host = has_kind(tables, CandidateKind::Host, AddrFamily::V6);
    let has_v4_host = has_kind(tables, CandidateKind::Host, AddrFamily::V4);
    if policy.allows_direct_probes() && has_v6_host && !schedule.v6_started {
        schedule.v6_started = true;
        if schedule.first_v6_ms.is_none() {
            schedule.first_v6_ms = Some(now_ms);
        }
        schedule.selected = Scheduled::CheckDirectV6;
        return Ok(Scheduled::CheckDirectV6);
    }
    if policy.allows_direct_probes()
        && has_v4_host
        && !schedule.v4_started
        && schedule.ipv4_eligible(policy)
    {
        schedule.v4_started = true;
        schedule.selected = Scheduled::CheckDirectV4;
        return Ok(Scheduled::CheckDirectV4);
    }
    Ok(schedule.selected)
}

fn has_kind(tables: &CandidateTables, kind: CandidateKind, family: AddrFamily) -> bool {
    let mut i = 0;
    while i < tables.local_len {
        if let Some(c) = tables.local[i] {
            if c.kind == kind && c.family == family {
                return true;
            }
        }
        i += 1;
    }
    false
}

/// Ordinary dual-stack gather + first ticks prove relay is not delayed by IPv6.
pub fn ordinary_first_actions() -> [Scheduled; 3] {
    let policy = PathPolicy::ORDINARY;
    let tables = gather_permitted(policy, true, true);
    let mut slot = ConnSlot::new(1);
    let mut sch = Schedule::new();
    let a0 = tick(&mut slot, &mut sch, policy, 0, &tables).unwrap();
    let a1 = tick(&mut slot, &mut sch, policy, 0, &tables).unwrap();
    let a2 = tick(&mut slot, &mut sch, policy, 250, &tables).unwrap();
    [a0, a1, a2]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_then_v6_then_v4() {
        assert_eq!(
            ordinary_first_actions(),
            [
                Scheduled::CheckRelay,
                Scheduled::CheckDirectV6,
                Scheduled::CheckDirectV4
            ]
        );
    }

    #[test]
    fn blackholed_v6_does_not_block_v4() {
        let policy = PathPolicy::ORDINARY;
        let tables = gather_permitted(policy, true, true);
        let mut slot = ConnSlot::new(1);
        let mut sch = Schedule::new();
        assert_eq!(
            tick(&mut slot, &mut sch, policy, 0, &tables).unwrap(),
            Scheduled::CheckRelay
        );
        assert_eq!(
            tick(&mut slot, &mut sch, policy, 0, &tables).unwrap(),
            Scheduled::CheckDirectV6
        );
        assert!(
            !sch.ipv4_eligible(policy),
            "IPv4 waits the prompt, not a family-wide timeout"
        );
        assert_eq!(
            tick(&mut slot, &mut sch, policy, 250, &tables).unwrap(),
            Scheduled::CheckDirectV4
        );
    }

    #[test]
    fn isolated_defers() {
        let policy = PathPolicy::ISOLATED;
        let tables = gather_permitted(policy, true, true);
        let mut slot = ConnSlot::new(1);
        let mut sch = Schedule::new();
        assert_eq!(
            tick(&mut slot, &mut sch, policy, 0, &tables).unwrap(),
            Scheduled::Defer
        );
        assert_eq!(slot.state, ConnState::Deferred);
    }
}
