//! Zero-Heap Live 10D / `EONT1` / `PROC1` / ABSM Inspector State (`inspector_10d.rs`)
//!
//! Implements Epic 9.3c of the QualiaDB Superior Game Engine architecture:
//! - Zero-heap data model and live inspector state for inspecting and editing a selected scene node's:
//!   1. [`Tensor10D`] (`[q, v, w, x, y, z, t, α, μ, σ]`) 10-dimensional manifold coordinate.
//!   2. [`Epistemic10DSliceParams`] 4D/5D volumetric slice and EMF field visualization controls.
//!   3. `.10d` `PROC1` [`ProceduralSynthesisHeader`] (cellular Voronoi, SDF smooth-fillet, allometric
//!      power law $y = b \cdot x^k$, logarithmic spiral ODE, PGA motor harmonic oscillation, and
//!      stress-cleavage fracture threshold).
//!   4. `.10d` `EONT1` [`EntityOntologyMiniHeader`], [`AffordanceBinaryRecord`] table, and
//!      [`DevelopmentalKeyframeRecord`] ontogenetic growth milestones.
//!   5. Multi-layer ABSM animation layer weights & bone masks ([`AbsmLayerState`], up to 8 layers).

use crate::render::gizmo_3d::Motor;

/// Maximum affordances cached in the zero-heap studio node inspector.
pub const MAX_INSPECTOR_AFFORDANCES: usize = 16;
/// Maximum ontogenetic developmental keyframes cached in the zero-heap studio node inspector.
pub const MAX_INSPECTOR_KEYFRAMES: usize = 8;
/// Maximum ABSM animation blending layers per inspected character/entity.
pub const MAX_INSPECTOR_ABSM_LAYERS: usize = 8;

/// 10D Tensor coordinate `[q, v, w, x, y, z, t, α, μ, σ]` (40 bytes, `#[repr(C)]`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tensor10D {
    pub q: f32,
    pub v: f32,
    pub w: f32,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub t: f32,
    pub alpha: f32,
    pub mu: f32,
    pub sigma: f32,
}

impl Default for Tensor10D {
    fn default() -> Self {
        Self {
            q: 0.0,
            v: 0.0,
            w: 0.0,
            x: 0.0,
            y: 0.0,
            z: 0.0,
            t: 0.0,
            alpha: 1.0,
            mu: 0.0,
            sigma: 0.0,
        }
    }
}

impl Tensor10D {
    #[inline]
    pub const fn new(
        q: f32,
        v: f32,
        w: f32,
        x: f32,
        y: f32,
        z: f32,
        t: f32,
        alpha: f32,
        mu: f32,
        sigma: f32,
    ) -> Self {
        Self {
            q,
            v,
            w,
            x,
            y,
            z,
            t,
            alpha,
            mu,
            sigma,
        }
    }

