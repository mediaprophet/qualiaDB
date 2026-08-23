//! The 10-Family Animation Presets Catalog for VibeScript (Zero-Heap).
//!
//! Provides canonical animation templates across:
//! 1. Spatial Kinematics
//! 2. Dynamics & Material Physics
//! 3. Mesh & Topology
//! 4. Thermodynamic Phase
//! 5. Optics & Spectral Waves
//! 6. Acoustic Synthesis
//! 7. Multi-Track Timelines
//! 8. HUD & UI Glassmorphism
//! 9. Sensory Outbound Haptics
//! 10. Generative Stochastic Fields

use crate::animation::families;
use crate::animation::pga::Motor;

/// The 10 Animation Families.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AnimationFamily {
    SpatialKinematics,
    PhysicalDynamics,
    MeshTopology,
    ThermodynamicPhase,
    OpticsWaves,
    AcousticSpectral,
    MultiTrackTimelines,
    HudGlassUi,
    OutboundHaptics,
    GenerativeFields,
}

impl AnimationFamily {
    pub fn from_name(name: &str) -> Option<Self> {
        match name.to_ascii_lowercase().replace('_', "-").as_str() {
            "spatial-kinematics" | "kinematics" => Some(Self::SpatialKinematics),
            "physical-dynamics" | "dynamics" => Some(Self::PhysicalDynamics),
            "mesh-topology" | "topology" => Some(Self::MeshTopology),
            "thermodynamic-phase" | "thermodynamics" => Some(Self::ThermodynamicPhase),
            "optics-waves" | "optics" => Some(Self::OpticsWaves),
            "acoustic-spectral" | "acoustic" => Some(Self::AcousticSpectral),
            "multitrack-timelines" | "multitrack" => Some(Self::MultiTrackTimelines),
            "hud-glass-ui" | "hud" | "glass" => Some(Self::HudGlassUi),
            "outbound-haptics" | "haptics" => Some(Self::OutboundHaptics),
            "generative-fields" | "generative" => Some(Self::GenerativeFields),
            _ => None,
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::SpatialKinematics => "spatial_kinematics",
            Self::PhysicalDynamics => "physical_dynamics",
            Self::MeshTopology => "mesh_topology",
            Self::ThermodynamicPhase => "thermodynamic_phase",
            Self::OpticsWaves => "optics_waves",
            Self::AcousticSpectral => "acoustic_spectral",
            Self::MultiTrackTimelines => "multitrack_timelines",
            Self::HudGlassUi => "hud_glass_ui",
            Self::OutboundHaptics => "outbound_haptics",
            Self::GenerativeFields => "generative_fields",
        }
    }
}

/// A standard animation evaluation output record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AnimationSample {
    pub scalar: f64,
    pub vector: [f64; 3],
    pub motor: Motor,
    pub secondary: f64,
    pub settled: bool,
}

impl Default for AnimationSample {
    fn default() -> Self {
        Self {
            scalar: 0.0,
            vector: [0.0; 3],
            motor: Motor::identity(),
            secondary: 0.0,
            settled: false,
        }
    }
}

/// Metadata describing an animation preset.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetInfo {
    pub family: &'static str,
    pub preset: &'static str,
    pub description: &'static str,
}

