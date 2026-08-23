//! HUD & UI Glassmorphism Animation Family (Glass reveals, Chromatic pulses, Focus halos).

use crate::animation::curves::EasingCurve;
use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
        "glass_reveal" | "reveal" => {
            let progress = (t * 1.2).clamp(0.0, 1.0);
            let scale = EasingCurve::BackOut.eval(progress);
            let blur = progress * 24.0;
            let opacity = EasingCurve::CubicOut.eval(progress);
            AnimationSample {
                scalar: scale,
                vector: [scale, scale, 1.0],
                motor: Motor::from_translation(0.0, (1.0 - opacity) * -20.0, 0.0),
                secondary: blur,
                settled: progress >= 1.0,
            }
        }
        "chromatic_pulse" | "pulse" => {
            let cycle = (t * 2.5).sin();
            let dispersion = (cycle * 0.5 + 0.5) * 4.0;
            let glow = (cycle * 0.5 + 0.5) * 0.8 + 0.2;
            AnimationSample {
                scalar: glow,
                vector: [glow, glow * 0.9, glow * 1.3],
                motor: Motor::identity(),
                secondary: dispersion,
                settled: false,
            }
        }
        "frosted_fade" => {
            let p = (t * 0.8).clamp(0.0, 1.0);
            let alpha = EasingCurve::SineInOut.eval(p);
            let trans = 0.85 * alpha;
            AnimationSample {
                scalar: alpha,
                vector: [trans, trans, trans],
                motor: Motor::identity(),
                secondary: 1.0 - alpha,
                settled: p >= 1.0,
            }
        }
        "focus_halo" => {
            let r = 50.0 + (t * 3.0).sin() * 15.0;
            let lum = 0.7 + (t * 3.0).cos() * 0.2;
            AnimationSample {
                scalar: r,
                vector: [r, r, lum],
                motor: Motor::identity(),
                secondary: lum,
                settled: false,
            }
        }
        _ => AnimationSample::default(),
    }
}
