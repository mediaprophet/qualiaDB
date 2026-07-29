//! Harmonic Pitch Class Profile (HPCP) from spectral peaks.
//!
//! This replaces the fake `bin % 12` "chroma" that used to live in `music.rs`.
//! Each spectral peak `(freq_hz, magnitude)` is mapped to a **real** pitch class
//! via `log2(f / ref_freq)` — an equal division of the octave into `n_pc` bins —
//! and its magnitude is accumulated into the nearest pitch-class bin.
//!
//! # Tuning is a declared parameter, not a universal truth
//! - `ref_freq_hz` is the **tuning reference** (the frequency of concert A, A4);
//!   it defaults to 440 Hz by convention only — the caller passes whatever the
//!   material actually uses (see [`super::tuning`] to estimate it). Nothing here
//!   hardcodes 440.
//! - `n_pc` is the number of pitch classes per octave. 12 is the 12-TET case, but
//!   any `n_pc ≥ 1` (e.g. 24 for quarter-tones, or a non-12 microtonal system) is
//!   accepted. Qualia is modality-first and does not privilege 12-TET.
//!
//! # Pitch-class origin
//! Bin 0 is the **C-equivalent** origin: with the chromatic convention, A sits
//! `0.75` of an octave (9 of 12 semitones) above C, so the reference A maps to bin
//! `0.75 * n_pc` and bin 0 corresponds to `ref_freq * 2^(-0.75)`. This origin is a
//! labelling convention for the 12-TET chromatic layout; the frequency→bin
//! mapping itself is fully general for any `n_pc`.
//!
//! Zero-heap: the profile is written into caller-owned `out`; no allocation.

use crate::types::AudioError;

/// A (concert A) lies this fraction of an octave above the C-origin bin 0
/// (9 semitones / 12 = 0.75). Used to place the tuning reference correctly.
const A_ABOVE_C_OCTAVES: f32 = 0.75;

