//! Tempo map: tick ↔ seconds conversion for a parsed SMF.
//!
//! A [`TempoMap`] is the ordered list of tempo changes (absolute tick →
//! microseconds-per-quarter) plus the file's [`Division`]. For metrical (PPQ)
//! division, elapsed seconds are integrated piecewise across tempo segments; for
//! SMPTE division the tick rate is fixed by the time-code and tempo is ignored.
//! Cold path — this runs at load/analysis time, not in the audio callback.
//!
//! Lane AU-MIDI-FILE.

use super::meta_event::MetaEvent;
use super::read::{Division, SmfFile, TrackEvent};

/// Default tempo before any tempo meta-event: 120 BPM = 500000 µs/quarter.
pub const DEFAULT_US_PER_QUARTER: u32 = 500_000;

/// One tempo change at an absolute tick position.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TempoEntry {
    /// Absolute tick (from the start of the sequence).
    pub tick: u64,
    /// Microseconds per quarter note in effect from this tick onward.
    pub us_per_quarter: u32,
}

/// An ordered tempo map for one SMF, with its time division.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TempoMap {
    division: Division,
    /// Tempo changes sorted by ascending tick. Always non-empty; a synthetic
    /// entry at tick 0 with [`DEFAULT_US_PER_QUARTER`] is inserted if the file
    /// declares no tempo at tick 0.
    entries: Vec<TempoEntry>,
}

impl TempoMap {
    /// Build a single-tempo map (useful for tests and tempo-less files).
    pub fn constant(division: Division, us_per_quarter: u32) -> Self {
        TempoMap {
            division,
            entries: vec![TempoEntry { tick: 0, us_per_quarter }],
        }
    }

    /// The tempo entries, sorted by tick.
    pub fn entries(&self) -> &[TempoEntry] {
        &self.entries
    }

    /// Convert an absolute tick position to elapsed seconds from the start.
    pub fn ticks_to_seconds(&self, tick: u64) -> f64 {
        match self.division {
            Division::Smpte { fps, ticks_per_frame } => {
                let per_second = f64::from(fps) * f64::from(ticks_per_frame);
                if per_second == 0.0 {
                    0.0
                } else {
                    tick as f64 / per_second
                }
            }
            Division::Ppq(ppq) => {
                let ppq = f64::from(ppq.max(1));
                let mut seconds = 0.0;
                for i in 0..self.entries.len() {
                    let seg_start = self.entries[i].tick;
                    if seg_start >= tick {
                        break;
                    }
                    let seg_end = self
                        .entries
                        .get(i + 1)
                        .map(|e| e.tick.min(tick))
                        .unwrap_or(tick);
                    let seg_ticks = seg_end.saturating_sub(seg_start) as f64;
                    let sec_per_tick = (f64::from(self.entries[i].us_per_quarter) / 1_000_000.0) / ppq;
                    seconds += seg_ticks * sec_per_tick;
                }
                seconds
            }
        }
    }

    /// Convert elapsed seconds from the start to an absolute tick position
    /// (rounded to the nearest tick).
    pub fn seconds_to_ticks(&self, seconds: f64) -> u64 {
        if seconds <= 0.0 {
            return 0;
        }
        match self.division {
            Division::Smpte { fps, ticks_per_frame } => {
                let per_second = f64::from(fps) * f64::from(ticks_per_frame);
                (seconds * per_second).round() as u64
            }
            Division::Ppq(ppq) => {
                let ppq = f64::from(ppq.max(1));
                let mut remaining = seconds;
                for i in 0..self.entries.len() {
                    let seg_start = self.entries[i].tick;
                    let sec_per_tick = (f64::from(self.entries[i].us_per_quarter) / 1_000_000.0) / ppq;
                    // Duration of this segment (until the next tempo change, or ∞).
                    let seg_ticks = self
                        .entries
                        .get(i + 1)
                        .map(|e| (e.tick - seg_start) as f64)
                        .unwrap_or(f64::INFINITY);
                    let seg_seconds = seg_ticks * sec_per_tick;
                    if remaining <= seg_seconds || !seg_seconds.is_finite() {
                        let ticks_in = if sec_per_tick > 0.0 {
                            remaining / sec_per_tick
                        } else {
                            0.0
                        };
                        return seg_start + ticks_in.round() as u64;
                    }
                    remaining -= seg_seconds;
                }
                self.entries.last().map(|e| e.tick).unwrap_or(0)
            }
        }
    }
}

