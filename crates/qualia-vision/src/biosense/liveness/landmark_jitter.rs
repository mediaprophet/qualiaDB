//! Landmark noise-floor / micromotion analysis (passive liveness signal).
//!
//! Human pulse and micro-tremors create natural jitter. A statically held 3D
//! mask has near-zero structural jitter; video replays often show overly smooth
//! interpolated trajectories. Track high-frequency residual over ~1 s.

use super::landmark_types::{LandmarkFrame, PadLandmarkId};

/// Default analysis window (ms).
pub const DEFAULT_JITTER_WINDOW_MS: u32 = 1000;
/// Minimum samples inside the window.
pub const MIN_JITTER_SAMPLES: usize = 8;
/// Human-typical floor (normalized by interocular, per-frame residual RMS).
pub const DEFAULT_MIN_JITTER: f32 = 0.00035;
/// Upper bound — too high may indicate tracking noise / injection glitch.
pub const DEFAULT_MAX_JITTER: f32 = 0.08;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum JitterVerdict {
    Natural { rms: f32 },
    TooStatic { rms: f32 },
    TooSmoothOrGlitchy { rms: f32 },
    InsufficientSamples,
}

#[derive(Debug, Clone, Copy)]
pub struct JitterThresholds {
    pub window_ms: u32,
    pub min_rms: f32,
    pub max_rms: f32,
}

impl Default for JitterThresholds {
    fn default() -> Self {
        Self {
            window_ms: DEFAULT_JITTER_WINDOW_MS,
            min_rms: DEFAULT_MIN_JITTER,
            max_rms: DEFAULT_MAX_JITTER,
        }
    }
}

/// High-frequency residual of a landmark track after removing a linear trend.
fn residual_rms_1d(t: &[f32], v: &[f32], n: usize) -> f32 {
    if n < 3 {
        return 0.0;
    }
    let (mean_t, mean_v) = {
        let mut st = 0.0f32;
        let mut sv = 0.0f32;
        for i in 0..n {
            st += t[i];
            sv += v[i];
        }
        (st / n as f32, sv / n as f32)
    };
    let mut num = 0.0f32;
    let mut den = 0.0f32;
    for i in 0..n {
        let dt = t[i] - mean_t;
        num += dt * (v[i] - mean_v);
        den += dt * dt;
    }
    let beta = if den > 1e-12 { num / den } else { 0.0 };
    let alpha = mean_v - beta * mean_t;
    let mut acc = 0.0f32;
    for i in 0..n {
        let e = v[i] - (alpha + beta * t[i]);
        acc += e * e;
    }
    (acc / n as f32).sqrt()
}

fn point_series(
    frames: &[LandmarkFrame],
    id: PadLandmarkId,
    t0: u32,
    window_ms: u32,
    t_out: &mut [f32; 64],
    x_out: &mut [f32; 64],
    y_out: &mut [f32; 64],
) -> usize {
    let mut n = 0usize;
    for f in frames {
        if f.t_ms < t0.saturating_sub(window_ms) {
            continue;
        }
        if f.t_ms > t0 {
            continue;
        }
        let Some(p) = f.get(id) else {
            continue;
        };
        if n >= 64 {
            break;
        }
        t_out[n] = f.t_ms as f32;
        x_out[n] = p.x;
        y_out[n] = p.y;
        n += 1;
    }
    n
}

