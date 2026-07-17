//! Recipe: consent → quality → ROI → rPPG ensemble HR.

use crate::biosense::{
    ensemble_hr, face_roi_center, frame_blur_score, reject_low_quality, roi_mean_rgb,
    BiosenseConsent, HrEstimate, QualityReject,
};
use crate::cv::buffer::{GrayView, RgbView};
use crate::cv::color::rgb_to_gray_u8;
use crate::cv::error::CvError;

/// `rgb_frames` consecutive RGB frames packed; each frame_bytes = w*h*3.
pub fn self_monitor_pulse(
    consent: BiosenseConsent,
    rgb_frames: &[u8],
    n_frames: usize,
    width: u32,
    height: u32,
    fps: f32,
) -> Result<HrEstimate, CvError> {
    if !consent.may_process() {
        return Err(CvError::InvalidParameter);
    }
    let fb = (width * height * 3) as usize;
    if n_frames < 32 || rgb_frames.len() < n_frames * fb {
        return Err(CvError::BufferTooSmall);
    }
    // Quality on first frame
    let mut gray = vec![0u8; (width * height) as usize];
    let v0 = RgbView::new(width, height, width * 3, &rgb_frames[..fb]).ok_or(CvError::EmptyInput)?;
    rgb_to_gray_u8(v0, &mut gray)?;
    let gv = GrayView::new(width, height, width, &gray).ok_or(CvError::EmptyInput)?;
    let blur = frame_blur_score(gv);
    if reject_low_quality(blur, 0.0, 10.0, 40.0) == QualityReject::TooBlurry {
        return Err(CvError::InvalidParameter);
    }
    let roi = face_roi_center(v0);
    let mut means = vec![0.0f32; n_frames * 3];
    for i in 0..n_frames {
        let slice = &rgb_frames[i * fb..(i + 1) * fb];
        let v = RgbView::new(width, height, width * 3, slice).ok_or(CvError::EmptyInput)?;
        let mut m = [0.0f32; 3];
        roi_mean_rgb(v, roi, &mut m);
        means[i * 3] = m[0];
        means[i * 3 + 1] = m[1];
        means[i * 3 + 2] = m[2];
    }
    ensemble_hr(consent, &means, n_frames, fps, 0.15)
}
