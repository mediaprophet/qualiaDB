//! MIDI 1.0 System Exclusive (SysEx) framing over caller buffers.
//!
//! A SysEx message is `F0 <payload...> F7`, where every payload byte is a data
//! byte (high bit clear, 0..=127). These helpers are zero-heap: [`frame_sysex`]
//! writes into a caller buffer and returns the length; [`sysex_payload`] borrows
//! the payload slice out of a complete frame without copying.

use crate::types::AudioError;

/// SysEx start byte.
pub const SYSEX_START: u8 = 0xF0;
/// SysEx end (EOX) byte.
pub const SYSEX_END: u8 = 0xF7;

/// Frame `payload` as `F0 <payload> F7` into `out`, returning the total number
/// of bytes written (`payload.len() + 2`).
///
/// Errors:
/// - [`AudioError::InvalidParameter`] if any payload byte has its high bit set
///   (SysEx payloads carry only 7-bit data bytes).
/// - [`AudioError::OutputBufferTooSmall`] if `out` cannot hold the frame.
pub fn frame_sysex(payload: &[u8], out: &mut [u8]) -> Result<usize, AudioError> {
    let total = payload.len() + 2;
    if out.len() < total {
        return Err(AudioError::OutputBufferTooSmall);
    }
    for &b in payload {
        if b > 127 {
            return Err(AudioError::InvalidParameter);
        }
    }
    out[0] = SYSEX_START;
    out[1..1 + payload.len()].copy_from_slice(payload);
    out[1 + payload.len()] = SYSEX_END;
    Ok(total)
}

/// Borrow the payload (bytes between `F0` and `F7`) from a complete SysEx frame.
///
/// Errors [`AudioError::MalformedAudio`] if `frame` does not start with `F0`,
/// does not end with `F7`, is shorter than 2 bytes, or contains a status byte
/// (high bit set) inside the payload.
pub fn sysex_payload(frame: &[u8]) -> Result<&[u8], AudioError> {
    if frame.len() < 2 || frame[0] != SYSEX_START || frame[frame.len() - 1] != SYSEX_END {
        return Err(AudioError::MalformedAudio);
    }
    let payload = &frame[1..frame.len() - 1];
    for &b in payload {
        if b > 127 {
            return Err(AudioError::MalformedAudio);
        }
    }
    Ok(payload)
}

/// Universal SysEx: build a Non-Real-Time / Real-Time universal header frame.
///
/// `F0 <7E|7F> <device_id> <sub_id1> <sub_id2> <payload...> F7`. `real_time`
/// selects `0x7F` (real-time) vs `0x7E` (non-real-time). Returns bytes written.
pub fn frame_universal(
    real_time: bool,
    device_id: u8,
    sub_id1: u8,
    sub_id2: u8,
    payload: &[u8],
    out: &mut [u8],
) -> Result<usize, AudioError> {
    let total = payload.len() + 6;
    if out.len() < total {
        return Err(AudioError::OutputBufferTooSmall);
    }
    if device_id > 127 || sub_id1 > 127 || sub_id2 > 127 {
        return Err(AudioError::InvalidParameter);
    }
    for &b in payload {
        if b > 127 {
            return Err(AudioError::InvalidParameter);
        }
    }
    out[0] = SYSEX_START;
    out[1] = if real_time { 0x7F } else { 0x7E };
    out[2] = device_id;
    out[3] = sub_id1;
    out[4] = sub_id2;
    out[5..5 + payload.len()].copy_from_slice(payload);
    out[5 + payload.len()] = SYSEX_END;
    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_and_unframe() {
        let payload = [0x43, 0x12, 0x00]; // e.g. Yamaha manufacturer + data
        let mut buf = [0u8; 8];
        let n = frame_sysex(&payload, &mut buf).unwrap();
        assert_eq!(n, 5);
        assert_eq!(&buf[..n], &[0xF0, 0x43, 0x12, 0x00, 0xF7]);
        assert_eq!(sysex_payload(&buf[..n]).unwrap(), &payload);
    }

    #[test]
    fn buffer_too_small() {
        let mut buf = [0u8; 2];
        assert_eq!(frame_sysex(&[1, 2, 3], &mut buf), Err(AudioError::OutputBufferTooSmall));
    }

    #[test]
    fn rejects_status_in_payload() {
        let mut buf = [0u8; 8];
        assert_eq!(frame_sysex(&[0x80], &mut buf), Err(AudioError::InvalidParameter));
        assert_eq!(sysex_payload(&[0xF0, 0x80, 0xF7]), Err(AudioError::MalformedAudio));
        assert_eq!(sysex_payload(&[0x00, 0xF7]), Err(AudioError::MalformedAudio));
    }

    #[test]
    fn universal_header() {
        let mut buf = [0u8; 16];
        // Non-real-time, device 0x7F (all), General MIDI (7E ... 09 01) style.
        let n = frame_universal(false, 0x7F, 0x09, 0x01, &[], &mut buf).unwrap();
        assert_eq!(&buf[..n], &[0xF0, 0x7E, 0x7F, 0x09, 0x01, 0xF7]);
    }
}
