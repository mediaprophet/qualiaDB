//! VibeScript Zero-Heap Animation System.
//!
//! Provides the complete 10-family animation taxonomy, analytical spring dynamics,
//! 3D Projective Geometric Algebra (PGA 𝒢_{3,0,1}) ScLERP motor interpolation,
//! SQUAD quaternion spline paths, and parametric curve evaluators.

pub mod curves;
pub mod families;
pub mod pga;
pub mod presets;
pub mod spring;
pub mod squad;

pub use curves::{CubicBezier, EasingCurve};
pub use pga::{Motor, MotorBivector};
pub use presets::{evaluate_preset, list_all_presets, AnimationFamily, AnimationSample, PresetInfo};
pub use spring::{SpringConfig, SpringState1D, SpringState3D};
pub use squad::Quat;
