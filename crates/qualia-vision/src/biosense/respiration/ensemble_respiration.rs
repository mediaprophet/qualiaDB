//! Ensemble respiratory rate from motion ± rPPG-harmonic estimates.
//!
//! Honest confidence: agreement-weighted blend; large disagreement or dual
//! abstain → fail closed. No training.

use super::rr_estimate::RrEstimate;
use crate::cv::error::CvError;

/// Max |Δ bpm| for full agreement weight; beyond this, confidence collapses.
const AGREE_BPM_FULL: f32 = 2.0;
/// Beyond this absolute disagreement, ensemble abstains even if both confident.
const AGREE_BPM_MAX: f32 = 6.0;

/// Combine optional motion and rPPG-harmonic RR estimates.
///
/// * At least one `Some` required; both preferred.
/// * `min_confidence` — fail closed if fused confidence is below this (e.g. 0.15).
pub fn ensemble_respiration(
    motion: Option<RrEstimate>,
    rppg_harmonic: Option<RrEstimate>,
    min_confidence: f32,
) -> Result<RrEstimate, CvError> {
    match (motion, rppg_harmonic) {
        (None, None) => Err(CvError::InvalidParameter),
        (Some(m), None) => gate_single(m, min_confidence, 0.85),
        (None, Some(h)) => gate_single(h, min_confidence, 0.80),
        (Some(m), Some(h)) => fuse_pair(m, h, min_confidence),
    }
}

fn gate_single(e: RrEstimate, min_confidence: f32, scale: f32) -> Result<RrEstimate, CvError> {
    let conf = (e.confidence * scale).clamp(0.0, 1.0);
    if conf < min_confidence {
        return Err(CvError::InvalidParameter);
    }
    Ok(RrEstimate {
        breaths_per_min: e.breaths_per_min,
        snr: e.snr,
        confidence: conf,
    })
}

fn fuse_pair(m: RrEstimate, h: RrEstimate, min_confidence: f32) -> Result<RrEstimate, CvError> {
    let delta = (m.breaths_per_min - h.breaths_per_min).abs();
    if delta > AGREE_BPM_MAX {
        return Err(CvError::InvalidParameter);
    }
    let wm = m.confidence.max(1e-3);
    let wh = h.confidence.max(1e-3);
    let bpm = (m.breaths_per_min * wm + h.breaths_per_min * wh) / (wm + wh);
    let snr = (m.snr * wm + h.snr * wh) / (wm + wh);
    // Agreement factor: 1 at ≤AGREE_BPM_FULL, linear down to 0 at AGREE_BPM_MAX.
    let agree = if delta <= AGREE_BPM_FULL {
        1.0
    } else {
        1.0 - (delta - AGREE_BPM_FULL) / (AGREE_BPM_MAX - AGREE_BPM_FULL)
    };
    let base = ((wm + wh) * 0.5).clamp(0.0, 1.0);
    // Dual source bonus when they agree.
    let conf = (base * agree * 1.05).clamp(0.0, 1.0);
    if conf < min_confidence {
        return Err(CvError::InvalidParameter);
    }
    Ok(RrEstimate {
        breaths_per_min: bpm,
        snr,
        confidence: conf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn est(bpm: f32, snr: f32, conf: f32) -> RrEstimate {
        RrEstimate {
            breaths_per_min: bpm,
            snr,
            confidence: conf,
        }
    }

    #[test]
    fn fuses_agreeing_estimates() {
        let e = ensemble_respiration(Some(est(15.0, 12.0, 0.7)), Some(est(15.5, 10.0, 0.6)), 0.15)
            .unwrap();
        assert!(
            (e.breaths_per_min - 15.25).abs() < 0.5,
            "bpm={}",
            e.breaths_per_min
        );
        assert!(e.confidence >= 0.15);
    }

    #[test]
    fn abstains_on_large_disagreement() {
        let r = ensemble_respiration(Some(est(12.0, 10.0, 0.8)), Some(est(24.0, 10.0, 0.8)), 0.1);
        assert!(r.is_err());
    }

    #[test]
    fn single_source_scaled() {
        let e = ensemble_respiration(Some(est(14.0, 8.0, 0.5)), None, 0.2).unwrap();
        assert!((e.breaths_per_min - 14.0).abs() < 0.01);
        assert!(e.confidence < 0.5); // scaled down vs dual
    }

    #[test]
    fn both_none_fails() {
        assert!(ensemble_respiration(None, None, 0.1).is_err());
    }

    #[test]
    fn low_confidence_fails_closed() {
        let r = ensemble_respiration(Some(est(14.0, 2.0, 0.05)), None, 0.2);
        assert!(r.is_err());
    }
}