/// Return a static list of all available presets across all 10 families.
pub fn list_all_presets() -> &'static [PresetInfo] {
    &[
        // 1. Spatial Kinematics
        PresetInfo { family: "spatial_kinematics", preset: "orbit_spin", description: "Harmonic circular orbit in XZ plane with rotational tracking" },
        PresetInfo { family: "spatial_kinematics", preset: "hero_drift", description: "Smooth cinematic ScLERP glide between two 3D rigid poses" },
        PresetInfo { family: "spatial_kinematics", preset: "elastic_snap", description: "High-stiffness overshoot translation to target coordinate" },
        PresetInfo { family: "spatial_kinematics", preset: "geodesic_warp", description: "Non-Euclidean metric contraction across space-time manifold" },
        // 2. Physical Dynamics
        PresetInfo { family: "physical_dynamics", preset: "spring_settle", description: "1D critically damped spring relaxation" },
        PresetInfo { family: "physical_dynamics", preset: "bouncy_drop", description: "Underdamped gravitational restitution bounce" },
        PresetInfo { family: "physical_dynamics", preset: "verlet_wave", description: "Symplectic Verlet wave height propagation" },
        PresetInfo { family: "physical_dynamics", preset: "gravity_well", description: "Inverse-square orbital attraction toward central attractor" },
        PresetInfo { family: "physical_dynamics", preset: "collision_rebound", description: "Instantaneous momentum-conserving impulse decay" },
        // 3. Mesh Topology
        PresetInfo { family: "mesh_topology", preset: "morph_laplacian", description: "Smooth geometric Laplacian surface interpolation" },
        PresetInfo { family: "mesh_topology", preset: "origami_unfold", description: "Simplicial complex dimension unfolding from 0-simplex to 2-simplex" },
        PresetInfo { family: "mesh_topology", preset: "stalk_focus", description: "Topological sheaf restriction focusing on active subtree" },
        PresetInfo { family: "mesh_topology", preset: "topology_collapse", description: "Homotopy contraction to single singular boundary point" },
        // 4. Thermodynamic Phase
        PresetInfo { family: "thermodynamic_phase", preset: "heat_diffuse", description: "Exponential thermal dissipation and entropy expansion" },
        PresetInfo { family: "thermodynamic_phase", preset: "crystallize", description: "First-order nucleation phase transition boundary" },
        PresetInfo { family: "thermodynamic_phase", preset: "thermal_glow", description: "Blackbody spectral radiation intensity curve" },
        PresetInfo { family: "thermodynamic_phase", preset: "implode", description: "High-pressure volumetric collapse into core singularity" },
        // 5. Optics & Waves
        PresetInfo { family: "optics_waves", preset: "doppler_shift", description: "Relativistic wavefront compression and frequency shift" },
        PresetInfo { family: "optics_waves", preset: "fresnel_glow", description: "Schlick approximation rim radiance curve" },
        PresetInfo { family: "optics_waves", preset: "refractive_pulse", description: "Snell law refractive index modulation across D7 manifold" },
        PresetInfo { family: "optics_waves", preset: "chromatic_aberration", description: "Wavelength dispersion splitting RGB channels" },
        PresetInfo { family: "optics_waves", preset: "caustic_flow", description: "Interferometric specular caustic illumination envelope" },
        // 6. Acoustic Spectral
        PresetInfo { family: "acoustic_spectral", preset: "adsr_pulse", description: "Standard Attack-Decay-Sustain-Release amplitude envelope" },
        PresetInfo { family: "acoustic_spectral", preset: "binaural_sweep", description: "Stereo interaural phase disparity spatial pan" },
        PresetInfo { family: "acoustic_spectral", preset: "resonant_formant", description: "Multi-pole acoustic vocal tract resonant peak sweep" },
        // 7. Multi-Track Timelines
        PresetInfo { family: "multitrack_timelines", preset: "polymetric_sync", description: "Polymetric 3:4:5 cross-rhythm phase synchronization" },
        PresetInfo { family: "multitrack_timelines", preset: "keyframe_blend", description: "Smooth Hermite spline curve through multi-channel keys" },
        PresetInfo { family: "multitrack_timelines", preset: "stagger_cascade", description: "Progressive cascade delay across N entity indices" },
        // 8. HUD & UI Glassmorphism
        PresetInfo { family: "hud_glass_ui", preset: "glass_reveal", description: "Smooth frosted-glass backdrop blur and scale-up reveal" },
        PresetInfo { family: "hud_glass_ui", preset: "chromatic_pulse", description: "Micro-dispersion neon border accent wave" },
        PresetInfo { family: "hud_glass_ui", preset: "frosted_fade", description: "Background surface transmission depth interpolation" },
        PresetInfo { family: "hud_glass_ui", preset: "focus_halo", description: "Radial gaze-tracking luminance focus vignette" },
        // 9. Outbound Haptics
        PresetInfo { family: "outbound_haptics", preset: "vibro_click", description: "30ms sharp tactile mechanical click transient" },
        PresetInfo { family: "outbound_haptics", preset: "heartbeat_pulse", description: "Dual-impulse systolic / diastolic cardiac envelope" },
        PresetInfo { family: "outbound_haptics", preset: "kinesthetic_resist", description: "Nonlinear motor torque brake resistance against velocity" },
        // 10. Generative Stochastic Fields
        PresetInfo { family: "generative_fields", preset: "simplex_flow", description: "Coherent turbulent vector field velocity advection" },
        PresetInfo { family: "generative_fields", preset: "vortex_swirl", description: "Logarithmic spiral angular momentum vortex" },
        PresetInfo { family: "generative_fields", preset: "brownian_drift", description: "Continuous Wiener process stochastic random walk" },
    ]
}

/// Evaluate a preset by family name and preset name at time `t` (seconds).
pub fn evaluate_preset(family: AnimationFamily, preset: &str, t: f64) -> AnimationSample {
    families::dispatch(family, preset, t)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_family_from_name() {
        assert_eq!(
            AnimationFamily::from_name("spatial-kinematics"),
            Some(AnimationFamily::SpatialKinematics)
        );
        assert_eq!(
            AnimationFamily::from_name("hud"),
            Some(AnimationFamily::HudGlassUi)
        );
        assert_eq!(
            AnimationFamily::from_name("dynamics"),
            Some(AnimationFamily::PhysicalDynamics)
        );
    }

    #[test]
    fn test_list_all_presets() {
        let all = list_all_presets();
        assert!(all.len() >= 30, "expected at least 30 presets in catalog");
    }

    #[test]
    fn test_evaluate_all_families() {
        let families = [
            AnimationFamily::SpatialKinematics,
            AnimationFamily::PhysicalDynamics,
            AnimationFamily::MeshTopology,
            AnimationFamily::ThermodynamicPhase,
            AnimationFamily::OpticsWaves,
            AnimationFamily::AcousticSpectral,
            AnimationFamily::MultiTrackTimelines,
            AnimationFamily::HudGlassUi,
            AnimationFamily::OutboundHaptics,
            AnimationFamily::GenerativeFields,
        ];

        for fam in families {
            let presets = list_all_presets().iter().filter(|p| p.family == fam.name());
            for p in presets {
                let sample = evaluate_preset(fam, p.preset, 0.5);
                assert!(!sample.scalar.is_nan());
                assert!(!sample.vector[0].is_nan());
            }
        }
    }
}
