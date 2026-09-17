//! Multi-Track Timelines Animation Family (Polymetric phase, Spline keyframes, Cascades).

use crate::animation::curves::EasingCurve;
use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
        "polymetric_sync" => {
            let p3 = (t * 3.0).sin();
            let p4 = (t * 4.0).sin();
            let p5 = (t * 5.0).sin();
            let composite = (p3 + p4 + p5) / 3.0;
            AnimationSample {
                scalar: composite,
                vector: [p3, p4, p5],
                motor: Motor::identity(),
                secondary: p3 * p5,
                settled: false,
            }
        }
        "keyframe_blend" => {
            let cycle = (t * 0.5) % 3.0;
            let idx = cycle.floor() as usize;
            let frac = cycle - cycle.floor();
            let ease = EasingCurve::CubicInOut.eval(frac);
            let val = (idx as f64) + ease;
            AnimationSample {
                scalar: val,
                vector: [val, ease, 1.0 - ease],
                motor: Motor::identity(),
                secondary: frac,
                settled: false,
            }
        }
        "stagger_cascade" => {
            let n = 5;
            let stagger = 0.15;
            let mut sum = 0.0;
            for i in 0..n {
                let delay = i as f64 * stagger;
                let active = (t - delay).clamp(0.0, 1.0);
                sum += EasingCurve::BackOut.eval(active);
            }
            let avg = sum / n as f64;
            AnimationSample {
                scalar: avg,
                vector: [avg, sum, 0.0],
                motor: Motor::identity(),
                secondary: sum,
                settled: t >= (n as f64 * stagger + 1.0),
            }
        }
        _ => AnimationSample::default(),
    }
}
