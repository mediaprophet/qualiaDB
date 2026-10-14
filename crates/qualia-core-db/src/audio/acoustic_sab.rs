//! SharedArrayBuffer layout for zero-copy U3 uniform + sonic token mirror (Phase 7.4).
//!
//! Layout: `[header 32][uniform 352][tokens 128][float mirror 328]` = 840 B (padded to 1024).

use bytemuck::{Pod, Zeroable};

use super::acoustic_plane::AcousticUniform;
use crate::sonic_token::SonicToken;

pub const ACOUSTIC_SAB_MAGIC: u32 = 0x5133_4153; // "Q3AS"
pub const ACOUSTIC_SAB_VERSION: u16 = 1;
pub const ACOUSTIC_SAB_HEADER_BYTES: usize = 32;
pub const ACOUSTIC_SAB_UNIFORM_OFFSET: usize = 32;
pub const ACOUSTIC_SAB_UNIFORM_PADDED: usize = 352;
pub const ACOUSTIC_SAB_TOKEN_OFFSET: usize =
    ACOUSTIC_SAB_UNIFORM_OFFSET + ACOUSTIC_SAB_UNIFORM_PADDED;
pub const ACOUSTIC_SAB_TOKEN_BYTES: usize = 128;
pub const ACOUSTIC_SAB_FLOAT_MIRROR_OFFSET: usize =
    ACOUSTIC_SAB_TOKEN_OFFSET + ACOUSTIC_SAB_TOKEN_BYTES;
pub const ACOUSTIC_SAB_FLOAT_MIRROR_BYTES: usize = 328;
pub const ACOUSTIC_SAB_BYTES: usize = 1024;

/// SAB header — worklet polls `uniform_seq` / `token_seq` (Release/Acquire).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable)]
pub struct AcousticSabHeader {
    pub magic: u32,
    pub version: u16,
    pub uniform_seq: u16,
    pub token_write: u32,
    pub token_read: u32,
    pub stft_frame: u32,
    pub sample_rate: u32,
    pub _pad: u32,
}

impl AcousticSabHeader {
    #[inline]
    pub const fn new() -> Self {
        Self {
            magic: ACOUSTIC_SAB_MAGIC,
            version: ACOUSTIC_SAB_VERSION,
            uniform_seq: 0,
            token_write: 0,
            token_read: 0,
            stft_frame: 0,
            sample_rate: 48_000,
            _pad: 0,
        }
    }

    #[inline]
    pub fn parse(bytes: &[u8]) -> Option<Self> {
        let hlen = std::mem::size_of::<Self>();
        if bytes.len() < hlen {
            return None;
        }
        let h = bytemuck::pod_read_unaligned::<AcousticSabHeader>(&bytes[..hlen]);
        (h.magic == ACOUSTIC_SAB_MAGIC && h.version == ACOUSTIC_SAB_VERSION).then_some(h)
    }
}

#[inline]
pub fn init_acoustic_sab(out: &mut [u8]) -> bool {
    if out.len() < ACOUSTIC_SAB_BYTES {
        return false;
    }
    let header = AcousticSabHeader::new();
    out[..ACOUSTIC_SAB_HEADER_BYTES].fill(0);
    let hlen = std::mem::size_of::<AcousticSabHeader>();
    out[..hlen].copy_from_slice(bytemuck::bytes_of(&header));
    out[ACOUSTIC_SAB_UNIFORM_OFFSET..].fill(0);
    true
}

/// Write pod uniform + optional f32 mirror for worklet `Float32Array` view.
#[inline]
pub fn write_uniform_to_sab(out: &mut [u8], uniform: &AcousticUniform) -> bool {
    write_uniform_to_sab_with_mirror(out, uniform, None)
}

#[inline]
pub fn write_uniform_to_sab_with_mirror(
    out: &mut [u8],
    uniform: &AcousticUniform,
    float_mirror: Option<&[f32]>,
) -> bool {
    if out.len() < ACOUSTIC_SAB_FLOAT_MIRROR_OFFSET + ACOUSTIC_SAB_FLOAT_MIRROR_BYTES {
        return false;
    }
    let bytes = bytemuck::bytes_of(uniform);
    let end = ACOUSTIC_SAB_UNIFORM_OFFSET + bytes.len();
    if end > ACOUSTIC_SAB_TOKEN_OFFSET {
        return false;
    }
    out[ACOUSTIC_SAB_UNIFORM_OFFSET..end].copy_from_slice(bytes);
    if let Some(mirror) = float_mirror {
        let n = mirror.len().min(ACOUSTIC_SAB_FLOAT_MIRROR_BYTES / 4);
        out[ACOUSTIC_SAB_FLOAT_MIRROR_OFFSET..ACOUSTIC_SAB_FLOAT_MIRROR_OFFSET + n * 4]
            .copy_from_slice(bytemuck::cast_slice(&mirror[..n]));
    }
    let header = AcousticSabHeader::parse(out).unwrap_or(AcousticSabHeader::new());
    let mut h = header;
    h.uniform_seq = h.uniform_seq.wrapping_add(1);
    let hlen = std::mem::size_of::<AcousticSabHeader>();
    out[..hlen].copy_from_slice(bytemuck::bytes_of(&h));
    true
}

