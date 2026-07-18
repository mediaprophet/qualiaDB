//! SMF meta-event model with parse/serialize.
//!
//! Meta-events live inside MTrk chunks, framed as `0xFF <type> <vlq len> <data>`.
//! This module owns the *payload* codec: [`parse_meta`] decodes a `(type, data)`
//! pair into a [`MetaEvent`], and [`serialize_meta`] writes the full
//! `0xFF type len data` framing back out. The named variants (tempo,
//! time-signature, track-name, end-of-track, key-signature, sequence number)
//! are decoded structurally; every other meta type is preserved byte-exact via
//! [`MetaEvent::Unknown`], so round-trips are lossless.
//!
//! Lane AU-MIDI-FILE.

use super::vlq::write_vlq;
use crate::types::AudioError;

/// Meta-event type bytes (the `<type>` after `0xFF`).
pub const META_SEQUENCE_NUMBER: u8 = 0x00;
pub const META_TRACK_NAME: u8 = 0x03;
pub const META_END_OF_TRACK: u8 = 0x2F;
pub const META_TEMPO: u8 = 0x51;
pub const META_TIME_SIGNATURE: u8 = 0x58;
pub const META_KEY_SIGNATURE: u8 = 0x59;

/// A parsed SMF meta-event payload.
///
/// `Unknown` carries any meta type this module does not decode structurally
/// (text, marker, SMPTE offset, sequencer-specific, …) so serialization is
/// byte-identical to the source.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetaEvent {
    /// `0x00` — optional 16-bit sequence number.
    SequenceNumber(u16),
    /// `0x03` — track name (raw bytes; SMF text is not guaranteed UTF-8).
    TrackName(Vec<u8>),
    /// `0x2F` — end of track (zero-length).
    EndOfTrack,
    /// `0x51` — tempo in microseconds per quarter note (24-bit range).
    Tempo(u32),
    /// `0x58` — time signature.
    TimeSignature {
        /// Numerator, as written.
        numerator: u8,
        /// Denominator as a negative power of two (`2` ⇒ 1/4). This is the raw
        /// SMF field, i.e. denominator = 2^value.
        denominator_pow2: u8,
        /// MIDI clocks per metronome click.
        clocks_per_click: u8,
        /// Number of notated 32nd-notes per MIDI quarter-note (usually 8).
        thirty_seconds_per_quarter: u8,
    },
    /// `0x59` — key signature.
    KeySignature {
        /// Sharps (positive) / flats (negative), -7..=7.
        sharps: i8,
        /// `true` for minor, `false` for major.
        minor: bool,
    },
    /// Any other meta type, preserved verbatim.
    Unknown { meta_type: u8, data: Vec<u8> },
}

/// Decode a meta-event from its type byte and payload `data` (the bytes after
/// the VLQ length, exactly `len` of them).
///
/// Structurally-decoded types validate their fixed length and error with
/// [`AudioError::MalformedAudio`] on a mismatch; unrecognised types are kept in
/// [`MetaEvent::Unknown`].
pub fn parse_meta(meta_type: u8, data: &[u8]) -> Result<MetaEvent, AudioError> {
    match meta_type {
        META_SEQUENCE_NUMBER => {
            // Spec allows length 0 (implicit) or 2.
            match data.len() {
                0 => Ok(MetaEvent::SequenceNumber(0)),
                2 => Ok(MetaEvent::SequenceNumber(
                    (u16::from(data[0]) << 8) | u16::from(data[1]),
                )),
                _ => Err(AudioError::MalformedAudio),
            }
        }
        META_TRACK_NAME => Ok(MetaEvent::TrackName(data.to_vec())),
        META_END_OF_TRACK => {
            if data.is_empty() {
                Ok(MetaEvent::EndOfTrack)
            } else {
                Err(AudioError::MalformedAudio)
            }
        }
        META_TEMPO => {
            if data.len() != 3 {
                return Err(AudioError::MalformedAudio);
            }
            let us = (u32::from(data[0]) << 16) | (u32::from(data[1]) << 8) | u32::from(data[2]);
            Ok(MetaEvent::Tempo(us))
        }
        META_TIME_SIGNATURE => {
            if data.len() != 4 {
                return Err(AudioError::MalformedAudio);
            }
            Ok(MetaEvent::TimeSignature {
                numerator: data[0],
                denominator_pow2: data[1],
                clocks_per_click: data[2],
                thirty_seconds_per_quarter: data[3],
            })
        }
        META_KEY_SIGNATURE => {
            if data.len() != 2 {
                return Err(AudioError::MalformedAudio);
            }
            Ok(MetaEvent::KeySignature {
                sharps: data[0] as i8,
                minor: data[1] != 0,
            })
        }
        other => Ok(MetaEvent::Unknown {
            meta_type: other,
            data: data.to_vec(),
        }),
    }
}

