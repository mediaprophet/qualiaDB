//! MIDI Capability Inquiry (MIDI-CI) Discovery message — minimal, spec-shaped.
//!
//! MIDI-CI messages are Universal Non-Real-Time System Exclusive messages:
//! `F0 7E <device_id> 0D <sub_id2> <ci_version> <src MUID> <dst MUID> …payload… F7`.
//! Multi-byte fields are little-endian groups of 7-bit bytes. The MUID is a
//! 28-bit value carried as four 7-bit bytes, LSB first; the broadcast MUID is
//! `0x0FFF_FFFF` (four `0x7F` bytes). This models the Discovery message
//! (`sub_id2 = 0x70`); build/parse operate over caller buffers (zero-heap).

use crate::types::AudioError;

/// SysEx start.
pub const SYSEX_START: u8 = 0xF0;
/// SysEx end.
pub const SYSEX_END: u8 = 0xF7;
/// Universal Non-Real-Time SysEx ID.
pub const UNIVERSAL_NON_REALTIME: u8 = 0x7E;
/// MIDI-CI sub-ID#1.
pub const MIDI_CI_SUB_ID: u8 = 0x0D;
/// Discovery message sub-ID#2.
pub const SUBID2_DISCOVERY: u8 = 0x70;
/// Broadcast MUID (28 bits all set).
pub const BROADCAST_MUID: u32 = 0x0FFF_FFFF;
/// Serialized length of a Discovery frame (bytes), `F0`..`F7` inclusive.
pub const DISCOVERY_LEN: usize = 32;

/// A MIDI-CI Discovery message (initiator → responder).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiCiDiscovery {
    /// Source device id (`0x7F` = whole MIDI port).
    pub device_id: u8,
    /// MIDI-CI version/format (e.g. `0x02` for MIDI-CI 1.2).
    pub ci_version: u8,
    /// Source MUID, 28-bit.
    pub source_muid: u32,
    /// Destination MUID, 28-bit (use [`BROADCAST_MUID`] for discovery).
    pub destination_muid: u32,
    /// Device manufacturer SysEx ID (3 bytes).
    pub manufacturer: [u8; 3],
    /// Device family, 14-bit.
    pub family: u16,
    /// Device family model number, 14-bit.
    pub model: u16,
    /// Software revision level (4 bytes).
    pub software_revision: [u8; 4],
    /// Capability Inquiry category-supported bitmap.
    pub category_supported: u8,
    /// Receivable maximum SysEx message size, 28-bit.
    pub max_sysex_size: u32,
    /// Initiator's output path id (MIDI-CI 1.2).
    pub output_path_id: u8,
}

/// Encode a 28-bit value as four 7-bit bytes, LSB first.
#[inline]
fn muid_to_bytes(v: u32) -> [u8; 4] {
    [
        (v & 0x7F) as u8,
        ((v >> 7) & 0x7F) as u8,
        ((v >> 14) & 0x7F) as u8,
        ((v >> 21) & 0x7F) as u8,
    ]
}

/// Decode four 7-bit bytes (LSB first) into a 28-bit value.
#[inline]
fn muid_from_bytes(b: &[u8]) -> u32 {
    (b[0] as u32 & 0x7F)
        | ((b[1] as u32 & 0x7F) << 7)
        | ((b[2] as u32 & 0x7F) << 14)
        | ((b[3] as u32 & 0x7F) << 21)
}

impl MidiCiDiscovery {
    /// Serialize the Discovery message into `out`, returning bytes written
    /// ([`DISCOVERY_LEN`]). Errors [`AudioError::OutputBufferTooSmall`] if `out`
    /// is too short, or [`AudioError::InvalidParameter`] on out-of-range fields.
    pub fn encode(self, out: &mut [u8]) -> Result<usize, AudioError> {
        if out.len() < DISCOVERY_LEN {
            return Err(AudioError::OutputBufferTooSmall);
        }
        if self.device_id > 0x7F
            || self.ci_version > 0x7F
            || self.source_muid > BROADCAST_MUID
            || self.destination_muid > BROADCAST_MUID
            || self.family > 0x3FFF
            || self.model > 0x3FFF
            || self.category_supported > 0x7F
            || self.max_sysex_size > BROADCAST_MUID
            || self.output_path_id > 0x7F
        {
            return Err(AudioError::InvalidParameter);
        }
        let src = muid_to_bytes(self.source_muid);
        let dst = muid_to_bytes(self.destination_muid);
        let size = muid_to_bytes(self.max_sysex_size);
        out[0] = SYSEX_START;
        out[1] = UNIVERSAL_NON_REALTIME;
        out[2] = self.device_id;
        out[3] = MIDI_CI_SUB_ID;
        out[4] = SUBID2_DISCOVERY;
        out[5] = self.ci_version;
        out[6..10].copy_from_slice(&src);
        out[10..14].copy_from_slice(&dst);
        out[14..17].copy_from_slice(&self.manufacturer);
        out[17] = (self.family & 0x7F) as u8;
        out[18] = ((self.family >> 7) & 0x7F) as u8;
        out[19] = (self.model & 0x7F) as u8;
        out[20] = ((self.model >> 7) & 0x7F) as u8;
        out[21..25].copy_from_slice(&self.software_revision);
        out[25] = self.category_supported;
        out[26..30].copy_from_slice(&size);
        out[30] = self.output_path_id;
        out[31] = SYSEX_END;
        Ok(DISCOVERY_LEN)
    }

