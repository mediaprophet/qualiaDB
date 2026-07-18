//! Real inverse STFT — resynthesise a time-domain signal from the two-sided
//! complex frames produced by [`crate::audio::stft::forward_stft`].
//!
//! This is the cold-path inverse companion to the forward STFT: it reuses the
//! already-built, caller-buffered WOLA resynthesiser in
//! [`qualia_audio::features::framing::istft`] (weighted overlap-add normalised by
//! the running Σ analysis·synthesis window product), so reconstruction is exact
//! over the fully-overlapped interior regardless of whether the window+hop
//! strictly satisfy COLA.
//!
//! Heap allocation is fine here: ISTFT runs at ingest / edit (cold path), never
//! in the zero-heap U3 hot worklet. Native only — `qualia-audio` is a non-wasm
//! dependency of `qualia-core-db`.

use crate::audio::stft_bake::StftBakeError;

/// Hann window coefficient `w[i] = 0.5*(1 - cos(2π·i/(N-1)))` for a length-`n`
/// frame — identical to [`crate::audio::stft`]'s analysis window, so the same
/// window drives both the analysis and synthesis args of the WOLA resynthesis.
#[inline]
fn hann(i: usize, n: usize) -> f32 {
    if n <= 1 {
        return 1.0;
    }
    0.5 * (1.0 - (core::f32::consts::TAU * i as f32 / (n - 1) as f32).cos())
}

/// Inverse STFT of the two-sided complex frames from
/// [`crate::audio::stft::forward_stft`] back into a single time-domain signal.
///
/// - `spec`: `num_frames` frames, each a length-`frame_size` two-sided spectrum
///   of `[re, im]` bins (exactly the [`crate::audio::stft::forward_stft`] output).
/// - `frame_size`: `N`, a power of two; the analysis/synthesis frame length.
/// - `hop`: sample advance between frames (`> 0`). A COLA Hann at `hop = N/2`
///   gives exact interior reconstruction.
///
/// The frames are flattened into a `num_frames × 2·N` interleaved-complex buffer
/// (`[re, im, …]`), a length-`N` Hann window is built for both the analysis and
/// synthesis arguments, and the WOLA resynthesiser
/// [`qualia_audio::features::framing::istft`] is invoked with caller-owned
/// scratch/norm/out buffers (`out_len = (num_frames − 1)·hop + N`).
///
/// The core-db forward sign (`exp(-2πi·k·j/N)`, [`crate::audio::stft`]) matches
/// `qualia-audio`'s inverse sign (`fft_radix2(_, false)` uses `e^{-j2πk/L}`), so
/// no conjugation is required — the forward output feeds straight in.
///
/// Returns the reconstructed samples (length `out_len`), or an empty `Vec` when
/// `spec` is empty. Errors map [`qualia_audio::types::AudioError`] →
/// [`StftBakeError`].
#[cfg(not(target_arch = "wasm32"))]
pub fn inverse_stft(
    spec: &[Vec<[f32; 2]>],
    frame_size: usize,
    hop: usize,
) -> Result<Vec<f32>, StftBakeError> {
    if !frame_size.is_power_of_two() || frame_size == 0 || hop == 0 {
        return Err(StftBakeError::InvalidFrameCount);
    }
    let num_frames = spec.len();
    if num_frames == 0 {
        return Ok(Vec::new());
    }
    // Every frame must be a full two-sided spectrum of `frame_size` bins.
    for frame in spec {
        if frame.len() != frame_size {
            return Err(StftBakeError::InvalidFrameCount);
        }
    }

    let two_n = 2 * frame_size;
    // Flatten frames into `num_frames × 2N` interleaved complex `[re, im, …]`.
    // No conjugation: forward and inverse share the `e^{-j2πk/L}` sign convention.
    let mut spectra = vec![0.0_f32; num_frames * two_n];
    for (f, frame) in spec.iter().enumerate() {
        let seg = &mut spectra[f * two_n..(f + 1) * two_n];
        for (k, &[re, im]) in frame.iter().enumerate() {
            seg[2 * k] = re;
            seg[2 * k + 1] = im;
        }
    }

    // Length-N Hann for both analysis and synthesis window args.
    let window: Vec<f32> = (0..frame_size).map(|i| hann(i, frame_size)).collect();

    let out_len = (num_frames - 1)
        .checked_mul(hop)
        .and_then(|v| v.checked_add(frame_size))
        .ok_or(StftBakeError::InvalidFrameCount)?;

    let mut ifft_scratch = vec![0.0_f32; frame_size];
    let mut norm = vec![0.0_f32; out_len];
    let mut out = vec![0.0_f32; out_len];

    let written = qualia_audio::features::framing::istft(
        &mut spectra,
        frame_size,
        hop,
        &window,
        &window,
        &mut ifft_scratch,
        &mut norm,
        &mut out,
    )
    .map_err(map_audio_err)?;

    out.truncate(written);
    Ok(out)
}

