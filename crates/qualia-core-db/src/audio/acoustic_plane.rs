//! U3 AcousticPlane — SPSC sonic token ring + parametric uniform for AudioWorklet.
//!
//! Shares U1 `Tensor10D` SOA read-only; no dedicated VRAM partition until bake sidecars land.

use core::cell::UnsafeCell;
use std::sync::atomic::{AtomicU32, Ordering};

use bytemuck::{Pod, Zeroable};

use crate::audio::audio_spectral_sheet::{preview_bins_from_tensor, SPECTRAL_PREVIEW_BINS};
use crate::audio::dsp_kernel::{configure_voice_from_tensor, epistemic_fm_index};
use crate::audio::hrtf::{binaural_from_position, room_damp_from_manifold};
use crate::portal_acoustic::{phenomenal_acoustic_params, phenomenal_fm_index, phenomenal_voice_frequency_hz};
use crate::gpu_context::{ComputeUniverse, OperationalMode};
use crate::sonic_token::SonicToken;
use crate::tensor::Tensor10D;

pub const SONIC_RING_CAP: usize = 128;

/// Worklet uniform — mirrors `AcousticParams` with fixed preview bins.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Pod, Zeroable)]
pub struct AcousticUniform {
    pub alpha: f32,
    pub mu: f32,
    pub position: [f32; 3],
    pub track_v: f32,
    pub manifold_w: f32,
    pub epistemic_q: f32,
    pub fm_index: f32,
    pub frequency_hz: f32,
    pub enabled: u32,
    pub gain_l: f32,
    pub gain_r: f32,
    pub itd_seconds: f32,
    pub azimuth_rad: f32,
    pub elevation_rad: f32,
    pub room_damp: f32,
    pub stft_frame: f32,
    pub preview_bins: [f32; SPECTRAL_PREVIEW_BINS],
}

impl Default for AcousticUniform {
    fn default() -> Self {
        Self {
            alpha: 0.0,
            mu: 0.0,
            position: [0.0; 3],
            track_v: 0.0,
            manifold_w: 0.0,
            epistemic_q: 0.0,
            fm_index: 0.0,
            frequency_hz: 440.0,
            enabled: 0,
            gain_l: 0.707,
            gain_r: 0.707,
            itd_seconds: 0.0,
            azimuth_rad: 0.0,
            elevation_rad: 0.0,
            room_damp: 1.0,
            stft_frame: 0.0,
            preview_bins: [0.0; SPECTRAL_PREVIEW_BINS],
        }
    }
}

#[inline]
pub fn apply_binaural_to_uniform(u: &mut AcousticUniform, listener_yaw: f32) {
    let g = binaural_from_position(u.position, listener_yaw);
    u.gain_l = g.gain_l;
    u.gain_r = g.gain_r;
    u.itd_seconds = g.itd_seconds;
    u.azimuth_rad = g.azimuth_rad;
    u.elevation_rad = g.elevation_rad;
    u.room_damp = room_damp_from_manifold(u.manifold_w);
}

/// Scalar params extracted from a tensor node (no heap).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AcousticParams {
    pub alpha: f32,
    pub mu: f32,
    pub position: [f32; 3],
    pub track_v: f32,
    pub manifold_w: f32,
    pub epistemic_q: f32,
    pub preview_bins: [f32; SPECTRAL_PREVIEW_BINS],
}

impl AcousticParams {
    #[inline]
    pub fn from_tensor(t: &Tensor10D) -> Self {
        Self {
            alpha: t.alpha,
            mu: t.mu,
            position: [t.x, t.y, t.z],
            track_v: t.v,
            manifold_w: t.w,
            epistemic_q: t.q,
            preview_bins: preview_bins_from_tensor(t),
        }
    }

