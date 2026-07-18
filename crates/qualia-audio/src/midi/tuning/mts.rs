//! MIDI Tuning Standard (MTS) — clean-room from the open MMA spec.
//!
//! Two concerns, both native and heap-free on the hot path:
//!
//! 1. **Tuning table.** [`build_tuning_table`] combines a Scala [`SclScale`] and [`KbmMapping`]
//!    into a fixed `[f64; 128]` of key→frequency. [`note_frequency`] is an allocation-free,
//!    panic-free lookup into that table. Tuning is fully PARAMETERISED — the table is a function
//!    of the supplied scale + mapping; 12-TET is just one possible input, never hardcoded.
//!
//! 2. **Single Note Tuning Change** SysEx (MTS sub-ID 0x02) — [`encode_single_note_tuning`] /
//!    [`decode_single_note_tuning`]. Frequency is carried as the MTS 3-byte value: a nearest
//!    equal-tempered semitone (MIDI note, `8.175799 Hz = note 0`) plus a 14-bit fraction of
//!    100 cents.

use super::scala_kbm::KbmMapping;
use super::scala_scl::SclScale;
use crate::types::AudioError;

/// Floor of `a / b` for a positive divisor `b` (Rust `/` truncates toward zero).
#[inline]
fn floor_div(a: i32, b: i32) -> i32 {
    let mut q = a / b;
    if a % b != 0 && (a < 0) != (b < 0) {
        q -= 1;
    }
    q
}

/// Cumulative cents (relative to the mapping's `middle_note` = `1/1`) of an ABSOLUTE scale
/// degree `d`, wrapping every `octave_degree` degrees by the scale's period.
#[inline]
fn absolute_degree_cents(scale: &SclScale, octave_degree: usize, d: i32) -> f64 {
    let oct = octave_degree as i32;
    let periods = floor_div(d, oct);
    let residue = (d - periods * oct) as usize; // 0..octave_degree
    let period_cents = scale.degree_cents(octave_degree);
    scale.degree_cents(residue) + (periods as f64) * period_cents
}

/// Build the 128-key frequency table from a scale + keyboard mapping, per MTS.
///
/// Keys the mapping does not cover (unmapped `x`, or outside `[first_midi, last_midi]`) are set
/// to `0.0` (a "no note" sentinel). Anchoring is exact: `reference_note` receives `reference_freq`,
/// and every other key is `reference_freq · 2^((cents(key) − cents(reference))/1200)`.
///
/// If the reference key itself is not mapped the table cannot be anchored and an all-zero table
/// is returned (build never panics).
pub fn build_tuning_table(scale: &SclScale, kbm: &KbmMapping) -> [f64; 128] {
    let mut table = [0.0_f64; 128];

    let ref_degree = match kbm.absolute_degree(kbm.reference_note) {
        Some(d) => d,
        None => return table, // cannot anchor
    };
    let ref_cents = absolute_degree_cents(scale, kbm.octave_degree, ref_degree);

    for (note, slot) in table.iter_mut().enumerate() {
        if let Some(d) = kbm.absolute_degree(note as u8) {
            let c = absolute_degree_cents(scale, kbm.octave_degree, d);
            *slot = kbm.reference_freq * 2.0_f64.powf((c - ref_cents) / 1200.0);
        }
    }
    table
}

/// Allocation-free, panic-free key→frequency lookup. `note` is masked into `0..128`, so any
/// `u8` is valid; an unmapped key returns `0.0`.
#[inline]
pub fn note_frequency(table: &[f64; 128], note: u8) -> f64 {
    table[(note & 0x7f) as usize]
}

// ---------------------------------------------------------------------------
// MTS Single Note Tuning Change (real-time SysEx, sub-ID#1 = 0x08, sub-ID#2 = 0x02)
// ---------------------------------------------------------------------------

/// Reference frequency of MIDI note 0 under MTS's equal-tempered frequency encoding
/// (`8.175799 Hz`, i.e. A4 = 440 Hz ⇒ note 69).
const MTS_NOTE0_HZ: f64 = 8.175_798_915_643_71;

/// Byte length of a one-change Single Note Tuning Change SysEx.
pub const SNTC_LEN: usize = 12;

