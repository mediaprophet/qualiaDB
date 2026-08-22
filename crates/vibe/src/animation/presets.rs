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

use crate::animation::curves::EasingCurve;
use crate::animation::pga::Motor;
use crate::animation::spring::{SpringConfig, SpringState1D};

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
    match family {
        AnimationFamily::SpatialKinematics => match preset {
            "orbit_spin" | "orbit" => {
                let angle = t * std::f64::consts::PI * 2.0 * 0.5; // 0.5 Hz
                let radius = 10.0;
                let x = radius * angle.cos();
                let z = radius * angle.sin();
                let motor = Motor::from_rotation_translation(
                    [(angle * 0.5).cos(), 0.0, (angle * 0.5).sin(), 0.0],
                    [x, 0.0, z],
                );
                AnimationSample {
                    scalar: angle,
                    vector: [x, 0.0, z],
                    motor,
                    secondary: radius,
                    settled: false,
                }
            }
            "hero_drift" => {
                let progress = (t * 0.2).clamp(0.0, 1.0);
                let ease = EasingCurve::CubicInOut.eval(progress);
                let m0 = Motor::identity();
                let m1 =
                    Motor::from_rotation_translation([0.9238, 0.0, 0.3826, 0.0], [5.0, 2.0, -10.0]);
                let motor = Motor::sclerp(&m0, &m1, ease);
                let (_, trans) = motor.to_rotation_translation();
                AnimationSample {
                    scalar: ease,
                    vector: trans,
                    motor,
                    secondary: 1.0 - ease,
                    settled: progress >= 1.0,
                }
            }
            "elastic_snap" => {
                let config = SpringConfig::snappy();
                let state = SpringState1D::new(0.0, 0.0, 10.0);
                let (pos, vel, settled) = state.evaluate_at(&config, t);
                let motor = Motor::from_rotation_translation([1.0, 0.0, 0.0, 0.0], [pos, 0.0, 0.0]);
                AnimationSample {
                    scalar: pos,
                    vector: [pos, 0.0, 0.0],
                    motor,
                    secondary: vel,
                    settled,
                }
            }
            "geodesic_warp" => {
                let warp = 1.0 / (1.0 + (t * 0.5).powi(2));
                let angle = (t * 1.5).sin() * warp;
                let motor = Motor::from_rotation_translation([(angle * 0.5).cos(), (angle * 0.5).sin(), 0.0, 0.0], [0.0, 0.0, warp * 5.0]);
                AnimationSample {
                    scalar: warp,
                    vector: [0.0, 0.0, warp * 5.0],
                    motor,
                    secondary: angle,
                    settled: warp < 0.01,
                }
            }
            _ => AnimationSample::default(),
        },

        AnimationFamily::PhysicalDynamics => match preset {
            "spring_settle" => {
                let config = SpringConfig::snappy();
                let state = SpringState1D::new(0.0, 0.0, 1.0);
                let (pos, vel, settled) = state.evaluate_at(&config, t);
                AnimationSample {
                    scalar: pos,
                    vector: [pos, 0.0, 0.0],
                    motor: Motor::from_rotation_translation([1.0, 0.0, 0.0, 0.0], [pos, 0.0, 0.0]),
                    secondary: vel,
                    settled,
                }
            }
            "bouncy_drop" => {
                let config = SpringConfig::bouncy();
                let state = SpringState1D::new(100.0, 0.0, 0.0);
                let (pos, vel, settled) = state.evaluate_at(&config, t);
                AnimationSample {
                    scalar: pos,
                    vector: [0.0, pos, 0.0],
                    motor: Motor::from_rotation_translation([1.0, 0.0, 0.0, 0.0], [0.0, pos, 0.0]),
                    secondary: vel,
                    settled,
                }
            }
            "verlet_wave" => {
                let k = 2.0;
                let omega = 4.0;
                let y = (k * 1.0 - omega * t).sin() * (-0.1 * t).exp();
                AnimationSample {
                    scalar: y,
                    vector: [0.0, y, 0.0],
                    motor: Motor::identity(),
                    secondary: omega,
                    settled: t > 10.0,
                }
            }
            "gravity_well" => {
                let r0 = 20.0;
                let mu = 100.0;
                let r = (r0 * r0 - 2.0 * mu * t).max(0.1).sqrt();
                let force = mu / (r * r);
                AnimationSample {
                    scalar: r,
                    vector: [r, 0.0, 0.0],
                    motor: Motor::identity(),
                    secondary: force,
                    settled: r <= 0.2,
                }
            }
            "collision_rebound" => {
                let e: f64 = 0.8; // coefficient of restitution
                let v0: f64 = 10.0;
                let t_bounce: f64 = 0.5;
                let bounce_count = (t / t_bounce).floor();
                let v = v0 * e.powf(bounce_count);
                let progress_in_bounce = (t % t_bounce) / t_bounce;
                let y = v * (1.0 - (progress_in_bounce * 2.0 - 1.0).powi(2));
                AnimationSample {
                    scalar: y,
                    vector: [0.0, y, 0.0],
                    motor: Motor::identity(),
                    secondary: v,
                    settled: v < 0.05,
                }
            }
            _ => AnimationSample::default(),
        },

        AnimationFamily::MeshTopology => match preset {
            "morph_laplacian" => {
                let progress = (t * 0.5).clamp(0.0, 1.0);
                let weight = EasingCurve::CubicInOut.eval(progress);
                AnimationSample {
                    scalar: weight,
                    vector: [weight, 1.0 - weight, 0.0],
                    motor: Motor::identity(),
                    secondary: 1.0 - weight,
                    settled: progress >= 1.0,
                }
            }
            "origami_unfold" => {
                let progress = (t * 0.75).clamp(0.0, 1.0);
                let angle = (1.0 - progress) * std::f64::consts::PI * 0.5;
                let fold_motor = Motor::from_rotation_translation([(angle * 0.5).cos(), (angle * 0.5).sin(), 0.0, 0.0], [0.0, progress, 0.0]);
                AnimationSample {
                    scalar: progress,
                    vector: [progress, angle.to_degrees(), 0.0],
                    motor: fold_motor,
                    secondary: angle,
                    settled: progress >= 1.0,
                }
            }
            "stalk_focus" => {
                let ease = EasingCurve::QuartOut.eval((t * 1.2).clamp(0.0, 1.0));
                AnimationSample {
                    scalar: ease,
                    vector: [ease, ease * 1.2, 1.0],
                    motor: Motor::identity(),
                    secondary: 1.0 - ease,
                    settled: t >= 0.83,
                }
            }
            "topology_collapse" => {
                let scale = (1.0 - t * 0.8).max(0.0).powi(3);
                AnimationSample {
                    scalar: scale,
                    vector: [scale, scale, scale],
                    motor: Motor::identity(),
                    secondary: 1.0 - scale,
                    settled: scale == 0.0,
                }
            }
            _ => AnimationSample::default(),
        },

        AnimationFamily::ThermodynamicPhase => match preset {
            "heat_diffuse" => {
                let alpha = 0.1; // Thermal diffusivity
                let temp = (-alpha * t).exp();
                AnimationSample {
                    scalar: temp,
                    vector: [temp, temp * 0.5, 0.0],
                    motor: Motor::identity(),
                    secondary: 1.0 - temp,
                    settled: temp < 1e-3,
                }
            }
            "crystallize" => {
                let progress = (t * 0.6).clamp(0.0, 1.0);
                let order = 1.0 / (1.0 + (-10.0 * (progress - 0.5)).exp()); // Sigmoidal phase change
                AnimationSample {
                    scalar: order,
                    vector: [order, order * 0.9, 1.0],
                    motor: Motor::identity(),
                    secondary: progress,
                    settled: progress >= 1.0,
                }
            }
            "thermal_glow" => {
                let kelvin = 300.0 + 2000.0 * (1.0 - (-0.4 * t).exp());
                let radiance = (kelvin / 2300.0).powi(4); // Stefan-Boltzmann ~ T^4
                AnimationSample {
                    scalar: radiance,
                    vector: [radiance.min(1.0), (radiance * 0.6).min(1.0), (radiance * 0.2).min(1.0)],
                    motor: Motor::identity(),
                    secondary: kelvin,
                    settled: kelvin >= 2290.0,
                }
            }
            "implode" => {
                let radius = (1.0 - t * 1.5).max(0.0).powi(2);
                let pressure = if radius > 0.001 { 1.0 / radius } else { 1000.0 };
                AnimationSample {
                    scalar: radius,
                    vector: [radius, radius, radius],
                    motor: Motor::identity(),
                    secondary: pressure,
                    settled: radius == 0.0,
                }
            }
            _ => AnimationSample::default(),
        },

        AnimationFamily::OpticsWaves => match preset {
            "doppler_shift" => {
                let v = 50.0; // Source velocity (m/s)
                let c = 343.0; // Speed of wave
                let f0 = 440.0; // Base frequency
                let f_observed = f0 * (c / (c - v));
                let wavelength = 550.0 * (c / (c + v)); // Spectral shift in nm
                AnimationSample {
                    scalar: f_observed,
                    vector: [wavelength, 0.0, 0.0],
                    motor: Motor::identity(),
                    secondary: f_observed / f0,
                    settled: false,
                }
            }
            "fresnel_glow" => {
                let angle = (t * 2.0).sin().abs();
                let f0 = 0.04;
                let fresnel = f0 + (1.0 - f0) * (1.0 - angle).powi(5);
                AnimationSample {
                    scalar: fresnel,
                    vector: [fresnel, fresnel * 0.8, 1.0],
                    motor: Motor::identity(),
                    secondary: angle,
                    settled: false,
                }
            }
            "refractive_pulse" => {
                let n = 1.33 + 0.3 * (t * 3.0).sin().abs(); // Water to dense glass
                AnimationSample {
                    scalar: n,
                    vector: [n, 1.0 / n, (t * 3.0).sin()],
                    motor: Motor::identity(),
                    secondary: (t * 3.0).sin(),
                    settled: false,
                }
            }
            "chromatic_aberration" => {
                let wave = (t * 4.0).sin();
                let shift = (wave * 0.5 + 0.5) * 5.0; // 0 to 5 px
                AnimationSample {
                    scalar: shift,
                    vector: [shift, 0.0, -shift],
                    motor: Motor::identity(),
                    secondary: wave,
                    settled: false,
                }
            }
            "caustic_flow" => {
                let c1 = (t * 2.1).sin();
                let c2 = (t * 3.4).cos();
                let intensity = (c1 * c2).abs().powi(3);
                AnimationSample {
                    scalar: intensity,
                    vector: [intensity, intensity * 0.9, intensity * 0.7],
                    motor: Motor::identity(),
                    secondary: c1 + c2,
                    settled: false,
                }
            }
            _ => AnimationSample::default(),
        },

        AnimationFamily::AcousticSpectral => match preset {
            "adsr_pulse" => {
                let attack = 0.05;
                let decay = 0.1;
                let sustain = 0.7;
                let release = 0.2;
                let note_len = 0.5;

                let amp = if t < attack {
                    t / attack
                } else if t < attack + decay {
                    1.0 - (1.0 - sustain) * ((t - attack) / decay)
                } else if t < note_len {
                    sustain
                } else if t < note_len + release {
                    sustain * (1.0 - (t - note_len) / release)
                } else {
                    0.0
                };

                AnimationSample {
                    scalar: amp,
                    vector: [amp, 0.0, 0.0],
                    motor: Motor::identity(),
                    secondary: amp * 440.0,
                    settled: t >= note_len + release,
                }
            }
            "binaural_sweep" => {
                let pan = (t * 0.8).sin(); // -1.0 left to 1.0 right
                let left_gain = (1.0 - pan) * 0.5;
                let right_gain = (1.0 + pan) * 0.5;
                AnimationSample {
                    scalar: pan,
                    vector: [left_gain, right_gain, 0.0],
                    motor: Motor::identity(),
                    secondary: (left_gain - right_gain).abs(),
                    settled: false,
                }
            }
            "resonant_formant" => {
                let formant_freq = 500.0 + 1500.0 * (t * 1.5).sin().abs();
                let q_factor = 8.0;
                AnimationSample {
                    scalar: formant_freq,
                    vector: [formant_freq, q_factor, 0.0],
                    motor: Motor::identity(),
                    secondary: q_factor,
                    settled: false,
                }
            }
            _ => AnimationSample::default(),
        },

        AnimationFamily::MultiTrackTimelines => match preset {
            "polymetric_sync" => {
                let bpm = 120.0;
                let bps = bpm / 60.0;
                let beat_phase = (t * bps).fract();
                let bar_phase = (t * bps / 4.0).fract();
                AnimationSample {
                    scalar: beat_phase,
                    vector: [beat_phase, bar_phase, (t * bps / 3.0).fract()],
                    motor: Motor::identity(),
                    secondary: (t * bps).floor(),
                    settled: false,
                }
            }
            "keyframe_blend" => {
                let progress = (t * 0.5).clamp(0.0, 1.0);
                let smooth = EasingCurve::CubicInOut.eval(progress);
                AnimationSample {
                    scalar: smooth,
                    vector: [smooth * 10.0, smooth * 5.0, smooth * -20.0],
                    motor: Motor::identity(),
                    secondary: 1.0 - smooth,
                    settled: progress >= 1.0,
                }
            }
            "stagger_cascade" => {
                let delay = 0.1;
                let entity_count = 5;
                let mut active_count = 0.0;
                for i in 0..entity_count {
                    if t > i as f64 * delay {
                        active_count += 1.0;
                    }
                }
                AnimationSample {
                    scalar: active_count,
                    vector: [active_count / entity_count as f64, 0.0, 0.0],
                    motor: Motor::identity(),
                    secondary: entity_count as f64 - active_count,
                    settled: active_count == entity_count as f64,
                }
            }
            _ => AnimationSample::default(),
        },

        AnimationFamily::HudGlassUi => match preset {
            "glass_reveal" => {
                let progress = (t * 1.5).clamp(0.0, 1.0);
                let spring = SpringConfig::snappy();
                let state = SpringState1D::new(0.0, 0.0, 1.0);
                let (scale, _, settled) = state.evaluate_at(&spring, t);
                let opacity = EasingCurve::CubicOut.eval(progress);
                let blur_radius = (1.0 - progress) * 20.0;

                AnimationSample {
                    scalar: opacity,
                    vector: [scale, scale, 1.0],
                    motor: Motor::from_rotation_translation(
                        [1.0, 0.0, 0.0, 0.0],
                        [0.0, (1.0 - scale) * -20.0, 0.0],
                    ),
                    secondary: blur_radius,
                    settled,
                }
            }
            "chromatic_pulse" => {
                let wave = (t * 4.0).sin();
                let dispersion = (wave * 0.5 + 0.5) * 4.0; // 0 to 4 px RGB shift
                AnimationSample {
                    scalar: dispersion,
                    vector: [dispersion, 0.0, -dispersion],
                    motor: Motor::identity(),
                    secondary: wave,
                    settled: false,
                }
            }
            "frosted_fade" => {
                let progress = (t * 1.0).clamp(0.0, 1.0);
                let alpha = EasingCurve::QuadInOut.eval(progress);
                let blur = alpha * 30.0;
                AnimationSample {
                    scalar: alpha,
                    vector: [alpha, blur, 0.0],
                    motor: Motor::identity(),
                    secondary: blur,
                    settled: progress >= 1.0,
                }
            }
            "focus_halo" => {
                let radius = 50.0 + 10.0 * (t * 3.0).sin();
                let brightness = 0.8 + 0.2 * (t * 3.0).cos();
                AnimationSample {
                    scalar: radius,
                    vector: [radius, brightness, 0.0],
                    motor: Motor::identity(),
                    secondary: brightness,
                    settled: false,
                }
            }
            _ => AnimationSample::default(),
        },

        AnimationFamily::OutboundHaptics => match preset {
            "vibro_click" => {
                let duration = 0.03; // 30ms sharp haptic pulse
                let amp = if t < duration {
                    (1.0 - t / duration) * (t * 500.0).sin()
                } else {
                    0.0
                };
                AnimationSample {
                    scalar: amp,
                    vector: [amp, 0.0, 0.0],
                    motor: Motor::identity(),
                    secondary: 150.0, // 150 Hz resonance
                    settled: t >= duration,
                }
            }
            "heartbeat_pulse" => {
                let cycle = (t * 1.2).fract(); // 72 bpm
                let amp = if cycle < 0.1 {
                    (cycle / 0.1) * (cycle * 300.0).sin()
                } else if cycle > 0.2 && cycle < 0.35 {
                    0.6 * ((cycle - 0.2) / 0.15) * (cycle * 250.0).sin()
                } else {
                    0.0
                };
                AnimationSample {
                    scalar: amp.abs(),
                    vector: [amp.abs(), 0.0, 0.0],
                    motor: Motor::identity(),
                    secondary: 72.0,
                    settled: false,
                }
            }
            "kinesthetic_resist" => {
                let velocity = (t * 2.0).sin();
                let resistance = velocity.powi(2) * 0.75;
                AnimationSample {
                    scalar: resistance,
                    vector: [-resistance * velocity.signum(), 0.0, 0.0],
                    motor: Motor::identity(),
                    secondary: velocity,
                    settled: false,
                }
            }
            _ => AnimationSample::default(),
        },

        AnimationFamily::GenerativeFields => match preset {
            "simplex_flow" => {
                let angle = (t * 0.5).sin() * std::f64::consts::PI;
                let vx = angle.cos();
                let vy = angle.sin();
                AnimationSample {
                    scalar: angle,
                    vector: [vx, vy, 0.0],
                    motor: Motor::identity(),
                    secondary: (vx * vx + vy * vy).sqrt(),
                    settled: false,
                }
            }
            "vortex_swirl" => {
                let radius = (t * 0.2).fract() * 20.0;
                let theta = t * 3.0;
                let x = radius * theta.cos();
                let y = radius * theta.sin();
                AnimationSample {
                    scalar: theta,
                    vector: [x, y, radius],
                    motor: Motor::identity(),
                    secondary: radius,
                    settled: false,
                }
            }
            "brownian_drift" => {
                // Deterministic pseudo-Brownian noise based on fractional trigonometric series
                let bx = (t * 1.7).sin() * 0.5 + (t * 5.3).cos() * 0.25 + (t * 13.1).sin() * 0.125;
                let by = (t * 2.3).cos() * 0.5 + (t * 7.1).sin() * 0.25 + (t * 17.3).cos() * 0.125;
                let bz = (t * 3.1).sin() * 0.5 + (t * 11.7).cos() * 0.25 + (t * 19.9).sin() * 0.125;
                AnimationSample {
                    scalar: (bx * bx + by * by + bz * bz).sqrt(),
                    vector: [bx, by, bz],
                    motor: Motor::identity(),
                    secondary: bx + by + bz,
                    settled: false,
                }
            }
            _ => AnimationSample::default(),
        },
    }
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
