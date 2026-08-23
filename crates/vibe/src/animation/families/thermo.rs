//! Thermodynamic Phase Animation Family (Heat diffusion, Nucleation, Blackbody glow).

use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
        "heat_diffuse" | "diffuse" => {
            let diff_const = 0.5;
            let temp = (-diff_const * t).exp();
            let radius = (1.0 + t * 2.0).sqrt();
            AnimationSample {
                scalar: temp,
                vector: [radius, radius, radius],
                motor: Motor::identity(),
                secondary: 1.0 - temp,
                settled: temp < 0.01,
            }
        }
        "crystallize" => {
            let phase = (t * 0.8).clamp(0.0, 1.0);
            let order_param = (phase * std::f64::consts::PI * 0.5).sin();
            AnimationSample {
                scalar: order_param,
                vector: [order_param, order_param, order_param],
                motor: Motor::identity(),
                secondary: phase,
                settled: phase >= 1.0,
            }
        }
        "thermal_glow" => {
            let temp_k = (300.0 + t * 1500.0).clamp(300.0, 6000.0);
            let normalized_intensity = ((temp_k - 300.0) / 5700.0).powi(4);
            AnimationSample {
                scalar: normalized_intensity,
                vector: [normalized_intensity, normalized_intensity * 0.6, normalized_intensity * 0.2],
                motor: Motor::identity(),
                secondary: temp_k,
                settled: temp_k >= 6000.0,
            }
        }
        "implode" => {
            let p = (t * 1.5).clamp(0.0, 1.0);
            let collapse = 1.0 - p.powi(4);
            AnimationSample {
                scalar: collapse,
                vector: [collapse, collapse, collapse],
                motor: Motor::identity(),
                secondary: 1.0 - collapse,
                settled: p >= 1.0,
            }
        }
        _ => AnimationSample::default(),
    }
}
