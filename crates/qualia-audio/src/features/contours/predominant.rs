//! Top-level predominant-F0-per-frame melody line (mono melody from poly input).
//!
//! Given a stream of per-frame pitch-salience curves (from
//! [`crate::features::salience::pitch_salience`]), emit one fundamental frequency
//! per frame: the pitch of the frame's strongest salience ridge, refined to
//! sub-bin resolution, **provided** that ridge is salient enough to be voiced.
//! Frames whose peak salience falls below a data-driven voicing floor **abstain**
//! (`out_f0 = 0.0`) rather than guess.
//!
//! ASSUMPTION (declared): this collapses each frame to a single F0 — a
//! **monophonic melody** reading of possibly polyphonic input. The output is an
//! epistemic *proposal* for the predominant voice, not an assertion that only one
//! pitch sounds. Callers needing all voices should use the salience peaks /
//! contour tracker directly.
//!
//! Zero-heap hot path: reads a caller-owned flat salience buffer and writes into
//! a caller-owned `out_f0`; allocates nothing.

use crate::types::AudioError;

/// Fraction of the global peak salience below which a frame is treated as
/// unvoiced (abstains). Melodia uses a salience-distribution threshold; this is
/// a simple, honest analogue driven by the loudest ridge in the excerpt.
const VOICING_RATIO: f32 = 0.1;

