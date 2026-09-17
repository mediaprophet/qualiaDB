//! Mesh & Topology Animation Family (Laplacian morphing, Origami unfold, Stalk focus).

use crate::animation::curves::EasingCurve;
use crate::animation::pga::Motor;
use crate::animation::presets::AnimationSample;

pub fn eval(preset: &str, t: f64) -> AnimationSample {
    match preset {
        "morph_laplacian" | "morph" => {
            let progress = (t * 0.5).clamp(0.0, 1.0);
            let factor = EasingCurve::QuintInOut.eval(progress);
            AnimationSample {
                scalar: factor,
                vector: [factor, factor, factor],
                motor: Motor::identity(),
                secondary: 1.0 - factor,
                settled: progress >= 1.0,
            }
        }
        "origami_unfold" | "unfold" => {
            let progress = (t * 0.4).clamp(0.0, 1.0);
            let fold_angle = (1.0 - progress) * std::f64::consts::PI * 0.5;
            let motor = Motor::from_rotation_translation(
                [(fold_angle * 0.5).cos(), (fold_angle * 0.5).sin(), 0.0, 0.0],
                [progress * 2.0, 0.0, 0.0],
            );
            AnimationSample {
                scalar: fold_angle,
                vector: [progress, 1.0 - progress, 0.0],
                motor,
                secondary: progress,
                settled: progress >= 1.0,
            }
        }
        "stalk_focus" => {
            let pulse = (t * 3.0).sin().abs();
            let focus_scale = 1.0 + pulse * 0.25;
            AnimationSample {
                scalar: focus_scale,
                vector: [focus_scale, focus_scale, 1.0],
                motor: Motor::identity(),
                secondary: pulse,
                settled: false,
            }
        }
        "topology_collapse" => {
            let progress = (t * 0.6).clamp(0.0, 1.0);
            let radius = (1.0 - progress).powi(3);
            AnimationSample {
                scalar: radius,
                vector: [radius, radius, radius],
                motor: Motor::identity(),
                secondary: 1.0 - progress,
                settled: progress >= 1.0,
            }
        }
        _ => AnimationSample::default(),
    }
}
