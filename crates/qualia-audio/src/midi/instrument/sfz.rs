//! SFZ text-instrument parser (clean-room from the open SFZ format; no vendored sfizz).
//!
//! Qualia ships NO sample content. This parses a USER-supplied `.sfz` text file into a
//! bounded set of key/velocity-mapped [`SfzRegion`]s. Each region only *references* a
//! sample by relative path (`sample_ref`); the bytes live in the user's hypermedia library
//! or a user/vendor content dir and are resolved later (see `resolver`).
//!
//! Supported subset (opcodes may appear at `<group>` scope and are inherited by the regions
//! that follow, or at `<region>` scope):
//!
//! - Headers: `<group>`, `<region>` (and `<global>`, treated like a group-level default).
//!   Any other header (`<control>`, `<curve>`, …) simply ends the current region.
//! - Opcodes: `sample=`, `lokey`/`hikey`/`key`, `lovel`/`hivel`, `pitch_keycenter`,
//!   `tune` (cents), `volume` (dB).
//!
//! Key values may be a MIDI number (`0..=127`) or a note name (`c4`, `F#3`, `bb2`, …) with the
//! SFZ/MIDI convention `c-1 = 0` ⇒ middle `c4 = 60`.
//!
//! Parsing is a COLD path (transient `Vec` allocation is fine). The produced
//! [`SfzInstrument`] borrows sample-path slices out of the input `text` (`sample_ref: &str`),
//! and the note→region LOOKUP over its regions is allocation-free (see `sample_map`).

use crate::types::AudioError;

/// Upper bound on regions in one instrument. A larger file is rejected (bounded, DoS-safe).
pub const MAX_REGIONS: usize = 4096;

/// One key/velocity-mapped region: a rectangle in (note, velocity) space plus playback params.
///
/// `sample_ref` borrows the relative sample path out of the parsed `text`; it is NOT loaded
/// here (Qualia ships no content). All ranges are inclusive.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SfzRegion<'a> {
    /// Lowest MIDI note this region responds to (inclusive).
    pub lokey: u8,
    /// Highest MIDI note this region responds to (inclusive).
    pub hikey: u8,
    /// Lowest velocity (inclusive).
    pub lovel: u8,
    /// Highest velocity (inclusive).
    pub hivel: u8,
    /// The unity/root key the sample was recorded at.
    pub pitch_keycenter: u8,
    /// Fine tune in cents (SFZ `tune`).
    pub tune_cents: i32,
    /// Gain in decibels (SFZ `volume`).
    pub volume_db: f32,
    /// Relative path to the sample file — a REFERENCE only, resolved by the caller.
    pub sample_ref: &'a str,
}

impl<'a> SfzRegion<'a> {
    /// Default region before any opcode is applied: full range, no sample.
    #[inline]
    const fn blank() -> Self {
        Self {
            lokey: 0,
            hikey: 127,
            lovel: 0,
            hivel: 127,
            pitch_keycenter: 60,
            tune_cents: 0,
            volume_db: 0.0,
            sample_ref: "",
        }
    }

    /// Does this region respond to `(note, velocity)`? (inclusive on all four bounds).
    #[inline]
    pub fn matches(&self, note: u8, velocity: u8) -> bool {
        note >= self.lokey && note <= self.hikey && velocity >= self.lovel && velocity <= self.hivel
    }
}

/// A parsed SFZ instrument: a bounded, ordered list of regions.
///
/// Regions borrow their `sample_ref` from the input text, so the instrument shares its
/// lifetime. The order is preserved (SFZ layering is first-match by authoring order).
#[derive(Debug, Clone)]
pub struct SfzInstrument<'a> {
    regions: Vec<SfzRegion<'a>>,
}

impl<'a> SfzInstrument<'a> {
    /// Regions in authoring order.
    #[inline]
    pub fn regions(&self) -> &[SfzRegion<'a>] {
        &self.regions
    }

    /// Number of regions.
    #[inline]
    pub fn len(&self) -> usize {
        self.regions.len()
    }

    /// True when the instrument has no regions.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }
}

/// Which header scope we are currently accumulating opcodes into.
#[derive(Clone, Copy, PartialEq)]
enum Scope {
    /// `<global>`/`<group>`: opcodes become inherited defaults for following regions.
    Group,
    /// `<region>`: opcodes apply to the region under construction.
    Region,
    /// A header we don't model — swallow its opcodes.
    Other,
}

