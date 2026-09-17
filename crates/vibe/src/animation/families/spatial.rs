//! Spatial Kinematics Animation Family (ScLERP, Orbits, Geodesic Warps).

use crate::animation::curves::EasingCurve;
use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;
use crate::animation::spring::{SpringConfig, SpringState1D};

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
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
            let phase = t * 0.8;
            let curvature = (phase * 1.5).sin() * 0.5;
            let r = 1.0 + curvature;
            let motor = Motor::from_rotation_translation(
                [(phase * 0.25).cos(), 0.0, (phase * 0.25).sin(), 0.0],
                [curvature * 2.0, 0.0, 0.0],
            );
            AnimationSample {
                scalar: curvature,
                vector: [r, r, r],
                motor,
                secondary: phase,
                settled: false,
            }
        }
        _ => AnimationSample::default(),
    }
}
