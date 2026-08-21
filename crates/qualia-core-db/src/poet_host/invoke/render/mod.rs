//! Future seam: `webizen-render` scene contract (no wgpu in the engine).
//!
//! Vibe authors a `RenderScene` description. The desktop host presents it
//! through `webizen-render` (PNG or native swapchain). HTML/GPU canvas stays
//! canvas (D13).

mod backend;
mod css;
mod emf_visualizer;
mod gpu;
mod gpu_compute;
mod scene;
mod scene_graph;
mod shader_compile;
pub mod spectral;
mod svg;

pub use backend::gpu_backend_info;
pub use css::{css_animation, css_color, css_transform};
pub use emf_visualizer::{emf_field_info, emf_render_slice, emf_upload_field};
pub use gpu::{
    gpu_adapter_info, gpu_destroy, gpu_init, gpu_pick, gpu_poll_pick, gpu_read_pixels,
    gpu_render_frame, gpu_resize, gpu_set_ambient, gpu_set_camera, gpu_upload_mesh,
    gpu_upload_tensor,
};
pub use gpu_compute::{gpu_compute_dispatch, gpu_compute_readback};
pub use scene::scene;
pub use scene_graph::{
    scene_add_camera, scene_add_node, scene_capture_frame, scene_create, scene_render,
    scene_set_clear_colour, scene_set_mesh, scene_set_transform, scene_set_viewport,
};
pub use shader_compile::{gpu_compile_shader, gpu_compile_to_glsl, gpu_validate_shader};
pub use svg::{svg_bezier, svg_circle, svg_field, svg_line, svg_path, svg_rect};
