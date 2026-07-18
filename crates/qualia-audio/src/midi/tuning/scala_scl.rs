//! Scala scale file (`.scl`) parsing — clean-room from the open Scala format spec.
//!
//! A `.scl` file describes ONE octave/period of a scale as an ordered list of pitches
//! above the implicit `1/1` (0 cents). Grammar (comment lines begin with `!`):
//!
//! ```text
//! ! name.scl
//! <description text, may be blank>   ← first non-comment line
//! <count>                            ← number of note lines that follow
//! <pitch 1>                          ← a ratio `p/q` (or `p`) OR a cents value `x.y`
//! ...
//! <pitch count>                      ← the last pitch is the formal octave (e.g. 2/1 or 1200.0)
//! ```
//!
//! A pitch TOKEN is a ratio when it has no `.` (either `p/q` or a bare integer `p` ⇒ `p/1`),
//! and cents when it contains a `.`. Anything after the value on a line is a description and
//! is ignored. Degree 0 (`1/1`) is implicit and never listed.
//!
//! Parsing is a COLD path and may allocate transiently; the produced [`SclScale`] is a
//! fixed-size, `Copy`, heap-free value so the hot note→frequency build/lookup never allocates.

use crate::types::AudioError;

/// Upper bound on scale degrees. A `.scl` beyond this is rejected (bounded, DoS-safe).
pub const MAX_SCALE_NOTES: usize = 512;

/// One pitch entry of a Scala scale, kept in its ORIGINAL exact form so that a
/// just-intonation ratio (e.g. `3/2`) stays exact and a cents value stays a float.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SclEntry {
    /// A rational interval `num/den` above `1/1` (`den` is never 0).
    Ratio { num: u64, den: u64 },
    /// An interval given directly in cents (1200 cents = one octave). May be negative.
    Cents(f64),
}

impl SclEntry {
    /// Linear frequency ratio of this interval relative to `1/1`.
    #[inline]
    pub fn ratio(self) -> f64 {
        match self {
            SclEntry::Ratio { num, den } => num as f64 / den as f64,
            SclEntry::Cents(c) => 2.0_f64.powf(c / 1200.0),
        }
    }

    /// Size of this interval in cents relative to `1/1`.
    #[inline]
    pub fn cents(self) -> f64 {
        match self {
            SclEntry::Ratio { num, den } => 1200.0 * (num as f64 / den as f64).log2(),
            SclEntry::Cents(c) => c,
        }
    }
}

/// A parsed Scala scale: `count` pitch entries describing one period above the implicit `1/1`.
/// Fixed-size and `Copy` — no heap, safe to embed in the tuning-table build path.
#[derive(Debug, Clone, Copy)]
pub struct SclScale {
    entries: [SclEntry; MAX_SCALE_NOTES],
    count: usize,
}

impl SclScale {
    /// Number of pitch degrees (the value on the `.scl` count line). Degree 0 (`1/1`) is implicit,
    /// so the scale spans degrees `0..=count`, with degree `count` being the formal octave/period.
    #[inline]
    pub fn count(&self) -> usize {
        self.count
    }

    /// Entry for scale degree `degree` in `1..=count` (degree 1 is the first listed pitch).
    /// Returns `None` for degree 0 (which is always `1/1`) or out of range.
    #[inline]
    pub fn entry(&self, degree: usize) -> Option<SclEntry> {
        if degree == 0 || degree > self.count {
            None
        } else {
            Some(self.entries[degree - 1])
        }
    }

    /// Cumulative cents of scale degree `degree` relative to `1/1`.
    /// Degree 0 ⇒ 0.0; degree `d` in `1..=count` ⇒ the `d`-th listed pitch's cents.
    /// Degrees above `count` are clamped to the period (callers handle octave wrapping).
    #[inline]
    pub fn degree_cents(&self, degree: usize) -> f64 {
        if degree == 0 {
            0.0
        } else if degree <= self.count {
            self.entries[degree - 1].cents()
        } else {
            self.entries[self.count - 1].cents()
        }
    }

    /// 12-tone equal temperament: twelve equal 100-cent steps to the `2/1` octave.
    /// Parameterised like every other scale — the engine hardcodes NOTHING; this is merely a
    /// convenience constructor for the ubiquitous default.
    pub fn twelve_tet() -> SclScale {
        let mut entries = [SclEntry::Cents(0.0); MAX_SCALE_NOTES];
        for (i, e) in entries.iter_mut().enumerate().take(12) {
            e_set(e, i);
        }
        SclScale { entries, count: 12 }
    }
}

#[inline]
fn e_set(e: &mut SclEntry, i: usize) {
    // Degree i+1 ⇒ (i+1)*100 cents; degree 12 ⇒ 1200.0 (exact octave).
    *e = SclEntry::Cents(((i + 1) as f64) * 100.0);
}

