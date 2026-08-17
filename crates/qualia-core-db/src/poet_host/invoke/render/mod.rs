//! Future seam: `webizen-render` scene contract (no wgpu in the engine).
//!
//! Vibe authors a `RenderScene` description. The desktop host presents it
//! through `webizen-render` (PNG or native swapchain). HTML/GPU canvas stays
//! canvas (D13).

mod scene;

pub use scene::scene;