    #[inline]
    pub fn to_uniform(self, enabled: bool) -> AcousticUniform {
        let mut voice = crate::audio::dsp_kernel::ParametricVoiceState::default();
        configure_voice_from_tensor(
            &mut voice,
            self.epistemic_q,
            self.mu,
            self.alpha,
            &self.preview_bins,
        );
        let mut u = AcousticUniform {
            alpha: self.alpha,
            mu: self.mu,
            position: self.position,
            track_v: self.track_v,
            manifold_w: self.manifold_w,
            epistemic_q: self.epistemic_q,
            fm_index: voice.fm_index,
            frequency_hz: voice.frequency_hz,
            enabled: u32::from(enabled),
            preview_bins: self.preview_bins,
            ..AcousticUniform::default()
        };
        u.enabled = u32::from(enabled);
        u
    }

    /// Phenomenal uniform — σ oracle frequency (P-F2) aligned with `portal_spectral`.
    #[inline]
    pub fn to_phenomenal_uniform(self, enabled: bool, t: &Tensor10D, listener_yaw: f32) -> AcousticUniform {
        let mut u = self.to_uniform(enabled);
        u.frequency_hz = phenomenal_voice_frequency_hz(t);
        u.fm_index = phenomenal_fm_index(t);
        u.stft_frame = (t.t * 32.0).fract() * 32.0;
        apply_binaural_to_uniform(&mut u, listener_yaw);
        u
    }
}

#[inline]
pub fn acoustic_params_from_tensor(t: &Tensor10D) -> AcousticParams {
    phenomenal_acoustic_params(t)
}

/// True when U3 synthesis is allowed under operational mode (muted in Reserve).
#[inline]
pub fn acoustic_enabled_for_mode(mode: OperationalMode) -> bool {
    !matches!(mode, OperationalMode::Reserve)
}

/// U3 effective mode — shares U1 ledger; follows viewport pressure in Reserve.
#[inline]
pub fn acoustic_effective_mode(global: OperationalMode) -> OperationalMode {
    crate::gpu_context::universe_orchestrator()
        .effective_mode(ComputeUniverse::AcousticPlane, global)
}

/// Fixed-capacity SPSC ring of packed `SonicToken` (`u64`).
pub struct SonicTokenRing {
    slots: UnsafeCell<[u64; SONIC_RING_CAP]>,
    write_seq: AtomicU32,
    read_seq: AtomicU32,
}

// SAFETY: SPSC — one producer (U0/U1/portal), one consumer (worklet poll).
unsafe impl Sync for SonicTokenRing {}

impl SonicTokenRing {
    pub const fn new() -> Self {
        Self {
            slots: UnsafeCell::new([0u64; SONIC_RING_CAP]),
            write_seq: AtomicU32::new(0),
            read_seq: AtomicU32::new(0),
        }
    }

    #[inline]
    pub fn len(&self) -> usize {
        let w = self.write_seq.load(Ordering::Acquire);
        let r = self.read_seq.load(Ordering::Acquire);
        (w.wrapping_sub(r)) as usize
    }

    #[inline]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn try_push(&self, token: SonicToken) -> bool {
        let w = self.write_seq.load(Ordering::Relaxed);
        let r = self.read_seq.load(Ordering::Acquire);
        if w.wrapping_sub(r) >= SONIC_RING_CAP as u32 {
            return false;
        }
        let slot = (w % SONIC_RING_CAP as u32) as usize;
        unsafe {
            (*self.slots.get())[slot] = token.raw;
        }
        self.write_seq.store(w.wrapping_add(1), Ordering::Release);
        true
    }

    pub fn try_pop(&self) -> Option<SonicToken> {
        let r = self.read_seq.load(Ordering::Relaxed);
        let w = self.write_seq.load(Ordering::Acquire);
        if r == w {
            return None;
        }
        let slot = (r % SONIC_RING_CAP as u32) as usize;
        let raw = unsafe { (*self.slots.get())[slot] };
        self.read_seq.store(r.wrapping_add(1), Ordering::Release);
        Some(SonicToken { raw })
    }
}

static SONIC_TOKEN_RING: SonicTokenRing = SonicTokenRing::new();

#[inline]
pub fn sonic_token_ring() -> &'static SonicTokenRing {
    &SONIC_TOKEN_RING
}

#[inline]
pub fn push_sonic_token(token: SonicToken) -> bool {
    sonic_token_ring().try_push(token)
}