/// Parse an SFZ text instrument into a bounded [`SfzInstrument`].
///
/// An empty file (or a file with no `<region>`) yields an empty instrument (`is_empty()`),
/// NOT an error — an instrument with zero playable regions is a valid, well-formed result.
/// Errors are returned only for genuinely malformed structure: a key/velocity value that is
/// not a parseable MIDI number or note name, or exceeding [`MAX_REGIONS`].
pub fn parse_sfz(text: &str) -> Result<SfzInstrument<'_>, AudioError> {
    // Flatten to an ordered token stream, stripping `//` comments per line. Tokens keep their
    // original `&str` so `sample=` values can borrow from `text`.
    let mut tokens: Vec<&str> = Vec::new();
    for line in text.lines() {
        let content = match line.find("//") {
            Some(i) => &line[..i],
            None => line,
        };
        for tok in content.split_whitespace() {
            tokens.push(tok);
        }
    }

    let mut regions: Vec<SfzRegion<'_>> = Vec::new();
    let mut group_default = SfzRegion::blank();
    let mut cur = SfzRegion::blank();
    let mut scope = Scope::Other;
    let mut region_open = false;

    let mut i = 0usize;
    while i < tokens.len() {
        let tok = tokens[i];

        if is_header(tok) {
            // A new header closes any region currently under construction.
            if region_open {
                regions.push(cur);
                if regions.len() > MAX_REGIONS {
                    return Err(AudioError::InvalidParameter);
                }
                region_open = false;
            }
            match header_name(tok) {
                HeaderKind::GroupLike => {
                    // A fresh group resets group-level defaults.
                    group_default = SfzRegion::blank();
                    scope = Scope::Group;
                }
                HeaderKind::Region => {
                    cur = group_default; // inherit current group defaults
                    region_open = true;
                    scope = Scope::Region;
                }
                HeaderKind::Other => {
                    scope = Scope::Other;
                }
            }
            i += 1;
            continue;
        }

        // Opcode `name=value`. `sample=` swallows following non-opcode tokens (paths may
        // contain spaces) until the next header or `name=` opcode.
        let Some(eq) = tok.find('=') else {
            // Stray token (e.g. a bare path fragment with no opcode) — ignore, cold-path robust.
            i += 1;
            continue;
        };
        let name = &tok[..eq];
        let first_val = &tok[eq + 1..];

        // Choose the target region for this opcode based on scope.
        let target: &mut SfzRegion<'_> = match scope {
            Scope::Group => &mut group_default,
            Scope::Region => &mut cur,
            Scope::Other => {
                i += 1;
                continue;
            }
        };

        if name == "sample" {
            // Recover the full path: value token + following tokens up to the next opcode/header.
            let start_byte = byte_offset_in(text, first_val);
            let mut end_byte = start_byte + first_val.len();
            let mut j = i + 1;
            while j < tokens.len() {
                let nt = tokens[j];
                if is_header(nt) || nt.contains('=') {
                    break;
                }
                end_byte = byte_offset_in(text, nt) + nt.len();
                j += 1;
            }
            target.sample_ref = match text.get(start_byte..end_byte) {
                Some(s) => s.trim(),
                None => first_val, // fallback: single token (never expected)
            };
            i = j;
            continue;
        }

        apply_opcode(target, name, first_val)?;
        i += 1;
    }

    if region_open {
        regions.push(cur);
        if regions.len() > MAX_REGIONS {
            return Err(AudioError::InvalidParameter);
        }
    }

    Ok(SfzInstrument { regions })
}

enum HeaderKind {
    GroupLike,
    Region,
    Other,
}

#[inline]
fn is_header(tok: &str) -> bool {
    tok.starts_with('<') && tok.ends_with('>') && tok.len() >= 2
}

fn header_name(tok: &str) -> HeaderKind {
    let inner = &tok[1..tok.len() - 1];
    if inner.eq_ignore_ascii_case("region") {
        HeaderKind::Region
    } else if inner.eq_ignore_ascii_case("group") || inner.eq_ignore_ascii_case("global") {
        HeaderKind::GroupLike
    } else {
        HeaderKind::Other
    }
}

/// Byte offset of a sub-slice within `parent` (both must come from the same allocation).
#[inline]
fn byte_offset_in(parent: &str, sub: &str) -> usize {
    (sub.as_ptr() as usize).saturating_sub(parent.as_ptr() as usize)
}

fn apply_opcode(r: &mut SfzRegion<'_>, name: &str, val: &str) -> Result<(), AudioError> {
    match name {
        "lokey" => r.lokey = parse_key(val)?,
        "hikey" => r.hikey = parse_key(val)?,
        "key" => {
            let k = parse_key(val)?;
            r.lokey = k;
            r.hikey = k;
            r.pitch_keycenter = k;
        }
        "lovel" => r.lovel = parse_vel(val)?,
        "hivel" => r.hivel = parse_vel(val)?,
        "pitch_keycenter" => r.pitch_keycenter = parse_key(val)?,
        "tune" | "pitch" => {
            r.tune_cents = val.parse::<i32>().map_err(|_| AudioError::InvalidParameter)?
        }
        "volume" => r.volume_db = val.parse::<f32>().map_err(|_| AudioError::InvalidParameter)?,
        // Unknown opcode in the subset — ignore (forward-compatible with richer SFZ files).
        _ => {}
    }
    Ok(())
}

