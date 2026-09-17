//! Generative Stochastic Fields Animation Family (Simplex flows, Vortices, Brownian walks).

use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
        "simplex_flow" | "simplex" => {
            let vx = (t * 0.7).sin() * 0.6 + (t * 1.3).cos() * 0.4;
            let vy = (t * 0.9).cos() * 0.5 + (t * 1.7).sin() * 0.5;
            let speed = (vx * vx + vy * vy).sqrt();
            AnimationSample {
                scalar: speed,
                vector: [vx, vy, 0.0],
                motor: Motor::from_translation(vx * 2.0, vy * 2.0, 0.0),
                secondary: speed,
                settled: false,
            }
        }
        "vortex_swirl" | "vortex" => {
            let r = 1.0 + t * 0.5;
            let theta = t * 4.0;
            let x = r * theta.cos();
            let z = r * theta.sin();
            let motor = Motor::from_rotation_translation(
                [(theta * 0.5).cos(), 0.0, (theta * 0.5).sin(), 0.0],
                [x, 0.0, z],
            );
            AnimationSample {
                scalar: theta,
                vector: [x, 0.0, z],
                motor,
                secondary: r,
                settled: false,
            }
        }
        "brownian_drift" | "brownian" => {
            let s1 = (t * 11.0).sin();
            let s2 = (t * 17.0).cos();
            let s3 = (t * 23.0).sin();
            let val = (s1 + s2 + s3) / 3.0;
            AnimationSample {
                scalar: val,
                vector: [s1 * 0.5, s2 * 0.5, s3 * 0.5],
                motor: Motor::from_translation(s1 * 0.2, s2 * 0.2, s3 * 0.2),
                secondary: val.abs(),
                settled: false,
            }
        }
        _ => AnimationSample::default(),
    }
}