    /// Parse a Discovery message from a complete SysEx frame. Errors
    /// [`AudioError::MalformedAudio`] if the framing / header bytes do not
    /// identify a MIDI-CI Discovery message of the expected length.
    pub fn parse(frame: &[u8]) -> Result<Self, AudioError> {
        if frame.len() != DISCOVERY_LEN
            || frame[0] != SYSEX_START
            || frame[1] != UNIVERSAL_NON_REALTIME
            || frame[3] != MIDI_CI_SUB_ID
            || frame[4] != SUBID2_DISCOVERY
            || frame[DISCOVERY_LEN - 1] != SYSEX_END
        {
            return Err(AudioError::MalformedAudio);
        }
        Ok(Self {
            device_id: frame[2],
            ci_version: frame[5],
            source_muid: muid_from_bytes(&frame[6..10]),
            destination_muid: muid_from_bytes(&frame[10..14]),
            manufacturer: [frame[14], frame[15], frame[16]],
            family: (frame[17] as u16 & 0x7F) | ((frame[18] as u16 & 0x7F) << 7),
            model: (frame[19] as u16 & 0x7F) | ((frame[20] as u16 & 0x7F) << 7),
            software_revision: [frame[21], frame[22], frame[23], frame[24]],
            category_supported: frame[25],
            max_sysex_size: muid_from_bytes(&frame[26..30]),
            output_path_id: frame[30],
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> MidiCiDiscovery {
        MidiCiDiscovery {
            device_id: 0x7F,
            ci_version: 0x02,
            source_muid: 0x0123_4567 & BROADCAST_MUID,
            destination_muid: BROADCAST_MUID,
            manufacturer: [0x00, 0x21, 0x09],
            family: 0x1234 & 0x3FFF,
            model: 0x0055,
            software_revision: [1, 0, 0, 0],
            category_supported: 0b0001_1110,
            max_sysex_size: 512,
            output_path_id: 0,
        }
    }

    #[test]
    fn discovery_roundtrip() {
        let d = sample();
        let mut buf = [0u8; DISCOVERY_LEN];
        let n = d.encode(&mut buf).unwrap();
        assert_eq!(n, DISCOVERY_LEN);
        // Header bytes are spec-shaped.
        assert_eq!(buf[0], 0xF0);
        assert_eq!(buf[1], 0x7E);
        assert_eq!(buf[3], 0x0D);
        assert_eq!(buf[4], 0x70);
        assert_eq!(buf[DISCOVERY_LEN - 1], 0xF7);
        let parsed = MidiCiDiscovery::parse(&buf).unwrap();
        assert_eq!(parsed, d);
        assert_eq!(parsed.destination_muid, BROADCAST_MUID);
        assert_eq!(parsed.category_supported, 0b0001_1110);
    }

    #[test]
    fn broadcast_muid_is_all_7f() {
        assert_eq!(muid_to_bytes(BROADCAST_MUID), [0x7F, 0x7F, 0x7F, 0x7F]);
        assert_eq!(muid_from_bytes(&[0x7F, 0x7F, 0x7F, 0x7F]), BROADCAST_MUID);
    }

    #[test]
    fn rejects_bad_frame() {
        let mut buf = [0u8; DISCOVERY_LEN];
        sample().encode(&mut buf).unwrap();
        buf[4] = 0x71; // not a discovery sub-id2
        assert_eq!(MidiCiDiscovery::parse(&buf), Err(AudioError::MalformedAudio));
        assert_eq!(MidiCiDiscovery::parse(&buf[..10]), Err(AudioError::MalformedAudio));
    }

    #[test]
    fn buffer_too_small() {
        let mut buf = [0u8; 8];
        assert_eq!(sample().encode(&mut buf), Err(AudioError::OutputBufferTooSmall));
    }
}
