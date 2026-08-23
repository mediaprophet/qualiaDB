//! Optics & Waves Animation Family (Doppler shifts, Fresnel glows, Chromatic dispersion).

use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
        "doppler_shift" => {
            let v = 0.6_f64; // 60% c
            let doppler_factor = ((1.0_f64 + v) / (1.0_f64 - v)).sqrt();
            let wave_phase = (t * 5.0 * doppler_factor).sin();
            AnimationSample {
                scalar: doppler_factor,
                vector: [wave_phase, 0.0, 0.0],
                motor: Motor::identity(),
                secondary: wave_phase,
                settled: false,
            }
        }
        "fresnel_glow" | "fresnel" => {
            let n1 = 1.0_f64;
            let n2 = 1.5_f64;
            let r0 = ((n1 - n2) / (n1 + n2)).powi(2);
            let cos_theta = (t * 0.5).cos().abs();
            let fresnel = r0 + (1.0 - r0) * (1.0 - cos_theta).powi(5);
            AnimationSample {
                scalar: fresnel,
                vector: [fresnel, fresnel * 0.8, fresnel * 1.2],
                motor: Motor::identity(),
                secondary: cos_theta,
                settled: false,
            }
        }
        "refractive_pulse" => {
            let n = 1.33 + (t * 4.0).sin() * 0.2;
            AnimationSample {
                scalar: n,
                vector: [n, n, n],
                motor: Motor::identity(),
                secondary: (t * 4.0).sin(),
                settled: false,
            }
        }
        "chromatic_aberration" | "aberration" => {
            let shift = (t * 2.0).sin() * 0.05;
            AnimationSample {
                scalar: shift.abs(),
                vector: [shift, 0.0, -shift],
                motor: Motor::identity(),
                secondary: shift,
                settled: false,
            }
        }
        "caustic_flow" => {
            let c1 = (t * 3.0).sin();
            let c2 = (t * 4.5 + 1.0).sin();
            let caustic = (c1 * c2).abs().powf(1.5);
            AnimationSample {
                scalar: caustic,
                vector: [caustic, caustic, caustic * 1.3],
                motor: Motor::identity(),
                secondary: c1,
                settled: false,
            }
        }
        _ => AnimationSample::default(),
    }
}