/// Serialize a meta-event as `0xFF <type> <vlq len> <data>`, appended to `out`.
///
/// Returns the number of bytes written.
pub fn serialize_meta(event: &MetaEvent, out: &mut Vec<u8>) -> Result<usize, AudioError> {
    let start = out.len();
    out.push(0xFF);
    match event {
        MetaEvent::SequenceNumber(n) => {
            out.push(META_SEQUENCE_NUMBER);
            write_vlq(2, out)?;
            out.push((n >> 8) as u8);
            out.push((n & 0xFF) as u8);
        }
        MetaEvent::TrackName(name) => {
            out.push(META_TRACK_NAME);
            write_len(name.len(), out)?;
            out.extend_from_slice(name);
        }
        MetaEvent::EndOfTrack => {
            out.push(META_END_OF_TRACK);
            write_vlq(0, out)?;
        }
        MetaEvent::Tempo(us) => {
            if *us > 0x00FF_FFFF {
                return Err(AudioError::InvalidParameter);
            }
            out.push(META_TEMPO);
            write_vlq(3, out)?;
            out.push((us >> 16) as u8);
            out.push((us >> 8) as u8);
            out.push((us & 0xFF) as u8);
        }
        MetaEvent::TimeSignature {
            numerator,
            denominator_pow2,
            clocks_per_click,
            thirty_seconds_per_quarter,
        } => {
            out.push(META_TIME_SIGNATURE);
            write_vlq(4, out)?;
            out.push(*numerator);
            out.push(*denominator_pow2);
            out.push(*clocks_per_click);
            out.push(*thirty_seconds_per_quarter);
        }
        MetaEvent::KeySignature { sharps, minor } => {
            out.push(META_KEY_SIGNATURE);
            write_vlq(2, out)?;
            out.push(*sharps as u8);
            out.push(if *minor { 1 } else { 0 });
        }
        MetaEvent::Unknown { meta_type, data } => {
            out.push(*meta_type);
            write_len(data.len(), out)?;
            out.extend_from_slice(data);
        }
    }
    Ok(out.len() - start)
}

/// Write a payload length as a VLQ, guarding the 28-bit VLQ ceiling.
fn write_len(len: usize, out: &mut Vec<u8>) -> Result<(), AudioError> {
    let len = u32::try_from(len).map_err(|_| AudioError::InvalidParameter)?;
    write_vlq(len, out)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Round-trip a meta-event through serialize → (strip 0xFF/type/len) → parse.
    fn round_trip(event: &MetaEvent) {
        let mut bytes = Vec::new();
        serialize_meta(event, &mut bytes).unwrap();
        assert_eq!(bytes[0], 0xFF);
        let meta_type = bytes[1];
        let (len, len_bytes) = super::super::vlq::read_vlq(&bytes[2..]).unwrap();
        let data_start = 2 + len_bytes;
        let data = &bytes[data_start..data_start + len as usize];
        let parsed = parse_meta(meta_type, data).unwrap();
        assert_eq!(&parsed, event);
    }

    #[test]
    fn tempo_120bpm_bytes() {
        // 120 BPM = 500000 µs/quarter = 0x07A120.
        let mut out = Vec::new();
        serialize_meta(&MetaEvent::Tempo(500_000), &mut out).unwrap();
        assert_eq!(out, vec![0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);
        assert_eq!(parse_meta(0x51, &[0x07, 0xA1, 0x20]).unwrap(), MetaEvent::Tempo(500_000));
    }

    #[test]
    fn end_of_track_bytes() {
        let mut out = Vec::new();
        serialize_meta(&MetaEvent::EndOfTrack, &mut out).unwrap();
        assert_eq!(out, vec![0xFF, 0x2F, 0x00]);
    }

    #[test]
    fn time_signature_4_4() {
        // 4/4, 24 clocks/click, 8 32nds/quarter.
        let ev = MetaEvent::TimeSignature {
            numerator: 4,
            denominator_pow2: 2,
            clocks_per_click: 24,
            thirty_seconds_per_quarter: 8,
        };
        let mut out = Vec::new();
        serialize_meta(&ev, &mut out).unwrap();
        assert_eq!(out, vec![0xFF, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08]);
        round_trip(&ev);
    }

    #[test]
    fn key_signature_a_major_and_f_minor() {
        // A major = 3 sharps, major.
        let a_major = MetaEvent::KeySignature { sharps: 3, minor: false };
        let mut out = Vec::new();
        serialize_meta(&a_major, &mut out).unwrap();
        assert_eq!(out, vec![0xFF, 0x59, 0x02, 0x03, 0x00]);
        // F minor = 4 flats, minor.
        let f_minor = MetaEvent::KeySignature { sharps: -4, minor: true };
        let mut out2 = Vec::new();
        serialize_meta(&f_minor, &mut out2).unwrap();
        assert_eq!(out2, vec![0xFF, 0x59, 0x02, 0xFC, 0x01]);
        assert_eq!(
            parse_meta(0x59, &[0xFC, 0x01]).unwrap(),
            MetaEvent::KeySignature { sharps: -4, minor: true }
        );
    }

    #[test]
    fn track_name_round_trip() {
        round_trip(&MetaEvent::TrackName(b"Lead".to_vec()));
    }

    #[test]
    fn unknown_meta_preserved() {
        // 0x01 (text) is not structurally decoded → Unknown, byte-exact.
        let ev = MetaEvent::Unknown { meta_type: 0x01, data: b"hi".to_vec() };
        let mut out = Vec::new();
        serialize_meta(&ev, &mut out).unwrap();
        assert_eq!(out, vec![0xFF, 0x01, 0x02, b'h', b'i']);
        round_trip(&ev);
    }

    #[test]
    fn bad_lengths_rejected() {
        assert_eq!(parse_meta(META_TEMPO, &[0x00, 0x00]), Err(AudioError::MalformedAudio));
        assert_eq!(parse_meta(META_END_OF_TRACK, &[0x00]), Err(AudioError::MalformedAudio));
        assert_eq!(parse_meta(META_KEY_SIGNATURE, &[0x00]), Err(AudioError::MalformedAudio));
        assert_eq!(parse_meta(META_TIME_SIGNATURE, &[0x04, 0x02]), Err(AudioError::MalformedAudio));
    }
}
