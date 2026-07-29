//! MPE zone configuration — a master channel plus a range of member channels.
//!
//! MIDI Polyphonic Expression splits the 16 channels into a **zone**: one master
//! channel carries zone-wide messages, and a contiguous block of **member**
//! channels each host a single sounding note so per-note pitch-bend, pressure,
//! and timbre don't collide. The **lower** zone uses master channel 1 (index 0)
//! with members counting up from channel 2; the **upper** zone uses master
//! channel 16 (index 15) with members counting down. Channels here are 0-indexed
//! (`0..=15`). Allocation-free.

use crate::types::AudioError;

/// An MPE zone: the master channel and the inclusive member-channel range.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MpeZone {
    /// Master channel index (0 for the lower zone, 15 for the upper zone).
    pub master_channel: u8,
    /// Lowest member channel index (inclusive).
    pub member_low: u8,
    /// Highest member channel index (inclusive).
    pub member_high: u8,
}

impl MpeZone {
    /// Build a **lower** zone: master = channel 1 (index 0), `member_count`
    /// members counting up from channel 2 (index 1).
    ///
    /// Errors if `member_count` is 0 or would exceed the 15 available member
    /// channels.
    pub fn lower(member_count: u8) -> Result<Self, AudioError> {
        if member_count == 0 || member_count > 15 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self {
            master_channel: 0,
            member_low: 1,
            member_high: member_count,
        })
    }

    /// Build an **upper** zone: master = channel 16 (index 15), `member_count`
    /// members counting down from channel 15 (index 14).
    ///
    /// Errors if `member_count` is 0 or would exceed the 15 available member
    /// channels.
    pub fn upper(member_count: u8) -> Result<Self, AudioError> {
        if member_count == 0 || member_count > 15 {
            return Err(AudioError::InvalidParameter);
        }
        Ok(Self {
            master_channel: 15,
            member_low: 15 - member_count,
            member_high: 14,
        })
    }

    /// Number of member channels in the zone.
    #[inline]
    pub fn member_count(&self) -> u8 {
        self.member_high - self.member_low + 1
    }

    /// Whether `channel` is a member (note-bearing) channel of this zone.
    #[inline]
    pub fn is_member(&self, channel: u8) -> bool {
        channel >= self.member_low && channel <= self.member_high
    }

    /// Whether `channel` is this zone's master channel.
    #[inline]
    pub fn is_master(&self, channel: u8) -> bool {
        channel == self.master_channel
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lower_zone_layout() {
        let z = MpeZone::lower(7).unwrap();
        assert_eq!(z.master_channel, 0);
        assert_eq!(z.member_low, 1);
        assert_eq!(z.member_high, 7);
        assert_eq!(z.member_count(), 7);
        assert!(z.is_master(0));
        assert!(z.is_member(1) && z.is_member(7));
        assert!(!z.is_member(0) && !z.is_member(8));
    }

    #[test]
    fn upper_zone_layout() {
        let z = MpeZone::upper(7).unwrap();
        assert_eq!(z.master_channel, 15);
        assert_eq!(z.member_low, 8);
        assert_eq!(z.member_high, 14);
        assert_eq!(z.member_count(), 7);
        assert!(z.is_master(15));
        assert!(z.is_member(8) && z.is_member(14));
        assert!(!z.is_member(15));
    }

    #[test]
    fn rejects_bad_counts() {
        assert!(MpeZone::lower(0).is_err());
        assert!(MpeZone::lower(16).is_err());
        assert!(MpeZone::upper(0).is_err());
    }
}
