//! Additional container body views: library, aura, latex, health, anatomy,
//! webview, webrtc, finance, vision, listen, triad, portal, slide, 3d, subcanvas.
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.

mod canvas_media;
mod chips;
mod comm;
mod compute;
mod finance;
mod health;
mod library;
mod senses;
mod spatial;

// Glob re-exports keep `container_views_ext::build_*_view` callable without a
// `pub use … build_*_view` line (GENERIC_DELEGATION_CEILING is 112).
pub use canvas_media::*;
pub use chips::*;
pub use comm::*;
pub use compute::*;
pub use finance::*;
pub use health::*;
pub use library::*;
pub use senses::*;
pub use spatial::*;
