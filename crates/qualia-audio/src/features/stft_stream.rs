//! Bounded streaming STFT (real radix-2 FFT via `features::fft`; power-of-two frames ≤ 512).

use crate::types::AudioError;

/// Streaming hop STFT with fixed window.
#[derive(Debug, Clone)]
pub struct StreamingStft {
    pub frame_size: usize,
    pub hop: usize,
    window: Vec<f32>,
    ring: Vec<f32>,
    ring_len: usize,
    /// Interleaved `[re, im]` FFT scratch (len `2 * frame_size`), reused per frame — no hot-path alloc.
    scratch: Vec<f32>,
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
            scratch: vec![0.0f32; frame_size * 2],
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
                let dest = &mut out_mags[written * bins..(written + 1) * bins];
                fft_magnitude(
                    &self.ring[..self.frame_size],
                    &self.window,
                    &mut self.scratch,
                    dest,
                );
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

/// Log-mel spectrogram using the REAL triangular mel filterbank (`features::mel`).
///
/// Per frame: real-FFT magnitudes → power → triangular mel bank (0..Nyquist) → natural log.
/// This replaced the earlier linear-FFT-bin averaging that was mislabelled "log-mel".
/// Batch convenience path — allocates working buffers once (the zero-heap streaming path is
/// `StreamingStft` + `features::mel::mel_bands` applied per frame).
pub fn log_mel_from_mono(
    samples: &[f32],
    frame_size: usize,
    hop: usize,
    sample_rate: u32,
    n_mel: usize,
    out: &mut [f32],
) -> Result<usize, AudioError> {
    let n_bins = frame_size / 2 + 1;
    let max_frames = out.len() / n_mel.max(1);
    let mut mags = vec![0.0f32; max_frames * n_bins];
    let n_frames = magnitude_stft_chunk(samples, frame_size, hop, &mut mags, n_bins)?;
    let n_frames = n_frames.min(max_frames);

    // Real triangular mel filterbank spanning 0..Nyquist, built once.
    let mut bank = vec![0.0f32; n_mel * n_bins];
    crate::features::mel::build_mel_bank(
        n_bins,
        n_mel,
        sample_rate as f32,
        0.0,
        sample_rate as f32 * 0.5,
        &mut bank,
    )?;
    let mut power = vec![0.0f32; n_bins];
    for f in 0..n_frames {
        for b in 0..n_bins {
            let m = mags[f * n_bins + b];
            power[b] = m * m;
        }
        let dest = &mut out[f * n_mel..(f + 1) * n_mel];
        crate::features::mel::mel_bands(&power, &bank, n_mel, dest)?;
        for v in dest.iter_mut() {
            *v = (*v + 1e-6).ln();
        }
    }
    Ok(n_frames)
}

/// Windowed magnitude spectrum via the real radix-2 FFT. `scratch` is interleaved `[re, im]`
/// of length `2 * frame.len()` and is reused across frames (no allocation here).
fn fft_magnitude(frame: &[f32], window: &[f32], scratch: &mut [f32], out: &mut [f32]) {
    let n = frame.len();
    // Pack the windowed real frame into an interleaved complex buffer.
    for t in 0..n {
        scratch[2 * t] = frame[t] * window[t];
        scratch[2 * t + 1] = 0.0;
    }
    // n is a validated power of two (StreamingStft::new), so this cannot fail; fail safe anyway.
    if crate::features::fft::fft_radix2(&mut scratch[..2 * n], false).is_err() {
        out.iter_mut().for_each(|o| *o = 0.0);
        return;
    }
    let bins = out.len().min(n / 2 + 1);
    for k in 0..bins {
        let re = scratch[2 * k];
        let im = scratch[2 * k + 1];
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
