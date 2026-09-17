//! 10-Family Modular Animation Evaluator Router.

pub mod acoustic;
pub mod dynamics;
pub mod generative;
pub mod haptics;
pub mod hud;
pub mod mesh;
pub mod optics;
pub mod spatial;
pub mod thermo;
pub mod timeline;

use crate::animation::presets::{AnimationFamily, AnimationSample};

/// Route evaluation to the appropriate family submodule.
pub fn dispatch(family: AnimationFamily, preset: &str, t: f64) -> AnimationSample {
    match family {
        AnimationFamily::SpatialKinematics => spatial::eval(preset, t),
        AnimationFamily::PhysicalDynamics => dynamics::eval(preset, t),
        AnimationFamily::MeshTopology => mesh::eval(preset, t),
        AnimationFamily::ThermodynamicPhase => thermo::eval(preset, t),
        AnimationFamily::OpticsWaves => optics::eval(preset, t),
        AnimationFamily::AcousticSpectral => acoustic::eval(preset, t),
        AnimationFamily::MultiTrackTimelines => timeline::eval(preset, t),
        AnimationFamily::HudGlassUi => hud::eval(preset, t),
        AnimationFamily::OutboundHaptics => haptics::eval(preset, t),
        AnimationFamily::GenerativeFields => generative::eval(preset, t),
    }
}
