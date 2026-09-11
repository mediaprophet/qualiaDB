//! Exclude prohibited paths, then rank remaining evidence. Never the reverse.

use super::carrier::{prohibited, PathClass};
use super::evidence::PathEvidence;
use crate::net::peer::connectivity::policy::Disclosure;

pub const MAX_RANKED: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RankedPath {
    pub index: u8,
    pub class: PathClass,
    pub score: i32,
}

/// Lower RTT and larger validated payload score higher. Unvalidated rows are skipped.
pub fn exclude_then_rank(
    disclosure: Disclosure,
    live_generation: u32,
    rows: &[PathEvidence],
    out: &mut [RankedPath],
) -> usize {
    let mut n = 0usize;
    let mut i = 0usize;
    while i < rows.len() {
        let e = rows[i];
        if prohibited(disclosure, e.class)
            || !e.validated()
            || e.stale_generation(live_generation)
            || n >= out.len()
            || n >= MAX_RANKED
        {
            i += 1;
            continue;
        }
        let rtt = if e.rtt_ms > 60_000 {
            60_000
        } else {
            e.rtt_ms
        };
        let score = (i32::from(e.max_payload) / 8) - (rtt as i32);
        out[n] = RankedPath {
            index: i as u8,
            class: e.class,
            score,
        };
        n += 1;
        i += 1;
    }
    let mut a = 0usize;
    while a + 1 < n {
        let mut b = a + 1;
        while b < n {
            if out[b].score > out[a].score {
                let t = out[a];
                out[a] = out[b];
                out[b] = t;
            }
            b += 1;
        }
        a += 1;
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::fabric::evidence::TransportWitness;

    fn local(class: PathClass, rtt: u32) -> PathEvidence {
        PathEvidence::from_witness(TransportWitness::from_local(class, 1, 1152, rtt, 10))
    }

    #[test]
    fn faster_direct_cannot_beat_relay_only_policy() {
        let rows = [
            local(PathClass::DirectV6, 1),
            local(PathClass::Relayed, 50),
        ];
        let mut out = [RankedPath {
            index: 0,
            class: PathClass::Offline,
            score: 0,
        }; MAX_RANKED];
        let n = exclude_then_rank(Disclosure::ApprovedRelaysOnly, 1, &rows, &mut out);
        assert_eq!(n, 1);
        assert_eq!(out[0].class, PathClass::Relayed);
    }

    #[test]
    fn ordinary_prefers_faster_validated_direct() {
        let rows = [
            local(PathClass::Relayed, 50),
            local(PathClass::DirectV6, 1),
        ];
        let mut out = [RankedPath {
            index: 0,
            class: PathClass::Offline,
            score: 0,
        }; MAX_RANKED];
        let n = exclude_then_rank(Disclosure::DirectPermitted, 1, &rows, &mut out);
        assert_eq!(n, 2);
        assert_eq!(out[0].class, PathClass::DirectV6);
    }
}
