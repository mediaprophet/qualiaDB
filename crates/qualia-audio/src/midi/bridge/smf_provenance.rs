//! Extract provenance `(key, value)` pairs from a parsed Standard MIDI File for
//! NQuin emission.
//!
//! Lane AU-MIDI-BRIDGE. Walks a parsed [`SmfFile`]'s meta-events and lifts the
//! human-meaningful provenance — track names, tempo, time signature, key
//! signature, sequence number, and the copyright notice — into a bounded,
//! `Copy`, heap-free [`ProvenancePair`] the graph layer can turn into NQuins.
//! Reuses the existing [`crate::midi::smf`] model; it parses nothing itself.
//!
//! # Epistemic contract — SMF provenance is AUTHORITATIVE
//!
//! An SMF is authored / imported content, so its embedded metadata (who named
//! the track, the declared tempo, the copyright holder) is an **authoritative**
//! assertion by the file's author — not a transcription proposal. These pairs
//! are safe to record as stated provenance; they carry no confidence because
//! none is implied.
//!
//! Zero-heap: pairs are written into the caller-supplied `out` slice; each pair
//! stores its value inline in a fixed [`MAX_PROV_VALUE`]-byte buffer (longer text
//! is truncated).

use crate::midi::smf::{MetaEvent, SmfFile, TrackEvent};
use crate::types::AudioError;

/// SMF copyright-notice meta type (`0xFF 0x02`); not structurally decoded by the
/// `smf` module, so it arrives as [`MetaEvent::Unknown`].
pub const META_COPYRIGHT: u8 = 0x02;

/// Maximum inline value length. Track-name / copyright text longer than this is
/// truncated; all structured values (tempo, time/key signature, sequence number)
/// fit comfortably.
pub const MAX_PROV_VALUE: usize = 64;

/// The kind of provenance a [`ProvenancePair`] carries.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProvenanceKey {
    /// `0x03` track name. Value = raw name bytes (SMF text, not guaranteed UTF-8).
    TrackName = 0,
    /// `0x51` tempo, microseconds per quarter note. Value = 4-byte big-endian `u32`.
    Tempo = 1,
    /// `0x58` time signature. Value = `[numerator, denominator_pow2,
    /// clocks_per_click, thirty_seconds_per_quarter]` (4 bytes).
    TimeSignature = 2,
    /// `0x59` key signature. Value = `[sharps_as_i8_bits, minor_flag]` (2 bytes).
    KeySignature = 3,
    /// `0x00` sequence number. Value = 2-byte big-endian `u16`.
    SequenceNumber = 4,
    /// `0x02` copyright notice. Value = raw text bytes.
    Copyright = 5,
}

/// One bounded provenance pair: a typed key + inline value bytes.
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ProvenancePair {
    /// Which track (index into [`SmfFile::tracks`]) the pair came from.
    pub track: u16,
    /// The provenance kind.
    pub key: ProvenanceKey,
    /// Number of valid bytes in `value`.
    pub value_len: u8,
    /// Inline value bytes; only `value[..value_len]` is meaningful.
    pub value: [u8; MAX_PROV_VALUE],
}

impl ProvenancePair {
    /// The bytes actually carried by this pair.
    #[inline]
    pub fn value(&self) -> &[u8] {
        &self.value[..self.value_len as usize]
    }

    /// Construct a pair, copying up to [`MAX_PROV_VALUE`] bytes of `bytes`
    /// (truncating any excess).
    #[inline]
    fn new(track: u16, key: ProvenanceKey, bytes: &[u8]) -> Self {
        let mut value = [0u8; MAX_PROV_VALUE];
        let len = bytes.len().min(MAX_PROV_VALUE);
        value[..len].copy_from_slice(&bytes[..len]);
        Self {
            track,
            key,
            value_len: len as u8,
            value,
        }
    }
}