/// Estimate the predominant fundamental frequency of each frame.
///
/// `salience_frames` is a flat row-major buffer of shape `n_frames × n_bins`
/// (`salience_frames[f * n_bins + b]`) on the logarithmic grid where bin `b` is
/// `f_min_hz * 2^(b / (12 * bins_per_semitone))`. For each frame the strongest
/// bin is found, parabolically refined, and converted to Hz; frames whose peak
/// salience is below `VOICING_RATIO` of the whole excerpt's peak salience abstain
/// with `out_f0 = 0.0`.
///
/// Returns the number of **voiced** frames (non-zero `out_f0`).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `n_bins < 3`, `f_min_hz` /
///   `bins_per_semitone` are not positive finite, or
///   `salience_frames.len() < n_frames * n_bins`.
/// - [`AudioError::OutputBufferTooSmall`] if `out_f0.len() < n_frames`.
pub fn predominant_melody(
    salience_frames: &[f32],
    n_frames: usize,
    n_bins: usize,
    f_min_hz: f32,
    bins_per_semitone: f32,
    out_f0: &mut [f32],
) -> Result<usize, AudioError> {
    if n_bins < 3
        || !(f_min_hz.is_finite() && f_min_hz > 0.0)
        || !(bins_per_semitone.is_finite() && bins_per_semitone > 0.0)
        || salience_frames.len() < n_frames.saturating_mul(n_bins)
    {
        return Err(AudioError::InvalidParameter);
    }
    if out_f0.len() < n_frames {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for v in out_f0.iter_mut().take(n_frames) {
        *v = 0.0;
    }
    if n_frames == 0 {
        return Ok(0);
    }

    // First pass: global peak salience → voicing floor.
    let mut global_peak = 0.0f32;
    for f in 0..n_frames {
        let row = f * n_bins;
        for b in 0..n_bins {
            let s = salience_frames[row + b];
            if s > global_peak {
                global_peak = s;
            }
        }
    }
    if global_peak <= 0.0 {
        return Ok(0); // wholly silent excerpt → abstain everywhere
    }
    let floor = VOICING_RATIO * global_peak;

    // Second pass: per-frame argmax + parabolic refine + voicing decision.
    let mut voiced = 0usize;
    for f in 0..n_frames {
        let row = f * n_bins;
        let mut bi = 0usize;
        let mut bv = salience_frames[row];
        for b in 1..n_bins {
            let s = salience_frames[row + b];
            if s > bv {
                bv = s;
                bi = b;
            }
        }
        if bv <= floor {
            continue; // abstain: not salient enough to commit to a pitch
        }
        let refined = parabolic_bin(salience_frames, row, bi, n_bins);
        let f0 = f_min_hz * (refined / (12.0 * bins_per_semitone)).exp2();
        out_f0[f] = f0;
        voiced += 1;
    }
    Ok(voiced)
}

/// Sub-bin refinement of the salience maximum at bin `bi` via a 3-point
/// parabola; returns the fractional bin. Boundary maxima fall back to `bi`.
#[inline]
fn parabolic_bin(sal: &[f32], row: usize, bi: usize, n_bins: usize) -> f32 {
    if bi == 0 || bi + 1 >= n_bins {
        return bi as f32;
    }
    let ym1 = sal[row + bi - 1];
    let y0 = sal[row + bi];
    let yp1 = sal[row + bi + 1];
    let denom = ym1 - 2.0 * y0 + yp1;
    if denom == 0.0 || !denom.is_finite() {
        return bi as f32;
    }
    let off = (0.5 * (ym1 - yp1) / denom).clamp(-0.5, 0.5);
    bi as f32 + off
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::features::salience::pitch_salience;

    const F_MIN: f32 = 55.0;
    const BPS: f32 = 10.0; // 10 cents/bin
    const N_BINS: usize = 720; // 6 octaves

    /// Ratio of two frequencies expressed in semitones.
    fn semitone_err(a: f32, b: f32) -> f32 {
        12.0 * (a / b).log2().abs()
    }

    /// GOLDEN end-to-end: a melody stepping 220 → 262 → 330 Hz across three
    /// voiced frames (built as real harmonic-salience frames via `pitch_salience`)
    /// plus one silent frame. `predominant_melody` recovers each f0 to within a
    /// semitone and abstains on the silent frame.
    #[test]
    fn recovers_stepping_melody_and_abstains() {
        let melody = [220.0f32, 262.0, 330.0, 0.0]; // last = silent
        let n_frames = melody.len();
        let mut frames = vec![0.0f32; n_frames * N_BINS];
        let mut frame_buf = vec![0.0f32; N_BINS];

        for (f, &f0) in melody.iter().enumerate() {
            if f0 > 0.0 {
                // Three harmonics for a tone at f0.
                let pf = [f0, 2.0 * f0, 3.0 * f0];
                let pm = [1.0f32, 0.6, 0.4];
                pitch_salience(&pf, &pm, 3, F_MIN, BPS, N_BINS, &mut frame_buf).expect("salience");
            } else {
                for s in frame_buf.iter_mut() {
                    *s = 0.0; // silent frame
                }
            }
            frames[f * N_BINS..(f + 1) * N_BINS].copy_from_slice(&frame_buf);
        }

        let mut f0_out = vec![0.0f32; n_frames];
        let voiced =
            predominant_melody(&frames, n_frames, N_BINS, F_MIN, BPS, &mut f0_out).expect("melody");

        assert_eq!(voiced, 3, "three voiced frames, one abstained");
        for (f, &want) in melody.iter().enumerate() {
            if want > 0.0 {
                let err = semitone_err(f0_out[f], want);
                assert!(
                    err < 1.0,
                    "frame {f}: recovered {} Hz vs {} Hz = {:.3} semitones",
                    f0_out[f],
                    want,
                    err
                );
            } else {
                assert_eq!(f0_out[f], 0.0, "silent frame must abstain");
            }
        }
    }

    /// A wholly silent excerpt abstains on every frame.
    #[test]
    fn all_silent_abstains() {
        let n_frames = 4usize;
        let frames = vec![0.0f32; n_frames * N_BINS];
        let mut f0_out = vec![9.0f32; n_frames];
        let voiced =
            predominant_melody(&frames, n_frames, N_BINS, F_MIN, BPS, &mut f0_out).expect("melody");
        assert_eq!(voiced, 0);
        assert!(f0_out.iter().all(|&v| v == 0.0));
    }

    #[test]
    fn rejects_bad_params() {
        let frames = vec![0.0f32; 10];
        let mut f0 = [0.0f32; 2];
        assert_eq!(
            predominant_melody(&frames, 2, 2, F_MIN, BPS, &mut f0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            predominant_melody(&frames, 2, 5, 0.0, BPS, &mut f0),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_output() {
        let frames = vec![0.0f32; 3 * 8];
        let mut f0 = [0.0f32; 2];
        assert_eq!(
            predominant_melody(&frames, 3, 8, F_MIN, BPS, &mut f0),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