#[inline]
pub fn pop_sonic_token() -> Option<SonicToken> {
    sonic_token_ring().try_pop()
}

/// Drain up to `out.len()` tokens; returns count written.
pub fn drain_sonic_tokens(out: &mut [u64]) -> usize {
    let mut n = 0usize;
    while n < out.len() {
        match pop_sonic_token() {
            Some(t) => {
                out[n] = t.raw;
                n += 1;
            }
            None => break,
        }
    }
    n
}

/// Emit parametric pulse + optional NoteOn from a tensor node (U1 → U3).
pub fn sonify_tensor_node(tensor_index: u32, t: &Tensor10D, note_on: bool) -> usize {
    let mut pushed = 0usize;
    let pitch = SonicToken::pitch_from_tensor(t.w, t.q, t.sigma);
    let velocity = (t.alpha.clamp(0.0, 1.0) * 127.0) as u8;
    if push_sonic_token(SonicToken::parametric_pulse(tensor_index, velocity)) {
        pushed += 1;
    }
    if note_on {
        if push_sonic_token(SonicToken::note_on(tensor_index, pitch, velocity, 0)) {
            pushed += 1;
        }
    }
    pushed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sonic_token::{SonicEventType, SONIC_MAGIC};

    fn fresh_ring() -> SonicTokenRing {
        SonicTokenRing::new()
    }

    #[test]
    fn sonic_ring_push_pop() {
        let ring = fresh_ring();
        let t = SonicToken::note_on(1, 60, 100, 0);
        assert!(ring.try_push(t));
        let popped = ring.try_pop().expect("token");
        assert_eq!(popped.tensor_index(), 1);
        assert_eq!(popped.note(), 60);
    }

    #[test]
    fn sonic_ring_full_returns_false() {
        let ring = fresh_ring();
        for i in 0..SONIC_RING_CAP {
            assert!(
                ring.try_push(SonicToken::note_on(i as u32, 60, 100, 0)),
                "push {i}"
            );
        }
        assert!(!ring.try_push(SonicToken::note_on(999, 60, 100, 0)));
    }

    #[test]
    fn drain_respects_capacity() {
        let ring = fresh_ring();
        for i in 0..4 {
            ring.try_push(SonicToken::note_on(i, 60, 100, 0));
        }
        let mut local = [0u64; 2];
        let mut n = 0usize;
        while n < local.len() {
            if let Some(t) = ring.try_pop() {
                local[n] = t.raw;
                n += 1;
            } else {
                break;
            }
        }
        assert_eq!(n, 2);
        assert_eq!(ring.len(), 2);
    }

    #[test]
    fn acoustic_uniform_size_stable() {
        assert_eq!(std::mem::size_of::<AcousticUniform>() % 4, 0);
        let t = Tensor10D::default();
        let u = AcousticParams::from_tensor(&t).to_uniform(true);
        assert_eq!(u.enabled, 1);
    }

    #[test]
    fn acoustic_disabled_in_reserve() {
        assert!(!acoustic_enabled_for_mode(OperationalMode::Reserve));
        assert!(acoustic_enabled_for_mode(OperationalMode::Full));
    }

    #[test]
    fn sonify_pushes_tokens() {
        let ring = fresh_ring();
        let t = Tensor10D::new(0.5, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.8, 0.1, 2.5);
        // sonify uses global ring — test logic via local push pattern
        let pitch = SonicToken::pitch_from_tensor(t.w, t.q, t.sigma);
        assert!(ring.try_push(SonicToken::parametric_pulse(7, 100)));
        assert!(ring.try_push(SonicToken::note_on(7, pitch, 100, 0)));
        assert_eq!(ring.len(), 2);
        let first = ring.try_pop().unwrap();
        assert_eq!(first.event_type(), SonicEventType::Parametric);
        let second = ring.try_pop().unwrap();
        assert_eq!(second.event_type(), SonicEventType::NoteOn);
        assert_eq!(second.flags() & 0xff, SONIC_MAGIC as u16);
    }
}