/// Parse Scala `.scl` text into a fixed-size [`SclScale`].
///
/// # Errors
/// [`AudioError::InvalidParameter`] if the count line is missing/unparseable, the declared count
/// exceeds [`MAX_SCALE_NOTES`], a pitch line is malformed, or fewer pitch lines are present than declared.
pub fn parse_scl(text: &str) -> Result<SclScale, AudioError> {
    // Data lines are all lines whose first non-whitespace char is not '!'.
    let mut data = text.lines().filter(|l| {
        let t = l.trim_start();
        !t.starts_with('!')
    });

    // Line 1: description (may be blank) — consumed and ignored.
    let _description = data.next().ok_or(AudioError::InvalidParameter)?;

    // Line 2: note count.
    let count_line = data.next().ok_or(AudioError::InvalidParameter)?;
    let count: usize = count_line
        .split_whitespace()
        .next()
        .ok_or(AudioError::InvalidParameter)?
        .parse()
        .map_err(|_| AudioError::InvalidParameter)?;
    if count == 0 || count > MAX_SCALE_NOTES {
        return Err(AudioError::InvalidParameter);
    }

    let mut entries = [SclEntry::Cents(0.0); MAX_SCALE_NOTES];
    for slot in entries.iter_mut().take(count) {
        let line = data.next().ok_or(AudioError::InvalidParameter)?;
        *slot = parse_pitch(line)?;
    }

    Ok(SclScale { entries, count })
}

/// Parse a single `.scl` pitch line into an [`SclEntry`]. The value is the first
/// whitespace-delimited token; any trailing text is an ignored description.
fn parse_pitch(line: &str) -> Result<SclEntry, AudioError> {
    let tok = line
        .split_whitespace()
        .next()
        .ok_or(AudioError::InvalidParameter)?;

    if tok.contains('.') {
        // Cents value (may be negative, e.g. a descending step).
        let c: f64 = tok.parse().map_err(|_| AudioError::InvalidParameter)?;
        if !c.is_finite() {
            return Err(AudioError::InvalidParameter);
        }
        Ok(SclEntry::Cents(c))
    } else if let Some((n, d)) = tok.split_once('/') {
        let num: u64 = n.parse().map_err(|_| AudioError::InvalidParameter)?;
        let den: u64 = d.parse().map_err(|_| AudioError::InvalidParameter)?;
        if num == 0 || den == 0 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(SclEntry::Ratio { num, den })
    } else {
        // Bare integer ⇒ p/1.
        let num: u64 = tok.parse().map_err(|_| AudioError::InvalidParameter)?;
        if num == 0 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(SclEntry::Ratio { num, den: 1 })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tuning_twelve_tet_constructor_is_equal_tempered() {
        let s = SclScale::twelve_tet();
        assert_eq!(s.count(), 12);
        // Each degree is a whole number of 100-cent steps.
        for d in 1..=12 {
            assert!((s.degree_cents(d) - (d as f64) * 100.0).abs() < 1e-9);
        }
        // Formal octave is an exact 2/1.
        assert!((s.entry(12).unwrap().ratio() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn tuning_parse_12tet_scl_reproduces_equal_temperament() {
        let text = "\
! 12tet.scl
12 equal steps
 12
 100.0
 200.0
 300.0
 400.0
 500.0
 600.0
 700.0
 800.0
 900.0
 1000.0
 1100.0
 1200.0
";
        let s = parse_scl(text).expect("parse 12-TET .scl");
        assert_eq!(s.count(), 12);
        let step = 2.0_f64.powf(1.0 / 12.0);
        for d in 1..=12 {
            // Each semitone == 100 cents, and its ratio == 2^(d/12).
            assert!((s.degree_cents(d) - (d as f64) * 100.0).abs() < 1e-6);
            assert!((s.entry(d).unwrap().ratio() - step.powi(d as i32)).abs() < 1e-9);
        }
    }

    #[test]
    fn tuning_parse_pythagorean_ratio_fifth_is_exactly_three_halves() {
        // A just / Pythagorean snippet with a ratio line for the perfect fifth.
        let text = "\
! just.scl
Just intonation snippet
 3
 9/8
 3/2
 2/1
";
        let s = parse_scl(text).expect("parse just .scl");
        assert_eq!(s.count(), 3);
        // Degree 2 is the perfect fifth 3/2 — EXACT, proving non-12-TET support.
        let fifth = s.entry(2).unwrap();
        assert_eq!(fifth, SclEntry::Ratio { num: 3, den: 2 });
        assert!((fifth.ratio() - 1.5).abs() < 1e-12);
        assert!((fifth.cents() - 701.955).abs() < 1e-3);
        // Octave exact.
        assert!((s.entry(3).unwrap().ratio() - 2.0).abs() < 1e-12);
    }

    #[test]
    fn tuning_parse_comments_and_blank_description_are_handled() {
        let text = "\
! leading comment
\t
! another comment
 2
! mid comment
 100.0
 2/1
";
        let s = parse_scl(text).expect("blank description + comments");
        assert_eq!(s.count(), 2);
        assert!((s.degree_cents(1) - 100.0).abs() < 1e-9);
    }

    #[test]
    fn tuning_parse_rejects_bad_count_and_short_files() {
        assert!(parse_scl("desc\nnotanumber\n").is_err());
        assert!(parse_scl("desc\n3\n100.0\n").is_err()); // fewer pitches than declared
        assert!(parse_scl("desc\n0\n").is_err()); // zero notes
    }
}