/// Build a [`TempoMap`] from a parsed SMF.
///
/// Collects every tempo meta-event across all tracks (converting each track's
/// per-track delta times to absolute ticks), sorts them by tick, and guarantees
/// an entry at tick 0. For format-1 files tempo events conventionally live in
/// track 0, but events found in any track are honoured.
pub fn build_tempo_map(file: &SmfFile) -> TempoMap {
    let mut entries: Vec<TempoEntry> = Vec::new();
    for track in &file.tracks {
        let mut abs_tick: u64 = 0;
        for ev in &track.events {
            abs_tick += u64::from(ev.delta_ticks);
            if let TrackEvent::Meta(MetaEvent::Tempo(us)) = &ev.event {
                entries.push(TempoEntry { tick: abs_tick, us_per_quarter: *us });
            }
        }
    }
    // Stable sort by tick; later-file-order wins ties (rare).
    entries.sort_by_key(|e| e.tick);

    // Guarantee a starting tempo at tick 0.
    if entries.first().map(|e| e.tick) != Some(0) {
        entries.insert(
            0,
            TempoEntry { tick: 0, us_per_quarter: DEFAULT_US_PER_QUARTER },
        );
    }

    TempoMap { division: file.division, entries }
}

#[cfg(test)]
mod tests {
    use super::super::read::{Event, SmfFile, Track};
    use super::*;

    #[test]
    fn golden_120bpm_480ppq_half_second() {
        // 500000 µs/quarter (120 BPM), division 480 → 480 ticks == 0.5 s.
        let map = TempoMap::constant(Division::Ppq(480), 500_000);
        let s = map.ticks_to_seconds(480);
        assert!((s - 0.5).abs() < 1e-9, "expected 0.5 s, got {s}");
        // And back.
        assert_eq!(map.seconds_to_ticks(0.5), 480);
        // One whole quarter (480 ticks) at 120 BPM is 0.5 s; a full bar of 4 is 2 s.
        assert!((map.ticks_to_seconds(480 * 4) - 2.0).abs() < 1e-9);
    }

    #[test]
    fn tempo_change_midway() {
        // 480 PPQ. 120 BPM for first 480 ticks (0.5 s), then 60 BPM (1e6 µs/q).
        let map = TempoMap {
            division: Division::Ppq(480),
            entries: vec![
                TempoEntry { tick: 0, us_per_quarter: 500_000 },
                TempoEntry { tick: 480, us_per_quarter: 1_000_000 },
            ],
        };
        // At tick 480: 0.5 s. At tick 960: 0.5 + (480 ticks @ 60 BPM = 1.0 s) = 1.5 s.
        assert!((map.ticks_to_seconds(480) - 0.5).abs() < 1e-9);
        assert!((map.ticks_to_seconds(960) - 1.5).abs() < 1e-9);
        assert_eq!(map.seconds_to_ticks(1.5), 960);
    }

    #[test]
    fn build_from_file_uses_default_when_no_tempo() {
        let file = SmfFile {
            format: 0,
            division: Division::Ppq(96),
            tracks: vec![Track { events: vec![] }],
        };
        let map = build_tempo_map(&file);
        assert_eq!(map.entries(), &[TempoEntry { tick: 0, us_per_quarter: 500_000 }]);
    }

    #[test]
    fn build_from_file_collects_tempo_events() {
        use super::super::read::TrackEvent;
        let events = vec![
            Event { delta_ticks: 0, event: TrackEvent::Meta(MetaEvent::Tempo(600_000)) },
            Event { delta_ticks: 240, event: TrackEvent::Meta(MetaEvent::Tempo(400_000)) },
        ];
        let file = SmfFile {
            format: 0,
            division: Division::Ppq(480),
            tracks: vec![Track { events }],
        };
        let map = build_tempo_map(&file);
        assert_eq!(
            map.entries(),
            &[
                TempoEntry { tick: 0, us_per_quarter: 600_000 },
                TempoEntry { tick: 240, us_per_quarter: 400_000 },
            ]
        );
    }

    #[test]
    fn smpte_division_is_tempo_independent() {
        // 25 fps, 40 ticks/frame → 1000 ticks/second.
        let map = TempoMap::constant(Division::Smpte { fps: 25, ticks_per_frame: 40 }, 500_000);
        assert!((map.ticks_to_seconds(1000) - 1.0).abs() < 1e-9);
        assert_eq!(map.seconds_to_ticks(1.0), 1000);
    }
}
