//! Bounded streaming STFT (CPU DFT floor; power-of-two frames ≤ 512).

use crate::types::AudioError;

/// Streaming hop STFT with fixed window.
#[derive(Debug, Clone)]
pub struct StreamingStft {
    pub frame_size: usize,
    pub hop: usize,
    window: Vec<f32>,
    ring: Vec<f32>,
    ring_len: usize,
}

impl StreamingStft {
    pub fn new(frame_size: usize, hop: usize) -> Result<Self, AudioError> {
        if !frame_size.is_power_of_two() || !(4..=512).contains(&frame_size) || hop == 0 {
            return Err(AudioError::MalformedAudio);
        }
        let mut window = vec![0.0f32; frame_size];
        for i in 0..frame_size {
            window[i] = 0.5
                * (1.0
                    - (core::f32::consts::TAU * i as f32 / (frame_size - 1).max(1) as f32).cos());
        }
        Ok(Self {
            frame_size,
            hop,
            window,
            ring: vec![0.0f32; frame_size * 2],
            ring_len: 0,
        })
    }

    /// Push mono samples; for each completed frame write `n_bins` magnitudes into `out_mags`
    /// (layout: frame-major, `n_bins` = frame_size/2+1). Returns frames written.
    pub fn push(
        &mut self,
        samples: &[f32],
        out_mags: &mut [f32],
        n_bins: usize,
    ) -> Result<usize, AudioError> {
        let bins = (self.frame_size / 2 + 1).min(n_bins);
        let max_frames = out_mags.len() / bins.max(1);
        let mut written = 0usize;
        for &s in samples {
            if self.ring_len < self.ring.len() {
                self.ring[self.ring_len] = s;
                self.ring_len += 1;
            } else {
                // shift by hop
                self.ring.copy_within(self.hop.., 0);
                self.ring_len = self.frame_size;
                self.ring[self.ring_len - 1] = s;
            }
            while self.ring_len >= self.frame_size && written < max_frames {
                let frame = &self.ring[..self.frame_size];
                let dest = &mut out_mags[written * bins..(written + 1) * bins];
                dft_magnitude(frame, &self.window, dest);
                // consume hop
                if self.hop >= self.ring_len {
                    self.ring_len = 0;
                } else {
                    self.ring.copy_within(self.hop..self.ring_len, 0);
                    self.ring_len -= self.hop;
                }
                written += 1;
            }
        }
        Ok(written)
    }

    pub fn reset(&mut self) {
        self.ring_len = 0;
        self.ring.fill(0.0);
    }
}

/// One-shot magnitude STFT for a mono buffer (same DFT).
pub fn magnitude_stft_chunk(
    samples: &[f32],
    frame_size: usize,
    hop: usize,
    out_mags: &mut [f32],
    n_bins: usize,
) -> Result<usize, AudioError> {
    let mut st = StreamingStft::new(frame_size, hop)?;
    st.push(samples, out_mags, n_bins)
}

/// Log-mel style filterbank: average magnitude bands into `n_mel` bins.
pub fn log_mel_from_mono(
    samples: &[f32],
    frame_size: usize,
    hop: usize,
    n_mel: usize,
    out: &mut [f32],
) -> Result<usize, AudioError> {
    let n_bins = frame_size / 2 + 1;
    let max_frames = out.len() / n_mel.max(1);
    let mut mags = vec![0.0f32; max_frames * n_bins];
    let n_frames = magnitude_stft_chunk(samples, frame_size, hop, &mut mags, n_bins)?;
    let n_frames = n_frames.min(max_frames);
    for f in 0..n_frames {
        for m in 0..n_mel {
            let b0 = m * n_bins / n_mel;
            let b1 = ((m + 1) * n_bins / n_mel).max(b0 + 1);
            let mut s = 0.0f32;
            for b in b0..b1.min(n_bins) {
                s += mags[f * n_bins + b];
            }
            let avg = s / (b1 - b0) as f32;
            out[f * n_mel + m] = (avg + 1e-6).ln();
        }
    }
    Ok(n_frames)
}

fn dft_magnitude(frame: &[f32], window: &[f32], out: &mut [f32]) {
    let n = frame.len();
    let bins = out.len().min(n / 2 + 1);
    for k in 0..bins {
        let mut re = 0.0f32;
        let mut im = 0.0f32;
        for t in 0..n {
            let x = frame[t] * window[t];
            let ang = -core::f32::consts::TAU * (k as f32) * (t as f32) / n as f32;
            re += x * ang.cos();
            im += x * ang.sin();
        }
        out[k] = (re * re + im * im).sqrt() / n as f32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sine_has_energy() {
        let sr = 256usize;
        let mut s = vec![0.0f32; sr];
        for i in 0..sr {
            s[i] = (core::f32::consts::TAU * 4.0 * i as f32 / sr as f32).sin();
        }
        let mut mags = vec![0.0f32; 8 * 65];
        let n = magnitude_stft_chunk(&s, 128, 64, &mut mags, 65).unwrap();
        assert!(n >= 1);
        let peak = mags[..65].iter().cloned().fold(0.0f32, f32::max);
        assert!(peak > 0.01);
    }
}
