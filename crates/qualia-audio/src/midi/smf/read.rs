//! Standard MIDI File parser: MThd + MTrk chunks → [`SmfFile`].
//!
//! Defines the structured in-memory model ([`SmfFile`], [`Track`], [`Event`],
//! [`TrackEvent`], [`Division`]) that the whole `smf` module shares, and
//! [`read_smf`], which parses a complete SMF byte buffer. Running status inside
//! tracks is expanded (each stored channel event carries its explicit status).
//! Cold path — `Vec` throughout is deliberate.
//!
//! Lane AU-MIDI-FILE.

use super::meta_event::{parse_meta, MetaEvent};
use super::vlq::read_vlq;
use crate::types::AudioError;

/// SMF time division (the MThd `division` field).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Division {
    /// Metrical time: ticks (pulses) per quarter note. High bit is 0.
    Ppq(u16),
    /// SMPTE time-code based division. High bit is 1.
    Smpte {
        /// Frames per second (25, 24, 29 for 29.97-drop, or 30).
        fps: u8,
        /// Ticks (subdivisions) per frame.
        ticks_per_frame: u8,
    },
}

impl Division {
    /// Decode the raw 16-bit MThd division field.
    pub fn from_raw(raw: u16) -> Self {
        if raw & 0x8000 != 0 {
            // High byte is a signed negative frame count (two's complement).
            let neg = ((raw >> 8) & 0xFF) as u8;
            let fps = (256u16 - u16::from(neg)) as u8;
            Division::Smpte {
                fps,
                ticks_per_frame: (raw & 0xFF) as u8,
            }
        } else {
            Division::Ppq(raw & 0x7FFF)
        }
    }

    /// Encode back to the raw 16-bit MThd field.
    pub fn to_raw(self) -> u16 {
        match self {
            Division::Ppq(ppq) => ppq & 0x7FFF,
            Division::Smpte { fps, ticks_per_frame } => {
                let neg = (256u16 - u16::from(fps)) & 0xFF;
                (neg << 8) | u16::from(ticks_per_frame)
            }
        }
    }
}

/// A single track event: MIDI channel message, meta-event, or system-exclusive.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TrackEvent {
    /// Channel voice/mode message. `status` includes the channel nibble; `data`
    /// holds the 1 or 2 data bytes that follow.
    Midi { status: u8, data: Vec<u8> },
    /// Meta-event (`0xFF ...`).
    Meta(MetaEvent),
    /// System-exclusive. `start` is `0xF0` (normal) or `0xF7` (escape/continued);
    /// `data` is the payload bytes that were length-prefixed in the file
    /// (including any terminating `0xF7` for a complete `0xF0` message).
    SysEx { start: u8, data: Vec<u8> },
}

/// A delta-timed track event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Event {
    /// Ticks since the previous event in the track.
    pub delta_ticks: u32,
    /// The event itself.
    pub event: TrackEvent,
}

/// One MTrk track: an ordered list of delta-timed events.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Track {
    pub events: Vec<Event>,
}

/// A parsed Standard MIDI File.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SmfFile {
    /// 0 (single multi-channel track), 1 (multiple simultaneous tracks), or 2
    /// (multiple independent sequences).
    pub format: u16,
    /// Time division from the header.
    pub division: Division,
    /// The tracks, in file order.
    pub tracks: Vec<Track>,
}

/// Number of data bytes following a channel-message status byte.
///
/// Returns `None` if `status` is not a channel voice/mode status (`0x80..=0xEF`).
pub(super) fn channel_data_len(status: u8) -> Option<usize> {
    match status & 0xF0 {
        0x80 | 0x90 | 0xA0 | 0xB0 | 0xE0 => Some(2),
        0xC0 | 0xD0 => Some(1),
        _ => None,
    }
}

/// Read a big-endian u32 at `pos`, advancing nothing.
fn read_u32_be(bytes: &[u8], pos: usize) -> Result<u32, AudioError> {
    let s = bytes.get(pos..pos + 4).ok_or(AudioError::MalformedAudio)?;
    Ok((u32::from(s[0]) << 24) | (u32::from(s[1]) << 16) | (u32::from(s[2]) << 8) | u32::from(s[3]))
}