#[inline]
fn write_sab_header(out: &mut [u8], h: &AcousticSabHeader) {
    let hlen = std::mem::size_of::<AcousticSabHeader>();
    out[..hlen].copy_from_slice(bytemuck::bytes_of(h));
}

#[inline]
pub fn read_uniform_from_sab(bytes: &[u8]) -> Option<AcousticUniform> {
    if bytes.len() < ACOUSTIC_SAB_UNIFORM_OFFSET + std::mem::size_of::<AcousticUniform>() {
        return None;
    }
    let u_size = std::mem::size_of::<AcousticUniform>();
    Some(bytemuck::pod_read_unaligned::<AcousticUniform>(
        &bytes[ACOUSTIC_SAB_UNIFORM_OFFSET..ACOUSTIC_SAB_UNIFORM_OFFSET + u_size],
    ))
}

/// Push one token into SAB ring (16 slots × 8 B).
#[inline]
pub fn push_token_to_sab(out: &mut [u8], token: SonicToken) -> bool {
    if out.len() < ACOUSTIC_SAB_BYTES {
        return false;
    }
    let mut h = AcousticSabHeader::parse(out).unwrap_or(AcousticSabHeader::new());
    let w = h.token_write;
    let r = h.token_read;
    if w.wrapping_sub(r) >= 16 {
        return false;
    }
    let slot = (w % 16) as usize;
    let off = ACOUSTIC_SAB_TOKEN_OFFSET + slot * 8;
    out[off..off + 8].copy_from_slice(bytemuck::bytes_of(&token.raw));
    h.token_write = w.wrapping_add(1);
    write_sab_header(out, &h);
    true
}

// ───────────────────────────────────────────────────────────────────────────
//  Double-Buffered 64-Voice SeqLock Table & Perceptual Salience Culling
// ───────────────────────────────────────────────────────────────────────────

pub const VOICE_DESCRIPTOR_SLOT_SIZE: usize = 64;
pub const MAX_ACOUSTIC_VOICE_SLOTS: usize = 64;

/// 64-byte cache-line-aligned voice descriptor slot with monotonic SeqLock synchronization.
#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct VoiceDescriptorSlot {
    /// Monotonic sequence counter: odd = write in progress, even = valid consistent state.
    pub sequence_number: u32,
    pub entity_id: u32,
    pub amplitude: f32,
    pub pan_room_send: f32,
    pub world_position: [f32; 3],
    pub acoustic_coeffs: [f32; 3], // Jakob-Hanika dual (c0, c1, c2)
    pub padding: [u32; 6],
}

impl VoiceDescriptorSlot {
    #[inline]
    pub const fn empty() -> Self {
        Self {
            sequence_number: 0,
            entity_id: 0,
            amplitude: 0.0,
            pan_room_send: 0.0,
            world_position: [0.0; 3],
            acoustic_coeffs: [0.0; 3],
            padding: [0; 6],
        }
    }
}

/// Decoded voice snapshot returned when a SeqLock read succeeds without tear.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VoiceSnapshot {
    pub entity_id: u32,
    pub amplitude: f32,
    pub pan_room_send: f32,
    pub world_position: [f32; 3],
    pub acoustic_coeffs: [f32; 3],
}

/// Calculate acoustic salience score $S_i = \frac{A_i \cdot \mathcal{D}}{1.0 + \|\vec{P}_i - \vec{P}_{\text{listener}}\|^2}$.
///
/// Emitters falling below $S_i < 10^{-4}$ are culled by the producer before writing to the SAB.
#[inline]
pub fn calculate_acoustic_salience(
    amplitude: f32,
    diffraction_scalar: f32,
    emitter_pos: [f32; 3],
    listener_pos: [f32; 3],
) -> f32 {
    let dx = emitter_pos[0] - listener_pos[0];
    let dy = emitter_pos[1] - listener_pos[1];
    let dz = emitter_pos[2] - listener_pos[2];
    let dist_sq = dx * dx + dy * dy + dz * dz;

    let diff = diffraction_scalar.clamp(0.1, 1.0);
    (amplitude.max(0.0) * diff) / (1.0 + dist_sq)
}

