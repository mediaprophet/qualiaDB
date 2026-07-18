//! Harmonic pitch-salience function over a cent/bin grid (Melodia-class).
//!
//! Given the spectral peaks of one frame `(peak_freqs, peak_mags)`, accumulate a
//! **pitch-salience curve** onto a logarithmic (constant-cents) frequency grid.
//! Each spectral peak at frequency `f_i` is treated as the possible `h`-th
//! harmonic of a fundamental `f0 = f_i / h` for `h = 1..N_HARMONICS`, so it casts
//! a weighted vote onto the salience bin of every such candidate fundamental. A
//! fundamental that actually generated a harmonic series therefore collects votes
//! from all of its harmonics and dominates the curve.
//!
//! EPISTEMIC NOTE: the salience curve is a *proposal* about where pitched energy
//! plausibly sits; it commits to no monophonic/polyphonic reading — a polyphonic
//! frame yields several salient ridges. Downstream selection
//! (`predominant`/`melodia`) is where a mono-vs-poly assumption is declared.
//!
//! Zero-heap hot path: the caller owns `out_salience` (length ≥ `n_bins`); the
//! function clears and fills it in place and allocates nothing.

use crate::types::AudioError;

/// Number of harmonics each spectral peak votes through (peak = h-th harmonic).
const N_HARMONICS: usize = 20;
/// Geometric harmonic-weight decay `α^(h-1)` (higher harmonics vote weaker).
const HARMONIC_ALPHA: f32 = 0.8;