/// Encode a single-note tuning change as a 12-byte real-time SysEx:
/// `F0 7F <dev> 08 02 <program> 01 <note> <xx yy zz> F7`.
///
/// The 3 frequency bytes are `xx` = nearest ET semitone (0..127), then a 14-bit fraction of one
/// semitone as `yy` (high 7 bits) and `zz` (low 7 bits).
///
/// # Errors
/// [`AudioError::InvalidParameter`] if `device_id`, `program`, or `note` exceed 7 bits, or `freq`
/// is not a positive, in-range frequency.
pub fn encode_single_note_tuning(
    device_id: u8,
    program: u8,
    note: u8,
    freq: f64,
) -> Result<[u8; SNTC_LEN], AudioError> {
    if device_id > 0x7f || program > 0x7f || note > 0x7f {
        return Err(AudioError::InvalidParameter);
    }
    if !(freq.is_finite() && freq > 0.0) {
        return Err(AudioError::InvalidParameter);
    }

    // Fractional MIDI semitone number for this frequency.
    let semis = 12.0 * (freq / MTS_NOTE0_HZ).log2();
    if !(semis.is_finite() && semis >= 0.0 && semis < 128.0) {
        return Err(AudioError::InvalidParameter);
    }
    let xx = semis.floor() as i64;
    // 14-bit fraction of a semitone.
    let frac = semis - xx as f64;
    let mut f14 = (frac * 16384.0).round() as i64;
    let mut xx = xx;
    if f14 >= 16384 {
        f14 -= 16384;
        xx += 1;
    }
    if !(0..=127).contains(&xx) {
        return Err(AudioError::InvalidParameter);
    }
    let yy = ((f14 >> 7) & 0x7f) as u8;
    let zz = (f14 & 0x7f) as u8;

    Ok([
        0xF0, 0x7F, device_id, 0x08, 0x02, program, 0x01, note, xx as u8, yy, zz, 0xF7,
    ])
}

/// A decoded single-note tuning change.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SingleNoteTuning {
    pub device_id: u8,
    pub program: u8,
    pub note: u8,
    pub freq: f64,
}

