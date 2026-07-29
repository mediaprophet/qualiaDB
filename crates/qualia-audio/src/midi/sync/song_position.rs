//! Song Position Pointer (SPP) — 14-bit MIDI-beat position codec.
//!
//! SPP is the System Common message `0xF2 LSB MSB`: two 7-bit data bytes form a
//! 14-bit value counting **MIDI beats** from the start of the song, where one
//! MIDI beat = 6 MIDI clocks = a sixteenth note. The value range is
//! `0 ..= 16383`. This module encodes a beat count into the two data bytes
//! (least-significant 7 bits first) and decodes them back, validating that the
//! data bytes have their high bit clear. Allocation-free.

use crate::types::AudioError;

/// Song Position Pointer status byte (System Common).
pub const SONG_POSITION: u8 = 0xF2;

/// Maximum representable MIDI-beat position (14 bits).
pub const MAX_BEATS: u16 = 0x3FFF;

/// Encode a MIDI-beat position into the two SPP data bytes `[LSB, MSB]`.
///
/// Errors with [`AudioError::InvalidParameter`] if `beats > 16383`.
#[inline]
pub fn encode_song_position(beats: u16) -> Result<[u8; 2], AudioError> {
    if beats > MAX_BEATS {
        return Err(AudioError::InvalidParameter);
    }
    let lsb = (beats & 0x7F) as u8;
    let msb = ((beats >> 7) & 0x7F) as u8;
    Ok([lsb, msb])
}

/// Decode the two SPP data bytes back into a MIDI-beat position.
///
/// Errors with [`AudioError::InvalidParameter`] if either byte has its high bit
/// set (data bytes must be 7-bit).
#[inline]
pub fn decode_song_position(lsb: u8, msb: u8) -> Result<u16, AudioError> {
    if lsb & 0x80 != 0 || msb & 0x80 != 0 {
        return Err(AudioError::InvalidParameter);
    }
    Ok(((msb as u16) << 7) | lsb as u16)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_eight_beats_round_trip() {
        let [lsb, msb] = encode_song_position(8).unwrap();
        assert_eq!([lsb, msb], [8, 0]);
        assert_eq!(decode_song_position(lsb, msb).unwrap(), 8);
    }

    #[test]
    fn round_trip_across_range() {
        for &b in &[0u16, 1, 127, 128, 1000, 8192, 16383] {
            let [lsb, msb] = encode_song_position(b).unwrap();
            assert_eq!(decode_song_position(lsb, msb).unwrap(), b);
        }
    }

    #[test]
    fn rejects_overflow_and_bad_bytes() {
        assert_eq!(
            encode_song_position(16384),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            decode_song_position(0x80, 0),
            Err(AudioError::InvalidParameter)
        );
        assert_eq!(
            decode_song_position(0, 0x80),
            Err(AudioError::InvalidParameter)
        );
    }
}