    /// Returns the 10 coordinates as a flat `[f32; 10]` array.
    #[inline]
    pub const fn to_array(self) -> [f32; 10] {
        [
            self.q, self.v, self.w, self.x, self.y, self.z, self.t, self.alpha, self.mu,
            self.sigma,
        ]
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<qualia_core_db::tensor::Tensor10D> for Tensor10D {
    #[inline]
    fn from(t: qualia_core_db::tensor::Tensor10D) -> Self {
        Self::new(t.q, t.v, t.w, t.x, t.y, t.z, t.t, t.alpha, t.mu, t.sigma)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<Tensor10D> for qualia_core_db::tensor::Tensor10D {
    #[inline]
    fn from(t: Tensor10D) -> Self {
        Self::new(t.q, t.v, t.w, t.x, t.y, t.z, t.t, t.alpha, t.mu, t.sigma)
    }
}

/// Volumetric 10D slice & EMF field visualization parameters (`Epistemic10D`).
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Epistemic10DSliceParams {
    /// Active quantum superposition slice filter `q_slice` in `[0.0, 1.0]`.
    pub q_slice: f32,
    /// Active topological variety band `v_band` (`0=Euclidean, 1=Cyclic, 2=Hyperbolic, 3=Clique`).
    pub v_band: f32,
    /// Active domain manifold slice `w_manifold` (`0..=4`).
    pub w_manifold: f32,
    /// Coordinate time scrub position `t_scrub`.
    pub t_scrub: f32,
    /// Minimum salience threshold `alpha_cutoff` for volumetric raymarching.
    pub alpha_cutoff: f32,
    /// Spectral wavelength window `[sigma_min, sigma_max]` in nm / normalized class units.
    pub sigma_window: [f32; 2],
    /// Whether bilateral deontic standpoint pull (`T_pull`) is active in the viewport.
    pub bilateral_pull_enabled: bool,
}

impl Default for Epistemic10DSliceParams {
    fn default() -> Self {
        Self {
            q_slice: 0.0,
            v_band: 0.0,
            w_manifold: 0.0,
            t_scrub: 0.0,
            alpha_cutoff: 0.05,
            sigma_window: [0.0, 1.0],
            bilateral_pull_enabled: true,
        }
    }
}

/// 64-byte POD `.10d` `PROC1` (`MAGIC_PROC`) procedural synthesis header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProceduralSynthesisHeader {
    pub magic: [u8; 4],
    pub version: u16,
    pub flags: u16,
    pub seed: u64,
    pub voronoi_frequency: f32,
    pub voronoi_jitter: f32,
    pub fillet_blend_radius: f32,
    pub fillet_threshold: f32,
    pub allometry_exponent_k: f32,
    pub allometry_base_b: f32,
    pub spiral_growth_rate_k: f32,
    pub spiral_initial_radius_a: f32,
    pub motor_frequency_hz: f32,
    pub motor_damping_ratio: f32,
    pub critical_stress_threshold: f32,
    pub reserved_f32: f32,
}

const _: [(); 64] = [(); core::mem::size_of::<ProceduralSynthesisHeader>()];

impl Default for ProceduralSynthesisHeader {
    fn default() -> Self {
        Self {
            magic: *b"PROC",
            version: 1,
            flags: 0x001F,
            seed: 0x517C_C1B7_2722_0A95,
            voronoi_frequency: 12.0,
            voronoi_jitter: 0.8,
            fillet_blend_radius: 0.15,
            fillet_threshold: 0.05,
            allometry_exponent_k: 1.25,
            allometry_base_b: 1.0,
            spiral_growth_rate_k: 0.18,
            spiral_initial_radius_a: 0.10,
            motor_frequency_hz: 1.8,
            motor_damping_ratio: 0.12,
            critical_stress_threshold: 50.0,
            reserved_f32: 0.0,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<qualia_core_db::container_10d::procedural_synthesis::ProceduralSynthesisHeader>
    for ProceduralSynthesisHeader
{
    #[inline]
    fn from(
        h: qualia_core_db::container_10d::procedural_synthesis::ProceduralSynthesisHeader,
    ) -> Self {
        Self {
            magic: h.magic,
            version: h.version,
            flags: h.flags,
            seed: h.seed,
            voronoi_frequency: h.voronoi_frequency,
            voronoi_jitter: h.voronoi_jitter,
            fillet_blend_radius: h.fillet_blend_radius,
            fillet_threshold: h.fillet_threshold,
            allometry_exponent_k: h.allometry_exponent_k,
            allometry_base_b: h.allometry_base_b,
            spiral_growth_rate_k: h.spiral_growth_rate_k,
            spiral_initial_radius_a: h.spiral_initial_radius_a,
            motor_frequency_hz: h.motor_frequency_hz,
            motor_damping_ratio: h.motor_damping_ratio,
            critical_stress_threshold: h.critical_stress_threshold,
            reserved_f32: h.reserved_f32,
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<ProceduralSynthesisHeader>
    for qualia_core_db::container_10d::procedural_synthesis::ProceduralSynthesisHeader
{
    #[inline]
    fn from(h: ProceduralSynthesisHeader) -> Self {
        Self {
            magic: h.magic,
            version: h.version,
            flags: h.flags,
            seed: h.seed,
            voronoi_frequency: h.voronoi_frequency,
            voronoi_jitter: h.voronoi_jitter,
            fillet_blend_radius: h.fillet_blend_radius,
            fillet_threshold: h.fillet_threshold,
            allometry_exponent_k: h.allometry_exponent_k,
            allometry_base_b: h.allometry_base_b,
            spiral_growth_rate_k: h.spiral_growth_rate_k,
            spiral_initial_radius_a: h.spiral_initial_radius_a,
            motor_frequency_hz: h.motor_frequency_hz,
            motor_damping_ratio: h.motor_damping_ratio,
            critical_stress_threshold: h.critical_stress_threshold,
            reserved_f32: h.reserved_f32,
        }
    }
}

/// 40-byte `.10d` `EONT1` (`MAGIC_EONT`) entity ontology mini-header.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EntityOntologyMiniHeader {
    pub magic: [u8; 4],
    pub version: u16,
    /// Developmental stage (`0=SeedOrSpore, 1=Juvenile, 2=Adult, 3=Senescent, 4=Deceased`).
    pub stage: u8,
    pub reserved_u8: u8,
    pub entity_did_hash: u64,
    pub taxon_iri_hash: u64,
    pub energy_permill: u16,
    pub damage_permill: u16,
    pub hydration_permill: u16,
    pub wear_permill: u16,
    pub affordance_count: u32,
    pub reserved_u32: u32,
}

const _: [(); 40] = [(); core::mem::size_of::<EntityOntologyMiniHeader>()];

impl Default for EntityOntologyMiniHeader {
    fn default() -> Self {
        Self {
            magic: *b"EONT",
            version: 1,
            stage: 2, // Adult
            reserved_u8: 0,
            entity_did_hash: 0,
            taxon_iri_hash: 0,
            energy_permill: 1000,
            damage_permill: 0,
            hydration_permill: 1000,
            wear_permill: 0,
            affordance_count: 0,
            reserved_u32: 0,
        }
    }
}

/// 48-byte fixed-stride `.10d` `EONT1` affordance binary record.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct AffordanceBinaryRecord {
    pub action_id: u64,
    pub required_capability_hash: u64,
    pub emitted_topic_hash: u64,
    pub category: u8,
    pub min_observer_status: u8,
    pub min_energy_permill: u16,
    pub max_damage_permill: u16,
    pub min_hydration_permill: u16,
    pub max_wear_permill: u16,
    pub energy_delta: i16,
    pub damage_delta: i16,
    pub hydration_delta: i16,
    pub wear_delta: i16,
    pub reserved_u16: u16,
    pub animation_clip_id: u32,
}

const _: [(); 48] = [(); core::mem::size_of::<AffordanceBinaryRecord>()];

/// 32-byte fixed-stride `.10d` `EONT1` ontogenetic developmental keyframe record.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DevelopmentalKeyframeRecord {
    pub age_days: f32,
    pub stage: u8,
    pub reserved_u8: u8,
    pub reserved_u16: u16,
    pub allometric_crown_scale: f32,
    pub allometric_trunk_scale: f32,
    pub lai_multiplier: f32,
    pub reserved_u32: u32,
    pub mesh_asset_hash: u64,
}

const _: [(); 32] = [(); core::mem::size_of::<DevelopmentalKeyframeRecord>()];

impl Default for DevelopmentalKeyframeRecord {
    fn default() -> Self {
        Self {
            age_days: 0.0,
            stage: 2,
            reserved_u8: 0,
            reserved_u16: 0,
            allometric_crown_scale: 1.0,
            allometric_trunk_scale: 1.0,
            lai_multiplier: 1.0,
            reserved_u32: 0,
            mesh_asset_hash: 0,
        }
    }
}

/// ABSM (Animation Blending State Machine) layer inspector state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AbsmLayerInspectorState {
    /// 128-bit bone mask (1 bit per skeleton joint).
    pub bone_mask: u128,
    /// Blend weight in `[0.0, 1.0]`.
    pub weight: f32,
    /// True if additive blending, false if override.
    pub is_additive: bool,
    /// Active state index in the layer's state machine.
    pub active_state_id: u16,
    /// Active 2D blend-space parameter `(param_x, param_y)`.
    pub blend_space_uv: [f32; 2],
}

impl Default for AbsmLayerInspectorState {
    fn default() -> Self {
        Self {
            bone_mask: u128::MAX,
            weight: 1.0,
            is_additive: false,
            active_state_id: 0,
            blend_space_uv: [0.0, 0.0],
        }
    }
}

/// Complete zero-heap live inspector state for a selected 10D scene node in `webizen-studio`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StudioNodeInspector10D {
    /// Selected scene node ID.
    pub node_id: u32,
    /// Local PGA `Motor` transform.
    pub local_motor: Motor,
    /// Local 3D scale `[sx, sy, sz]`.
    pub local_scale: [f32; 3],
    /// 10D manifold coordinate `[q, v, w, x, y, z, t, α, μ, σ]`.
    pub tensor_10d: Tensor10D,
    /// Volumetric 10D slice & EMF field parameters.
    pub slice_params: Epistemic10DSliceParams,
    /// `.10d` `PROC1` procedural synthesis parameters.
    pub proc_header: ProceduralSynthesisHeader,
    /// `.10d` `EONT1` entity ontology header (stage + metabolic permills).
    pub eont_header: EntityOntologyMiniHeader,
    /// Active affordances (`[AffordanceBinaryRecord; MAX_INSPECTOR_AFFORDANCES]`).
    pub affordances: [AffordanceBinaryRecord; MAX_INSPECTOR_AFFORDANCES],
    pub affordance_count: usize,
    /// Ontogenetic developmental keyframes (`[DevelopmentalKeyframeRecord; MAX_INSPECTOR_KEYFRAMES]`).
    pub keyframes: [DevelopmentalKeyframeRecord; MAX_INSPECTOR_KEYFRAMES],
    pub keyframe_count: usize,
    /// Multi-layer ABSM animation layer states.
    pub absm_layers: [AbsmLayerInspectorState; MAX_INSPECTOR_ABSM_LAYERS],
    pub absm_layer_count: usize,
}

impl Default for StudioNodeInspector10D {
    fn default() -> Self {
        Self::new(0)
    }
}

impl StudioNodeInspector10D {
    /// Creates a new zero-heap inspector state for `node_id`.
    pub fn new(node_id: u32) -> Self {
        Self {
            node_id,
            local_motor: Motor::identity(),
            local_scale: [1.0, 1.0, 1.0],
            tensor_10d: Tensor10D::default(),
            slice_params: Epistemic10DSliceParams::default(),
            proc_header: ProceduralSynthesisHeader::default(),
            eont_header: EntityOntologyMiniHeader::default(),
            affordances: [AffordanceBinaryRecord::default(); MAX_INSPECTOR_AFFORDANCES],
            affordance_count: 0,
            keyframes: [DevelopmentalKeyframeRecord::default(); MAX_INSPECTOR_KEYFRAMES],
            keyframe_count: 0,
            absm_layers: [AbsmLayerInspectorState::default(); MAX_INSPECTOR_ABSM_LAYERS],
            absm_layer_count: 1,
        }
    }

    /// Evaluates the allometric crown & trunk scale multipliers from the node's current
    /// `EONT1` developmental stage and `PROC1` Huxley power law $y = b \cdot x^k$.
    #[inline]
    pub fn evaluate_ontogenetic_allometric_scale(&self) -> [f32; 3] {
        let stage_progress: f32 = match self.eont_header.stage {
            0 => 0.15, // SeedOrSpore
            1 => 0.55, // Juvenile
            2 => 1.00, // Adult
            3 => 0.95, // Senescent
            _ => 0.80, // Deceased
        };
        let b = self.proc_header.allometry_base_b.max(0.05);
        let k = self.proc_header.allometry_exponent_k.clamp(0.1, 3.0);
        let allometric = b * stage_progress.powf(k);
        let hydration_factor =
            0.85 + 0.15 * (self.eont_header.hydration_permill.min(1000) as f32 / 1000.0);
        [
            self.local_scale[0] * allometric * hydration_factor,
            self.local_scale[1] * allometric,
            self.local_scale[2] * allometric * hydration_factor,
        ]
    }

    /// Sets the blend weight for ABSM layer `layer_idx` in `[0.0, 1.0]`.
    #[inline]
    pub fn set_absm_layer_weight(&mut self, layer_idx: usize, weight: f32) -> bool {
        if layer_idx >= MAX_INSPECTOR_ABSM_LAYERS {
            return false;
        }
        if layer_idx >= self.absm_layer_count {
            self.absm_layer_count = layer_idx + 1;
        }
        self.absm_layers[layer_idx].weight = weight.clamp(0.0, 1.0);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspector_10d_evaluates_ontogenetic_allometry_and_absm_weights() {
        let mut insp = StudioNodeInspector10D::new(42);
        insp.eont_header.stage = 1; // Juvenile
        let juv_scale = insp.evaluate_ontogenetic_allometric_scale();

        insp.eont_header.stage = 2; // Adult
        let adult_scale = insp.evaluate_ontogenetic_allometric_scale();

        assert!(
            adult_scale[1] > juv_scale[1],
            "Adult allometric height ({}) must exceed juvenile ({})",
            adult_scale[1],
            juv_scale[1]
        );

        assert!(insp.set_absm_layer_weight(2, 0.75));
        assert_eq!(insp.absm_layer_count, 3);
        assert!((insp.absm_layers[2].weight - 0.75).abs() < 1e-6);
    }
}
