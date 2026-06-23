//! Interface Control Plane (ICP) — fixed-size `PortalControlCommand` envelopes.
//!
//! Hot path: packed `u64` SPSC ring; producers (local HID, remote relay) push;
//! `QualiaPortal::tick` drains and applies. Separate from Sonic Token `0x53` magic.

use std::cell::UnsafeCell;
use std::sync::atomic::{AtomicU32, Ordering};

pub const ICP_MAGIC_BIT: u64 = 1u64 << 63;

pub const OP_SET_CAMERA_DELTA: u8 = 0x60;
pub const OP_NAVIGATE_INDEX: u8 = 0x61;
pub const OP_COLLAPSE_Q: u8 = 0x62;
pub const OP_SET_STANDPOINT_SCALAR: u8 = 0x63;
pub const OP_MENU_ACTION: u8 = 0x64;
pub const OP_SONIC_TOKEN_FORWARD: u8 = 0x65;
pub const OP_SWIPE_GESTURE: u8 = 0x66;
pub const OP_BUTTON_ACTION: u8 = 0x67;
pub const OP_TILT_FRAME: u8 = 0x68;

pub const MENU_ACTION_HOME: u16 = 1;
pub const MENU_ACTION_SONIFY_TOGGLE: u16 = 2;

pub const STANDPOINT_SCALAR_T_SLICE: u8 = 0;
pub const STANDPOINT_SCALAR_T_WINDOW: u8 = 1;
pub const STANDPOINT_SCALAR_EPISTEMIC_Q: u8 = 2;

pub const CONTROL_RING_CAP: usize = 256;

/// Packed ICP command (8 bytes).
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PortalControlCommand {
    pub raw: u64,
}

impl PortalControlCommand {
    #[inline]
    pub const fn empty() -> Self {
        Self { raw: 0 }
    }

    #[inline]
    pub fn opcode(self) -> u8 {
        (self.raw as u8) & 0x7f
    }

    #[inline]
    pub fn has_icp_magic(self) -> bool {
        (self.raw & ICP_MAGIC_BIT) != 0
    }

    #[inline]
    pub fn tensor_or_menu_index(self) -> u16 {
        ((self.raw >> 16) & 0xffff) as u16
    }

    #[inline]
    pub fn param_a_i16(self) -> i16 {
        ((self.raw >> 32) & 0xffff) as i16
    }

    #[inline]
    pub fn param_b_i8(self) -> i8 {
        ((self.raw >> 48) & 0xff) as i8
    }

    #[inline]
    pub fn channel(self) -> u8 {
        ((self.raw >> 8) & 0xff) as u8
    }

    #[inline]
    pub fn with_magic(mut self) -> Self {
        self.raw |= ICP_MAGIC_BIT;
        self
    }

    #[inline]
    pub fn pack(opcode: u8, channel: u8, index: u16, param_a: i16, param_b: i8) -> Self {
        let raw = (opcode as u64)
            | ((channel as u64) << 8)
            | ((index as u64) << 16)
            | ((param_a as u64 & 0xffff) << 32)
            | (((param_b as i64 as u64) & 0xff) << 48)
            | ICP_MAGIC_BIT;
        Self { raw }
    }

    /// Pack camera deltas: `param_a` = dyaw×1000, index = dpitch×1000 (i16), `param_b` = dzoom×1000.
    #[inline]
    pub fn set_camera_delta_scaled(dyaw: f32, dpitch: f32, dzoom: f32) -> Self {
        let ya = (dyaw * 1000.0).clamp(-32767.0, 32767.0) as i16;
        let pi = (dpitch * 1000.0).clamp(-32767.0, 32767.0) as i16;
        let zo = (dzoom * 1000.0).clamp(-127.0, 127.0) as i8;
        Self::pack(OP_SET_CAMERA_DELTA, 0, pi as u16, ya, zo).with_magic()
    }

    #[inline]
    pub fn decode_camera_delta(self) -> (f32, f32, f32) {
        let dyaw = self.param_a_i16() as f32 / 1000.0;
        let dpitch = self.tensor_or_menu_index() as i16 as f32 / 1000.0;
        let dzoom = self.param_b_i8() as f32 / 1000.0;
        (dyaw, dpitch, dzoom)
    }

    #[inline]
    pub fn navigate_index(index: u16) -> Self {
        Self::pack(OP_NAVIGATE_INDEX, 0, index, 0, 0).with_magic()
    }

    #[inline]
    pub fn collapse_q(index: u16) -> Self {
        Self::pack(OP_COLLAPSE_Q, 0, index, 0, 0).with_magic()
    }

    #[inline]
    pub fn menu_action(menu_id: u16) -> Self {
        Self::pack(OP_MENU_ACTION, 0, menu_id, 0, 0).with_magic()
    }

