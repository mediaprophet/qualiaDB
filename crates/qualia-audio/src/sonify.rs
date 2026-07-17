//! U3-style hear path: events → parametric mono PCM (navigate by time).

use crate::types::AuditoryEvent;

/// Map class hash low bits to a hearable frequency (deterministic, not semantic truth).
pub fn class_to_hz(class_hash: u64) -> f32 {
    let bucket = (class_hash % 12) as f32;
    220.0 * 2.0_f32.powf(bucket / 12.0)
}

/// Render events into mono f32 of `total_frames` at `sample_rate`.
/// Overlapping events sum with soft clip.
pub fn sonify_events_mono(
    events: &[AuditoryEvent],
    sample_rate: u32,
    total_frames: usize,
    out: &mut [f32],
) -> usize {
    let n = total_frames.min(out.len());
    out[..n].fill(0.0);
    if sample_rate == 0 || n == 0 {
        return 0;
    }
    for e in events {
        if e.class_hash == 0 && e.confidence_u16 == 0 {
            continue;
        }
        let f0 = class_to_hz(e.class_hash);
        let gain = e.confidence_f32() * 0.25;
        let start = e.start_frame.min(n as u64) as usize;
        let end = e.end_frame.min(n as u64) as usize;
        if end <= start {
            continue;
        }
        for i in start..end {
            let t = (i - start) as f32 / sample_rate as f32;
            // Simple envelope
            let env = if i - start < 64 {
                (i - start) as f32 / 64.0
            } else if end - i < 64 {
                (end - i) as f32 / 64.0
            } else {
                1.0
            };
            out[i] += (core::f32::consts::TAU * f0 * t).sin() * gain * env;
        }
    }
    for s in out.iter_mut().take(n) {
        *s = s.clamp(-1.0, 1.0);
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::AuditoryEvent;

    #[test]
    fn sonify_nonzero() {
        let mut e = AuditoryEvent::empty();
        e.class_hash = 0xABC;
        e.confidence_u16 = 40_000;
        e.start_frame = 0;
        e.end_frame = 1000;
        let mut out = [0.0f32; 2000];
        let n = sonify_events_mono(&[e], 16000, 2000, &mut out);
        assert_eq!(n, 2000);
        assert!(out.iter().any(|&x| x.abs() > 0.01));
    }
}
