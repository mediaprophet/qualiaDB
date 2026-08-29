//! Outbound Sensory Haptics Animation Family (Vibro clicks, Heartbeats, Kinesthetic resistance).

use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
        "vibro_click" | "click" => {
            let dur = 0.030; // 30ms click
            let intensity = if t <= dur {
                (1.0 - t / dur) * (t * 1000.0).sin()
            } else {
                0.0
            };
            AnimationSample {
                scalar: intensity,
                vector: [intensity, 0.0, 0.0],
                motor: Motor::identity(),
                secondary: intensity.abs(),
                settled: t > dur,
            }
        }
        "heartbeat_pulse" | "heartbeat" => {
            let cycle = t % 0.85; // ~70 BPM
            let s1 = if cycle < 0.12 {
                (cycle / 0.12 * std::f64::consts::PI).sin()
            } else {
                0.0
            };
            let s2 = if cycle > 0.22 && cycle < 0.34 {
                ((cycle - 0.22) / 0.12 * std::f64::consts::PI).sin() * 0.6
            } else {
                0.0
            };
            let amp = s1 + s2;
            AnimationSample {
                scalar: amp,
                vector: [amp, amp * 0.5, 0.0],
                motor: Motor::identity(),
                secondary: cycle,
                settled: false,
            }
        }
        "kinesthetic_resist" | "resist" => {
            let v = (t * 5.0).sin();
            let torque = -1.5 * v.signum() * v.abs().powf(1.8);
            AnimationSample {
                scalar: torque,
                vector: [torque, 0.0, 0.0],
                motor: Motor::identity(),
                secondary: v,
                settled: false,
            }
        }
        _ => AnimationSample::default(),
    }
}
