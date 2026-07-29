//! Time-signature representation and bars/beats/ticks conversion.
//!
//! A [`TimeSignature`] is `numerator/denominator` (e.g. `4/4`, `6/8`). The
//! sequencer measures absolute time in PPQ ticks (ticks per quarter note); this
//! module maps an absolute tick position to a musical [`BarBeatTick`] address
//! and back.
//!
//! One "beat" is one note of the denominator's value: a quarter note in `4/4`,
//! an eighth note in `6/8`. Since PPQ counts ticks per *quarter* note, a beat
//! spans `ppq * 4 / denominator` ticks. All fields are 0-indexed (bar 0, beat 0,
//! tick 0 is the downbeat of the first bar); add 1 for musician-facing display.

use crate::types::AudioError;

/// A musical time signature `numerator/denominator`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimeSignature {
    /// Beats per bar (top number). Must be ≥ 1.
    pub numerator: u16,
    /// Beat unit as a note value (bottom number): 1, 2, 4, 8, 16, 32… Must be ≥ 1.
    pub denominator: u16,
}

/// A musical position as (bar, beat, tick), all 0-indexed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BarBeatTick {
    /// Bar index from the start of the timeline (0 = first bar).
    pub bar: u64,
    /// Beat within the bar (0 = downbeat), `0 ..= numerator-1`.
    pub beat: u32,
    /// Tick within the beat, `0 ..= ticks_per_beat-1`.
    pub tick: u32,
}

impl TimeSignature {
    /// Common time, `4/4`.
    pub const COMMON: TimeSignature = TimeSignature {
        numerator: 4,
        denominator: 4,
    };

    /// Construct, validating that both numbers are ≥ 1.
    pub fn new(numerator: u16, denominator: u16) -> Result<Self, AudioError> {
        if numerator == 0 || denominator == 0 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    /// Ticks in one beat (one denominator-note) at the given PPQ.
    ///
    /// Errors if `ppq == 0`, the signature is malformed, or the denominator
    /// does not divide `ppq * 4` evenly (which would lose tick precision).
    pub fn ticks_per_beat(self, ppq: u32) -> Result<u32, AudioError> {
        if ppq == 0 || self.numerator == 0 || self.denominator == 0 {
            return Err(AudioError::InvalidParameter);
        }
        let quarter_times_four = (ppq as u64) * 4;
        if quarter_times_four % self.denominator as u64 != 0 {
            return Err(AudioError::InvalidParameter);
        }
        Ok((quarter_times_four / self.denominator as u64) as u32)
    }

    /// Ticks in one full bar at the given PPQ.
    pub fn ticks_per_bar(self, ppq: u32) -> Result<u64, AudioError> {
        let tpb = self.ticks_per_beat(ppq)? as u64;
        Ok(tpb * self.numerator as u64)
    }

    /// Map an absolute tick position to a (bar, beat, tick) address.
    pub fn ticks_to_bbt(self, ticks: u64, ppq: u32) -> Result<BarBeatTick, AudioError> {
        let tpb = self.ticks_per_beat(ppq)? as u64;
        let tpbar = tpb * self.numerator as u64;
        let bar = ticks / tpbar;
        let rem = ticks % tpbar;
        let beat = (rem / tpb) as u32;
        let tick = (rem % tpb) as u32;
        Ok(BarBeatTick { bar, beat, tick })
    }

    /// Map a (bar, beat, tick) address back to an absolute tick position.
    ///
    /// Errors if `beat` is not within the bar or `tick` overflows a beat.
    pub fn bbt_to_ticks(self, pos: BarBeatTick, ppq: u32) -> Result<u64, AudioError> {
        let tpb = self.ticks_per_beat(ppq)? as u64;
        if pos.beat as u64 >= self.numerator as u64 || pos.tick as u64 >= tpb {
            return Err(AudioError::InvalidParameter);
        }
        Ok(pos.bar * tpb * self.numerator as u64 + pos.beat as u64 * tpb + pos.tick as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn four_four_at_480() {
        let ts = TimeSignature::COMMON;
        assert_eq!(ts.ticks_per_beat(480).unwrap(), 480);
        assert_eq!(ts.ticks_per_bar(480).unwrap(), 1920);
    }

    #[test]
    fn six_eight_beat_is_eighth_note() {
        let ts = TimeSignature::new(6, 8).unwrap();
        assert_eq!(ts.ticks_per_beat(480).unwrap(), 240);
        assert_eq!(ts.ticks_per_bar(480).unwrap(), 1440);
    }

    #[test]
    fn bbt_round_trip() {
        let ts = TimeSignature::COMMON;
        // Bar 2, beat 3, tick 100 in 4/4 @480: 2*1920 + 3*480 + 100 = 5380
        let pos = BarBeatTick {
            bar: 2,
            beat: 3,
            tick: 100,
        };
        let ticks = ts.bbt_to_ticks(pos, 480).unwrap();
        assert_eq!(ticks, 5380);
        assert_eq!(ts.ticks_to_bbt(ticks, 480).unwrap(), pos);
    }

    #[test]
    fn rejects_out_of_range() {
        let ts = TimeSignature::COMMON;
        assert!(TimeSignature::new(0, 4).is_err());
        assert!(ts
            .bbt_to_ticks(
                BarBeatTick {
                    bar: 0,
                    beat: 4,
                    tick: 0
                },
                480
            )
            .is_err());
        assert!(ts
            .bbt_to_ticks(
                BarBeatTick {
                    bar: 0,
                    beat: 0,
                    tick: 480
                },
                480
            )
            .is_err());
    }
}
