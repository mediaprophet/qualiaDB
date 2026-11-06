//! Frequency ↔ MIDI note-number conversion with parameterised reference tuning,
//! delegated to [`etalon_signal::pitch`].

/// The lowest legal MIDI note number.
pub const MIDI_MIN: u8 = 0;
/// The highest legal MIDI note number.
pub const MIDI_MAX: u8 = 127;

/// Convert a frequency in Hz to `(midi_note, cents_deviation)`.
#[inline]
pub fn hz_to_midi(freq_hz: f32, ref_a4_hz: f32) -> (u8, f32) {
    etalon_signal::pitch::hz_to_midi_quantized(freq_hz, ref_a4_hz)
}

/// Convert an integer MIDI note number back to its frequency in Hz.
#[inline]
pub fn midi_to_hz(note: u8, ref_a4_hz: f32) -> f32 {
    etalon_signal::pitch::midi_note_to_hz(note, ref_a4_hz)
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
        let (hi, _) = hz_to_midi(40_000.0, 440.0);
        assert_eq!(hi, 127);
        let (lo, _) = hz_to_midi(1.0, 440.0);
        assert_eq!(lo, 0);
    }
}