/// Evaluate jitter on nose + eye corners over the last `window_ms` ending at last frame.
pub fn evaluate_landmark_jitter(
    frames: &[LandmarkFrame],
    thr: &JitterThresholds,
) -> JitterVerdict {
    if frames.len() < MIN_JITTER_SAMPLES {
        return JitterVerdict::InsufficientSamples;
    }
    let t_end = frames.last().map(|f| f.t_ms).unwrap_or(0);
    let mut t_buf = [0.0f32; 64];
    let mut x_buf = [0.0f32; 64];
    let mut y_buf = [0.0f32; 64];

    let ids = [
        PadLandmarkId::NoseTip,
        PadLandmarkId::LeftEyeOuter,
        PadLandmarkId::RightEyeOuter,
    ];
    let mut rms_acc = 0.0f32;
    let mut parts = 0u32;
    let mut iod_ref = 0.0f32;

    for id in ids {
        let n = point_series(
            frames,
            id,
            t_end,
            thr.window_ms,
            &mut t_buf,
            &mut x_buf,
            &mut y_buf,
        );
        if n < MIN_JITTER_SAMPLES {
            continue;
        }
        // Normalize by mean interocular in window when available.
        let mut iod_sum = 0.0f32;
        let mut iod_n = 0u32;
        for f in frames {
            if f.t_ms + thr.window_ms < t_end {
                continue;
            }
            if let Some(iod) = f.interocular() {
                iod_sum += iod;
                iod_n += 1;
            }
        }
        let iod = if iod_n > 0 {
            iod_sum / iod_n as f32
        } else {
            1.0
        };
        iod_ref = iod;
        let rx = residual_rms_1d(&t_buf, &x_buf, n) / iod;
        let ry = residual_rms_1d(&t_buf, &y_buf, n) / iod;
        rms_acc += (rx * rx + ry * ry).sqrt();
        parts += 1;
    }

    if parts == 0 || iod_ref <= 0.0 {
        return JitterVerdict::InsufficientSamples;
    }
    let rms = rms_acc / parts as f32;

    if rms < thr.min_rms {
        JitterVerdict::TooStatic { rms }
    } else if rms > thr.max_rms {
        JitterVerdict::TooSmoothOrGlitchy { rms }
    } else {
        JitterVerdict::Natural { rms }
    }
}

/// Convenience: mean motion of packed points between two frames (legacy mesh_motion).
pub fn mean_landmark_motion(a: &LandmarkFrame, b: &LandmarkFrame) -> f32 {
    let mut acc = 0.0f32;
    let mut n = 0u32;
    for i in 0..PadLandmarkId::COUNT {
        let mask = 1u8 << i;
        if (a.valid_mask & mask) == 0 || (b.valid_mask & mask) == 0 {
            continue;
        }
        acc += a.points[i].dist(b.points[i]);
        n += 1;
    }
    if n == 0 {
        0.0
    } else {
        acc / n as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::biosense::liveness::landmark_types::{Landmark2, LandmarkFrame, PadLandmarkId};

    fn series(noise: f32) -> [LandmarkFrame; 16] {
        let mut out = [LandmarkFrame::empty(0); 16];
        for i in 0..16 {
            let mut f = LandmarkFrame::empty((i as u32) * 60);
            let n = (i as f32) * 0.02 + noise * ((i as f32 * 1.7).sin());
            f.set(PadLandmarkId::NoseTip, Landmark2::new(140.0 + n, 150.0 + n * 0.5));
            f.set(PadLandmarkId::LeftEyeOuter, Landmark2::new(100.0 + n * 0.3, 120.0));
            f.set(
                PadLandmarkId::RightEyeOuter,
                Landmark2::new(180.0 + n * 0.3, 120.0),
            );
            out[i] = f;
        }
        out
    }

    #[test]
    fn static_series_fails() {
        let frames = series(0.0);
        // Pure linear trend only → residual near 0.
        let v = evaluate_landmark_jitter(&frames, &JitterThresholds::default());
        match v {
            JitterVerdict::TooStatic { .. } => {}
            JitterVerdict::Natural { rms } => assert!(rms < 0.01, "rms={}", rms),
            other => panic!("unexpected {:?}", other),
        }
    }

    #[test]
    fn noisy_series_natural_or_elevated() {
        let frames = series(0.8);
        let thr = JitterThresholds {
            min_rms: 0.0001,
            max_rms: 1.0,
            ..Default::default()
        };
        let v = evaluate_landmark_jitter(&frames, &thr);
        assert!(
            matches!(v, JitterVerdict::Natural { .. } | JitterVerdict::TooSmoothOrGlitchy { .. }),
            "got {:?}",
            v
        );
    }
}
