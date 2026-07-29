//! Convert transcribed pitch→MIDI [`NoteEvent`] proposals into sequencer
//! [`TimedEvent`] note-on / note-off pairs.
//!
//! Lane AU-MIDI-BRIDGE. This reconciles two already-built pieces without
//! reimplementing either: [`crate::features::pitch_midi::NoteEvent`] (the output
//! of `segment_notes` / `audio_to_midi`) and [`crate::midi::sequencer::TimedEvent`]
//! (the sequencer's timeline event currency).
//!
//! # Epistemic contract — these events are PROPOSALS
//!
//! Every [`TimedEvent`] this function emits derives from a *transcribed*
//! [`NoteEvent`], which carries a per-note voicing `confidence` and is a
//! **proposal**, never an authoritative transcription. `TimedEvent` is a bare
//! `(tick, status, data1, data2)` POD with no room for confidence, so the
//! proposal status **cannot travel inside the event** — it is a property of the
//! path: anything produced by [`note_events_to_timed`] is proposal-grade and
//! must be treated as such downstream (weighted, reviewed, or rejected on the
//! originating note's confidence). This is the opposite of events flattened from
//! an authored/imported SMF via [`super::to_note_events`], which are
//! AUTHORITATIVE. See [`PROPOSAL_NOTE`].
//!
//! Zero-heap: note-on/off events are written into the caller-supplied `out`
//! slice; the function allocates nothing.

use crate::features::pitch_midi::NoteEvent;
use crate::midi::message::note::{STATUS_NOTE_OFF, STATUS_NOTE_ON};
use crate::midi::sequencer::TimedEvent;
use crate::types::AudioError;

/// The MIDI channel transcribed proposals are placed on. Transcription has no
/// channel concept, so a single default channel is used.
pub const PROPOSAL_CHANNEL: u8 = 0;

/// Release velocity written into the generated note-off events.
pub const NOTE_OFF_VELOCITY: u8 = 0;

/// Human-readable statement of the epistemic status of every event this module
/// emits, for surfacing in provenance / UI. Transcribed MIDI is a proposal.
pub const PROPOSAL_NOTE: &str =
    "transcribed (audio→MIDI) — epistemic PROPOSAL carrying source confidence, not authoritative";

/// Map an audio frame index to a sequencer tick given `frames_per_tick`.
///
/// `tick = round(frame / frames_per_tick)`. `frames_per_tick` is the width of
/// one PPQ tick measured in analysis frames — e.g. with a hop that yields 10
/// frames per tick, frame 100 maps to tick 10.
#[inline]
fn frame_to_tick(frame: u32, frames_per_tick: f32) -> u64 {
    let t = (frame as f32 / frames_per_tick).round();
    if t <= 0.0 {
        0
    } else {
        t as u64
    }
}

