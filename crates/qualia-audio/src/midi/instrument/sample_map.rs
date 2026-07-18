//! Allocation-free note→region lookup over a parsed [`SfzInstrument`].
//!
//! This is the HOT path used when a note-on must pick the sample region to play. It performs
//! a linear scan of the instrument's fixed region slice and returns the FIRST matching region
//! (SFZ layering is by authoring order). No allocation, no sorting — the region list was
//! built once during the cold `parse_sfz` pass.

use super::sfz::{SfzInstrument, SfzRegion};

/// Return the first region matching `(note, velocity)`, or `None` if the instrument has no
/// region covering that key/velocity rectangle.
///
/// Allocation-free: borrows straight out of `instr`'s region slice.
#[inline]
pub fn region_for<'a>(
    instr: &'a SfzInstrument<'a>,
    note: u8,
    velocity: u8,
) -> Option<&'a SfzRegion<'a>> {
    instr.regions().iter().find(|r| r.matches(note, velocity))
}

#[cfg(test)]
mod tests {
    use super::super::sfz::parse_sfz;
    use super::*;

    fn two_region_instr() -> String {
        "\
<region> sample=a.wav lokey=60 hikey=71 pitch_keycenter=60
<region> sample=b.wav lokey=72 hikey=83 pitch_keycenter=72"
            .to_string()
    }

    #[test]
    fn golden_first_region() {
        let text = two_region_instr();
        let instr = parse_sfz(&text).expect("parse");
        let r = region_for(&instr, 65, 100).expect("match");
        assert_eq!(r.sample_ref, "a.wav");
        assert_eq!(r.lokey, 60);
    }

    #[test]
    fn golden_second_region() {
        let text = two_region_instr();
        let instr = parse_sfz(&text).expect("parse");
        let r = region_for(&instr, 75, 100).expect("match");
        assert_eq!(r.sample_ref, "b.wav");
        assert_eq!(r.hikey, 83);
    }

    #[test]
    fn out_of_range_none() {
        let text = two_region_instr();
        let instr = parse_sfz(&text).expect("parse");
        assert!(region_for(&instr, 40, 100).is_none());
        assert!(region_for(&instr, 90, 100).is_none());
    }

    #[test]
    fn velocity_filters() {
        let text = "\
<region> sample=soft.wav lokey=60 hikey=60 lovel=0 hivel=63
<region> sample=loud.wav lokey=60 hikey=60 lovel=64 hivel=127";
        let instr = parse_sfz(text).expect("parse");
        assert_eq!(region_for(&instr, 60, 20).unwrap().sample_ref, "soft.wav");
        assert_eq!(region_for(&instr, 60, 100).unwrap().sample_ref, "loud.wav");
    }

    #[test]
    fn empty_instrument_none() {
        let instr = parse_sfz("").expect("parse");
        assert!(region_for(&instr, 60, 100).is_none());
    }
}
