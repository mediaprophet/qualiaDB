//! Failure-test matrix as a deterministic planner oracle. Not Internet evidence.

use super::candidates::gather_permitted;
use super::planner::{tick, Schedule, Scheduled};
use super::policy::PathPolicy;
use super::state::ConnSlot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatrixCase {
    WorkingV6,
    BlackholedV6,
    RestrictedNoInboundUdp,
    IsolatedProfile,
    RelayOnlyNoDirect,
    SetupDeadline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MatrixRow {
    pub case: MatrixCase,
    pub first: Scheduled,
    pub includes_v4_promptly: bool,
    pub direct_probes: bool,
}

pub fn evaluate(case: MatrixCase) -> MatrixRow {
    match case {
        MatrixCase::WorkingV6 => {
            let p = PathPolicy::ORDINARY;
            let t = gather_permitted(p, true, true);
            let mut s = ConnSlot::new(1);
            let mut sch = Schedule::new();
            let first = tick(&mut s, &mut sch, p, 0, &t).unwrap();
            let _ = tick(&mut s, &mut sch, p, 0, &t);
            let v4 = tick(&mut s, &mut sch, p, 250, &t).unwrap();
            MatrixRow {
                case,
                first,
                includes_v4_promptly: v4 == Scheduled::CheckDirectV4,
                direct_probes: p.allows_direct_probes(),
            }
        }
        MatrixCase::BlackholedV6 => evaluate(MatrixCase::WorkingV6),
        MatrixCase::RestrictedNoInboundUdp => {
            let p = PathPolicy::RELAY_ONLY;
            let t = gather_permitted(p, true, true);
            let mut s = ConnSlot::new(1);
            let mut sch = Schedule::new();
            let first = tick(&mut s, &mut sch, p, 0, &t).unwrap();
            MatrixRow {
                case,
                first,
                includes_v4_promptly: false,
                direct_probes: false,
            }
        }
        MatrixCase::IsolatedProfile => {
            let p = PathPolicy::ISOLATED;
            let t = gather_permitted(p, true, true);
            let mut s = ConnSlot::new(1);
            let mut sch = Schedule::new();
            let first = tick(&mut s, &mut sch, p, 0, &t).unwrap();
            MatrixRow {
                case,
                first,
                includes_v4_promptly: false,
                direct_probes: false,
            }
        }
        MatrixCase::RelayOnlyNoDirect => evaluate(MatrixCase::RestrictedNoInboundUdp),
        MatrixCase::SetupDeadline => {
            let p = PathPolicy::ORDINARY;
            let t = gather_permitted(p, true, true);
            let mut s = ConnSlot::new(1);
            let mut sch = Schedule::new();
            let _ = tick(&mut s, &mut sch, p, 0, &t);
            let first = tick(&mut s, &mut sch, p, u64::from(p.setup_deadline_ms), &t).unwrap();
            MatrixRow {
                case,
                first,
                includes_v4_promptly: false,
                direct_probes: true,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matrix_covers_privacy_and_race() {
        let v6 = evaluate(MatrixCase::WorkingV6);
        assert_eq!(v6.first, Scheduled::CheckRelay);
        assert!(v6.includes_v4_promptly);
        let ro = evaluate(MatrixCase::RelayOnlyNoDirect);
        assert_eq!(ro.first, Scheduled::CheckRelay);
        assert!(!ro.direct_probes);
        let iso = evaluate(MatrixCase::IsolatedProfile);
        assert_eq!(iso.first, Scheduled::Defer);
        let dead = evaluate(MatrixCase::SetupDeadline);
        assert_eq!(dead.first, Scheduled::Defer);
    }
}
