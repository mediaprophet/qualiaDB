//! Scala keyboard mapping (`.kbm`) parsing — clean-room from the open Scala format spec.
//!
//! A `.kbm` maps MIDI key numbers onto scale degrees and anchors the whole tuning to a
//! reference key/frequency. Grammar (comment lines begin with `!`; each field is the first
//! token of its own non-comment line):
//!
//! ```text
//! <map size>        number of keyboard-map entries; the pattern repeats every this many keys
//! <first MIDI>      first MIDI note to retune (0..127)
//! <last MIDI>       last MIDI note to retune (0..127)
//! <middle note>     the key that scale degree 0 (1/1) is mapped to
//! <reference note>  the key whose frequency is given below
//! <reference freq>  frequency (Hz) to tune the reference note to
//! <octave degree>   scale degree treated as the formal octave (usually the scale size)
//! <map 0>           `mapsize` lines follow, each a scale degree or `x` (key not mapped)
//! ...
//! ```
//!
//! Cold path; the produced [`KbmMapping`] is fixed-size and `Copy` so it can feed the
//! allocation-free tuning-table build.

use crate::types::AudioError;

/// Upper bound on keyboard-map entries (a map cannot usefully exceed the 128 MIDI keys).
pub const MAX_KBM_ENTRIES: usize = 128;

/// Sentinel in [`KbmMapping::mapping`] for a key that is not mapped to any scale degree (`x`).
pub const KBM_UNMAPPED: i16 = -1;

/// A parsed Scala keyboard mapping. Fixed-size, `Copy`, heap-free.
#[derive(Debug, Clone, Copy)]
pub struct KbmMapping {
    /// Number of valid entries in `mapping` (the map repeats every `map_size` keys).
    pub map_size: usize,
    /// First MIDI key to retune.
    pub first_midi: u8,
    /// Last MIDI key to retune.
    pub last_midi: u8,
    /// The MIDI key that carries scale degree 0 (`1/1`).
    pub middle_note: u8,
    /// The MIDI key whose absolute frequency is pinned by `reference_freq`.
    pub reference_note: u8,
    /// Absolute frequency (Hz) assigned to `reference_note`.
    pub reference_freq: f64,
    /// Scale degree that constitutes the formal octave/period (typically the scale size).
    pub octave_degree: usize,
    /// Per-key scale degree, or [`KBM_UNMAPPED`]. Only `[0, map_size)` is meaningful.
    pub mapping: [i16; MAX_KBM_ENTRIES],
}

impl KbmMapping {
    /// A linear (identity) mapping: `map_size` keys map to scale degrees `0,1,…,map_size-1`,
    /// with `octave_degree == map_size`. This is the implicit default when no `.kbm` accompanies
    /// a scale. The reference key/frequency and middle key are explicit parameters — nothing is
    /// hardcoded; A4=440 is only the caller's choice, not a baked assumption.
    pub fn linear(map_size: usize, middle_note: u8, reference_note: u8, reference_freq: f64) -> KbmMapping {
        let n = map_size.min(MAX_KBM_ENTRIES).max(1);
        let mut mapping = [KBM_UNMAPPED; MAX_KBM_ENTRIES];
        for (i, m) in mapping.iter_mut().enumerate().take(n) {
            *m = i as i16;
        }
        KbmMapping {
            map_size: n,
            first_midi: 0,
            last_midi: 127,
            middle_note,
            reference_note,
            reference_freq,
            octave_degree: n,
            mapping,
        }
    }

    /// The canonical 12-key linear map anchored at A4 (`reference_note = 69 → reference_freq Hz`)
    /// with `middle_note = 60`. Pass the reference frequency explicitly (e.g. 440.0 or 432.0).
    pub fn linear_12(reference_freq: f64) -> KbmMapping {
        KbmMapping::linear(12, 60, 69, reference_freq)
    }

    /// Scale degree a MIDI key maps to under this keyboard map, or `None` if the key is
    /// unmapped or outside `[first_midi, last_midi]`. Returns the ABSOLUTE scale degree,
    /// i.e. `octave * octave_degree + degree_within_period` (may be negative for low keys).
    #[inline]
    pub fn absolute_degree(&self, note: u8) -> Option<i32> {
        if note < self.first_midi || note > self.last_midi {
            return None;
        }
        if self.map_size == 0 {
            return None;
        }
        let p = note as i32 - self.middle_note as i32;
        let size = self.map_size as i32;
        // Proper floor division / modulo (Rust `/`,`%` truncate toward zero).
        let mut octave = p / size;
        let mut index = p % size;
        if index < 0 {
            index += size;
            octave -= 1;
        }
        let degree = self.mapping[index as usize];
        if degree == KBM_UNMAPPED {
            return None;
        }
        Some(octave * self.octave_degree as i32 + degree as i32)
    }
}

