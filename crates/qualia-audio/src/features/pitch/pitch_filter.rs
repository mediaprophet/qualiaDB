//! Post-process a raw pitch track: fold octave errors back onto the local
//! median and drop isolated outliers, using a caller-supplied scratch buffer
//! (no per-call allocation).

use crate::types::AudioError;

/// Clean a pitch track in `track` into `out`.
///
/// For each voiced value (`> 0`) the local median over a centred window of
/// `window` samples is computed; the value is then folded by factors of two
/// toward that median (removing octave jumps), and any residual gross outlier
/// (still more than ~50% from the median after folding) is replaced by the
/// median. Unvoiced samples (`≤ 0`) are copied through as `0.0`. When the local
/// window has no other voiced support the sample is passed through unchanged.
///
/// - `track`: input f0 values (Hz), `0.0` = unvoiced.
/// - `out`: cleaned track; must be at least `track.len()` long.
/// - `window`: odd median window size (≥ 3). Even values are rounded up.
/// - `scratch`: gather buffer for the window; must hold at least `window`
///   floats.
///
/// Returns the number of samples written (`track.len()`).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `window < 3`.
/// - [`AudioError::OutputBufferTooSmall`] if `out` or `scratch` are too short.
pub fn pitch_filter(
    track: &[f32],
    out: &mut [f32],
    window: usize,
    scratch: &mut [f32],
) -> Result<usize, AudioError> {
    if window < 3 {
        return Err(AudioError::InvalidParameter);
    }
    let win = if window.is_multiple_of(2) { window + 1 } else { window };
    if out.len() < track.len() || scratch.len() < win {
        return Err(AudioError::OutputBufferTooSmall);
    }
    let n = track.len();
    let half = win / 2;

    for i in 0..n {
        let v = track[i];
        if v.is_nan() || v <= 0.0 {
            out[i] = 0.0;
            continue;
        }
        // Gather voiced neighbours (excluding self) into scratch.
        let lo = i.saturating_sub(half);
        let hi = (i + half + 1).min(n);
        let mut count = 0usize;
        for (idx, &t) in track[lo..hi].iter().enumerate() {
            if lo + idx == i || t.is_nan() || t <= 0.0 {
                continue;
            }
            scratch[count] = t;
            count += 1;
        }
        if count == 0 {
            out[i] = v; // no support — pass through
            continue;
        }
        let median = median_of(&mut scratch[..count]);

        // Octave correction: fold `v` by whole octaves toward the median and
        // accept the fold only if it lands within ~8% of the median (a genuine
        // octave error). Otherwise keep `v` for the outlier test below.
        let ratio = (v / median).max(1e-9);
        let octaves = ratio.log2().round();
        let candidate = v * 2f32.powf(-octaves);
        let resid = (candidate / median).log2().abs();
        let mut corrected = if resid < 0.12 { candidate } else { v };

        // Isolated gross outlier (non-octave, > 1.5× off the median) → snap.
        let r2 = corrected / median;
        if !(1.0 / 1.5..=1.5).contains(&r2) {
            corrected = median;
        }
        out[i] = corrected;
    }
    Ok(n)
}

/// Median of `buf` via partial insertion sort (buf is scratch, order destroyed).
fn median_of(buf: &mut [f32]) -> f32 {
    let m = buf.len();
    for i in 1..m {
        let mut j = i;
        while j > 0 && buf[j - 1] > buf[j] {
            buf.swap(j - 1, j);
            j -= 1;
        }
    }
    if !m.is_multiple_of(2) {
        buf[m / 2]
    } else {
        0.5 * (buf[m / 2 - 1] + buf[m / 2])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_single_octave_jump() {
        // One frame doubled to the octave above; neighbours are steady 220 Hz.
        let track = [220.0f32, 220.0, 440.0, 220.0, 220.0];
        let mut out = [0.0f32; 5];
        let mut scratch = [0.0f32; 8];
        let n = pitch_filter(&track, &mut out, 3, &mut scratch).expect("filter");
        assert_eq!(n, 5);
        assert!((out[2] - 220.0).abs() < 1.0, "octave not removed: {}", out[2]);
        // Steady values untouched.
        for i in [0usize, 1, 3, 4] {
            assert!((out[i] - 220.0).abs() < 1.0, "out[{i}]={}", out[i]);
        }
    }

    #[test]
    fn removes_low_octave_jump() {
        let track = [330.0f32, 330.0, 165.0, 330.0, 330.0];
        let mut out = [0.0f32; 5];
        let mut scratch = [0.0f32; 8];
        pitch_filter(&track, &mut out, 3, &mut scratch).expect("filter");
        assert!((out[2] - 330.0).abs() < 1.0, "low octave not raised: {}", out[2]);
    }

    #[test]
    fn snaps_isolated_outlier() {
        // Non-octave spurious spike surrounded by a stable pitch.
        let track = [200.0f32, 200.0, 730.0, 200.0, 200.0];
        let mut out = [0.0f32; 5];
        let mut scratch = [0.0f32; 8];
        pitch_filter(&track, &mut out, 3, &mut scratch).expect("filter");
        assert!((out[2] - 200.0).abs() < 1.0, "outlier not snapped: {}", out[2]);
    }

    #[test]
    fn passes_unvoiced_and_clean() {
        let track = [0.0f32, 300.0, 300.0, 0.0, 300.0];
        let mut out = [0.0f32; 5];
        let mut scratch = [0.0f32; 8];
        pitch_filter(&track, &mut out, 3, &mut scratch).expect("filter");
        assert_eq!(out[0], 0.0);
        assert_eq!(out[3], 0.0);
        assert!((out[1] - 300.0).abs() < 1.0);
    }

    #[test]
    fn rejects_bad_window() {
        let track = [220.0f32; 4];
        let mut out = [0.0f32; 4];
        let mut scratch = [0.0f32; 8];
        assert_eq!(
            pitch_filter(&track, &mut out, 2, &mut scratch),
            Err(AudioError::InvalidParameter)
        );
    }
}
