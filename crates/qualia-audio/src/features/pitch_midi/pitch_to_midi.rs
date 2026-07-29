//! Frequency ↔ MIDI note-number conversion with parameterised reference tuning.
//!
//! The equal-tempered mapping is `midi = 69 + 12·log2(f / ref_a4)`, i.e. MIDI
//! note 69 (A4) sits at the reference pitch (default 440 Hz) and every octave is
//! 12 semitones of 100 cents each. [`hz_to_midi`] returns the nearest integer
//! note plus the signed cents deviation of the true pitch from that note;
//! [`midi_to_hz`] is the exact inverse for an integer note.
//!
//! Zero-heap: both functions are pure arithmetic on scalars — nothing is
//! allocated. A transcribed note is an epistemic *proposal*; the cents value
//! preserves the sub-semitone truth so downstream code never mistakes the
//! quantised note for the measured pitch.

/// The lowest legal MIDI note number.
pub const MIDI_MIN: u8 = 0;
/// The highest legal MIDI note number.
pub const MIDI_MAX: u8 = 127;
/// MIDI note number of the A4 reference pitch.
const A4_MIDI: f32 = 69.0;

/// Convert a frequency in Hz to `(midi_note, cents_deviation)`.
///
/// - `freq_hz`: measured fundamental in Hz.
/// - `ref_a4_hz`: reference tuning for A4 (default concert pitch is `440.0`).
///
/// The returned note is `round(69 + 12·log2(freq/ref))` clamped into the legal
/// MIDI range `[0, 127]`. `cents` is `(exact_midi − note) · 100`, the signed
/// deviation of the true pitch from the returned note (positive = sharp). For a
/// note struck exactly in tune `cents ≈ 0`; for a saturated (out-of-range) pitch
/// the note clamps and `cents` reports the residual honestly rather than
/// silently losing it.
///
/// Out-of-range handling: a non-finite or non-positive `freq_hz`/`ref_a4_hz`
/// yields `(0, 0.0)` — there is no meaningful note for a silent or invalid
/// frame, and the signature is infallible by contract.
#[inline]
pub fn hz_to_midi(freq_hz: f32, ref_a4_hz: f32) -> (u8, f32) {
    if !freq_hz.is_finite() || freq_hz <= 0.0 || !ref_a4_hz.is_finite() || ref_a4_hz <= 0.0 {
        return (0, 0.0);
    }
    // Exact (fractional) MIDI note number.
    let exact = A4_MIDI + 12.0 * (freq_hz / ref_a4_hz).log2();
    // Nearest integer note, clamped to the legal MIDI range.
    let rounded = exact.round();
    let note = rounded.clamp(MIDI_MIN as f32, MIDI_MAX as f32) as u8;
    // Cents relative to the (possibly clamped) note; saturation surfaces here.
    let cents = (exact - note as f32) * 100.0;
    (note, cents)
}

/// Convert an integer MIDI note number back to its frequency in Hz.
///
/// `freq = ref_a4 · 2^((note − 69) / 12)`. This is the exact inverse of the
/// integer part of [`hz_to_midi`]. A non-finite or non-positive `ref_a4_hz`
/// yields `0.0`.
#[inline]
pub fn midi_to_hz(note: u8, ref_a4_hz: f32) -> f32 {
    if !ref_a4_hz.is_finite() || ref_a4_hz <= 0.0 {
        return 0.0;
    }
    ref_a4_hz * ((note as f32 - A4_MIDI) / 12.0).exp2()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a4_maps_to_69_in_tune() {
        let (note, cents) = hz_to_midi(440.0, 440.0);
        assert_eq!(note, 69);
        assert!(cents.abs() < 1e-3, "cents={cents}");
    }

    #[test]
    fn middle_c_maps_to_60() {
        let (note, cents) = hz_to_midi(261.63, 440.0);
        assert_eq!(note, 60);
        assert!(cents.abs() < 1.0, "cents={cents}");
    }

    #[test]
    fn sharp_442_is_69_plus_785_cents() {
        let (note, cents) = hz_to_midi(442.0, 440.0);
        assert_eq!(note, 69);
        assert!((cents - 7.85).abs() < 0.05, "cents={cents}");
    }

    #[test]
    fn alternate_reference_shifts_pitch() {
        // With A4 = 442, a 442 Hz tone is exactly note 69, 0 cents.
        let (note, cents) = hz_to_midi(442.0, 442.0);
        assert_eq!(note, 69);
        assert!(cents.abs() < 1e-3, "cents={cents}");
    }

    #[test]
    fn midi_to_hz_inverts_hz_to_midi() {
        for &note in &[21u8, 60, 69, 108] {
            let f = midi_to_hz(note, 440.0);
            let (back, cents) = hz_to_midi(f, 440.0);
            assert_eq!(back, note, "note={note} f={f}");
            assert!(cents.abs() < 1e-2, "cents={cents}");
        }
    }

    #[test]
    fn a4_frequency_is_440() {
        assert!((midi_to_hz(69, 440.0) - 440.0).abs() < 1e-3);
    }

    #[test]
    fn out_of_range_inputs_are_safe() {
        assert_eq!(hz_to_midi(0.0, 440.0), (0, 0.0));
        assert_eq!(hz_to_midi(-5.0, 440.0), (0, 0.0));
        assert_eq!(hz_to_midi(f32::NAN, 440.0), (0, 0.0));
        assert_eq!(hz_to_midi(440.0, 0.0), (0, 0.0));
        assert_eq!(midi_to_hz(69, 0.0), 0.0);
    }

    #[test]
    fn extreme_frequencies_clamp_to_midi_range() {
        // Far above the MIDI range saturates at 127, not a wraparound.
        let (hi, _) = hz_to_midi(40_000.0, 440.0);
        assert_eq!(hi, 127);
        // Far below saturates at 0.
        let (lo, _) = hz_to_midi(1.0, 440.0);
        assert_eq!(lo, 0);
    }
}
