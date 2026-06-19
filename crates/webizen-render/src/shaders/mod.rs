//! WGSL shaders for Webizen Renderer
//!
//! Contains vertex and fragment shaders for PGA-based rendering and
//! ambient knowledge visualization.

pub const PROJECTOR_WGSL: &str = include_str!("projector.wgsl");
pub const EPISTEMIC_WGSL: &str = include_str!("epistemic.wgsl");
pub const AMBIENT_WGSL: &str = include_str!("ambient.wgsl");
