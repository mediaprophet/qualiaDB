//! Peak frequency in BPM band via naive DFT power.

use crate::cv::error::CvError;

#[derive(Debug, Clone, Copy)]
pub struct HrEstimate {
    pub bpm: f32,
    pub snr: f32,
    pub confidence: f32,
}

/// Estimate HR from pulse trace. `fps` sample rate. Band 40–180 BPM.
pub fn spectral_hr_peak(trace: &[f32], fps: f32) -> Result<HrEstimate, CvError> {
    let n = trace.len();
    if n < 16 || fps <= 1.0 {
        return Err(CvError::InvalidParameter);
    }
    // Demean
    let mean = trace.iter().sum::<f32>() / n as f32;
    let mut best_f = 1.0f32;
    let mut best_p = 0.0f32;
    let mut total = 0.0f32;
    let f_lo = 40.0 / 60.0;
    let f_hi = 180.0 / 60.0;
    let steps = 64usize;
    for k in 0..steps {
        let f = f_lo + (f_hi - f_lo) * (k as f32 / steps as f32);
        let mut re = 0.0f32;
        let mut im = 0.0f32;
        for (i, &v) in trace.iter().enumerate() {
            let t = i as f32 / fps;
            let ang = core::f32::consts::TAU * f * t;
            re += (v - mean) * ang.cos();
            im += (v - mean) * ang.sin();
        }
        let p = re * re + im * im;
        total += p;
        if p > best_p {
            best_p = p;
            best_f = f;
        }
    }
    let snr = if total > 1e-6 {
        best_p / (total / steps as f32).max(1e-6)
    } else {
        0.0
    };
    let conf = (snr / 20.0).clamp(0.0, 1.0);
    Ok(HrEstimate {
        bpm: best_f * 60.0,
        snr,
        confidence: conf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finds_60bpm() {
        let fps = 30.0f32;
        let n = 150;
        let mut t = vec![0.0f32; n];
        for i in 0..n {
            t[i] = (core::f32::consts::TAU * 1.0 * i as f32 / fps).sin();
        }
        let e = spectral_hr_peak(&t, fps).unwrap();
        assert!((e.bpm - 60.0).abs() < 8.0, "bpm={}", e.bpm);
    }
}