/// Map a `qualia-audio` [`qualia_audio::types::AudioError`] onto the core-db
/// sidecar [`StftBakeError`] surface.
#[cfg(not(target_arch = "wasm32"))]
#[inline]
fn map_audio_err(e: qualia_audio::types::AudioError) -> StftBakeError {
    use qualia_audio::types::AudioError;
    match e {
        AudioError::OutputBufferTooSmall | AudioError::WorkspaceTooSmall => {
            StftBakeError::OutputTooSmall
        }
        _ => StftBakeError::InvalidFrameCount,
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::audio::stft::forward_stft;
    use core::f32::consts::TAU;

    /// GOLDEN round trip: `forward_stft` → `inverse_stft` reconstructs the original
    /// signal over the steady-state (fully-overlapped) interior with a COLA Hann at
    /// hop = N/2 (mean-abs-error < 1e-2). Also proves no conjugate fix is needed.
    #[test]
    fn istft_round_trip_reconstructs_interior() {
        const N: usize = 256;
        const HOP: usize = N / 2; // COLA Hann
        let fs = 16_000.0f32;
        let len = 4096usize;
        let signal: Vec<f32> = (0..len)
            .map(|i| {
                let t = i as f32 / fs;
                0.6 * (TAU * 440.0 * t).sin() + 0.35 * (TAU * 1234.0 * t).cos()
            })
            .collect();

        let spec = forward_stft(&signal, N, HOP).expect("forward stft");
        let recon = inverse_stft(&spec, N, HOP).expect("inverse stft");

        // Steady-state interior [N, out_len - N): every sample seen by full overlap.
        let out_len = recon.len();
        assert!(out_len >= 2 * N + 1, "not enough overlap for interior test");
        let (lo, hi) = (N, out_len - N);
        let mut mae = 0.0f64;
        for k in lo..hi {
            mae += (recon[k] - signal[k]).abs() as f64;
        }
        mae /= (hi - lo) as f64;
        assert!(mae < 1e-2, "round-trip steady-state MAE {mae} exceeds 1e-2");
    }

    #[test]
    fn istft_empty_spec_yields_empty() {
        let empty: Vec<Vec<[f32; 2]>> = Vec::new();
        let out = inverse_stft(&empty, 256, 128).expect("empty ok");
        assert!(out.is_empty());
    }

    #[test]
    fn istft_output_length_matches_formula() {
        const N: usize = 64;
        const HOP: usize = 32;
        let signal = vec![0.2f32; N * 5];
        let spec = forward_stft(&signal, N, HOP).expect("forward");
        let num_frames = spec.len();
        let recon = inverse_stft(&spec, N, HOP).expect("inverse");
        assert_eq!(recon.len(), (num_frames - 1) * HOP + N);
    }

    #[test]
    fn istft_rejects_non_power_of_two() {
        let spec = vec![vec![[0.0f32; 2]; 48]];
        assert_eq!(inverse_stft(&spec, 48, 16), Err(StftBakeError::InvalidFrameCount));
    }

    #[test]
    fn istft_rejects_wrong_frame_width() {
        // Frame length (10) disagrees with frame_size (256).
        let spec = vec![vec![[0.0f32; 2]; 10]];
        assert_eq!(
            inverse_stft(&spec, 256, 128),
            Err(StftBakeError::InvalidFrameCount)
        );
    }
}
