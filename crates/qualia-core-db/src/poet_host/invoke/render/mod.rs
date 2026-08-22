//! Future seam: `webizen-render` scene contract (no wgpu in the engine).
//!
//! Vibe authors a `RenderScene` description. The desktop host presents it
//! through `webizen-render` (PNG or native swapchain). HTML/GPU canvas stays
//! canvas (D13).

mod animation;
mod backend;
mod css;
mod emf_visualizer;
mod gpu;
mod gpu_compute;
mod gpu_state;
mod scene;
mod scene_graph;
mod shader_compile;
pub mod spectral;
mod svg;

pub use animation::{
    animation_eval_curve, animation_eval_preset, animation_list_presets, animation_sclerp,
    animation_spring_step, animation_squad_step,
};
pub use backend::gpu_backend_info;
pub use css::{css_animation, css_color, css_transform};
pub use emf_visualizer::{emf_field_info, emf_render_slice, emf_upload_field};
pub use gpu::{
    gpu_adapter_info, gpu_destroy, gpu_init, gpu_init_surface, gpu_pick, gpu_poll_pick,
    gpu_read_pixels, gpu_render_frame, gpu_resize, gpu_set_ambient, gpu_set_camera,
    gpu_upload_mesh, gpu_upload_tensor,
};
pub use gpu_state::{
    gpu_artefact_refused, gpu_camera_state, gpu_has_mesh, gpu_has_tensor, gpu_observer_standpoint,
    gpu_particle_count, gpu_required_rgba8_bytes, gpu_set_artefact_joint, gpu_set_artefact_world,
    gpu_set_standpoint, gpu_surface_size, gpu_sync_bloom, gpu_tensor_node_count,
    gpu_upload_mesh_colored,
};
pub use gpu_compute::{animation_compute_pass, gpu_compute_dispatch, gpu_compute_readback};
pub use scene::scene;
pub use scene_graph::{
    scene_add_camera, scene_add_light, scene_add_node, scene_capture_frame, scene_create,
    scene_duplicate_node, scene_ik_ccd, scene_ik_look_at, scene_link_semantic, scene_render,
    scene_set_clear_colour, scene_set_mesh, scene_set_render_budget, scene_set_transform,
    scene_set_viewport, scene_smooth_damp, scene_smooth_damp_vec3,
};
pub use shader_compile::{gpu_compile_shader, gpu_compile_to_glsl, gpu_validate_shader};
pub use svg::{svg_bezier, svg_circle, svg_field, svg_line, svg_path, svg_rect};
