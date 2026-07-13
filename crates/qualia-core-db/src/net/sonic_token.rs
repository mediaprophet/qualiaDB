//! 64-bit Sonic Token — symbolic audio events (U0/U1 → U3 AcousticPlane).
//!
//! Hot path: `Pod` packed `u64`; no heap. MIDI export is a cold-path serializer.

use bytemuck::{Pod, Zeroable};

pub const SONIC_MAGIC: u8 = 0x53; // 'S'

/// Packed sonic event (8 bytes).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct SonicToken {
    pub raw: u64,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SonicEventType {
    NoteOn = 0,
    NoteOff = 1,
    ControlChange = 2,
    Parametric = 3,
}

impl SonicToken {
    #[inline]
    pub const fn empty() -> Self {
        Self { raw: 0 }
    }

    #[inline]
    pub fn pack(
        delta_time: u8,
        event_type: SonicEventType,
        channel: u8,
        note: u8,
        velocity: u8,
        tensor_index: u32,
        flags: u16,
    ) -> Self {
        let et = event_type as u64;
        let raw = (delta_time as u64)
            | ((et & 0x0f) << 8)
            | ((channel as u64 & 0x0f) << 12)
            | ((note as u64) << 16)
            | ((velocity as u64) << 24)
            | ((tensor_index as u64 & 0xffff) << 32)
            | ((flags as u64 & 0xffff) << 48);
        Self { raw }
    }

    #[inline]
    pub fn delta_time(self) -> u8 {
        self.raw as u8
    }

    #[inline]
    pub fn event_type(self) -> SonicEventType {
        match (self.raw >> 8) & 0x0f {
            1 => SonicEventType::NoteOff,
            2 => SonicEventType::ControlChange,
            3 => SonicEventType::Parametric,
            _ => SonicEventType::NoteOn,
        }
    }

    #[inline]
    pub fn channel(self) -> u8 {
        ((self.raw >> 12) & 0x0f) as u8
    }

    #[inline]
    pub fn note(self) -> u8 {
        ((self.raw >> 16) & 0xff) as u8
    }

    #[inline]
    pub fn velocity(self) -> u8 {
        ((self.raw >> 24) & 0xff) as u8
    }

    #[inline]
    pub fn tensor_index(self) -> u32 {
        ((self.raw >> 32) & 0xffff) as u32
    }

    #[inline]
    pub fn flags(self) -> u16 {
        (self.raw >> 48) as u16
    }

    /// Map manifold `w` + epistemic `q` to a MIDI note (0–127).
    #[inline]
    pub fn pitch_from_tensor(w: f32, q: f32, sigma: f32) -> u8 {
        let base = 48.0 + w * 12.0 + sigma.fract() * 24.0;
        let jitter = q * 6.0;
        (base + jitter).clamp(0.0, 127.0) as u8
    }

    /// Build NoteOn from a tensor node index and pitch/velocity.
    #[inline]
    pub fn note_on(tensor_index: u32, note: u8, velocity: u8, channel: u8) -> Self {
        Self::pack(
            0,
            SonicEventType::NoteOn,
            channel,
            note,
            velocity,
            tensor_index,
            SONIC_MAGIC as u16,
        )
    }

    #[inline]
    pub fn parametric_pulse(tensor_index: u32, velocity: u8) -> Self {
        Self::pack(
            0,
            SonicEventType::Parametric,
            0,
            0,
            velocity,
            tensor_index,
            SONIC_MAGIC as u16,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_roundtrip() {
        let t = SonicToken::pack(3, SonicEventType::NoteOn, 2, 60, 100, 42, 0x0053);
        assert_eq!(t.delta_time(), 3);
        assert_eq!(t.event_type(), SonicEventType::NoteOn);
        assert_eq!(t.channel(), 2);
        assert_eq!(t.note(), 60);
        assert_eq!(t.velocity(), 100);
        assert_eq!(t.tensor_index(), 42);
        assert_eq!(t.flags(), 0x0053);
        assert_eq!(std::mem::size_of::<SonicToken>(), 8);
    }

    #[test]
    fn pitch_from_tensor_in_range() {
        let n = SonicToken::pitch_from_tensor(2.0, 0.5, 3.7);
        assert!(n <= 127);
    }
}
