//! Respiratory rate proxy from vertical motion energy band.

use crate::cv::error::CvError;

/// `vert_motion` per-frame vertical motion scalar. Estimate BPM-like breath rate 6–30 /min.
pub fn respiration_from_motion(vert_motion: &[f32], fps: f32) -> Result<(f32, f32), CvError> {
    if vert_motion.len() < 32 || fps <= 1.0 {
        return Err(CvError::InvalidParameter);
    }
    let mean = vert_motion.iter().sum::<f32>() / vert_motion.len() as f32;
    let mut best_f = 0.2f32;
    let mut best_p = 0.0f32;
    for k in 0..40 {
        let f = 0.1 + 0.4 * (k as f32 / 40.0); // Hz ~6–30 /min
        let mut re = 0.0f32;
        let mut im = 0.0f32;
        for (i, &v) in vert_motion.iter().enumerate() {
            let t = i as f32 / fps;
            let ang = core::f32::consts::TAU * f * t;
            re += (v - mean) * ang.cos();
            im += (v - mean) * ang.sin();
        }
        let p = re * re + im * im;
        if p > best_p {
            best_p = p;
            best_f = f;
        }
    }
    let bpm = best_f * 60.0;
    let conf = (best_p / 1e6).clamp(0.05, 0.9);
    Ok((bpm, conf))
}
