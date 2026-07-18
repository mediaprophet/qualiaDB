//! MIDI Machine Control (MMC) — build and parse transport commands.
//!
//! MMC commands ride inside a Universal Real-Time SysEx:
//! `F0 7F <device> 06 <command…> F7`, where `06` is the MMC sub-ID. This module
//! handles the common transport verbs — Stop, Play, Deferred Play, Fast Forward,
//! Rewind, Pause — plus **Locate (target)**, which carries a standard-time field
//! `44 06 01 hr mn sc fr sf` (the `hr` byte packs a 2-bit frame-rate code in bits
//! 5–6, per SMPTE). Messages are built into a caller-supplied buffer and parsed
//! from a byte slice, allocation-free.

use crate::types::AudioError;

use super::mtc::{FrameRate, Timecode};

const SYSEX_START: u8 = 0xF0;
const SYSEX_END: u8 = 0xF7;
const UNIVERSAL_REALTIME: u8 = 0x7F;
const MMC_SUB_ID: u8 = 0x06;

const CMD_STOP: u8 = 0x01;
const CMD_PLAY: u8 = 0x02;
const CMD_DEFERRED_PLAY: u8 = 0x03;
const CMD_FAST_FORWARD: u8 = 0x04;
const CMD_REWIND: u8 = 0x05;
const CMD_PAUSE: u8 = 0x09;
const CMD_LOCATE: u8 = 0x44;

/// A parsed / to-be-built MMC transport command.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MmcCommand {
    /// Stop transport.
    Stop,
    /// Start playback.
    Play,
    /// Deferred play (play at the next locate/edit point).
    DeferredPlay,
    /// Fast forward.
    FastForward,
    /// Rewind.
    Rewind,
    /// Pause.
    Pause,
    /// Locate to a target timecode.
    Locate(Timecode),
}

/// Build an MMC message for `device` and `command` into `out`, returning the
/// number of bytes written. `device` is a 7-bit device id (`0x7F` = all).
///
/// Errors with [`AudioError::OutputBufferTooSmall`] if `out` is too short, or
/// [`AudioError::InvalidParameter`] if `device` has its high bit set.
pub fn build(device: u8, command: MmcCommand, out: &mut [u8]) -> Result<usize, AudioError> {
    if device & 0x80 != 0 {
        return Err(AudioError::InvalidParameter);
    }
    match command {
        MmcCommand::Locate(tc) => {
            const LEN: usize = 12; // F0 7F dev 06 44 06 01 hr mn sc fr F7
            if out.len() < LEN {
                return Err(AudioError::OutputBufferTooSmall);
            }
            out[0] = SYSEX_START;
            out[1] = UNIVERSAL_REALTIME;
            out[2] = device;
            out[3] = MMC_SUB_ID;
            out[4] = CMD_LOCATE;
            out[5] = 0x06; // length of the information field
            out[6] = 0x01; // "TARGET" sub-command
            out[7] = (tc.hours & 0x1F) | (tc.rate.code() << 5); // 0rrhhhhh
            out[8] = tc.minutes & 0x3F;
            out[9] = tc.seconds & 0x3F;
            out[10] = tc.frames & 0x1F;
            out[11] = SYSEX_END;
            Ok(LEN)
        }
        simple => {
            const LEN: usize = 6; // F0 7F dev 06 cmd F7
            if out.len() < LEN {
                return Err(AudioError::OutputBufferTooSmall);
            }
            out[0] = SYSEX_START;
            out[1] = UNIVERSAL_REALTIME;
            out[2] = device;
            out[3] = MMC_SUB_ID;
            out[4] = simple_command_byte(simple);
            out[5] = SYSEX_END;
            Ok(LEN)
        }
    }
}

