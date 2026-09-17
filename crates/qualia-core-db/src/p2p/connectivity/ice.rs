//! Bounded ICE checklist. Procedure for UDP/TURN, not a second transport.

#![cfg(not(target_arch = "wasm32"))]

use crate::net::peer::connectivity::candidates::{
    gather_permitted, AddrFamily, CandidateKind, CandidateTables, PairState,
};
use crate::net::peer::connectivity::path::{consent_fresh, resolve_role_conflict};
use crate::net::peer::connectivity::planner::{tick, Schedule, Scheduled};
use crate::net::peer::connectivity::policy::PathPolicy;
use crate::net::peer::connectivity::state::ConnSlot;
use crate::p2p::stun_observe::{encode_binding_success_v4, parse_xor_mapped_v4};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IceRole {
    Controlling,
    Controlled,
}

pub struct IceAgent {
    pub role: IceRole,
    pub policy: PathPolicy,
    pub tables: CandidateTables,
    pub slot: ConnSlot,
    pub schedule: Schedule,
}

impl IceAgent {
    pub fn new(role: IceRole, policy: PathPolicy, has_v6: bool, has_v4: bool) -> Self {
        let mut tables = gather_permitted(policy, has_v6, has_v4);
        let mut i = 0;
        while i < tables.local_len {
            if let Some(c) = tables.local[i] {
                let _ = tables.push_remote(c);
            }
            i += 1;
        }
        tables.form_pairs(matches!(role, IceRole::Controlling));
        Self {
            role,
            policy,
            tables,
            slot: ConnSlot::new(1),
            schedule: Schedule::new(),
        }
    }

    pub fn next(&mut self, now_ms: u64) -> Scheduled {
        tick(
            &mut self.slot,
            &mut self.schedule,
            self.policy,
            now_ms,
            &self.tables,
        )
        .unwrap_or(Scheduled::Close)
    }

    /// Record that a real STUN/TURN transaction succeeded for `kind`.
    pub fn mark_checked(&mut self, kind: CandidateKind) -> bool {
        let mut i = 0;
        while i < self.tables.pair_len {
            if let Some(mut p) = self.tables.pairs[i] {
                if let Some(l) = self.tables.local[p.local_idx as usize] {
                    if l.kind == kind && p.state == PairState::Waiting {
                        p.checked = true;
                        self.tables.pairs[i] = Some(p);
                        return true;
                    }
                }
            }
            i += 1;
        }
        false
    }

    /// Nominate only a pair that already passed a connectivity check.
    pub fn nominate(&mut self, kind: CandidateKind, now_ms: u64) -> bool {
        if kind != CandidateKind::Relayed && !self.policy.allows_direct_probes() {
            return false;
        }
        if kind == CandidateKind::Host
            && matches!(self.family_of_first_host(), Some(AddrFamily::V4))
            && !self.schedule.ipv4_eligible(self.policy)
        {
            let _ = now_ms;
            return false;
        }
        let mut i = 0;
        while i < self.tables.pair_len {
            if let Some(mut p) = self.tables.pairs[i] {
                if let Some(l) = self.tables.local[p.local_idx as usize] {
                    if l.kind == kind && p.state == PairState::Waiting && p.checked {
                        p.state = PairState::Succeeded;
                        self.tables.pairs[i] = Some(p);
                        return true;
                    }
                }
            }
            i += 1;
        }
        false
    }

    fn family_of_first_host(&self) -> Option<AddrFamily> {
        let mut i = 0;
        while i < self.tables.local_len {
            if let Some(c) = self.tables.local[i] {
                if c.kind == CandidateKind::Host {
                    return Some(c.family);
                }
            }
            i += 1;
        }
        None
    }

    pub fn has_succeeded(&self) -> bool {
        let mut i = 0;
        while i < self.tables.pair_len {
            if self.tables.pairs[i].is_some_and(|p| p.state == PairState::Succeeded) {
                return true;
            }
            i += 1;
        }
        false
    }

    /// RFC 8445 role conflict: larger tie-breaker stays controlling.
    pub fn apply_role_conflict(&mut self, local_tie: u64, remote_tie: u64) {
        self.role = if resolve_role_conflict(local_tie, remote_tie) {
            IceRole::Controlling
        } else {
            IceRole::Controlled
        };
    }

    /// RFC 7675: a nominated pair without fresh consent cannot keep sending.
    pub fn nominated_may_send(&self, last_ok_ms: u64, now_ms: u64) -> bool {
        self.has_succeeded() && consent_fresh(last_ok_ms, now_ms)
    }
}

/// STUN Binding success still parses after a same-socket observe (fixture).
pub fn stun_check_roundtrip() -> bool {
    let tid = [2u8; 12];
    let mapped = "203.0.113.9:3478".parse().unwrap();
    let msg = encode_binding_success_v4(&tid, mapped).unwrap();
    parse_xor_mapped_v4(&msg, &tid).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn relay_only_cannot_nominate_host() {
        let mut a = IceAgent::new(IceRole::Controlling, PathPolicy::RELAY_ONLY, true, true);
        assert_eq!(a.next(0), Scheduled::CheckRelay);
        assert!(!a.nominate(CandidateKind::Host, 0));
        assert!(
            !a.nominate(CandidateKind::Relayed, 0),
            "nominate without a check must fail"
        );
        assert!(a.mark_checked(CandidateKind::Relayed));
        assert!(a.nominate(CandidateKind::Relayed, 0));
        assert!(a.has_succeeded());
        assert!(stun_check_roundtrip());
    }

    #[test]
    fn ordinary_races_relay_before_direct() {
        let mut a = IceAgent::new(IceRole::Controlling, PathPolicy::ORDINARY, true, true);
        assert_eq!(a.next(0), Scheduled::CheckRelay);
        assert_eq!(a.next(0), Scheduled::CheckDirectV6);
    }

    #[test]
    fn role_conflict_and_consent_freshness() {
        let mut a = IceAgent::new(IceRole::Controlled, PathPolicy::ORDINARY, true, true);
        a.apply_role_conflict(10, 1);
        assert_eq!(a.role, IceRole::Controlling);
        a.apply_role_conflict(1, 10);
        assert_eq!(a.role, IceRole::Controlled);
        assert!(a.mark_checked(CandidateKind::Relayed));
        assert!(a.nominate(CandidateKind::Relayed, 0));
        assert!(a.nominated_may_send(0, 4_999));
        assert!(!a.nominated_may_send(0, 30_000));
    }
}