/// Accumulate a harmonic pitch-salience curve from the frame's spectral peaks.
///
/// The salience grid is logarithmic: bin `b` is the pitch
/// `f_min_hz * 2^(b / (12 * bins_per_semitone))`, i.e. a constant
/// `100 / bins_per_semitone` cents per bin. A peak at `f_i` with magnitude `m_i`
/// contributes, for each harmonic number `h`, to the candidate fundamental
/// `f0 = f_i / h`: the vote `m_i * HARMONIC_ALPHA^(h-1)` is spread over the grid
/// bins within one semitone of `f0` with a `cos²` window (zero at ±1 semitone),
/// which softens grid quantisation without smearing across notes.
///
/// - `peak_freqs` / `peak_mags`: the frame's spectral peaks (Hz, linear
///   magnitude), as produced by [`crate::features::peaks::spectral_peaks`]. Only
///   the first `n_peaks` entries are read.
/// - `f_min_hz`: frequency of salience bin 0 (the low edge of the pitch range).
/// - `bins_per_semitone`: grid resolution (e.g. `10.0` → 10 cents per bin).
/// - `n_bins`: number of salience bins to fill.
/// - `out_salience`: caller buffer, cleared then filled in `[0, n_bins)`.
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `f_min_hz`, `bins_per_semitone` are not
///   positive finite values.
/// - [`AudioError::OutputBufferTooSmall`] if `out_salience.len() < n_bins`.
pub fn pitch_salience(
    peak_freqs: &[f32],
    peak_mags: &[f32],
    n_peaks: usize,
    f_min_hz: f32,
    bins_per_semitone: f32,
    n_bins: usize,
    out_salience: &mut [f32],
) -> Result<(), AudioError> {
    if !(f_min_hz.is_finite()
        && f_min_hz > 0.0
        && bins_per_semitone.is_finite()
        && bins_per_semitone > 0.0)
    {
        return Err(AudioError::InvalidParameter);
    }
    if out_salience.len() < n_bins {
        return Err(AudioError::OutputBufferTooSmall);
    }
    // Clear the active range (caller buffer may be reused across frames).
    for s in out_salience.iter_mut().take(n_bins) {
        *s = 0.0;
    }
    if n_bins == 0 {
        return Ok(());
    }

    let cap = peak_freqs.len().min(peak_mags.len()).min(n_peaks);

    for i in 0..cap {
        let f = peak_freqs[i];
        let m = peak_mags[i];
        if !(f.is_finite() && f > 0.0 && m.is_finite() && m > 0.0) {
            continue;
        }
        let mut hw = 1.0f32; // HARMONIC_ALPHA^(h-1)
        for h in 1..=N_HARMONICS {
            let f0 = f / h as f32;
            if f0 < f_min_hz {
                break; // f0 only decreases as h grows → no further in-range candidates
            }
            // Fractional grid position of this candidate fundamental.
            let b_center = 12.0 * bins_per_semitone * (f0 / f_min_hz).log2();
            let lo = (b_center - bins_per_semitone).ceil();
            let hi = (b_center + bins_per_semitone).floor();
            if hi < 0.0 || lo >= n_bins as f32 {
                hw *= HARMONIC_ALPHA;
                continue;
            }
            let b0 = lo.max(0.0) as usize;
            let b1 = (hi.min((n_bins - 1) as f32)) as usize;
            for b in b0..=b1 {
                // Distance in semitones from the candidate fundamental.
                let d_semi = (b as f32 - b_center) / bins_per_semitone;
                if d_semi.abs() >= 1.0 {
                    continue;
                }
                let c = (d_semi * core::f32::consts::FRAC_PI_2).cos();
                let w = c * c; // cos² window, 1 at centre → 0 at ±1 semitone
                out_salience[b] += m * hw * w;
            }
            hw *= HARMONIC_ALPHA;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Convert a fundamental frequency to its (fractional) salience bin.
    fn hz_to_bin(f: f32, f_min: f32, bps: f32) -> f32 {
        12.0 * bps * (f / f_min).log2()
    }

    /// Index of the maximum over `[0, n)`.
    fn argmax(s: &[f32], n: usize) -> usize {
        let mut bi = 0usize;
        let mut bv = s[0];
        for i in 1..n {
            if s[i] > bv {
                bv = s[i];
                bi = i;
            }
        }
        bi
    }

    /// GOLDEN: a harmonic tone at 220 Hz (peaks at 220/440/660) must produce a
    /// salience curve whose global maximum sits at 220 Hz's bin — the harmonics
    /// all vote onto the true fundamental.
    #[test]
    fn salience_peaks_at_fundamental() {
        let f_min = 55.0f32;
        let bps = 10.0f32; // 10 cents/bin
        let n_bins = 720usize; // 6 octaves
        let f0 = 220.0f32;
        let peak_freqs = [f0, 2.0 * f0, 3.0 * f0];
        let peak_mags = [1.0f32, 0.6, 0.4];
        let mut sal = vec![0.0f32; n_bins];
        pitch_salience(&peak_freqs, &peak_mags, 3, f_min, bps, n_bins, &mut sal)
            .expect("salience");

        let want = hz_to_bin(f0, f_min, bps).round() as usize; // 240
        let got = argmax(&sal, n_bins);
        assert_eq!(want, 240, "expected fundamental bin math");
        assert!(
            (got as i32 - want as i32).abs() <= 1,
            "peak bin={got} want≈{want}"
        );
        // The fundamental collects all three harmonics → strictly the strongest.
        let f2bin = hz_to_bin(2.0 * f0, f_min, bps).round() as usize;
        assert!(
            sal[got] > sal[f2bin],
            "fundamental {} !> octave {}",
            sal[got],
            sal[f2bin]
        );
    }

    #[test]
    fn rejects_bad_params() {
        let mut sal = [0.0f32; 4];
        assert_eq!(
            pitch_salience(&[100.0], &[1.0], 1, 0.0, 10.0, 4, &mut sal),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            pitch_salience(&[100.0], &[1.0], 1, 55.0, 0.0, 4, &mut sal),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn rejects_small_buffer() {
        let mut sal = [0.0f32; 3];
        assert_eq!(
            pitch_salience(&[100.0], &[1.0], 1, 55.0, 10.0, 8, &mut sal),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    /// A silent frame (no peaks) leaves an all-zero salience curve.
    #[test]
    fn silent_frame_is_zero() {
        let mut sal = vec![9.0f32; 32];
        pitch_salience(&[], &[], 0, 55.0, 10.0, 32, &mut sal).expect("ok");
        assert!(sal.iter().all(|&v| v == 0.0));
    }
}
