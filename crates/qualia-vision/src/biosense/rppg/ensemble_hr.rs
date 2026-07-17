//! Ensemble POS+CHROM HR with confidence gate.

use super::chrom_rppg_trace::chrom_rppg_trace;
use super::pos_rppg_trace::pos_rppg_trace;
use super::spectral_hr_peak::{spectral_hr_peak, HrEstimate};
use crate::biosense::consent::BiosenseConsent;
use crate::cv::error::CvError;

pub fn ensemble_hr(
    consent: BiosenseConsent,
    rgb_means: &[f32],
    n_frames: usize,
    fps: f32,
    min_confidence: f32,
) -> Result<HrEstimate, CvError> {
    if !consent.may_process() {
        return Err(CvError::InvalidParameter);
    }
    let mut a = vec![0.0f32; n_frames];
    let mut b = vec![0.0f32; n_frames];
    pos_rppg_trace(rgb_means, n_frames, &mut a)?;
    chrom_rppg_trace(rgb_means, n_frames, &mut b)?;
    for i in 0..n_frames {
        a[i] = 0.5 * (a[i] + b[i]);
    }
    let est = spectral_hr_peak(&a, fps)?;
    if est.confidence < min_confidence {
        return Err(CvError::InvalidParameter); // fail closed: insufficient SNR
    }
    Ok(est)
}
