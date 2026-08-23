//! Physical Dynamics Animation Family (Springs, Bounces, Gravity Wells, Impulses).

use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;
use crate::animation::spring::{SpringConfig, SpringState1D};

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
        "spring_settle" | "spring" => {
            let config = SpringConfig::default();
            let state = SpringState1D::new(0.0, 0.0, 1.0);
            let (pos, vel, settled) = state.evaluate_at(&config, t);
            AnimationSample {
                scalar: pos,
                vector: [pos, 0.0, 0.0],
                motor: Motor::from_translation(pos, 0.0, 0.0),
                secondary: vel,
                settled,
            }
        }
        "bouncy_drop" | "bounce" => {
            let config = SpringConfig::bouncy();
            let state = SpringState1D::new(10.0, 0.0, 0.0);
            let (pos, vel, settled) = state.evaluate_at(&config, t);
            let y = pos.abs();
            AnimationSample {
                scalar: y,
                vector: [0.0, y, 0.0],
                motor: Motor::from_translation(0.0, y, 0.0),
                secondary: vel,
                settled,
            }
        }
        "verlet_wave" => {
            let k = 2.0;
            let omega = 3.0;
            let height = (k * 1.0 - omega * t).sin();
            let slope = (k * 1.0 - omega * t).cos() * k;
            AnimationSample {
                scalar: height,
                vector: [0.0, height, 0.0],
                motor: Motor::from_rotation_translation([1.0, 0.0, slope * 0.1, 0.0], [0.0, height, 0.0]),
                secondary: slope,
                settled: false,
            }
        }
        "gravity_well" => {
            let g = 9.81;
            let m = 100.0;
            let dist = (10.0 - t * 2.0).max(0.5);
            let force = (g * m) / (dist * dist);
            AnimationSample {
                scalar: force,
                vector: [dist, 0.0, 0.0],
                motor: Motor::from_translation(dist, 0.0, 0.0),
                secondary: force,
                settled: dist <= 0.5,
            }
        }
        "collision_rebound" => {
            let decay = (-t * 4.0).exp();
            let impulse = (t * 20.0).cos() * decay;
            AnimationSample {
                scalar: impulse,
                vector: [0.0, impulse, 0.0],
                motor: Motor::from_translation(0.0, impulse, 0.0),
                secondary: decay,
                settled: decay < 0.001,
            }
        }
        _ => AnimationSample::default(),
    }
}