#[inline]
fn simple_command_byte(cmd: MmcCommand) -> u8 {
    match cmd {
        MmcCommand::Stop => CMD_STOP,
        MmcCommand::Play => CMD_PLAY,
        MmcCommand::DeferredPlay => CMD_DEFERRED_PLAY,
        MmcCommand::FastForward => CMD_FAST_FORWARD,
        MmcCommand::Rewind => CMD_REWIND,
        MmcCommand::Pause => CMD_PAUSE,
        MmcCommand::Locate(_) => CMD_LOCATE, // handled separately in build()
    }
}

/// Parse an MMC SysEx message, returning `(device, command)`.
///
/// Validates the SysEx envelope and MMC sub-ID. Errors with
/// [`AudioError::InvalidParameter`] on any malformed / unrecognized message.
pub fn parse(msg: &[u8]) -> Result<(u8, MmcCommand), AudioError> {
    if msg.len() < 6
        || msg[0] != SYSEX_START
        || msg[1] != UNIVERSAL_REALTIME
        || msg[3] != MMC_SUB_ID
        || msg[msg.len() - 1] != SYSEX_END
    {
        return Err(AudioError::InvalidParameter);
    }
    let device = msg[2];
    let command = match msg[4] {
        CMD_STOP => MmcCommand::Stop,
        CMD_PLAY => MmcCommand::Play,
        CMD_DEFERRED_PLAY => MmcCommand::DeferredPlay,
        CMD_FAST_FORWARD => MmcCommand::FastForward,
        CMD_REWIND => MmcCommand::Rewind,
        CMD_PAUSE => MmcCommand::Pause,
        CMD_LOCATE => {
            // F0 7F dev 06 44 06 01 hr mn sc fr F7  (len 12)
            if msg.len() != 12 || msg[5] != 0x06 || msg[6] != 0x01 {
                return Err(AudioError::InvalidParameter);
            }
            let rate = FrameRate::from_code(msg[7] >> 5);
            let hours = msg[7] & 0x1F;
            let minutes = msg[8] & 0x3F;
            let seconds = msg[9] & 0x3F;
            let frames = msg[10] & 0x1F;
            MmcCommand::Locate(Timecode::new(hours, minutes, seconds, frames, rate)?)
        }
        _ => return Err(AudioError::InvalidParameter),
    };
    Ok((device, command))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_commands_round_trip() {
        let cmds = [
            MmcCommand::Stop,
            MmcCommand::Play,
            MmcCommand::DeferredPlay,
            MmcCommand::FastForward,
            MmcCommand::Rewind,
            MmcCommand::Pause,
        ];
        let mut buf = [0u8; 16];
        for &cmd in &cmds {
            let n = build(0x7F, cmd, &mut buf).unwrap();
            assert_eq!(parse(&buf[..n]).unwrap(), (0x7F, cmd));
        }
    }

    #[test]
    fn play_bytes_are_canonical() {
        let mut buf = [0u8; 6];
        let n = build(0x7F, MmcCommand::Play, &mut buf).unwrap();
        assert_eq!(&buf[..n], &[0xF0, 0x7F, 0x7F, 0x06, 0x02, 0xF7]);
    }

    #[test]
    fn locate_round_trip() {
        let tc = Timecode::new(1, 23, 45, 12, FrameRate::Fps25).unwrap();
        let mut buf = [0u8; 16];
        let n = build(0x03, MmcCommand::Locate(tc), &mut buf).unwrap();
        assert_eq!(parse(&buf[..n]).unwrap(), (0x03, MmcCommand::Locate(tc)));
    }

    #[test]
    fn rejects_truncated_and_bad_envelope() {
        assert!(parse(&[0xF0, 0x7F, 0x00]).is_err());
        let mut buf = [0u8; 6];
        build(0x7F, MmcCommand::Stop, &mut buf).unwrap();
        buf[0] = 0x00; // corrupt SysEx start
        assert!(parse(&buf).is_err());
    }

    #[test]
    fn buffer_too_small() {
        let mut small = [0u8; 3];
        assert_eq!(build(0x7F, MmcCommand::Stop, &mut small), Err(AudioError::OutputBufferTooSmall));
    }
}
