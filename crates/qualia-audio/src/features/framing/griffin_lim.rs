//! Griffin-Lim magnitude-only reconstruction.
//!
//! Recovers a time-domain signal from an STFT **magnitude** target when the phase
//! is unknown, by alternating projection (Griffin & Lim, 1984): invert the
//! current complex estimate to the time domain (ISTFT), re-analyse it (STFT),
//! then replace the magnitude with the target while keeping the freshly-estimated
//! phase. Each round is non-increasing in the magnitude inconsistency.
//!
//! Reuses [`crate::features::istft`] for the inverse and
//! [`crate::features::fft::fft_radix2`] for the forward analysis. Zero-heap hot
//! path: every buffer (including the evolving spectra) is caller-owned.

use crate::features::fft::fft_radix2;
use crate::features::framing::istft::istft;
use crate::types::AudioError;

/// Run `iterations` Griffin-Lim rounds and write the reconstructed time signal.
///
/// - `target_mags`: `num_frames × (N/2 + 1)` non-negative magnitude targets,
///   frame `i` at `target_mags[i*(N/2+1) .. (i+1)*(N/2+1)]`.
/// - `spectra`: `num_frames × 2N` interleaved-complex working buffer, **seeded by
///   the caller** with an initial estimate (typically `target magnitude × e^{jφ}`
///   for some initial phase φ, Hermitian-symmetric). Updated in place;
///   `num_frames = spectra.len() / (2N)`.
/// - `fft_size`: `N`, a power of two.
/// - `hop`: sample advance between frames (`> 0`).
/// - `iterations`: number of projection rounds (`0` just inverts the seed).
/// - `analysis_window` / `synthesis_window`: length-`N` windows (WOLA pair).
/// - `ifft_scratch`: `≥ N` floats (clobbered).
/// - `norm`: `≥ out_len` floats, ISTFT normalisation scratch (clobbered).
/// - `out`: `≥ out_len` floats; receives the reconstruction, where
///   `out_len = (num_frames − 1)·hop + N`.
///
/// Returns `out_len`. Errors mirror [`istft`], plus
/// [`AudioError::InvalidParameter`] if `target_mags.len()` does not equal
/// `num_frames × (N/2 + 1)`.
#[allow(clippy::too_many_arguments)]
pub fn griffin_lim(
    target_mags: &[f32],
    spectra: &mut [f32],
    fft_size: usize,
    hop: usize,
    iterations: usize,
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
    let bins = n / 2 + 1;
    if target_mags.len() != num_frames * bins {
        return Err(AudioError::InvalidParameter);
    }
    let out_len = (num_frames - 1)
        .checked_mul(hop)
        .and_then(|v| v.checked_add(n))
        .ok_or(AudioError::InvalidParameter)?;
    if ifft_scratch.len() < n || norm.len() < out_len || out.len() < out_len {
        return Err(AudioError::OutputBufferTooSmall);
    }

    for _ in 0..iterations {
        // Project onto the consistent-STFT set: invert to time.
        istft(spectra, n, hop, analysis_window, synthesis_window, ifft_scratch, norm, out)?;
        // Re-analyse and project onto the target-magnitude set.
        for i in 0..num_frames {
            let seg = &mut spectra[i * two_n..(i + 1) * two_n];
            let base = i * hop;
            for j in 0..n {
                seg[2 * j] = out[base + j] * analysis_window[j];
                seg[2 * j + 1] = 0.0;
            }
            fft_radix2(seg, false)?;
            impose_magnitude(seg, &target_mags[i * bins..(i + 1) * bins], n);
        }
    }

    // Final inversion of the magnitude-consistent estimate.
    istft(spectra, n, hop, analysis_window, synthesis_window, ifft_scratch, norm, out)?;
    Ok(out_len)
}