/// Parse a velocity `0..=127`.
fn parse_vel(val: &str) -> Result<u8, AudioError> {
    let v: i32 = val.parse().map_err(|_| AudioError::InvalidParameter)?;
    if (0..=127).contains(&v) {
        Ok(v as u8)
    } else {
        Err(AudioError::InvalidParameter)
    }
}

/// Parse a key as a MIDI number (`0..=127`) or a note name (`c4`, `F#3`, `bb2`).
fn parse_key(val: &str) -> Result<u8, AudioError> {
    if let Ok(n) = val.parse::<i32>() {
        return if (0..=127).contains(&n) {
            Ok(n as u8)
        } else {
            Err(AudioError::InvalidParameter)
        };
    }
    parse_note_name(val)
}

/// Note name → MIDI number using `c-1 = 0` (so `c4 = 60`). Accepts `#`/`b` accidentals.
fn parse_note_name(val: &str) -> Result<u8, AudioError> {
    let bytes = val.as_bytes();
    if bytes.is_empty() {
        return Err(AudioError::InvalidParameter);
    }
    let mut idx = 0;
    let base = match bytes[0].to_ascii_lowercase() {
        b'c' => 0i32,
        b'd' => 2,
        b'e' => 4,
        b'f' => 5,
        b'g' => 7,
        b'a' => 9,
        b'b' => 11,
        _ => return Err(AudioError::InvalidParameter),
    };
    idx += 1;
    let mut accidental = 0i32;
    while idx < bytes.len() {
        match bytes[idx] {
            b'#' => accidental += 1,
            b'b' | b'B' => accidental -= 1,
            _ => break,
        }
        idx += 1;
    }
    let octave: i32 = val[idx..]
        .parse()
        .map_err(|_| AudioError::InvalidParameter)?;
    let midi = (octave + 1) * 12 + base + accidental;
    if (0..=127).contains(&midi) {
        Ok(midi as u8)
    } else {
        Err(AudioError::InvalidParameter)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn instrument_two_regions_ranges() {
        let text = "\
<region> sample=a.wav lokey=60 hikey=71 pitch_keycenter=60
<region> sample=b.wav lokey=72 hikey=83 pitch_keycenter=72";
        let instr = parse_sfz(text).expect("parse");
        assert_eq!(instr.len(), 2);
        let r0 = &instr.regions()[0];
        assert_eq!(r0.lokey, 60);
        assert_eq!(r0.hikey, 71);
        assert_eq!(r0.pitch_keycenter, 60);
        assert_eq!(r0.sample_ref, "a.wav");
        let r1 = &instr.regions()[1];
        assert_eq!(r1.lokey, 72);
        assert_eq!(r1.hikey, 83);
        assert_eq!(r1.sample_ref, "b.wav");
    }

    #[test]
    fn empty_is_empty_not_error() {
        let instr = parse_sfz("").expect("empty ok");
        assert!(instr.is_empty());
        let instr2 = parse_sfz("// only a comment\n\n").expect("comment ok");
        assert!(instr2.is_empty());
    }

    #[test]
    fn group_defaults_inherited() {
        let text = "\
<group> volume=-6 lovel=0 hivel=63
<region> sample=soft.wav lokey=60 hikey=60
<region> sample=soft2.wav lokey=61 hikey=61 hivel=127";
        let instr = parse_sfz(text).expect("parse");
        assert_eq!(instr.len(), 2);
        assert_eq!(instr.regions()[0].volume_db, -6.0);
        assert_eq!(instr.regions()[0].hivel, 63); // inherited
        assert_eq!(instr.regions()[1].hivel, 127); // overridden
    }

    #[test]
    fn key_opcode_sets_all_three() {
        let instr = parse_sfz("<region> sample=x.wav key=64").expect("parse");
        let r = &instr.regions()[0];
        assert_eq!(r.lokey, 64);
        assert_eq!(r.hikey, 64);
        assert_eq!(r.pitch_keycenter, 64);
    }

    #[test]
    fn note_name_keys() {
        // c4 = 60 under c-1 = 0.
        assert_eq!(parse_key("c4").unwrap(), 60);
        assert_eq!(parse_key("a4").unwrap(), 69);
        assert_eq!(parse_key("f#3").unwrap(), 54);
        assert_eq!(parse_key("60").unwrap(), 60);
        assert!(parse_key("h9").is_err());
    }

    #[test]
    fn sample_path_with_spaces() {
        let text = "<region> sample=Grand Piano/C4 v1.wav lokey=60 hikey=60";
        let instr = parse_sfz(text).expect("parse");
        assert_eq!(instr.regions()[0].sample_ref, "Grand Piano/C4 v1.wav");
        assert_eq!(instr.regions()[0].lokey, 60);
    }

    #[test]
    fn bad_key_is_error() {
        assert_eq!(
            parse_sfz("<region> sample=x.wav lokey=999").err(),
            Some(AudioError::InvalidParameter)
        );
    }
}
