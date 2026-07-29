//! Second-order IIR band-pass on a scalar time series (cold/Tier-2 OK for Vec-free caller buffers).

use crate::cv::error::CvError;

/// One sample of a biquad band-pass (Direct Form I, simplified RBJ-style coefficients).
#[derive(Debug, Clone, Copy)]
pub struct BandpassState {
    pub x1: f32,
    pub x2: f32,
    pub y1: f32,
    pub y2: f32,
    pub b0: f32,
    pub b1: f32,
    pub b2: f32,
    pub a1: f32,
    pub a2: f32,
}

impl BandpassState {
    /// Design band-pass around geometric centre of [f_lo, f_hi] at `fps`.
    pub fn design(fps: f32, f_lo: f32, f_hi: f32) -> Result<Self, CvError> {
        if fps <= 1.0 || f_lo <= 0.0 || f_hi <= f_lo || f_hi >= fps * 0.45 {
            return Err(CvError::InvalidParameter);
        }
        let f0 = (f_lo * f_hi).sqrt();
        let bw = f_hi - f_lo;
        let q = (f0 / bw).clamp(0.3, 10.0);
        let w0 = core::f32::consts::TAU * f0 / fps;
        let alpha = w0.sin() / (2.0 * q);
        let cosw = w0.cos();
        let b0 = alpha;
        let b1 = 0.0;
        let b2 = -alpha;
        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cosw;
        let a2 = 1.0 - alpha;
        Ok(Self {
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
        })
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1
            - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// Filter entire trace into `out` (same length).
pub fn temporal_bandpass_iir(
    input: &[f32],
    fps: f32,
    f_lo: f32,
    f_hi: f32,
    out: &mut [f32],
) -> Result<(), CvError> {
    if input.len() != out.len() || input.len() < 8 {
        return Err(CvError::BufferTooSmall);
    }
    let mut st = BandpassState::design(fps, f_lo, f_hi)?;
    for i in 0..input.len() {
        out[i] = st.process(input[i]);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn passes_in_band_sinusoid() {
        let fps = 30.0f32;
        let n = 256;
        let mut x = vec![0.0f32; n];
        let f = 1.2f32; // in 0.7–4
        for i in 0..n {
            x[i] = (core::f32::consts::TAU * f * (i as f32 / fps)).sin();
        }
        let mut y = vec![0.0f32; n];
        temporal_bandpass_iir(&x, fps, 0.7, 4.0, &mut y).unwrap();
        let power: f32 = y.iter().map(|v| v * v).sum();
        assert!(power > 1.0, "power={}", power);
    }
}
