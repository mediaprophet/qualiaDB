//! Inverse STFT: resynthesise a time-domain signal from per-frame complex spectra
//! via inverse FFT + weighted overlap-add (WOLA).
//!
//! Reuses [`crate::features::fft::ifft_to_real`] for the per-frame inverse.
//! Reconstruction is normalised by the running sum of the analysis·synthesis
//! window product, so the output is exact wherever frames overlap (the NOLA
//! condition) regardless of whether the window+hop strictly satisfies COLA — the
//! standard robust ISTFT. Zero-heap hot path: every buffer is caller-owned; the
//! spectra are inverse-transformed in place.

use crate::features::fft::ifft_to_real;
use crate::types::AudioError;

/// Inverse STFT of a run of complex spectra into a single time-domain signal.
///
/// - `spectra`: `num_frames × (2·fft_size)` interleaved-complex bins
///   (`[re, im, …]`), frame `i` at `spectra[i*2N .. (i+1)*2N]`. **Clobbered** in
///   place (each frame is inverse-transformed). `num_frames` is inferred as
///   `spectra.len() / (2·fft_size)`.
/// - `fft_size`: `N`, a power of two; equals the analysis frame size.
/// - `hop`: sample advance between frames (`> 0`).
/// - `analysis_window` / `synthesis_window`: the length-`N` windows used in
///   analysis and applied in synthesis. Their product forms the normalisation.
/// - `ifft_scratch`: `≥ N` floats; per-frame real inverse output (clobbered).
/// - `norm`: `≥ out_len` floats; accumulates `Σ w_a·w_s` (clobbered).
/// - `out`: `≥ out_len` floats; receives the reconstructed signal, where
///   `out_len = (num_frames − 1)·hop + N`.
///
/// Returns the number of output samples written (`out_len`).
///
/// Errors: [`AudioError::InvalidParameter`] for a non-power-of-two/zero
/// `fft_size`, zero `hop`, wrong window lengths, or a `spectra` length that is
/// not a multiple of `2·fft_size`; [`AudioError::OutputBufferTooSmall`] if
/// `ifft_scratch`, `norm`, or `out` are too short.
#[allow(clippy::too_many_arguments)]
pub fn istft(
    spectra: &mut [f32],
    fft_size: usize,
    hop: usize,
    analysis_window: &[f32],
    synthesis_window: &[f32],
    ifft_scratch: &mut [f32],
    norm: &mut [f32],
    out: &mut [f32],
) -> Result<usize, AudioError> {
    let n = fft_size;
    if n == 0 || !n.is_power_of_two() || hop == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if analysis_window.len() != n || synthesis_window.len() != n {
        return Err(AudioError::InvalidParameter);
    }
    let two_n = 2 * n;
    if !spectra.len().is_multiple_of(two_n) {
        return Err(AudioError::InvalidParameter);
    }
    let num_frames = spectra.len() / two_n;
    if num_frames == 0 {
        return Ok(0);
    }
    let out_len = (num_frames - 1)
        .checked_mul(hop)
        .and_then(|v| v.checked_add(n))
        .ok_or(AudioError::InvalidParameter)?;
    if ifft_scratch.len() < n || norm.len() < out_len || out.len() < out_len {
        return Err(AudioError::OutputBufferTooSmall);
    }

    for v in out[..out_len].iter_mut() {
        *v = 0.0;
    }
    for v in norm[..out_len].iter_mut() {
        *v = 0.0;
    }

    for i in 0..num_frames {
        let frame_spec = &mut spectra[i * two_n..(i + 1) * two_n];
        ifft_to_real(frame_spec, &mut ifft_scratch[..n])?;
        let base = i * hop;
        for j in 0..n {
            let wa = analysis_window[j];
            let ws = synthesis_window[j];
            out[base + j] += ws * ifft_scratch[j];
            norm[base + j] += wa * ws;
        }
    }

    // Normalise by the window-overlap envelope (WOLA). Below the epsilon the
    // envelope carries no energy, so the output stays zero rather than dividing.
    const EPS: f32 = 1e-8;
    for k in 0..out_len {
        if norm[k].abs() > EPS {
            out[k] /= norm[k];
        }
    }
    Ok(out_len)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::fft::fft_radix2;
    use crate::features::framing::frame_cutter::{cut_frame, frame_count};
    use crate::features::window::{apply_window, hann_window};
    use core::f32::consts::TAU;

    /// Forward analysis: window each frame and FFT it into an interleaved-complex
    /// spectra buffer (`num_frames × 2N`). Test-only helper (may allocate).
    fn analyse(
        signal: &[f32],
        n: usize,
        hop: usize,
        window: &[f32],
    ) -> (Vec<f32>, usize) {
        let frames = frame_count(signal.len(), n, hop);
        let two_n = 2 * n;
        let mut spectra = vec![0.0f32; frames * two_n];
        let mut frame = vec![0.0f32; n];
        for i in 0..frames {
            cut_frame(signal, n, hop, i, &mut frame).unwrap();
            apply_window(&mut frame, window).unwrap();
            let seg = &mut spectra[i * two_n..(i + 1) * two_n];
            for j in 0..n {
                seg[2 * j] = frame[j];
                seg[2 * j + 1] = 0.0;
            }
            fft_radix2(seg, false).unwrap();
        }
        (spectra, frames)
    }

    #[test]
    fn stft_then_istft_reconstructs_signal() {
        // GOLDEN identity: analyse then synthesise a two-tone signal and recover
        // it within a tight tolerance over the steady-state (fully overlapped)
        // interior, using a COLA window+hop (periodic Hann, hop = N/2).
        let n = 512usize;
        let hop = n / 2;
        let len = 8192usize;
        let fs = 16_000.0f32;
        let signal: Vec<f32> = (0..len)
            .map(|i| {
                let t = i as f32 / fs;
                0.6 * (TAU * 440.0 * t).sin() + 0.4 * (TAU * 1015.0 * t).sin()
            })
            .collect();

        let mut window = vec![0.0f32; n];
        hann_window(&mut window).unwrap();

        let (mut spectra, frames) = analyse(&signal, n, hop, &window);
        let out_len = (frames - 1) * hop + n;
        let mut ifft_scratch = vec![0.0f32; n];
        let mut norm = vec![0.0f32; out_len];
        let mut out = vec![0.0f32; out_len];
        istft(&mut spectra, n, hop, &window, &window, &mut ifft_scratch, &mut norm, &mut out)
            .unwrap();

        // Steady-state region [n, out_len - n): every sample seen by full overlap.
        let (lo, hi) = (n, out_len - n);
        let mut mae = 0.0f64;
        for k in lo..hi {
            mae += (out[k] - signal[k]).abs() as f64;
        }
        mae /= (hi - lo) as f64;
        assert!(mae < 1e-3, "steady-state MAE {mae} exceeds 1e-3");
    }

    #[test]
    fn rejects_non_power_of_two_fft() {
        let mut spectra = vec![0.0f32; 2 * 6];
        let w = vec![0.0f32; 6];
        let mut sc = vec![0.0f32; 6];
        let mut nm = vec![0.0f32; 6];
        let mut out = vec![0.0f32; 6];
        assert_eq!(
            istft(&mut spectra, 6, 3, &w, &w, &mut sc, &mut nm, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_wrong_window_length() {
        let n = 8usize;
        let mut spectra = vec![0.0f32; 2 * n];
        let w = vec![0.0f32; n - 1];
        let mut sc = vec![0.0f32; n];
        let mut nm = vec![0.0f32; n];
        let mut out = vec![0.0f32; n];
        assert_eq!(
            istft(&mut spectra, n, 4, &w, &w, &mut sc, &mut nm, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_short_output() {
        let n = 8usize;
        let mut spectra = vec![0.0f32; 2 * 2 * n]; // 2 frames
        let w = vec![0.0f32; n];
        let mut sc = vec![0.0f32; n];
        let mut nm = vec![0.0f32; 100];
        let mut out = vec![0.0f32; 4]; // far too short
        assert_eq!(
            istft(&mut spectra, n, 4, &w, &w, &mut sc, &mut nm, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
