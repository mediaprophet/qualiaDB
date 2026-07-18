//! Standard MIDI File serializer: [`SmfFile`] → bytes.
//!
//! The inverse of [`super::read::read_smf`]. Channel messages are always written
//! with an explicit status byte (no running-status compression), so output is
//! deterministic; a file parsed from explicit-status bytes therefore round-trips
//! byte-for-byte. Cold path.
//!
//! Lane AU-MIDI-FILE.

use super::meta_event::serialize_meta;
use super::read::{SmfFile, Track, TrackEvent};
use super::vlq::write_vlq;
use crate::types::AudioError;

/// Serialize an [`SmfFile`] to a fresh byte buffer.
///
/// Errors with [`AudioError::InvalidParameter`] if a channel message carries the
/// wrong number of data bytes, if `format` is not 0/1/2, or if a chunk/VLQ field
/// overflows its encodable range.
pub fn write_smf(file: &SmfFile) -> Result<Vec<u8>, AudioError> {
    if file.format > 2 {
        return Err(AudioError::InvalidParameter);
    }
    let ntracks = u16::try_from(file.tracks.len()).map_err(|_| AudioError::InvalidParameter)?;

    let mut out = Vec::new();
    // --- Header chunk ---
    out.extend_from_slice(b"MThd");
    out.extend_from_slice(&6u32.to_be_bytes());
    out.extend_from_slice(&file.format.to_be_bytes());
    out.extend_from_slice(&ntracks.to_be_bytes());
    out.extend_from_slice(&file.division.to_raw().to_be_bytes());

    // --- Track chunks ---
    for track in &file.tracks {
        write_track(track, &mut out)?;
    }
    Ok(out)
}

/// Serialize one track as a full `MTrk` chunk (id + length + body) into `out`.
fn write_track(track: &Track, out: &mut Vec<u8>) -> Result<(), AudioError> {
    // Encode the body first so we know its length.
    let mut body = Vec::new();
    for ev in &track.events {
        write_vlq(ev.delta_ticks, &mut body)?;
        write_event(&ev.event, &mut body)?;
    }

    let body_len = u32::try_from(body.len()).map_err(|_| AudioError::InvalidParameter)?;
    out.extend_from_slice(b"MTrk");
    out.extend_from_slice(&body_len.to_be_bytes());
    out.extend_from_slice(&body);
    Ok(())
}

/// Serialize a single event's bytes (without its preceding delta) into `out`.
fn write_event(event: &TrackEvent, out: &mut Vec<u8>) -> Result<(), AudioError> {
    match event {
        TrackEvent::Midi { status, data } => {
            let expected = super::read::channel_data_len(*status).ok_or(AudioError::InvalidParameter)?;
            if data.len() != expected {
                return Err(AudioError::InvalidParameter);
            }
            out.push(*status);
            out.extend_from_slice(data);
        }
        TrackEvent::Meta(meta) => {
            serialize_meta(meta, out)?;
        }
        TrackEvent::SysEx { start, data } => {
            out.push(*start);
            let len = u32::try_from(data.len()).map_err(|_| AudioError::InvalidParameter)?;
            write_vlq(len, out)?;
            out.extend_from_slice(data);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::super::meta_event::MetaEvent;
    use super::super::read::{read_smf, Division, Event, SmfFile, Track, TrackEvent};
    use super::*;

    /// Build a minimal format-0 SMF: one track with tempo, note-on, delayed
    /// note-off, and end-of-track.
    fn minimal_smf() -> SmfFile {
        let events = vec![
            Event {
                delta_ticks: 0,
                event: TrackEvent::Meta(MetaEvent::Tempo(500_000)),
            },
            Event {
                delta_ticks: 0,
                event: TrackEvent::Midi { status: 0x90, data: vec![60, 100] },
            },
            Event {
                delta_ticks: 480,
                event: TrackEvent::Midi { status: 0x80, data: vec![60, 0] },
            },
            Event {
                delta_ticks: 0,
                event: TrackEvent::Meta(MetaEvent::EndOfTrack),
            },
        ];
        SmfFile {
            format: 0,
            division: Division::Ppq(480),
            tracks: vec![Track { events }],
        }
    }

    #[test]
    fn round_trip_structure_and_bytes() {
        let original = minimal_smf();
        let bytes = write_smf(&original).unwrap();

        // Parse back and compare structure.
        let parsed = read_smf(&bytes).unwrap();
        assert_eq!(parsed, original);
        assert_eq!(parsed.division, Division::Ppq(480));
        assert_eq!(parsed.tracks[0].events[2].delta_ticks, 480);

        // bytes → read → write → bytes is identical.
        let bytes2 = write_smf(&parsed).unwrap();
        assert_eq!(bytes, bytes2);
    }

    #[test]
    fn exact_header_bytes() {
        let bytes = write_smf(&minimal_smf()).unwrap();
        // MThd, len 6, format 0, ntracks 1, division 480 (0x01E0).
        assert_eq!(
            &bytes[0..14],
            &[b'M', b'T', b'h', b'd', 0, 0, 0, 6, 0, 0, 0, 1, 0x01, 0xE0]
        );
    }

    #[test]
    fn rejects_wrong_data_byte_count() {
        let bad = SmfFile {
            format: 0,
            division: Division::Ppq(480),
            tracks: vec![Track {
                events: vec![Event {
                    delta_ticks: 0,
                    // Note-on needs 2 data bytes; give 1.
                    event: TrackEvent::Midi { status: 0x90, data: vec![60] },
                }],
            }],
        };
        assert_eq!(write_smf(&bad), Err(AudioError::InvalidParameter));
    }
}
