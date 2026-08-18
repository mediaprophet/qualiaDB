//! Future seam: `webizen-render` scene contract (no wgpu in the engine).
//!
//! Vibe authors a `RenderScene` description. The desktop host presents it
//! through `webizen-render` (PNG or native swapchain). HTML/GPU canvas stays
//! canvas (D13).

mod css;
mod scene;
pub mod spectral;
mod svg;

pub use css::{css_animation, css_color, css_transform};
pub use scene::scene;
pub use svg::{svg_bezier, svg_circle, svg_field, svg_line, svg_path, svg_rect};
