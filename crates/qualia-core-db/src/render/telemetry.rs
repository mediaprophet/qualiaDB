//! GPU telemetry and observer contracts for the Qualia portal (U2 viewport).

use bytemuck::{Pod, Zeroable};

use crate::gpu_context::sample_ambient_telemetry;

/// Untethered commons spectator (default onboarding ramp).
pub const STANDPOINT_SPECTATOR: u32 = 0;
/// Ephemeral session — local activity qualia, no cryptographic permanence.
pub const STANDPOINT_EPHEMERAL: u32 = 1;
/// Verified Human-Centric identifier (DID) — opt-in permanence; vault data plane is sealed.
pub const STANDPOINT_DID: u32 = 2;
/// Private vault slice — bilateral lane, collapsed epistemic aperture.
pub const STANDPOINT_VAULT: u32 = 3;

/// Permissive Commons routing lane (`PermissiveRoutingLane::EnforcePermissiveCommons`).
pub const DEONTIC_LANE_COMMONS: u32 = 1;
/// Bilateral Micro-Commons (`PermissiveRoutingLane::EnforceBilateralMicroCommons`).
pub const DEONTIC_LANE_BILATERAL: u32 = 2;

/// Telemetry and fabric writes stay on the local viewport machine.
pub const FABRIC_VIEWPORT_LOCAL: u32 = 0;
/// Shared tensor fabric (requires authenticated standpoint).
pub const FABRIC_SHARED: u32 = 1;

/// System telemetry for ambient / projector shaders (`#[repr(C)]`, 48 bytes).
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, Pod, Zeroable, serde::Serialize, serde::Deserialize)]
pub struct SystemTelemetry {
    pub memory_pressure: f32,
    pub network_ripple: f32,
    pub baking_crystallization: f32,
    pub logic_flashes: f32,
    pub llm_heat: f32,
    pub quantum_activity: f32,
    pub spectral_shift: f32,
    pub temporal_pulse: f32,
    pub epistemic_density: f32,
    pub manifold_pressure: f32,
    pub _padding: [f32; 2],
}

impl SystemTelemetry {
    #[inline]
    pub fn from_samples(samples: &[f32; 11]) -> Self {
        Self {
            memory_pressure: samples[0],
            network_ripple: samples[1],
            baking_crystallization: samples[2],
            logic_flashes: samples[3],
            llm_heat: samples[4],
            quantum_activity: samples[5],
            spectral_shift: samples[6],
            temporal_pulse: samples[7],
            epistemic_density: samples[8],
            manifold_pressure: samples[9],
            _padding: [0.0, samples[10]],
        }
    }

    #[inline]
    pub fn refresh_from_ledger(&mut self) {
        *self = Self::from_samples(&sample_ambient_telemetry());
    }

    #[inline]
    pub fn apply_floats(&mut self, floats: &[f32]) {
        let mut samples = sample_ambient_telemetry();
        for (i, slot) in samples.iter_mut().enumerate() {
            if let Some(v) = floats.get(i) {
                *slot = *v;
            }
        }
        *self = Self::from_samples(&samples);
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct AmbientUniforms {
    pub time: f32,
    pub view_width: f32,
    pub view_height: f32,
    pub _padding: f32,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ParticleInstance {
    pub position: [f32; 3],
    /// Epistemic q-state: 0 = collapsed ground truth, >0 = generative sandbox.
    pub epistemic_q: f32,
}

/// Vertex instance for `projector.wgsl` (vec3 position + vec4 color, 32 B aligned).
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct TensorRenderInstance {
    pub position: [f32; 3],
    pub _pad0: f32,
    pub color: [f32; 4],
}

/// Camera IPC uniform — shared by ambient (binding 3) and `projector.wgsl` (binding 0).
///
/// `view_projection` is pre-multiplied on the CPU; PGA motors in the projector shader
/// apply per-object semantic transforms on top of this view.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct CameraUniform {
    pub view_projection: [[f32; 4]; 4],
    pub yaw: f32,
    pub pitch: f32,
    pub zoom: f32,
    /// Non-zero when tensor SOA is resident — ambient/projector use `view_projection`.
    pub tensor_mode: u32,
    /// `_padding[0]` = frame time (seconds) for PGA epistemic spin in `projector.wgsl`.
    /// `_padding[1..4]` = camera eye `(x, y, z)` for Phase 2c bilateral `T_pull`.
    pub _padding: [f32; 12],
}

/// Human-Centric observer standpoint — semantic right to perceive (decoupled from `CameraUniform`).
///
/// Anchors the viewport to the human and their chosen context (spectator → ephemeral → DID → vault),
/// not to cryptographic vault weight alone. WGSL reads `standpoint_hash` / `session_nonce` as
/// `vec2<u32>` (little-endian u64). Temporal scrub: projector discards vertices when
/// `|tensor.t - t_slice| > t_window`.
#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct ObserverStandpoint {
    pub standpoint_hash: u64,
    pub session_nonce: u64,
    /// Spectator default = 1.0 (full commons aperture); vault collapse → 0.0.
    pub epistemic_q: f32,
    /// Active temporal slice coordinate (scrub axis).
    pub t_slice: f32,
    /// Half-width of visible temporal band; large values show the full trace.
    pub t_window: f32,
    /// Deontic routing lane (`DEONTIC_LANE_COMMONS` for spectator).
    pub deontic_lane: u32,
    /// `STANDPOINT_*` class — spectator / ephemeral / DID / vault.
    pub standpoint_class: u32,
    /// `FABRIC_VIEWPORT_LOCAL` until authenticated DID opens shared fabric.
    pub fabric_gate: u32,
    pub _padding: [f32; 22],
}

impl ObserverStandpoint {
    #[inline]
    pub const fn new(
        standpoint_hash: u64,
        session_nonce: u64,
        standpoint_class: u32,
        epistemic_q: f32,
        t_slice: f32,
        t_window: f32,
        deontic_lane: u32,
        fabric_gate: u32,
    ) -> Self {
        Self {
            standpoint_hash,
            session_nonce,
            epistemic_q,
            t_slice,
            t_window,
            deontic_lane,
            standpoint_class,
            fabric_gate,
            _padding: [0.0; 22],
        }
    }

    #[inline]
    pub fn with_temporal(mut self, t_slice: f32, t_window: f32) -> Self {
        self.t_slice = t_slice;
        self.t_window = t_window.max(0.0);
        self
    }

    #[inline]
    pub fn temporal_visible(&self, tensor_t: f32) -> bool {
        (tensor_t - self.t_slice).abs() <= self.t_window
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_telemetry_is_48_bytes() {
        assert_eq!(std::mem::size_of::<SystemTelemetry>(), 48);
        assert_eq!(std::mem::size_of::<AmbientUniforms>(), 16);
        assert_eq!(std::mem::size_of::<ParticleInstance>(), 16);
        assert_eq!(std::mem::size_of::<CameraUniform>(), 128);
        assert_eq!(std::mem::size_of::<ObserverStandpoint>(), 128);
    }

    #[test]
    fn temporal_filter_respects_window() {
        let sp = ObserverStandpoint::new(0, 0, STANDPOINT_SPECTATOR, 1.0, 0.5, 0.1, 1, 0);
        assert!(sp.temporal_visible(0.45));
        assert!(!sp.temporal_visible(0.7));
    }
}
