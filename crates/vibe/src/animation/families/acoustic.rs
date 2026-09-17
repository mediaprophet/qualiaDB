//! Acoustic Spectral Animation Family (ADSR envelopes, Binaural sweeps, Formants).

use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
        "adsr_pulse" | "adsr" => {
            let (a, d, s, r) = (0.1, 0.2, 0.6, 0.4);
            let total = a + d + 0.5 + r;
            let cycle = t % total;
            let amp = if cycle < a {
                cycle / a
            } else if cycle < a + d {
                1.0 - (1.0 - s) * ((cycle - a) / d)
            } else if cycle < a + d + 0.5 {
                s
            } else {
                s * (1.0 - (cycle - a - d - 0.5) / r)
            };
            AnimationSample {
                scalar: amp,
                vector: [amp, amp, amp],
                motor: Motor::identity(),
                secondary: cycle,
                settled: false,
            }
        }
        "binaural_sweep" => {
            let pan = (t * 1.5).sin(); // -1.0 to 1.0
            let left = ((1.0 - pan) * 0.5).sqrt();
            let right = ((1.0 + pan) * 0.5).sqrt();
            AnimationSample {
                scalar: pan,
                vector: [left, 0.0, right],
                motor: Motor::from_translation(pan * 2.0, 0.0, 0.0),
                secondary: pan.abs(),
                settled: false,
            }
        }
        "resonant_formant" => {
            let f1 = 500.0 + (t * 2.0).sin() * 300.0;
            let f2 = 1500.0 + (t * 3.0).cos() * 500.0;
            AnimationSample {
                scalar: f1,
                vector: [f1 / 1000.0, f2 / 2000.0, 0.0],
                motor: Motor::identity(),
                secondary: f2,
                settled: false,
            }
        }
        _ => AnimationSample::default(),
    }
}