/// Convert transcribed [`NoteEvent`] proposals into paired note-on / note-off
/// [`TimedEvent`]s on the sequencer timeline.
///
/// For each input note two events are written to `out`, in order: a note-on at
/// the tick its `start_frame` maps to (status `0x90`, velocity from the note),
/// then a note-off at the tick its `end_frame` maps to (status `0x80`, release
/// velocity [`NOTE_OFF_VELOCITY`]). Both carry [`PROPOSAL_CHANNEL`].
///
/// The output is **not** re-sorted: it is emitted in input order as on/off
/// pairs. Because `segment_notes` yields notes in start-frame order, callers who
/// need a tick-sorted [`crate::midi::sequencer::Track`] should `insert` each
/// event (which keeps the track sorted) rather than assume global tick order.
///
/// - `events`: transcribed note proposals (see [`crate::features::pitch_midi`]).
/// - `frames_per_tick`: analysis frames per PPQ tick; must be finite and `> 0`.
/// - `out`: caller-owned destination; must hold `2 * events.len()` events.
///
/// Returns the number of [`TimedEvent`]s written (always `2 * events.len()`).
///
/// Every emitted event is a **proposal** (see the module docs / [`PROPOSAL_NOTE`]).
///
/// # Errors
/// - [`AudioError::InvalidParameter`] if `frames_per_tick` is not finite/`> 0`,
///   or a note's `note`/`velocity` is out of MIDI range (`> 127`).
/// - [`AudioError::OutputBufferTooSmall`] if `out` cannot hold every event.
pub fn note_events_to_timed(
    events: &[NoteEvent],
    frames_per_tick: f32,
    out: &mut [TimedEvent],
) -> Result<usize, AudioError> {
    if !frames_per_tick.is_finite() || frames_per_tick <= 0.0 {
        return Err(AudioError::InvalidParameter);
    }
    let needed = events
        .len()
        .checked_mul(2)
        .ok_or(AudioError::InvalidParameter)?;
    if out.len() < needed {
        return Err(AudioError::OutputBufferTooSmall);
    }

    let mut n = 0usize;
    for ev in events {
        if ev.note > 127 || ev.velocity > 127 {
            return Err(AudioError::InvalidParameter);
        }
        let on_tick = frame_to_tick(ev.start_frame, frames_per_tick);
        let off_tick = frame_to_tick(ev.end_frame, frames_per_tick);

        out[n] = TimedEvent::new(
            on_tick,
            STATUS_NOTE_ON | (PROPOSAL_CHANNEL & 0x0F),
            ev.note,
            ev.velocity,
        );
        out[n + 1] = TimedEvent::new(
            off_tick,
            STATUS_NOTE_OFF | (PROPOSAL_CHANNEL & 0x0F),
            ev.note,
            NOTE_OFF_VELOCITY,
        );
        n += 2;
    }
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note_event(note: u8, start: u32, end: u32, vel: u8) -> NoteEvent {
        NoteEvent {
            note,
            velocity: vel,
            start_frame: start,
            end_frame: end,
            confidence: 0.9,
        }
    }

    #[test]
    fn golden_a4_maps_to_on_at_0_off_at_10() {
        // NoteEvent(note 69, start 0, end 100); frames_per_tick 10 → off tick 10.
        let events = [note_event(69, 0, 100, 100)];
        let mut out = [TimedEvent::ZERO; 2];
        let n = note_events_to_timed(&events, 10.0, &mut out).expect("convert");
        assert_eq!(n, 2);

        // Note-on at tick 0.
        assert_eq!(out[0].tick, 0);
        assert_eq!(out[0].status, 0x90);
        assert_eq!(out[0].data1, 69);
        assert_eq!(out[0].data2, 100);

        // Note-off at tick 10.
        assert_eq!(out[1].tick, 10);
        assert_eq!(out[1].status, 0x80);
        assert_eq!(out[1].data1, 69);
        assert_eq!(out[1].data2, NOTE_OFF_VELOCITY);
    }

    #[test]
    fn two_notes_produce_four_events_in_order() {
        let events = [note_event(60, 0, 48, 80), note_event(62, 48, 96, 90)];
        let mut out = [TimedEvent::ZERO; 4];
        let n = note_events_to_timed(&events, 48.0, &mut out).expect("convert");
        assert_eq!(n, 4);
        assert_eq!((out[0].tick, out[0].status, out[0].data1), (0, 0x90, 60));
        assert_eq!((out[1].tick, out[1].status, out[1].data1), (1, 0x80, 60));
        assert_eq!((out[2].tick, out[2].status, out[2].data1), (1, 0x90, 62));
        assert_eq!((out[3].tick, out[3].status, out[3].data1), (2, 0x80, 62));
    }

    #[test]
    fn bad_frames_per_tick_rejected() {
        let events = [note_event(69, 0, 100, 100)];
        let mut out = [TimedEvent::ZERO; 2];
        assert_eq!(
            note_events_to_timed(&events, 0.0, &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            note_events_to_timed(&events, f32::NAN, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }

    #[test]
    fn output_too_small_reported() {
        let events = [note_event(69, 0, 100, 100)];
        let mut out = [TimedEvent::ZERO; 1];
        assert_eq!(
            note_events_to_timed(&events, 10.0, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }

    #[test]
    fn out_of_range_note_rejected() {
        let events = [note_event(200, 0, 100, 100)];
        let mut out = [TimedEvent::ZERO; 2];
        assert_eq!(
            note_events_to_timed(&events, 10.0, &mut out),
            Err(AudioError::InvalidParameter)
        );
    }
}
