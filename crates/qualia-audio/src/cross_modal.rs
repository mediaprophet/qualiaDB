//! Swarm X — eyes + ears correlation (overlap ≠ causality).

use crate::hash::q_hash;

/// Proposed co-occurrence of a visual instance and auditory instance on shared media time.
#[derive(Debug, Clone, Copy)]
pub struct AvCorrelationProposal {
    pub media_hash: u64,
    pub visual_instance: u64,
    pub auditory_instance: u64,
    /// Overlap start/end in media_time_ms.
    pub overlap_start_ms: u64,
    pub overlap_end_ms: u64,
    pub confidence_u16: u16,
    /// Always false for pure temporal join — causal claims need separate evidence.
    pub asserts_causality: bool,
}

/// Interval on media timeline (ms).
#[derive(Debug, Clone, Copy)]
pub struct TimeIntervalMs {
    pub start_ms: u64,
    pub end_ms: u64,
    pub instance: u64,
}

fn overlap(a: TimeIntervalMs, b: TimeIntervalMs) -> Option<(u64, u64)> {
    let s = a.start_ms.max(b.start_ms);
    let e = a.end_ms.min(b.end_ms);
    if s < e {
        Some((s, e))
    } else {
        None
    }
}

/// Propose correlations where visual and auditory intervals overlap.
/// `asserts_causality` is **always false** here.
pub fn propose_temporal_correlations(
    media_hash: u64,
    visual: &[TimeIntervalMs],
    auditory: &[TimeIntervalMs],
    out: &mut [AvCorrelationProposal],
) -> usize {
    let mut w = 0usize;
    for v in visual {
        for a in auditory {
            if let Some((s, e)) = overlap(*v, *a) {
                if w >= out.len() {
                    return w;
                }
                let dur = e - s;
                let conf = ((dur.min(5000) as f32 / 5000.0) * 40000.0) as u16;
                out[w] = AvCorrelationProposal {
                    media_hash,
                    visual_instance: v.instance,
                    auditory_instance: a.instance,
                    overlap_start_ms: s,
                    overlap_end_ms: e,
                    confidence_u16: conf,
                    asserts_causality: false,
                };
                w += 1;
            }
        }
    }
    w
}

/// Convert audio frames to media_time_ms given sample rate and media time origin.
pub fn frames_to_media_ms(frame: u64, sample_rate: u32, origin_ms: u64) -> u64 {
    if sample_rate == 0 {
        return origin_ms;
    }
    origin_ms + (frame * 1000) / sample_rate as u64
}

pub fn correlation_predicate() -> u64 {
    q_hash("https://ns.webizen.org/q42/proposesAvCooccurrence")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlap_not_causal() {
        let v = [TimeIntervalMs {
            start_ms: 0,
            end_ms: 1000,
            instance: 1,
        }];
        let a = [TimeIntervalMs {
            start_ms: 500,
            end_ms: 1500,
            instance: 2,
        }];
        let mut out = [AvCorrelationProposal {
            media_hash: 0,
            visual_instance: 0,
            auditory_instance: 0,
            overlap_start_ms: 0,
            overlap_end_ms: 0,
            confidence_u16: 0,
            asserts_causality: true,
        }; 4];
        let n = propose_temporal_correlations(9, &v, &a, &mut out);
        assert_eq!(n, 1);
        assert!(!out[0].asserts_causality);
        assert_eq!(out[0].overlap_start_ms, 500);
        assert_eq!(out[0].overlap_end_ms, 1000);
    }
}