/// Read a big-endian u16 at `pos`.
fn read_u16_be(bytes: &[u8], pos: usize) -> Result<u16, AudioError> {
    let s = bytes.get(pos..pos + 2).ok_or(AudioError::MalformedAudio)?;
    Ok((u16::from(s[0]) << 8) | u16::from(s[1]))
}

/// Parse a complete Standard MIDI File byte buffer.
///
/// Validates the `MThd` header (must declare format 0/1/2 and a 6-byte body),
/// then parses exactly `ntracks` `MTrk` chunks. Non-`MTrk` chunks encountered
/// where a track is expected are rejected. Returns [`AudioError::MalformedAudio`]
/// for structural problems and [`AudioError::UnsupportedFormat`] for an unknown
/// SMF format number.
pub fn read_smf(bytes: &[u8]) -> Result<SmfFile, AudioError> {
    // --- Header chunk (MThd) ---
    if bytes.len() < 14 || &bytes[0..4] != b"MThd" {
        return Err(AudioError::MalformedAudio);
    }
    let header_len = read_u32_be(bytes, 4)?;
    if header_len != 6 {
        return Err(AudioError::MalformedAudio);
    }
    let format = read_u16_be(bytes, 8)?;
    if format > 2 {
        return Err(AudioError::UnsupportedFormat);
    }
    let ntracks = read_u16_be(bytes, 10)? as usize;
    let division = Division::from_raw(read_u16_be(bytes, 12)?);

    // Header body may in principle be longer than 6, but we validated == 6.
    let mut pos = 8 + header_len as usize;

    // --- Track chunks (MTrk) ---
    let mut tracks = Vec::with_capacity(ntracks);
    for _ in 0..ntracks {
        // Chunk header: 4-byte id + 4-byte length.
        let id = bytes.get(pos..pos + 4).ok_or(AudioError::MalformedAudio)?;
        let chunk_len = read_u32_be(bytes, pos + 4)? as usize;
        let body_start = pos + 8;
        let body_end = body_start
            .checked_add(chunk_len)
            .ok_or(AudioError::MalformedAudio)?;
        let body = bytes.get(body_start..body_end).ok_or(AudioError::MalformedAudio)?;

        if id == b"MTrk" {
            tracks.push(parse_track(body)?);
        } else {
            // Alien chunk where a track was declared: reject rather than guess.
            return Err(AudioError::MalformedAudio);
        }
        pos = body_end;
    }

    Ok(SmfFile { format, division, tracks })
}