/// Accumulate spectral peaks into a Harmonic Pitch Class Profile.
///
/// - `peak_freqs` / `peak_mags`: parallel arrays of peak frequency (Hz) and
///   magnitude, e.g. from [`crate::features::peaks::spectral_peaks`].
/// - `n_peaks`: how many leading entries of those arrays to use (clamped to the
///   shorter of the two).
/// - `ref_freq_hz`: tuning reference frequency (frequency of A4); **parameterised**,
///   default-by-convention 440 Hz — never assume it.
/// - `n_pc`: pitch classes per octave (12 for 12-TET; any `≥ 1` accepted).
/// - `out`: destination profile; the first `n_pc` entries are overwritten with the
///   accumulated magnitudes (raw, not normalised).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `ref_freq_hz` is not positive-finite, or
///   `n_pc == 0`.
/// - [`AudioError::OutputBufferTooSmall`] if `out.len() < n_pc`.
pub fn hpcp(
    peak_freqs: &[f32],
    peak_mags: &[f32],
    n_peaks: usize,
    ref_freq_hz: f32,
    n_pc: usize,
    out: &mut [f32],
) -> Result<(), AudioError> {
    if !(ref_freq_hz > 0.0) || !ref_freq_hz.is_finite() || n_pc == 0 {
        return Err(AudioError::InvalidParameter);
    }
    if out.len() < n_pc {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for slot in out.iter_mut().take(n_pc) {
        *slot = 0.0;
    }

    let np = n_peaks.min(peak_freqs.len()).min(peak_mags.len());
    let bins = n_pc as f32;
    // Where the reference A lands, in bins above the C-origin.
    let a_offset = A_ABOVE_C_OCTAVES * bins;

    for k in 0..np {
        let f = peak_freqs[k];
        let m = peak_mags[k];
        if !(f > 0.0) || !f.is_finite() || !(m > 0.0) || !m.is_finite() {
            continue;
        }
        // Bins above the reference A, then shift to the C origin.
        let rel = bins * (f / ref_freq_hz).log2();
        let mut b = (rel + a_offset) % bins;
        if b < 0.0 {
            b += bins;
        }
        // Nearest pitch-class bin (round may reach `n_pc`; fold with `% n_pc`).
        let idx = (b.round() as usize) % n_pc;
        out[idx] += m;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Equal-tempered note frequency `n` semitones from the A4 reference.
    fn semis(a4: f32, n: i32) -> f32 {
        a4 * 2f32.powf(n as f32 / 12.0)
    }

    /// GOLDEN: a C-major triad (C4, E4, G4) at the standard 440 Hz reference lands
    /// exactly on pitch classes C=0, E=4, G=7 — and those three bins dominate.
    #[test]
    fn golden_c_major_triad_peaks_at_c_e_g() {
        let a4 = 440.0f32;
        // C4 = -9, E4 = -5, G4 = -2 semitones from A4.
        let freqs = [semis(a4, -9), semis(a4, -5), semis(a4, -2)];
        let mags = [1.0f32, 0.9, 0.8];
        let mut out = [0.0f32; 12];
        hpcp(&freqs, &mags, 3, a4, 12, &mut out).expect("hpcp");

        // The chord tones carry all the energy.
        assert!(out[0] > 0.99 && out[0] < 1.01, "C bin = {}", out[0]);
        assert!(out[4] > 0.89 && out[4] < 0.91, "E bin = {}", out[4]);
        assert!(out[7] > 0.79 && out[7] < 0.81, "G bin = {}", out[7]);
        // Every non-chord pitch class is strictly smaller than each chord tone.
        for (pc, &v) in out.iter().enumerate() {
            if pc != 0 && pc != 4 && pc != 7 {
                assert!(v < out[7], "pc {pc} = {v} not below G bin {}", out[7]);
            }
        }
    }

    /// Non-440 tuning: the same triad synthesised at A4 = 432 Hz still maps to
    /// C/E/G when `ref_freq_hz` is told the real tuning — tuning is parameterised.
    #[test]
    fn respects_non_440_reference() {
        let a4 = 432.0f32;
        let freqs = [semis(a4, -9), semis(a4, -5), semis(a4, -2)];
        let mags = [1.0f32, 1.0, 1.0];
        let mut out = [0.0f32; 12];
        hpcp(&freqs, &mags, 3, a4, 12, &mut out).expect("hpcp");
        assert!(out[0] > 0.99 && out[4] > 0.99 && out[7] > 0.99);
    }

    /// `n_pc` is a real parameter: a 24-bin (quarter-tone) profile places A at
    /// bin 18 (= 0.75 * 24) and is not tied to 12.
    #[test]
    fn n_pc_is_parameterised() {
        let a4 = 440.0f32;
        let freqs = [a4];
        let mags = [1.0f32];
        let mut out = [0.0f32; 24];
        hpcp(&freqs, &mags, 1, a4, 24, &mut out).expect("hpcp");
        assert!(out[18] > 0.99, "A should be at bin 18 in a 24-bin profile");
    }

    #[test]
    fn rejects_bad_params() {
        let mut out = [0.0f32; 12];
        assert_eq!(
            hpcp(&[440.0], &[1.0], 1, 0.0, 12, &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            hpcp(&[440.0], &[1.0], 1, 440.0, 0, &mut out),
            Err(AudioError::InvalidParameter)
        );
        let mut tiny = [0.0f32; 4];
        assert_eq!(
            hpcp(&[440.0], &[1.0], 1, 440.0, 12, &mut tiny),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    /// Non-positive / non-finite peaks are skipped, not accumulated.
    #[test]
    fn skips_invalid_peaks() {
        let freqs = [0.0f32, -10.0, f32::NAN, 440.0];
        let mags = [1.0f32, 1.0, 1.0, 1.0];
        let mut out = [0.0f32; 12];
        hpcp(&freqs, &mags, 4, 440.0, 12, &mut out).expect("hpcp");
        let total: f32 = out.iter().sum();
        assert!(
            (total - 1.0).abs() < 1e-6,
            "only the 440 peak counts, total={total}"
        );
    }
}
