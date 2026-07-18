//! MIDI 1.0 Channel Mode messages.
//!
//! Channel-mode messages are Control Change messages with controller numbers
//! 120..=127. They are modelled here as a distinct typed enum for clarity.

use crate::types::AudioError;

/// Control Change status nibble (`0xBn`) — channel-mode messages ride on CC.
pub const STATUS_CONTROL_CHANGE: u8 = 0xB0;

// Channel-mode controller numbers.
/// All Sound Off (CC 120).
pub const CC_ALL_SOUND_OFF: u8 = 120;
/// Reset All Controllers (CC 121).
pub const CC_RESET_ALL_CONTROLLERS: u8 = 121;
/// Local Control On/Off (CC 122).
pub const CC_LOCAL_CONTROL: u8 = 122;
/// All Notes Off (CC 123).
pub const CC_ALL_NOTES_OFF: u8 = 123;
/// Omni Mode Off (CC 124).
pub const CC_OMNI_OFF: u8 = 124;
/// Omni Mode On (CC 125).
pub const CC_OMNI_ON: u8 = 125;
/// Mono Mode On / Poly Off (CC 126); data = number of channels.
pub const CC_MONO_ON: u8 = 126;
/// Poly Mode On / Mono Off (CC 127).
pub const CC_POLY_ON: u8 = 127;

/// The specific channel-mode operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelModeMessage {
    /// All Sound Off (immediately mute all voices).
    AllSoundOff,
    /// Reset All Controllers.
    ResetAllControllers,
    /// Local Control on (`true`) or off (`false`).
    LocalControl(bool),
    /// All Notes Off.
    AllNotesOff,
    /// Omni Mode Off.
    OmniModeOff,
    /// Omni Mode On.
    OmniModeOn,
    /// Mono Mode On; payload is the channel count (0 = "all voices on 1 ch").
    MonoModeOn(u8),
    /// Poly Mode On.
    PolyModeOn,
}

/// A channel-mode message on a specific channel.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChannelMode {
    /// MIDI channel, 0..=15.
    pub channel: u8,
    /// The channel-mode operation.
    pub message: ChannelModeMessage,
}

impl ChannelMode {
    /// Construct a validated channel-mode message.
    pub fn new(channel: u8, message: ChannelModeMessage) -> Result<Self, AudioError> {
        if channel > 15 {
            return Err(AudioError::InvalidParameter);
        }
        if let ChannelModeMessage::MonoModeOn(n) = message {
            if n > 127 {
                return Err(AudioError::InvalidParameter);
            }
        }
        Ok(Self { channel, message })
    }

    /// The (controller, value) data-byte pair for this mode message.
    #[inline]
    fn controller_value(self) -> (u8, u8) {
        match self.message {
            ChannelModeMessage::AllSoundOff => (CC_ALL_SOUND_OFF, 0),
            ChannelModeMessage::ResetAllControllers => (CC_RESET_ALL_CONTROLLERS, 0),
            ChannelModeMessage::LocalControl(on) => (CC_LOCAL_CONTROL, if on { 127 } else { 0 }),
            ChannelModeMessage::AllNotesOff => (CC_ALL_NOTES_OFF, 0),
            ChannelModeMessage::OmniModeOff => (CC_OMNI_OFF, 0),
            ChannelModeMessage::OmniModeOn => (CC_OMNI_ON, 0),
            ChannelModeMessage::MonoModeOn(n) => (CC_MONO_ON, n),
            ChannelModeMessage::PolyModeOn => (CC_POLY_ON, 0),
        }
    }

    /// Serialize to `[0xBn, controller, value]`.
    #[inline]
    pub fn to_bytes(self) -> [u8; 3] {
        let (controller, value) = self.controller_value();
        [STATUS_CONTROL_CHANGE | (self.channel & 0x0F), controller, value]
    }

    /// Parse from a slice whose first byte is a Control Change status with a
    /// channel-mode controller (120..=127).
    pub fn parse(bytes: &[u8]) -> Result<Self, AudioError> {
        if bytes.len() < 3 {
            return Err(AudioError::MalformedAudio);
        }
        if bytes[0] & 0xF0 != STATUS_CONTROL_CHANGE {
            return Err(AudioError::UnsupportedFormat);
        }
        if bytes[1] > 127 || bytes[2] > 127 {
            return Err(AudioError::MalformedAudio);
        }
        let channel = bytes[0] & 0x0F;
        let value = bytes[2];
        let message = match bytes[1] {
            CC_ALL_SOUND_OFF => ChannelModeMessage::AllSoundOff,
            CC_RESET_ALL_CONTROLLERS => ChannelModeMessage::ResetAllControllers,
            CC_LOCAL_CONTROL => ChannelModeMessage::LocalControl(value >= 64),
            CC_ALL_NOTES_OFF => ChannelModeMessage::AllNotesOff,
            CC_OMNI_OFF => ChannelModeMessage::OmniModeOff,
            CC_OMNI_ON => ChannelModeMessage::OmniModeOn,
            CC_MONO_ON => ChannelModeMessage::MonoModeOn(value),
            CC_POLY_ON => ChannelModeMessage::PolyModeOn,
            // Not a channel-mode controller (0..=119 is a normal CC).
            _ => return Err(AudioError::UnsupportedFormat),
        };
        Ok(Self { channel, message })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_notes_off_roundtrip() {
        let m = ChannelMode::new(0, ChannelModeMessage::AllNotesOff).unwrap();
        assert_eq!(m.to_bytes(), [0xB0, 123, 0]);
        assert_eq!(ChannelMode::parse(&m.to_bytes()).unwrap(), m);
    }

    #[test]
    fn local_control_and_mono() {
        let off = ChannelMode::new(1, ChannelModeMessage::LocalControl(false)).unwrap();
        assert_eq!(off.to_bytes(), [0xB1, 122, 0]);
        let on = ChannelMode::new(1, ChannelModeMessage::LocalControl(true)).unwrap();
        assert_eq!(on.to_bytes(), [0xB1, 122, 127]);

        let mono = ChannelMode::new(2, ChannelModeMessage::MonoModeOn(4)).unwrap();
        assert_eq!(mono.to_bytes(), [0xB2, 126, 4]);
        assert_eq!(ChannelMode::parse(&mono.to_bytes()).unwrap(), mono);
    }

    #[test]
    fn non_mode_cc_rejected() {
        // Controller 7 is channel volume, not a channel-mode message.
        assert_eq!(ChannelMode::parse(&[0xB0, 7, 100]), Err(AudioError::UnsupportedFormat));
    }
}