/// Parse a single MTrk body (the bytes between the chunk length and its end)
/// into a [`Track`], expanding running status.
fn parse_track(body: &[u8]) -> Result<Track, AudioError> {
    let mut events = Vec::new();
    let mut pos = 0usize;
    let mut running_status: Option<u8> = None;

    while pos < body.len() {
        // Delta time.
        let (delta_ticks, used) = read_vlq(&body[pos..])?;
        pos += used;

        let first = *body.get(pos).ok_or(AudioError::MalformedAudio)?;

        let event = if first == 0xFF {
            // Meta-event: 0xFF <type> <vlq len> <data>. Clears running status.
            running_status = None;
            pos += 1;
            let meta_type = *body.get(pos).ok_or(AudioError::MalformedAudio)?;
            pos += 1;
            let (len, used) = read_vlq(&body[pos..])?;
            pos += used;
            let len = len as usize;
            let data = body.get(pos..pos + len).ok_or(AudioError::MalformedAudio)?;
            pos += len;
            TrackEvent::Meta(parse_meta(meta_type, data)?)
        } else if first == 0xF0 || first == 0xF7 {
            // SysEx (0xF0) or escape/continuation (0xF7): <status> <vlq len> <data>.
            running_status = None;
            let start = first;
            pos += 1;
            let (len, used) = read_vlq(&body[pos..])?;
            pos += used;
            let len = len as usize;
            let data = body.get(pos..pos + len).ok_or(AudioError::MalformedAudio)?;
            pos += len;
            TrackEvent::SysEx { start, data: data.to_vec() }
        } else if first & 0x80 != 0 {
            // Explicit channel status byte.
            let status = first;
            pos += 1;
            let dlen = channel_data_len(status).ok_or(AudioError::MalformedAudio)?;
            let data = body.get(pos..pos + dlen).ok_or(AudioError::MalformedAudio)?;
            pos += dlen;
            running_status = Some(status);
            TrackEvent::Midi { status, data: data.to_vec() }
        } else {
            // Data byte with no status → running status: reuse last channel status.
            let status = running_status.ok_or(AudioError::MalformedAudio)?;
            let dlen = channel_data_len(status).ok_or(AudioError::MalformedAudio)?;
            let data = body.get(pos..pos + dlen).ok_or(AudioError::MalformedAudio)?;
            pos += dlen;
            TrackEvent::Midi { status, data: data.to_vec() }
        };

        events.push(Event { delta_ticks, event });
    }

    Ok(Track { events })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn division_raw_round_trip() {
        assert_eq!(Division::from_raw(480), Division::Ppq(480));
        assert_eq!(Division::Ppq(480).to_raw(), 480);
        // 25 fps, 40 ticks/frame → high byte 0xE7 (256-25), low 0x28.
        let smpte = Division::from_raw(0xE728);
        assert_eq!(smpte, Division::Smpte { fps: 25, ticks_per_frame: 0x28 });
        assert_eq!(smpte.to_raw(), 0xE728);
    }

    #[test]
    fn channel_data_lengths() {
        assert_eq!(channel_data_len(0x90), Some(2)); // note on
        assert_eq!(channel_data_len(0xC0), Some(1)); // program change
        assert_eq!(channel_data_len(0xE3), Some(2)); // pitch bend, ch 3
        assert_eq!(channel_data_len(0xF0), None); // system
    }

    #[test]
    fn rejects_bad_magic() {
        assert_eq!(read_smf(b"NOPExxxxxxxxxx"), Err(AudioError::MalformedAudio));
    }

    #[test]
    fn rejects_unknown_format() {
        // Valid MThd framing but format = 3.
        let mut b = Vec::new();
        b.extend_from_slice(b"MThd");
        b.extend_from_slice(&6u32.to_be_bytes());
        b.extend_from_slice(&3u16.to_be_bytes());
        b.extend_from_slice(&0u16.to_be_bytes());
        b.extend_from_slice(&96u16.to_be_bytes());
        assert_eq!(read_smf(&b), Err(AudioError::UnsupportedFormat));
    }

    #[test]
    fn running_status_expands() {
        // MThd format 0, 1 track, division 96.
        let mut b = Vec::new();
        b.extend_from_slice(b"MThd");
        b.extend_from_slice(&6u32.to_be_bytes());
        b.extend_from_slice(&0u16.to_be_bytes());
        b.extend_from_slice(&1u16.to_be_bytes());
        b.extend_from_slice(&96u16.to_be_bytes());
        // Track: note-on (explicit 0x90 3C 40), then running-status note-on (00 3E 40),
        // then end-of-track.
        let track_body: &[u8] = &[
            0x00, 0x90, 0x3C, 0x40, // delta 0, note on C, vel 64
            0x00, 0x3E, 0x40, // delta 0, running status note on D, vel 64
            0x00, 0xFF, 0x2F, 0x00, // end of track
        ];
        b.extend_from_slice(b"MTrk");
        b.extend_from_slice(&(track_body.len() as u32).to_be_bytes());
        b.extend_from_slice(track_body);

        let smf = read_smf(&b).unwrap();
        assert_eq!(smf.tracks.len(), 1);
        let evs = &smf.tracks[0].events;
        assert_eq!(evs.len(), 3);
        assert_eq!(evs[0].event, TrackEvent::Midi { status: 0x90, data: vec![0x3C, 0x40] });
        // Running status expanded to explicit 0x90.
        assert_eq!(evs[1].event, TrackEvent::Midi { status: 0x90, data: vec![0x3E, 0x40] });
        assert_eq!(evs[2].event, TrackEvent::Meta(MetaEvent::EndOfTrack));
    }
}