/// Extract provenance pairs from every track of `smf` into `out`, in track then
/// event order.
///
/// One [`ProvenancePair`] is emitted per recognised meta-event:
/// [`MetaEvent::TrackName`], [`MetaEvent::Tempo`], [`MetaEvent::TimeSignature`],
/// [`MetaEvent::KeySignature`], [`MetaEvent::SequenceNumber`], and the copyright
/// notice ([`MetaEvent::Unknown`] with type [`META_COPYRIGHT`]). All other events
/// (channel messages, SysEx, end-of-track, other Unknown metas) are skipped.
///
/// Returns the number of pairs written.
///
/// # Errors
/// - [`AudioError::OutputBufferTooSmall`] if `out` cannot hold every pair.
pub fn extract_smf_provenance(
    smf: &SmfFile,
    out: &mut [ProvenancePair],
) -> Result<usize, AudioError> {
    let mut count = 0usize;

    for (ti, track) in smf.tracks.iter().enumerate() {
        let track_idx = ti as u16;
        for ev in &track.events {
            let pair = match &ev.event {
                TrackEvent::Meta(MetaEvent::TrackName(name)) => {
                    ProvenancePair::new(track_idx, ProvenanceKey::TrackName, name)
                }
                TrackEvent::Meta(MetaEvent::Tempo(us)) => {
                    ProvenancePair::new(track_idx, ProvenanceKey::Tempo, &us.to_be_bytes())
                }
                TrackEvent::Meta(MetaEvent::TimeSignature {
                    numerator,
                    denominator_pow2,
                    clocks_per_click,
                    thirty_seconds_per_quarter,
                }) => ProvenancePair::new(
                    track_idx,
                    ProvenanceKey::TimeSignature,
                    &[
                        *numerator,
                        *denominator_pow2,
                        *clocks_per_click,
                        *thirty_seconds_per_quarter,
                    ],
                ),
                TrackEvent::Meta(MetaEvent::KeySignature { sharps, minor }) => ProvenancePair::new(
                    track_idx,
                    ProvenanceKey::KeySignature,
                    &[*sharps as u8, u8::from(*minor)],
                ),
                TrackEvent::Meta(MetaEvent::SequenceNumber(n)) => {
                    ProvenancePair::new(track_idx, ProvenanceKey::SequenceNumber, &n.to_be_bytes())
                }
                TrackEvent::Meta(MetaEvent::Unknown { meta_type, data })
                    if *meta_type == META_COPYRIGHT =>
                {
                    ProvenancePair::new(track_idx, ProvenanceKey::Copyright, data)
                }
                _ => continue,
            };

            if count >= out.len() {
                return Err(AudioError::OutputBufferTooSmall);
            }
            out[count] = pair;
            count += 1;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::midi::smf::read_smf;

    /// Build a minimal format-0 SMF (division 96) carrying a single track whose
    /// body is `track_body`.
    fn smf_with_track(track_body: &[u8]) -> Vec<u8> {
        let mut b = Vec::new();
        b.extend_from_slice(b"MThd");
        b.extend_from_slice(&6u32.to_be_bytes());
        b.extend_from_slice(&0u16.to_be_bytes()); // format 0
        b.extend_from_slice(&1u16.to_be_bytes()); // 1 track
        b.extend_from_slice(&96u16.to_be_bytes()); // division
        b.extend_from_slice(b"MTrk");
        b.extend_from_slice(&(track_body.len() as u32).to_be_bytes());
        b.extend_from_slice(track_body);
        b
    }

    #[test]
    fn golden_reads_track_name() {
        // Track: delta 0, meta track-name (0xFF 0x03 len "Lead"), then end-of-track.
        let mut body = Vec::new();
        body.extend_from_slice(&[0x00, 0xFF, 0x03, 0x04]);
        body.extend_from_slice(b"Lead");
        body.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
        let bytes = smf_with_track(&body);
        let smf = read_smf(&bytes).expect("parse smf");

        let mut out = [ProvenancePair {
            track: 0,
            key: ProvenanceKey::TrackName,
            value_len: 0,
            value: [0; MAX_PROV_VALUE],
        }; 8];
        let n = extract_smf_provenance(&smf, &mut out).expect("extract");
        assert_eq!(n, 1);
        assert_eq!(out[0].key, ProvenanceKey::TrackName);
        assert_eq!(out[0].track, 0);
        assert_eq!(out[0].value(), b"Lead");
    }

    #[test]
    fn extracts_tempo_time_sig_and_copyright() {
        let mut body = Vec::new();
        // Copyright notice (0xFF 0x02) "(c)Q".
        body.extend_from_slice(&[0x00, 0xFF, 0x02, 0x04]);
        body.extend_from_slice(b"(c)Q");
        // Tempo 500000 µs/qtr = 0x07A120.
        body.extend_from_slice(&[0x00, 0xFF, 0x51, 0x03, 0x07, 0xA1, 0x20]);
        // Time signature 4/4, 24 clocks/click, 8 32nds/qtr.
        body.extend_from_slice(&[0x00, 0xFF, 0x58, 0x04, 0x04, 0x02, 0x18, 0x08]);
        // End of track.
        body.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
        let bytes = smf_with_track(&body);
        let smf = read_smf(&bytes).expect("parse smf");

        let mut out = [ProvenancePair {
            track: 0,
            key: ProvenanceKey::TrackName,
            value_len: 0,
            value: [0; MAX_PROV_VALUE],
        }; 8];
        let n = extract_smf_provenance(&smf, &mut out).expect("extract");
        assert_eq!(n, 3);

        assert_eq!(out[0].key, ProvenanceKey::Copyright);
        assert_eq!(out[0].value(), b"(c)Q");

        assert_eq!(out[1].key, ProvenanceKey::Tempo);
        assert_eq!(
            u32::from_be_bytes(out[1].value().try_into().unwrap()),
            500_000
        );

        assert_eq!(out[2].key, ProvenanceKey::TimeSignature);
        assert_eq!(out[2].value(), &[4, 2, 24, 8]);
    }

    #[test]
    fn output_too_small_reported() {
        let mut body = Vec::new();
        body.extend_from_slice(&[0x00, 0xFF, 0x03, 0x01, b'A']);
        body.extend_from_slice(&[0x00, 0xFF, 0x03, 0x01, b'B']);
        body.extend_from_slice(&[0x00, 0xFF, 0x2F, 0x00]);
        let bytes = smf_with_track(&body);
        let smf = read_smf(&bytes).expect("parse smf");

        let mut out = [ProvenancePair {
            track: 0,
            key: ProvenanceKey::TrackName,
            value_len: 0,
            value: [0; MAX_PROV_VALUE],
        }; 1];
        assert_eq!(
            extract_smf_provenance(&smf, &mut out),
            Err(AudioError::OutputBufferTooSmall)
        );
    }
}
