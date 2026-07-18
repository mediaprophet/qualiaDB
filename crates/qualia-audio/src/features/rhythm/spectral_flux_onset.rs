//! Onset detection from consecutive magnitude frames via true spectral flux and
//! a causal adaptive threshold. One public function; caller-buffered; zero-heap
//! hot path (a bounded fixed-size stack ring is the only working state).

use crate::features::spectral::spectral_flux;
use crate::types::AudioError;

/// Length of the causal moving-mean window (in flux frames) used to form the
/// adaptive threshold. Fixed → the ring buffer lives on the stack.
const THRESH_WINDOW: usize = 8;
/// Adaptive-threshold multiplier applied to the causal mean flux.
const LAMBDA: f32 = 1.5;
/// A candidate must also exceed this fraction of the running-max flux, which
/// rejects small ripples in near-silent stretches.
const NOISE_FLOOR_FRAC: f32 = 0.10;
/// Minimum spacing between accepted onsets, in frames (de-bounces one attack).
const MIN_ONSET_GAP: usize = 3;

/// Detect note/percussion onsets in a magnitude spectrogram.
///
/// `mags` is a row-major spectrogram of `n_frames` consecutive one-sided
/// magnitude spectra, each `n_bins` long (frame `f` occupies
/// `mags[f*n_bins .. (f+1)*n_bins]`), exactly the layout produced by
/// [`crate::features::stft_stream::magnitude_stft_chunk`]. Between every
/// adjacent pair of frames the **true per-bin** spectral flux
/// (`Σ_k max(cur[k]-prev[k], 0)`, via [`spectral_flux`]) forms an onset novelty
/// curve; the flux value for the transition `f-1 → f` is attributed to frame
/// `f`.
///
/// A frame is emitted as an onset when its flux is a local maximum
/// (`flux[f] > flux[f-1] && flux[f] >= flux[f+1]`), exceeds the adaptive
/// threshold `LAMBDA · mean(previous THRESH_WINDOW flux values)`, exceeds
/// `NOISE_FLOOR_FRAC · running_max_flux`, and is at least `MIN_ONSET_GAP` frames
/// after the previous accepted onset. Onset **frame indices** are written to
/// `out_onsets` in ascending order.
///
/// The novelty curve is consumed with a three-tap sliding delay, so no
/// per-frame heap buffer is needed; the only state is a fixed
/// `THRESH_WINDOW`-element stack ring. This replaces the frame-energy novelty
/// placeholder in `music.rs::detect_onsets` with real per-bin flux.
///
/// Returns the number of onsets written.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `n_bins == 0`, or `n_frames*n_bins`
///   exceeds `mags.len()`.
/// - [`AudioError::OutputBufferTooSmall`] if more onsets are found than
///   `out_onsets` can hold.
pub fn onset_detection(
    mags: &[f32],
    n_frames: usize,
    n_bins: usize,
    out_onsets: &mut [u32],
) -> Result<usize, AudioError> {
    if n_bins == 0 {
        return Err(AudioError::InvalidParameter);
    }
    let needed = n_frames.checked_mul(n_bins).ok_or(AudioError::InvalidParameter)?;
    if needed > mags.len() {
        return Err(AudioError::InvalidParameter);
    }
    if n_frames < 3 {
        return Ok(0);
    }

    // Causal moving-mean ring over past flux values.
    let mut ring = [0.0f32; THRESH_WINDOW];
    let mut ring_len = 0usize;
    let mut ring_pos = 0usize;
    let mut ring_sum = 0.0f64;

    // Three-tap flux delay: f_prev2 (frame fi-2), f_prev1 (frame fi-1, the
    // candidate under test), and the current flux at frame fi.
    let mut f_prev2 = 0.0f32;
    let mut f_prev1 = 0.0f32;
    let mut running_max = 0.0f32;
    // Frame index of the last accepted onset (sentinel keeps the first eligible
    // candidate un-gated).
    let mut last_onset: i64 = -(MIN_ONSET_GAP as i64) - 1;
    let mut w = 0usize;

    for fi in 1..n_frames {
        let prev = &mags[(fi - 1) * n_bins..fi * n_bins];
        let cur = &mags[fi * n_bins..(fi + 1) * n_bins];
        let flux = spectral_flux(prev, cur)?;
        if flux > running_max {
            running_max = flux;
        }

        // Finalize the candidate at frame fi-1 once its right neighbour (this
        // flux) is known. Requires fi >= 2 so f_prev2 is a real value.
        if fi >= 2 {
            let cand_frame = fi - 1;
            let mean = if ring_len > 0 {
                (ring_sum / ring_len as f64) as f32
            } else {
                0.0
            };
            let threshold = LAMBDA * mean;
            let is_local_max = f_prev1 > f_prev2 && f_prev1 >= flux;
            let above_thresh = f_prev1 >= threshold;
            let above_floor = f_prev1 >= NOISE_FLOOR_FRAC * running_max && f_prev1 > 0.0;
            let gap_ok = (cand_frame as i64 - last_onset) >= MIN_ONSET_GAP as i64;
            if is_local_max && above_thresh && above_floor && gap_ok {
                if w == out_onsets.len() {
                    return Err(AudioError::OutputBufferTooSmall);
                }
                out_onsets[w] = cand_frame as u32;
                w += 1;
                last_onset = cand_frame as i64;
            }

            // Slide the past-value ring forward by admitting f_prev2 (frame
            // fi-2), which now sits strictly before every future candidate.
            if ring_len < THRESH_WINDOW {
                ring[ring_pos] = f_prev2;
                ring_sum += f_prev2 as f64;
                ring_len += 1;
            } else {
                ring_sum -= ring[ring_pos] as f64;
                ring[ring_pos] = f_prev2;
                ring_sum += f_prev2 as f64;
            }
            ring_pos = (ring_pos + 1) % THRESH_WINDOW;
        }

        f_prev2 = f_prev1;
        f_prev1 = flux;
    }

    Ok(w)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a spectrogram: a flat baseline spectrum in every frame, with a
    /// one-frame energy jump at each `onset_frames` position. Each jump yields a
    /// single positive-flux spike at that frame (the return to baseline is
    /// rectified away), so onset detection must recover exactly those frames.
    fn synth_spectrogram(
        n_frames: usize,
        n_bins: usize,
        onset_frames: &[usize],
        baseline: f32,
        peak: f32,
    ) -> Vec<f32> {
        let mut m = vec![baseline; n_frames * n_bins];
        for &f in onset_frames {
            for b in 0..n_bins {
                m[f * n_bins + b] = peak;
            }
        }
        m
    }

    #[test]
    fn recovers_known_impulse_pattern() {
        let n_bins = 4;
        let n_frames = 40;
        let onsets = [5usize, 15, 25, 35];
        let mags = synth_spectrogram(n_frames, n_bins, &onsets, 0.1, 1.0);
        let mut out = [0u32; 16];
        let n = onset_detection(&mags, n_frames, n_bins, &mut out).expect("onset");
        assert_eq!(n, onsets.len(), "onset count");
        for (k, &f) in onsets.iter().enumerate() {
            assert_eq!(out[k], f as u32, "onset {k} position");
        }
    }

    #[test]
    fn silence_has_no_onsets() {
        let n_bins = 8;
        let n_frames = 32;
        let mags = vec![0.0f32; n_frames * n_bins];
        let mut out = [0u32; 16];
        let n = onset_detection(&mags, n_frames, n_bins, &mut out).expect("onset");
        assert_eq!(n, 0);
    }

    #[test]
    fn min_gap_debounces_adjacent_frames() {
        // Two spikes only two frames apart -> the second is inside MIN_ONSET_GAP
        // of the first and must be suppressed.
        let n_bins = 4;
        let n_frames = 20;
        let mags = synth_spectrogram(n_frames, n_bins, &[8, 10], 0.1, 1.0);
        let mut out = [0u32; 8];
        let n = onset_detection(&mags, n_frames, n_bins, &mut out).expect("onset");
        assert_eq!(n, 1);
        assert_eq!(out[0], 8);
    }

    #[test]
    fn rejects_bad_dimensions() {
        let mags = [0.0f32; 12];
        assert_eq!(
            onset_detection(&mags, 4, 0, &mut [0u32; 4]),
            Err(AudioError::InvalidParameter)
        );
        // n_frames*n_bins = 20 > 12
        assert_eq!(
            onset_detection(&mags, 5, 4, &mut [0u32; 4]),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn buffer_too_small_errors() {
        let n_bins = 4;
        let n_frames = 40;
        let onsets = [5usize, 15, 25, 35];
        let mags = synth_spectrogram(n_frames, n_bins, &onsets, 0.1, 1.0);
        let mut out = [0u32; 2];
        assert_eq!(
            onset_detection(&mags, n_frames, n_bins, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
