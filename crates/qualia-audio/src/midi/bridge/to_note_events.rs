//! Pair a stream of timed MIDI events into whole notes (`note-on` matched to its
//! `note-off`).
//!
//! Lane AU-MIDI-BRIDGE. The input is a tick-ordered slice of
//! [`crate::midi::sequencer::TimedEvent`] — the sequencer's event currency, and
//! the natural flattened form of an authored [`crate::midi::smf`] track (each
//! delta-timed [`crate::midi::smf::TrackEvent::Midi`] carries a status + data
//! bytes at an absolute tick). This function walks that stream, matches each
//! note-on to the next note-off (or velocity-0 note-on, the running-status note
//! off) on the same channel + note, and emits one [`PairedNote`] per completed
//! note with its `start_tick`, `end_tick`, and attack `velocity`.
//!
//! # Epistemic contract — authored MIDI is AUTHORITATIVE
//!
//! Notes paired here come from *authored / imported* MIDI (a sequencer track or
//! a parsed SMF), which is **authoritative** ground truth — unlike transcribed
//! proposals produced by [`super::from_pitch_midi`]. This function adds no
//! confidence field because none is warranted: an authored note is asserted, not
//! proposed. Callers must not blend a [`PairedNote`] with a transcription
//! proposal without tracking which stream it came from.
//!
//! Zero-heap: pairing state lives in a fixed on-stack table; completed notes are
//! written into the caller-supplied `out` slice.

use crate::midi::sequencer::TimedEvent;
use crate::types::AudioError;

/// Maximum number of notes that may be sounding simultaneously while pairing.
/// A stream with more than this many concurrently-open note-ons is rejected.
pub const MAX_OPEN_NOTES: usize = 128;

/// A whole note recovered by pairing a note-on with its note-off.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairedNote {
    /// MIDI channel, 0..=15.
    pub channel: u8,
    /// Note number, 0..=127.
    pub note: u8,
    /// Attack velocity from the note-on, 0..=127.
    pub velocity: u8,
    /// Tick of the note-on.
    pub start_tick: u64,
    /// Tick of the matched note-off.
    pub end_tick: u64,
}

impl PairedNote {
    /// An all-zero placeholder for initialising a caller-owned `out` array.
    pub const ZERO: PairedNote = PairedNote {
        channel: 0,
        note: 0,
        velocity: 0,
        start_tick: 0,
        end_tick: 0,
    };
}

/// One currently-sounding note-on awaiting its note-off.
#[derive(Clone, Copy)]
struct OpenNote {
    channel: u8,
    note: u8,
    velocity: u8,
    start_tick: u64,
}

/// True if `status` is a note-on (`0x9n`).
#[inline]
fn is_note_on(status: u8) -> bool {
    status & 0xF0 == 0x90
}

/// True if `status` is a note-off (`0x8n`).
#[inline]
fn is_note_off(status: u8) -> bool {
    status & 0xF0 == 0x80
}

/// Pair a tick-ordered timed-MIDI stream into whole notes.
///
/// A note opens on a note-on with `velocity > 0`. It closes on the first
/// subsequent note-off (`0x8n`) **or** velocity-0 note-on (`0x9n` vel 0) with the
/// same channel and note; the newest matching open note is closed first (LIFO),
/// which is the standard resolution for a repeated note re-struck before its
/// first release. Unmatched note-ons still open at the end of the stream are
/// dropped (no synthetic note-off is invented). Non-note events are ignored.
///
/// - `events`: the timed MIDI stream (sequencer track / flattened SMF track).
/// - `out`: caller-owned destination for completed [`PairedNote`]s.
///
/// Returns the number of notes written to `out`.
///
/// # Errors
/// - [`AudioError::OutputBufferTooSmall`] if `out` cannot hold every completed
///   note.
/// - [`AudioError::InvalidParameter`] if more than [`MAX_OPEN_NOTES`] notes are
///   sounding at once (pairing table exhausted).
pub fn pair_note_events(
    events: &[TimedEvent],
    out: &mut [PairedNote],
) -> Result<usize, AudioError> {
    let mut open: [Option<OpenNote>; MAX_OPEN_NOTES] = [None; MAX_OPEN_NOTES];
    let mut open_len = 0usize;
    let mut count = 0usize;

    for ev in events {
        let channel = ev.status & 0x0F;
        let note = ev.data1;
        let velocity = ev.data2;

        let closes = is_note_off(ev.status) || (is_note_on(ev.status) && velocity == 0);
        let opens = is_note_on(ev.status) && velocity > 0;

        if closes {
            // Close the newest matching open note (LIFO), if any.
            if let Some(slot) = find_newest_match(&open, open_len, channel, note) {
                let on = open[slot].take().expect("matched slot is occupied");
                // Compact: pull the last live entry into the freed slot.
                open_len = compact(&mut open, open_len, slot);
                if count >= out.len() {
                    return Err(AudioError::OutputBufferTooSmall);
                }
                out[count] = PairedNote {
                    channel: on.channel,
                    note: on.note,
                    velocity: on.velocity,
                    start_tick: on.start_tick,
                    end_tick: ev.tick,
                };
                count += 1;
            }
            // An unmatched note-off is a stray release; ignore it.
        } else if opens {
            if open_len >= MAX_OPEN_NOTES {
                return Err(AudioError::InvalidParameter);
            }
            open[open_len] = Some(OpenNote {
                channel,
                note,
                velocity,
                start_tick: ev.tick,
            });
            open_len += 1;
        }
        // All other events (CC, pitch-bend, meta-as-status, …) are ignored.
    }

    Ok(count)
}

