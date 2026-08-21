//! Scene graph — node hierarchy, lights, semantic links, duplication, render budget.
//!
//! Provides the build-new scene graph functionality that the render engine
//! lacks: light sources, node duplication, semantic links, render budget,
//! inverse kinematics (look-at + CCD), and smooth damp.

pub mod ik;
pub mod light;
pub mod node;
pub mod smooth_damp;

pub use ik::{ccd_ik, look_at_ik, IkResult};
pub use light::{Light, LightType};
pub use node::{duplicate_node, link_semantic, SceneGraph, SceneNode, SemanticLink};
pub use smooth_damp::{smooth_damp, smooth_damp_vec3};
