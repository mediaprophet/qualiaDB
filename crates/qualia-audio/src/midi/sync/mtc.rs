//! MIDI Time Code (MTC) — quarter-frame encode/decode of an `hh:mm:ss:ff`
//! timecode.
//!
//! A full SMPTE timecode is streamed as eight *quarter-frame* messages, each a
//! `0xF1` status byte followed by one data byte of the form `0iiidddd`: the
//! upper three bits `iii` select which of the eight nibbles (piece 0..=7) is
//! carried in the lower four bits `dddd`. Reassembling all eight pieces yields
//! the frame; the top piece also carries the high hour bit and a 2-bit frame-rate
//! code. This module encodes a [`Timecode`] to the eight data bytes and decodes
//! them back, allocation-free.

use crate::types::AudioError;

/// SMPTE frame rate, encoded in the 2-bit rate field of MTC piece 7.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FrameRate {
    /// 24 fps (film).
    Fps24,
    /// 25 fps (EBU / PAL).
    Fps25,
    /// 29.97 fps drop-frame (NTSC).
    Fps2997Drop,
    /// 30 fps (non-drop).
    Fps30,
}

impl FrameRate {
    /// The 2-bit code as it appears in MTC / SMPTE hour bytes.
    #[inline]
    pub const fn code(self) -> u8 {
        match self {
            FrameRate::Fps24 => 0,
            FrameRate::Fps25 => 1,
            FrameRate::Fps2997Drop => 2,
            FrameRate::Fps30 => 3,
        }
    }

    /// Decode a 2-bit rate code.
    #[inline]
    pub const fn from_code(code: u8) -> FrameRate {
        match code & 0b11 {
            0 => FrameRate::Fps24,
            1 => FrameRate::Fps25,
            2 => FrameRate::Fps2997Drop,
            _ => FrameRate::Fps30,
        }
    }

    /// The nominal upper frame count for range validation (max frame index + 1).
    #[inline]
    pub const fn frames_per_second(self) -> u8 {
        match self {
            FrameRate::Fps24 => 24,
            FrameRate::Fps25 => 25,
            FrameRate::Fps2997Drop | FrameRate::Fps30 => 30,
        }
    }
}

/// An SMPTE / MTC timecode: `hours:minutes:seconds:frames` at a [`FrameRate`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Timecode {
    /// Hours, 0..=23.
    pub hours: u8,
    /// Minutes, 0..=59.
    pub minutes: u8,
    /// Seconds, 0..=59.
    pub seconds: u8,
    /// Frames, 0..=(fps-1).
    pub frames: u8,
    /// Frame rate.
    pub rate: FrameRate,
}

impl Timecode {
    /// Construct and validate field ranges against the frame rate.
    pub fn new(
        hours: u8,
        minutes: u8,
        seconds: u8,
        frames: u8,
        rate: FrameRate,
    ) -> Result<Self, AudioError> {
        if hours > 23 || minutes > 59 || seconds > 59 || frames >= rate.frames_per_second() {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self { hours, minutes, seconds, frames, rate })
    }
}

/// Encode a timecode into the eight MTC quarter-frame *data bytes* (the byte
/// that follows each `0xF1` status). Element `i` has the form `(i << 4) | nibble`.
pub fn encode_quarter_frames(tc: Timecode) -> [u8; 8] {
    let nibbles = [
        tc.frames & 0x0F,               // 0: frame LSN
        (tc.frames >> 4) & 0x0F,        // 1: frame MSN
        tc.seconds & 0x0F,              // 2: seconds LSN
        (tc.seconds >> 4) & 0x0F,       // 3: seconds MSN
        tc.minutes & 0x0F,              // 4: minutes LSN
        (tc.minutes >> 4) & 0x0F,       // 5: minutes MSN
        tc.hours & 0x0F,                // 6: hours LSN
        ((tc.hours >> 4) & 0x01) | (tc.rate.code() << 1), // 7: hours MSN + rate
    ];
    let mut out = [0u8; 8];
    let mut i = 0;
    while i < 8 {
        out[i] = ((i as u8) << 4) | (nibbles[i] & 0x0F);
        i += 1;
    }
    out
}

/// Decode eight MTC quarter-frame data bytes back into a [`Timecode`].
///
/// Each byte must carry its own piece index in the upper nibble (0..=7) and the
/// eight indices must all be present exactly once. Errors on a malformed set or
/// out-of-range reconstructed fields.
pub fn decode_quarter_frames(data: &[u8; 8]) -> Result<Timecode, AudioError> {
    let mut nibbles = [0u8; 8];
    let mut seen = [false; 8];
    for &b in data.iter() {
        let idx = (b >> 4) as usize;
        if idx >= 8 || seen[idx] {
            return Err(AudioError::InvalidParameter);
        }
        seen[idx] = true;
        nibbles[idx] = b & 0x0F;
    }
    if seen.iter().any(|&s| !s) {
        return Err(AudioError::InvalidParameter);
    }
    let frames = nibbles[0] | (nibbles[1] << 4);
    let seconds = nibbles[2] | (nibbles[3] << 4);
    let minutes = nibbles[4] | (nibbles[5] << 4);
    let rate = FrameRate::from_code(nibbles[7] >> 1);
    let hours = nibbles[6] | ((nibbles[7] & 0x01) << 4);
    Timecode::new(hours, minutes, seconds, frames, rate)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_round_trip() {
        let tc = Timecode::new(17, 42, 53, 12, FrameRate::Fps25).unwrap();
        let bytes = encode_quarter_frames(tc);
        let back = decode_quarter_frames(&bytes).unwrap();
        assert_eq!(tc, back);
    }

    #[test]
    fn round_trip_hour_high_bit() {
        // hours 23 needs the high bit in piece 7.
        let tc = Timecode::new(23, 59, 59, 29, FrameRate::Fps30).unwrap();
        let bytes = encode_quarter_frames(tc);
        assert_eq!(decode_quarter_frames(&bytes).unwrap(), tc);
    }

    #[test]
    fn rate_preserved() {
        for rate in [FrameRate::Fps24, FrameRate::Fps25, FrameRate::Fps2997Drop, FrameRate::Fps30] {
            let tc = Timecode::new(1, 2, 3, 4, rate).unwrap();
            assert_eq!(decode_quarter_frames(&encode_quarter_frames(tc)).unwrap().rate, rate);
        }
    }

    #[test]
    fn rejects_duplicate_piece() {
        let mut bytes = encode_quarter_frames(Timecode::new(1, 2, 3, 4, FrameRate::Fps24).unwrap());
        bytes[1] = bytes[0]; // duplicate piece 0
        assert_eq!(decode_quarter_frames(&bytes), Err(AudioError::InvalidParameter));
    }

    #[test]
    fn rejects_out_of_range_field() {
        assert!(Timecode::new(24, 0, 0, 0, FrameRate::Fps24).is_err());
        assert!(Timecode::new(0, 0, 0, 24, FrameRate::Fps24).is_err());
    }
}