/// Index of the newest (highest) occupied slot in `open[..len]` matching
/// `channel` + `note`, or `None`.
#[inline]
fn find_newest_match(
    open: &[Option<OpenNote>],
    len: usize,
    channel: u8,
    note: u8,
) -> Option<usize> {
    for i in (0..len).rev() {
        if let Some(o) = open[i] {
            if o.channel == channel && o.note == note {
                return Some(i);
            }
        }
    }
    None
}

/// After clearing `open[slot]`, move the last live entry (`open[len-1]`) into it
/// so the live entries stay packed in `open[..len-1]`. Returns the new length.
#[inline]
fn compact(open: &mut [Option<OpenNote>], len: usize, slot: usize) -> usize {
    debug_assert!(open[slot].is_none());
    let last = len - 1;
    if slot != last {
        open[slot] = open[last].take();
    } else {
        open[last] = None;
    }
    last
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_pairs_one_note() {
        // Note-on at tick 0, note-off at tick 480, same channel + note.
        let events = [
            TimedEvent::new(0, 0x90, 60, 100),
            TimedEvent::new(480, 0x80, 60, 0),
        ];
        let mut out = [PairedNote::ZERO; 4];
        let n = pair_note_events(&events, &mut out).expect("pair");
        assert_eq!(n, 1);
        assert_eq!(
            out[0],
            PairedNote {
                channel: 0,
                note: 60,
                velocity: 100,
                start_tick: 0,
                end_tick: 480
            }
        );
    }

    #[test]
    fn velocity_zero_note_on_closes_the_note() {
        // Running-status note off: 0x90 note 62 vel 0.
        let events = [
            TimedEvent::new(10, 0x90, 62, 90),
            TimedEvent::new(100, 0x90, 62, 0),
        ];
        let mut out = [PairedNote::ZERO; 2];
        let n = pair_note_events(&events, &mut out).expect("pair");
        assert_eq!(n, 1);
        assert_eq!(out[0].start_tick, 10);
        assert_eq!(out[0].end_tick, 100);
        assert_eq!(out[0].velocity, 90);
    }

    #[test]
    fn two_overlapping_notes_on_different_channels() {
        let events = [
            TimedEvent::new(0, 0x90, 60, 100), // ch0 on
            TimedEvent::new(5, 0x91, 64, 80),  // ch1 on
            TimedEvent::new(50, 0x81, 64, 0),  // ch1 off
            TimedEvent::new(60, 0x80, 60, 0),  // ch0 off
        ];
        let mut out = [PairedNote::ZERO; 4];
        let n = pair_note_events(&events, &mut out).expect("pair");
        assert_eq!(n, 2);
        // ch1 note closes first.
        assert_eq!(
            (
                out[0].channel,
                out[0].note,
                out[0].start_tick,
                out[0].end_tick
            ),
            (1, 64, 5, 50)
        );
        assert_eq!(
            (
                out[1].channel,
                out[1].note,
                out[1].start_tick,
                out[1].end_tick
            ),
            (0, 60, 0, 60)
        );
    }

    #[test]
    fn restruck_note_pairs_lifo() {
        // Same note struck twice before any release, then two releases.
        let events = [
            TimedEvent::new(0, 0x90, 60, 100),
            TimedEvent::new(10, 0x90, 60, 110),
            TimedEvent::new(20, 0x80, 60, 0), // closes the newest (start 10)
            TimedEvent::new(30, 0x80, 60, 0), // closes the older (start 0)
        ];
        let mut out = [PairedNote::ZERO; 4];
        let n = pair_note_events(&events, &mut out).expect("pair");
        assert_eq!(n, 2);
        assert_eq!(
            (out[0].start_tick, out[0].end_tick, out[0].velocity),
            (10, 20, 110)
        );
        assert_eq!(
            (out[1].start_tick, out[1].end_tick, out[1].velocity),
            (0, 30, 100)
        );
    }

    #[test]
    fn unmatched_note_on_is_dropped() {
        let events = [TimedEvent::new(0, 0x90, 60, 100)];
        let mut out = [PairedNote::ZERO; 2];
        let n = pair_note_events(&events, &mut out).expect("pair");
        assert_eq!(n, 0);
    }

    #[test]
    fn non_note_events_ignored() {
        let events = [
            TimedEvent::new(0, 0xB0, 7, 100),  // control change
            TimedEvent::new(0, 0x90, 60, 100), // note on
            TimedEvent::new(0, 0xE0, 0, 64),   // pitch bend
            TimedEvent::new(20, 0x80, 60, 0),  // note off
        ];
        let mut out = [PairedNote::ZERO; 2];
        let n = pair_note_events(&events, &mut out).expect("pair");
        assert_eq!(n, 1);
        assert_eq!(out[0].note, 60);
    }

    #[test]
    fn output_too_small_reported() {
        let events = [
            TimedEvent::new(0, 0x90, 60, 100),
            TimedEvent::new(10, 0x80, 60, 0),
            TimedEvent::new(20, 0x90, 62, 100),
            TimedEvent::new(30, 0x80, 62, 0),
        ];
        let mut out = [PairedNote::ZERO; 1];
        assert_eq!(
            pair_note_events(&events, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