    #[inline]
    pub fn standpoint_scalar(kind: u8, delta: f32) -> Self {
        let scaled = (delta * 1000.0).clamp(-32767.0, 32767.0) as i16;
        Self::pack(OP_SET_STANDPOINT_SCALAR, kind, 0, scaled, 0).with_magic()
    }

    /// Embed a Sonic Token payload (bits 0..62); opcode + ICP magic in low byte / bit 63.
    #[inline]
    pub fn sonic_token_forward(token_raw: u64) -> Self {
        let body = token_raw & 0x7fff_ffff_ffff_ff00;
        Self {
            raw: body | (OP_SONIC_TOKEN_FORWARD as u64) | ICP_MAGIC_BIT,
        }
    }

    #[inline]
    pub fn embedded_sonic_raw(self) -> u64 {
        self.raw & 0x7fff_ffff_ffff_ff00
    }
}

/// Fixed-capacity SPSC ring of packed control commands.
pub struct ControlCommandRing {
    slots: UnsafeCell<[u64; CONTROL_RING_CAP]>,
    write_seq: AtomicU32,
    read_seq: AtomicU32,
}

unsafe impl Sync for ControlCommandRing {}

impl ControlCommandRing {
    pub const fn new() -> Self {
        Self {
            slots: UnsafeCell::new([0u64; CONTROL_RING_CAP]),
            write_seq: AtomicU32::new(0),
            read_seq: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        let w = self.write_seq.load(Ordering::Acquire);
        let r = self.read_seq.load(Ordering::Acquire);
        w.wrapping_sub(r) as usize
    }

    pub fn try_push(&self, cmd: PortalControlCommand) -> bool {
        let w = self.write_seq.load(Ordering::Relaxed);
        let r = self.read_seq.load(Ordering::Acquire);
        if w.wrapping_sub(r) >= CONTROL_RING_CAP as u32 {
            return false;
        }
        let slot = (w % CONTROL_RING_CAP as u32) as usize;
        unsafe {
            (*self.slots.get())[slot] = cmd.raw;
        }
        self.write_seq.store(w.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn try_pop(&self) -> Option<PortalControlCommand> {
        let r = self.read_seq.load(Ordering::Relaxed);
        let w = self.write_seq.load(Ordering::Acquire);
        if r == w {
            return None;
        }
        let slot = (r % CONTROL_RING_CAP as u32) as usize;
        let raw = unsafe { (*self.slots.get())[slot] };
        self.read_seq.store(r.wrapping_add(1), Ordering::Release);
        Some(PortalControlCommand { raw })
    }
}

static CONTROL_RING: ControlCommandRing = ControlCommandRing::new();

#[inline]
pub fn control_ring() -> &'static ControlCommandRing {
    &CONTROL_RING
}

#[inline]
pub fn push_control_command(cmd: PortalControlCommand) -> bool {
    control_ring().try_push(cmd)
}

#[inline]
pub fn push_control_raw(raw: u64) -> bool {
    push_control_command(PortalControlCommand { raw })
}

#[inline]
pub fn pop_control_command() -> Option<PortalControlCommand> {
    control_ring().try_pop()
}

#[inline]
pub fn control_pending() -> u32 {
    control_ring().len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fresh_ring() -> ControlCommandRing {
        ControlCommandRing::new()
    }

    #[test]
    fn icp_pack_magic_and_opcode() {
        let cmd = PortalControlCommand::navigate_index(42);
        assert!(cmd.has_icp_magic());
        assert_eq!(cmd.opcode(), OP_NAVIGATE_INDEX);
        assert_eq!(cmd.tensor_or_menu_index(), 42);
    }

    #[test]
    fn camera_delta_roundtrip() {
        let cmd = PortalControlCommand::set_camera_delta_scaled(0.05, -0.02, 0.1);
        let (y, p, z) = cmd.decode_camera_delta();
        assert!((y - 0.05).abs() < 0.002);
        assert!((p - (-0.02)).abs() < 0.002);
        assert!((z - 0.1).abs() < 0.002);
    }

    #[test]
    fn control_ring_push_pop() {
        let ring = fresh_ring();
        assert!(ring.try_push(PortalControlCommand::menu_action(MENU_ACTION_HOME)));
        let popped = ring.try_pop().expect("cmd");
        assert_eq!(popped.opcode(), OP_MENU_ACTION);
    }

    #[test]
    fn opcode_constants_distinct_from_sonic() {
        assert_ne!(OP_SET_CAMERA_DELTA, 0x53);
        assert_ne!(OP_NAVIGATE_INDEX, OP_COLLAPSE_Q);
    }
}