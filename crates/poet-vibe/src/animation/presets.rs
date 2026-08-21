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
    fn test_evaluate_glass_reveal_preset() {
        let sample = evaluate_preset(AnimationFamily::HudGlassUi, "glass_reveal", 0.5);
        assert!(sample.scalar > 0.0);
        assert!(sample.vector[0] > 0.0);
    }

    #[test]
    fn test_evaluate_orbit_preset() {
        let sample = evaluate_preset(AnimationFamily::SpatialKinematics, "orbit", 1.0);
        let (_, trans) = sample.motor.to_rotation_translation();
        assert!((trans[0] * trans[0] + trans[2] * trans[2] - 100.0).abs() < 1e-3);
    }
}