/// Decode a one-change Single Note Tuning Change SysEx produced by [`encode_single_note_tuning`].
///
/// # Errors
/// [`AudioError::InvalidParameter`] if the framing/header is wrong, the change count is not 1,
/// or the message length is unexpected.
pub fn decode_single_note_tuning(msg: &[u8]) -> Result<SingleNoteTuning, AudioError> {
    if msg.len() != SNTC_LEN {
        return Err(AudioError::InvalidParameter);
    }
    if msg[0] != 0xF0
        || msg[1] != 0x7F
        || msg[3] != 0x08
        || msg[4] != 0x02
        || msg[6] != 0x01
        || msg[11] != 0xF7
    {
        return Err(AudioError::InvalidParameter);
    }
    let device_id = msg[2];
    let program = msg[5];
    let note = msg[7];
    let xx = msg[8];
    let yy = msg[9];
    let zz = msg[10];
    if device_id > 0x7f || program > 0x7f || note > 0x7f || xx > 0x7f || yy > 0x7f || zz > 0x7f {
        return Err(AudioError::InvalidParameter);
    }
    let f14 = ((yy as i64) << 7) | (zz as i64);
    let semis = xx as f64 + (f14 as f64) / 16384.0;
    let freq = MTS_NOTE0_HZ * 2.0_f64.powf(semis / 12.0);
    Ok(SingleNoteTuning {
        device_id,
        program,
        note,
        freq,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::tuning::scala_kbm::{parse_kbm, KbmMapping};
    use crate::midi::tuning::scala_scl::{parse_scl, SclScale};

    #[test]
    fn tuning_twelve_tet_reference_a440() {
        let scale = SclScale::twelve_tet();
        let kbm = KbmMapping::linear_12(440.0);
        let table = build_tuning_table(&scale, &kbm);

        // Note 69 (A4) is the reference — exactly 440.
        assert!((note_frequency(&table, 69) - 440.0).abs() < 1e-3);
        // Middle C (note 60) ≈ 261.626 Hz.
        assert!((note_frequency(&table, 60) - 261.6255653).abs() < 1e-3);
        // One octave above A4 (note 81) == 880.
        assert!((note_frequency(&table, 81) - 880.0).abs() < 1e-3);
        // One octave below (note 57) == 220.
        assert!((note_frequency(&table, 57) - 220.0).abs() < 1e-3);
    }

    #[test]
    fn tuning_detuned_reference_a432() {
        let scale = SclScale::twelve_tet();
        let kbm = KbmMapping::linear_12(432.0);
        let table = build_tuning_table(&scale, &kbm);
        assert!((note_frequency(&table, 69) - 432.0).abs() < 1e-3);
        // The whole grid scales by 432/440.
        assert!((note_frequency(&table, 81) - 864.0).abs() < 1e-3);
    }

    #[test]
    fn tuning_scl_plus_kbm_end_to_end_equals_equal_temperament() {
        // Parsed from text (not the constructor) to prove the parse path drives the table.
        let scl = "12tet\n12\n100.\n200.\n300.\n400.\n500.\n600.\n700.\n800.\n900.\n1000.\n1100.\n1200.\n";
        let kbm_txt = "12\n0\n127\n60\n69\n440.0\n12\n0\n1\n2\n3\n4\n5\n6\n7\n8\n9\n10\n11\n";
        let scale = parse_scl(scl).unwrap();
        let kbm = parse_kbm(kbm_txt).unwrap();
        let table = build_tuning_table(&scale, &kbm);
        assert!((note_frequency(&table, 69) - 440.0).abs() < 1e-3);
        assert!((note_frequency(&table, 60) - 261.6255653).abs() < 1e-3);
    }

    #[test]
    fn tuning_non_12tet_just_scale_perfect_fifth() {
        // A tuning where degree 7's fifth is a JUST 3/2 rather than 12-TET's 2^(7/12).
        // Full 12-note just-ish scale; degree 7 (key 67, G) is 3/2 above middle C's 1/1.
        let scl = "just\n12\n16/15\n9/8\n6/5\n5/4\n4/3\n45/32\n3/2\n8/5\n5/3\n9/5\n15/8\n2/1\n";
        let scale = parse_scl(scl).unwrap();
        // Anchor 1/1 at middle_note = 60 (C), reference the same C at 264 Hz for a clean check.
        let kbm = KbmMapping::linear(12, 60, 60, 264.0);
        let table = build_tuning_table(&scale, &kbm);
        // Key 67 = degree 7 = 3/2 above the 1/1 at key 60.
        let c = note_frequency(&table, 60);
        let g = note_frequency(&table, 67);
        assert!((c - 264.0).abs() < 1e-3);
        assert!((g / c - 1.5).abs() < 1e-9, "just fifth must be exactly 3/2, got {}", g / c);
        // And it differs from 12-TET's tempered fifth.
        let tempered = 2.0_f64.powf(7.0 / 12.0);
        assert!((g / c - tempered).abs() > 1e-3);
    }

    #[test]
    fn tuning_note_frequency_lookup_is_masked_and_total() {
        let table = build_tuning_table(&SclScale::twelve_tet(), &KbmMapping::linear_12(440.0));
        // note & 0x7f keeps any u8 in range; 197 & 0x7f == 69.
        assert_eq!(note_frequency(&table, 197), note_frequency(&table, 69));
    }

    #[test]
    fn tuning_mts_single_note_roundtrip() {
        let msg = encode_single_note_tuning(0, 0, 69, 440.0).unwrap();
        assert_eq!(msg.len(), SNTC_LEN);
        assert_eq!(msg[0], 0xF0);
        assert_eq!(msg[11], 0xF7);
        let d = decode_single_note_tuning(&msg).unwrap();
        assert_eq!(d.note, 69);
        assert!((d.freq - 440.0).abs() < 0.05, "decoded {}", d.freq);

        // A detuned pitch survives the 14-bit-fraction round trip within one MTS step (~0.006 c).
        let f = 261.6255653 * 2.0_f64.powf(23.0 / 1200.0); // +23 cents
        let m = encode_single_note_tuning(3, 5, 60, f).unwrap();
        let back = decode_single_note_tuning(&m).unwrap();
        assert_eq!(back.device_id, 3);
        assert_eq!(back.program, 5);
        assert!((back.freq - f).abs() < 0.02, "roundtrip {} vs {}", back.freq, f);
    }

    #[test]
    fn tuning_mts_rejects_bad_frames() {
        assert!(decode_single_note_tuning(&[0u8; 4]).is_err());
        let mut msg = encode_single_note_tuning(0, 0, 69, 440.0).unwrap();
        msg[4] = 0x01; // wrong sub-ID#2
        assert!(decode_single_note_tuning(&msg).is_err());
        assert!(encode_single_note_tuning(0, 0, 69, -1.0).is_err());
    }
}