/// Parse Scala `.kbm` text into a fixed-size [`KbmMapping`].
///
/// # Errors
/// [`AudioError::InvalidParameter`] if a required field line is missing/unparseable, the map size
/// exceeds [`MAX_KBM_ENTRIES`], a MIDI field is out of `0..=127`, or fewer map entries are present
/// than declared.
pub fn parse_kbm(text: &str) -> Result<KbmMapping, AudioError> {
    let mut data = text.lines().filter(|l| {
        let t = l.trim_start();
        !t.starts_with('!')
    });

    let map_size = next_usize(&mut data)?;
    if map_size == 0 || map_size > MAX_KBM_ENTRIES {
        return Err(AudioError::InvalidParameter);
    }
    let first_midi = next_u8(&mut data)?;
    let last_midi = next_u8(&mut data)?;
    let middle_note = next_u8(&mut data)?;
    let reference_note = next_u8(&mut data)?;
    let reference_freq = next_f64(&mut data)?;
    let octave_degree = next_usize(&mut data)?;
    if octave_degree == 0 || octave_degree > MAX_KBM_ENTRIES || last_midi < first_midi {
        return Err(AudioError::InvalidParameter);
    }
    if !(reference_freq.is_finite() && reference_freq > 0.0) {
        return Err(AudioError::InvalidParameter);
    }

    let mut mapping = [KBM_UNMAPPED; MAX_KBM_ENTRIES];
    for slot in mapping.iter_mut().take(map_size) {
        let line = data.next().ok_or(AudioError::InvalidParameter)?;
        let tok = line
            .split_whitespace()
            .next()
            .ok_or(AudioError::InvalidParameter)?;
        // `x` (any case) marks an unmapped key; otherwise a scale-degree index.
        if tok.eq_ignore_ascii_case("x") {
            *slot = KBM_UNMAPPED;
        } else {
            let d: i16 = tok.parse().map_err(|_| AudioError::InvalidParameter)?;
            if d < 0 {
                return Err(AudioError::InvalidParameter);
            }
            *slot = d;
        }
    }

    Ok(KbmMapping {
        map_size,
        first_midi,
        last_midi,
        middle_note,
        reference_note,
        reference_freq,
        octave_degree,
        mapping,
    })
}

fn next_token<'a>(data: &mut impl Iterator<Item = &'a str>) -> Result<&'a str, AudioError> {
    let line = data.next().ok_or(AudioError::InvalidParameter)?;
    line.split_whitespace()
        .next()
        .ok_or(AudioError::InvalidParameter)
}

fn next_usize<'a>(data: &mut impl Iterator<Item = &'a str>) -> Result<usize, AudioError> {
    next_token(data)?.parse().map_err(|_| AudioError::InvalidParameter)
}

fn next_u8<'a>(data: &mut impl Iterator<Item = &'a str>) -> Result<u8, AudioError> {
    let v: i32 = next_token(data)?.parse().map_err(|_| AudioError::InvalidParameter)?;
    if (0..=127).contains(&v) {
        Ok(v as u8)
    } else {
        Err(AudioError::InvalidParameter)
    }
}

fn next_f64<'a>(data: &mut impl Iterator<Item = &'a str>) -> Result<f64, AudioError> {
    next_token(data)?.parse().map_err(|_| AudioError::InvalidParameter)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_kbm_linear_12_defaults() {
        let k = KbmMapping::linear_12(440.0);
        assert_eq!(k.map_size, 12);
        assert_eq!(k.middle_note, 60);
        assert_eq!(k.reference_note, 69);
        assert_eq!(k.octave_degree, 12);
        assert!((k.reference_freq - 440.0).abs() < 1e-12);
        // Linear identity mapping.
        assert_eq!(k.absolute_degree(60), Some(0));
        assert_eq!(k.absolute_degree(69), Some(9));
        assert_eq!(k.absolute_degree(72), Some(12)); // one octave up
        assert_eq!(k.absolute_degree(48), Some(-12)); // one octave down (floor div)
    }

    #[test]
    fn tuning_parse_kbm_full_map() {
        let text = "\
! example.kbm
! size
 12
! first
 0
! last
 127
! middle
 60
! reference note
 69
! reference freq
 440.0
! octave degree
 12
! mapping
 0
 1
 2
 3
 4
 5
 6
 7
 8
 9
 10
 11
";
        let k = parse_kbm(text).expect("parse .kbm");
        assert_eq!(k.map_size, 12);
        assert_eq!(k.first_midi, 0);
        assert_eq!(k.last_midi, 127);
        assert_eq!(k.middle_note, 60);
        assert_eq!(k.reference_note, 69);
        assert!((k.reference_freq - 440.0).abs() < 1e-12);
        assert_eq!(k.octave_degree, 12);
        assert_eq!(k.absolute_degree(69), Some(9));
    }

    #[test]
    fn tuning_parse_kbm_unmapped_key() {
        let text = "\
2
0
127
60
69
440.0
2
0
x
";
        let k = parse_kbm(text).expect("parse .kbm with unmapped");
        assert_eq!(k.mapping[0], 0);
        assert_eq!(k.mapping[1], KBM_UNMAPPED);
        // Key that lands on the unmapped slot yields no degree.
        assert_eq!(k.absolute_degree(61), None);
        assert_eq!(k.absolute_degree(60), Some(0));
    }

    #[test]
    fn tuning_parse_kbm_rejects_bad_fields() {
        assert!(parse_kbm("0\n0\n127\n60\n69\n440\n12\n").is_err()); // zero size
        assert!(parse_kbm("1\n0\n200\n60\n69\n440\n1\n0\n").is_err()); // midi > 127
        assert!(parse_kbm("2\n0\n127\n60\n69\n440\n2\n0\n").is_err()); // short mapping
    }
}