/// Replace the magnitude of the half-spectrum `0..=N/2` of `seg` (interleaved
/// complex, length `2N`) with `target`, keeping each bin's phase, then restore
/// Hermitian symmetry so the inverse transform is real. DC and Nyquist are made
/// purely real.
fn impose_magnitude(seg: &mut [f32], target: &[f32], n: usize) {
    const EPS: f32 = 1e-12;
    let half = n / 2;
    for k in 0..=half {
        let re = seg[2 * k];
        let im = seg[2 * k + 1];
        let mag = (re * re + im * im).sqrt();
        let (nr, ni) = if mag > EPS {
            let s = target[k] / mag;
            (re * s, im * s)
        } else {
            // No phase information: assume zero phase at the target magnitude.
            (target[k], 0.0)
        };
        seg[2 * k] = nr;
        seg[2 * k + 1] = ni;
    }
    // DC and Nyquist are real for a real signal.
    seg[1] = 0.0;
    seg[2 * half + 1] = 0.0;
    // Hermitian mirror: X[N-k] = conj(X[k]) for k in 1..N/2.
    for k in 1..half {
        seg[2 * (n - k)] = seg[2 * k];
        seg[2 * (n - k) + 1] = -seg[2 * k + 1];
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::fft::real_fft_magnitude;
    use crate::features::framing::frame_cutter::frame_count;
    use crate::features::window::hann_window;
    use core::f32::consts::{PI, TAU};

    /// Deterministic LCG in [0, 1) — reproducible "random" phase seed.
    struct Lcg(u64);
    impl Lcg {
        fn next_f32(&mut self) -> f32 {
            self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            ((self.0 >> 40) as f32) / ((1u64 << 24) as f32)
        }
    }

    fn analyse_mags(signal: &[f32], n: usize, hop: usize, window: &[f32]) -> (Vec<f32>, usize) {
        let frames = frame_count(signal.len(), n, hop);
        let bins = n / 2 + 1;
        let mut mags = vec![0.0f32; frames * bins];
        let mut frame = vec![0.0f32; n];
        let mut scratch = vec![0.0f32; 2 * n];
        for i in 0..frames {
            for j in 0..n {
                frame[j] = signal[i * hop + j] * window[j];
            }
            real_fft_magnitude(&frame, &mut scratch, &mut mags[i * bins..(i + 1) * bins]).unwrap();
        }
        (mags, frames)
    }

    /// Total absolute STFT-magnitude error of `signal` against `target` mags.
    fn magnitude_error(signal: &[f32], target: &[f32], n: usize, hop: usize, window: &[f32]) -> f64 {
        let (got, _) = analyse_mags(signal, n, hop, window);
        got.iter().zip(target.iter()).map(|(a, b)| (a - b).abs() as f64).sum()
    }

    /// Build a Hermitian interleaved-complex seed = target magnitude × e^{jφ}.
    fn seed_random_phase(target: &[f32], n: usize, frames: usize, rng: &mut Lcg) -> Vec<f32> {
        let bins = n / 2 + 1;
        let two_n = 2 * n;
        let half = n / 2;
        let mut spectra = vec![0.0f32; frames * two_n];
        for i in 0..frames {
            let seg = &mut spectra[i * two_n..(i + 1) * two_n];
            let mags = &target[i * bins..(i + 1) * bins];
            for k in 0..=half {
                let phi = (rng.next_f32() * 2.0 - 1.0) * PI;
                seg[2 * k] = mags[k] * phi.cos();
                seg[2 * k + 1] = mags[k] * phi.sin();
            }
            seg[1] = 0.0;
            seg[2 * half + 1] = 0.0;
            for k in 1..half {
                seg[2 * (n - k)] = seg[2 * k];
                seg[2 * (n - k) + 1] = -seg[2 * k + 1];
            }
        }
        spectra
    }

    #[test]
    fn griffin_lim_reduces_magnitude_error() {
        // GOLDEN: from a random-phase start, more Griffin-Lim iterations lower the
        // STFT-magnitude inconsistency of the reconstruction against the target.
        let n = 256usize;
        let hop = n / 4; // 75% overlap — good conditioning for GL.
        let len = 4096usize;
        let fs = 16_000.0f32;
        let signal: Vec<f32> = (0..len)
            .map(|i| {
                let t = i as f32 / fs;
                0.5 * (TAU * 300.0 * t).sin() + 0.3 * (TAU * 900.0 * t).sin()
            })
            .collect();

        let mut window = vec![0.0f32; n];
        hann_window(&mut window).unwrap();
        let (target, frames) = analyse_mags(&signal, n, hop, &window);
        let out_len = (frames - 1) * hop + n;

        // Same random-phase seed for both runs.
        let mut rng = Lcg(0x1234_5678_9abc_def0);
        let seed = seed_random_phase(&target, n, frames, &mut rng);

        let run = |iters: usize| -> Vec<f32> {
            let mut spectra = seed.clone();
            let mut ifft_scratch = vec![0.0f32; n];
            let mut norm = vec![0.0f32; out_len];
            let mut out = vec![0.0f32; out_len];
            griffin_lim(
                &target, &mut spectra, n, hop, iters, &window, &window, &mut ifft_scratch,
                &mut norm, &mut out,
            )
            .unwrap();
            out
        };

        let out1 = run(1);
        let out40 = run(40);
        let e1 = magnitude_error(&out1, &target, n, hop, &window);
        let e40 = magnitude_error(&out40, &target, n, hop, &window);

        assert!(e40 < e1, "Griffin-Lim did not improve: e1={e1}, e40={e40}");
        // Substantial, not marginal, improvement expected on a clean two-tone.
        assert!(e40 < e1 * 0.9, "improvement too small: e1={e1}, e40={e40}");
    }

    #[test]
    fn rejects_mismatched_target_len() {
        let n = 8usize;
        let mut spectra = vec![0.0f32; 2 * 2 * n]; // 2 frames
        let target = vec![0.0f32; 2 * (n / 2 + 1) - 1]; // wrong length
        let w = vec![0.0f32; n];
        let mut sc = vec![0.0f32; n];
        let out_len = hop_out_len(2, 4, n);
        let mut nm = vec![0.0f32; out_len];
        let mut out = vec![0.0f32; out_len];
        assert_eq!(
            griffin_lim(&target, &mut spectra, n, 4, 5, &w, &w, &mut sc, &mut nm, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    fn hop_out_len(frames: usize, hop: usize, n: usize) -> usize {
        (frames - 1) * hop + n
    }
}