/// Producer update: writes parameters into a voice slot using odd-even SeqLock signaling.
#[inline]
pub fn update_voice_slot(
    slot: &mut VoiceDescriptorSlot,
    entity_id: u32,
    amplitude: f32,
    pan_room_send: f32,
    world_pos: [f32; 3],
    coeffs: [f32; 3],
) {
    let seq = slot.sequence_number;
    // Step 1: Mark odd (write in progress)
    slot.sequence_number = seq.wrapping_add(1) | 1;

    // Step 2: Store payload fields
    slot.entity_id = entity_id;
    slot.amplitude = amplitude;
    slot.pan_room_send = pan_room_send;
    slot.world_position = world_pos;
    slot.acoustic_coeffs = coeffs;

    // Step 3: Mark even (write complete)
    slot.sequence_number = seq.wrapping_add(2) & !1;
}

/// Consumer read: reads parameters from a voice slot; returns `None` if producer was writing or torn read occurred.
#[inline]
pub fn read_voice_slot(slot: &VoiceDescriptorSlot) -> Option<VoiceSnapshot> {
    let seq1 = slot.sequence_number;
    // Odd sequence means producer is currently writing
    if (seq1 & 1) != 0 {
        return None;
    }

    let snapshot = VoiceSnapshot {
        entity_id: slot.entity_id,
        amplitude: slot.amplitude,
        pan_room_send: slot.pan_room_send,
        world_position: slot.world_position,
        acoustic_coeffs: slot.acoustic_coeffs,
    };

    let seq2 = slot.sequence_number;
    // If sequence changed during read, a torn read occurred
    if seq1 != seq2 {
        None
    } else {
        Some(snapshot)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sab_header_pod_fits_slot() {
        let hlen = std::mem::size_of::<AcousticSabHeader>();
        assert!(hlen <= ACOUSTIC_SAB_HEADER_BYTES);
        assert_eq!(hlen, 28);
    }

    #[test]
    fn sab_roundtrip_uniform() {
        let mut sab = [0u8; ACOUSTIC_SAB_BYTES];
        assert!(init_acoustic_sab(&mut sab));
        let u = AcousticUniform::default();
        assert!(write_uniform_to_sab(&mut sab, &u));
        let h = AcousticSabHeader::parse(&sab).unwrap();
        assert_eq!(h.uniform_seq, 1);
        let read = read_uniform_from_sab(&sab).unwrap();
        assert_eq!(read.frequency_hz, u.frequency_hz);
    }

    #[test]
    fn sab_token_push() {
        let mut sab = [0u8; ACOUSTIC_SAB_BYTES];
        init_acoustic_sab(&mut sab);
        let t = SonicToken::note_on(1, 60, 100, 0);
        assert!(push_token_to_sab(&mut sab, t));
        let h = AcousticSabHeader::parse(&sab).unwrap();
        assert_eq!(h.token_write, 1);
    }

    #[test]
    fn voice_descriptor_slot_layout_and_seqlock() {
        assert_eq!(std::mem::size_of::<VoiceDescriptorSlot>(), 64);
        assert_eq!(std::mem::align_of::<VoiceDescriptorSlot>(), 64);

        let mut slot = VoiceDescriptorSlot::empty();
        assert_eq!(read_voice_slot(&slot).unwrap().amplitude, 0.0);

        // Update slot
        update_voice_slot(
            &mut slot,
            42,
            0.85,
            0.5,
            [10.0, 2.0, -5.0],
            [0.1, -0.4, 0.9],
        );

        // Successful read
        let snap = read_voice_slot(&slot).expect("valid read");
        assert_eq!(snap.entity_id, 42);
        assert_eq!(snap.amplitude, 0.85);
        assert_eq!(snap.world_position, [10.0, 2.0, -5.0]);
        assert_eq!(snap.acoustic_coeffs, [0.1, -0.4, 0.9]);

        // Simulating write in progress (odd sequence number)
        slot.sequence_number = 3;
        assert!(read_voice_slot(&slot).is_none());
    }

    #[test]
    fn test_acoustic_salience_culling() {
        let listener = [0.0, 0.0, 0.0];
        let near_emitter = [1.0, 0.0, 0.0]; // dist = 1m
        let far_emitter = [100.0, 0.0, 0.0]; // dist = 100m

        let s_near = calculate_acoustic_salience(1.0, 1.0, near_emitter, listener);
        let s_far = calculate_acoustic_salience(1.0, 1.0, far_emitter, listener);

        assert!(s_near > 0.4);
        assert!(s_far < 1e-4); // Below audibility threshold!
    }
}
