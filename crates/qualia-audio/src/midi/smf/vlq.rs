//! Variable-Length Quantity (VLQ) codec for Standard MIDI Files.
//!
//! SMF delta-times and several length fields are stored as VLQs: a big-endian
//! base-128 number where each byte carries 7 payload bits and the high bit
//! (`0x80`) signals "more bytes follow". The SMF spec caps a VLQ at 4 bytes,
//! i.e. 28 payload bits (`0x0FFF_FFFF`). Cold path — `Vec`/slice use is fine.
//!
//! Lane AU-MIDI-FILE.

use crate::types::AudioError;

/// Largest value representable in a 4-byte SMF VLQ (28 bits).
pub const VLQ_MAX: u32 = 0x0FFF_FFFF;

/// Decode a VLQ from the front of `bytes`.
///
/// Returns `(value, bytes_consumed)`. Errors with [`AudioError::MalformedAudio`]
/// if the input ends mid-number or the quantity exceeds the 4-byte / 28-bit
/// SMF limit.
pub fn read_vlq(bytes: &[u8]) -> Result<(u32, usize), AudioError> {
    let mut value: u32 = 0;
    let mut consumed: usize = 0;
    loop {
        // A VLQ is at most 4 bytes; a 5th continuation byte is malformed.
        if consumed >= 4 {
            return Err(AudioError::MalformedAudio);
        }
        let byte = *bytes.get(consumed).ok_or(AudioError::MalformedAudio)?;
        consumed += 1;
        value = (value << 7) | u32::from(byte & 0x7F);
        if byte & 0x80 == 0 {
            return Ok((value, consumed));
        }
    }
}

/// Encode `value` as a VLQ appended to `out`.
///
/// Returns the number of bytes written (1..=4). Errors with
/// [`AudioError::InvalidParameter`] if `value` exceeds [`VLQ_MAX`].
pub fn write_vlq(value: u32, out: &mut Vec<u8>) -> Result<usize, AudioError> {
    if value > VLQ_MAX {
        return Err(AudioError::InvalidParameter);
    }
    // Build the 7-bit groups most-significant first, setting the continuation
    // bit on every group except the last.
    let mut buffer = [0u8; 4];
    let mut count = 1usize;
    buffer[0] = (value & 0x7F) as u8;
    let mut v = value >> 7;
    while v != 0 {
        // Prepend the next group with its continuation bit set.
        buffer[count] = ((v & 0x7F) as u8) | 0x80;
        count += 1;
        v >>= 7;
    }
    // `buffer` holds groups least-significant first; emit reversed.
    for i in (0..count).rev() {
        out.push(buffer[i]);
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn read_golden_values() {
        assert_eq!(read_vlq(&[0x00]).unwrap(), (0, 1));
        assert_eq!(read_vlq(&[0x7F]).unwrap(), (127, 1));
        assert_eq!(read_vlq(&[0x81, 0x00]).unwrap(), (128, 2));
        assert_eq!(read_vlq(&[0xFF, 0x7F]).unwrap(), (16383, 2));
        // Spec canonical extremes.
        assert_eq!(read_vlq(&[0x81, 0x80, 0x00]).unwrap(), (16384, 3));
        assert_eq!(read_vlq(&[0xFF, 0xFF, 0xFF, 0x7F]).unwrap(), (VLQ_MAX, 4));
    }

    #[test]
    fn write_golden_values() {
        let cases: &[(u32, &[u8])] = &[
            (0, &[0x00]),
            (127, &[0x7F]),
            (128, &[0x81, 0x00]),
            (16383, &[0xFF, 0x7F]),
            (16384, &[0x81, 0x80, 0x00]),
            (VLQ_MAX, &[0xFF, 0xFF, 0xFF, 0x7F]),
        ];
        for (value, expected) in cases {
            let mut out = Vec::new();
            let n = write_vlq(*value, &mut out).unwrap();
            assert_eq!(n, expected.len(), "byte count for {value}");
            assert_eq!(out.as_slice(), *expected, "bytes for {value}");
        }
    }

    #[test]
    fn write_read_round_trip() {
        for &value in &[0u32, 1, 127, 128, 16383, 16384, 2_097_151, VLQ_MAX] {
            let mut out = Vec::new();
            write_vlq(value, &mut out).unwrap();
            let (decoded, consumed) = read_vlq(&out).unwrap();
            assert_eq!(decoded, value);
            assert_eq!(consumed, out.len());
        }
    }

    #[test]
    fn read_truncated_is_error() {
        // Continuation bit set but no following byte.
        assert_eq!(read_vlq(&[0x81]), Err(AudioError::MalformedAudio));
        assert_eq!(read_vlq(&[]), Err(AudioError::MalformedAudio));
    }

    #[test]
    fn read_overlong_is_error() {
        // Five continuation bytes exceed the 4-byte cap.
        assert_eq!(
            read_vlq(&[0x80, 0x80, 0x80, 0x80, 0x00]),
            Err(AudioError::MalformedAudio)
        );
    }

    #[test]
    fn write_rejects_oversize() {
        let mut out = Vec::new();
        assert_eq!(
            write_vlq(VLQ_MAX + 1, &mut out),
            Err(AudioError::InvalidParameter)
        );
        assert!(out.is_empty());
    }
}
