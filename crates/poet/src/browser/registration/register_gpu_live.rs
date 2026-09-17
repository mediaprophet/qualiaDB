//! Render GPU Live remainder — already-bound `Render.gpu_*` dual-path tools.

use super::*;

fn gpu_live_tool(
    id: &'static str,
    label: &'static str,
    scope: &'static str,
    description: &'static str,
) -> Box<dyn crate::tool_chest::core::tool::Tool> {
    Box::new(SimpleTool::new(
        ToolMetadata {
            id: id.into(),
            label: label.into(),
            icon: "media".into(),
            kind: ToolKind::RunAction,
            capability_scope: Some(scope.into()),
            ontology_prefix: "hm".into(),
            description: description.into(),
        },
        ActionType::Invoke,
    ))
}

pub(super) fn gpu_live_tools() -> Vec<Box<dyn crate::tool_chest::core::tool::Tool>> {
    vec![
        gpu_live_tool("render:gpu_live_init", "GPU init", "Render.gpu_init", "Create an offscreen PortalGpu via Render.gpu_init."),
        gpu_live_tool("render:gpu_live_init_surface", "GPU init surface", "Render.gpu_init_surface", "Swapchain PortalGpu via Render.gpu_init_surface."),
        gpu_live_tool("render:gpu_live_render_frame", "Render frame", "Render.gpu_render_frame", "Render one frame via Render.gpu_render_frame (data-gpu-handle)."),
        gpu_live_tool("render:gpu_live_read_pixels", "Read pixels", "Render.gpu_read_pixels", "Read RGBA8 via Render.gpu_read_pixels."),
        gpu_live_tool("render:gpu_live_upload_mesh", "Upload mesh", "Render.gpu_upload_mesh", "Upload a triangle mesh via Render.gpu_upload_mesh."),
        gpu_live_tool("render:gpu_live_upload_tensor", "Upload tensor", "Render.gpu_upload_tensor", "Upload tensor bytes via Render.gpu_upload_tensor."),
        gpu_live_tool("render:gpu_live_pick", "Pick", "Render.gpu_pick", "Queue a pick via Render.gpu_pick."),
        gpu_live_tool("render:gpu_live_poll_pick", "Poll pick", "Render.gpu_poll_pick", "Poll pick via Render.gpu_poll_pick."),
        gpu_live_tool("render:gpu_live_resize", "Resize", "Render.gpu_resize", "Resize viewport via Render.gpu_resize."),
        gpu_live_tool("render:gpu_live_set_ambient", "Set ambient", "Render.gpu_set_ambient", "Toggle ambient field via Render.gpu_set_ambient."),
        gpu_live_tool("render:gpu_live_destroy", "Destroy", "Render.gpu_destroy", "Destroy PortalGpu via Render.gpu_destroy."),
        gpu_live_tool("render:gpu_live_compute_dispatch", "Compute dispatch", "Render.gpu_compute_dispatch", "Dispatch WGSL compute via Render.gpu_compute_dispatch."),
        gpu_live_tool("render:gpu_live_compute_readback", "Compute readback", "Render.gpu_compute_readback", "Read compute results via Render.gpu_compute_readback."),
        gpu_live_tool("render:gpu_live_validate_shader", "Validate shader", "Render.gpu_validate_shader", "Validate WGSL via Render.gpu_validate_shader."),
        gpu_live_tool("render:gpu_live_compile_shader", "Compile shader", "Render.gpu_compile_shader", "Compile WGSL via Render.gpu_compile_shader."),
        gpu_live_tool("render:gpu_live_compile_to_glsl", "Compile to GLSL", "Render.gpu_compile_to_glsl", "Cross-compile WGSL via Render.gpu_compile_to_glsl."),
        gpu_live_tool("render:gpu_live_backend_info", "Backend info", "Render.gpu_backend_info", "Probe GPU backend via Render.gpu_backend_info."),
        gpu_live_tool("render:gpu_live_upload_mesh_colored", "Upload mesh colored", "Render.gpu_upload_mesh_colored", "Upload a colored triangle mesh via Render.gpu_upload_mesh_colored."),
        gpu_live_tool("render:gpu_live_set_standpoint", "Set standpoint", "Render.gpu_set_standpoint", "Set observer standpoint via Render.gpu_set_standpoint."),
        gpu_live_tool("render:gpu_live_observer_standpoint", "Observer standpoint", "Render.gpu_observer_standpoint", "Read standpoint via Render.gpu_observer_standpoint."),
        gpu_live_tool("render:gpu_live_camera_state", "Camera state", "Render.gpu_camera_state", "Read yaw/pitch/zoom via Render.gpu_camera_state."),
        gpu_live_tool("render:gpu_live_surface_size", "Surface size", "Render.gpu_surface_size", "Read extent via Render.gpu_surface_size."),
        gpu_live_tool("render:gpu_live_has_mesh", "Has mesh", "Render.gpu_has_mesh", "Query mesh via Render.gpu_has_mesh."),
        gpu_live_tool("render:gpu_live_has_tensor", "Has tensor", "Render.gpu_has_tensor", "Query tensor via Render.gpu_has_tensor."),
        gpu_live_tool("render:gpu_live_tensor_node_count", "Tensor node count", "Render.gpu_tensor_node_count", "Count tensor nodes via Render.gpu_tensor_node_count."),
        gpu_live_tool("render:gpu_live_particle_count", "Particle count", "Render.gpu_particle_count", "Count particles via Render.gpu_particle_count."),
        gpu_live_tool("render:gpu_live_sync_bloom", "Sync bloom", "Render.gpu_sync_bloom", "Reconcile bloom targets via Render.gpu_sync_bloom."),
        gpu_live_tool("render:gpu_live_set_artefact_joint", "Set artefact joint", "Render.gpu_set_artefact_joint", "Set kinematic joint via Render.gpu_set_artefact_joint."),
        gpu_live_tool("render:gpu_live_set_artefact_world", "Set artefact world", "Render.gpu_set_artefact_world", "Set world AABB via Render.gpu_set_artefact_world."),
        gpu_live_tool("render:gpu_live_artefact_refused", "Artefact refused", "Render.gpu_artefact_refused", "Query refused pose via Render.gpu_artefact_refused."),
        gpu_live_tool("render:gpu_live_required_rgba8_bytes", "Required RGBA8 bytes", "Render.gpu_required_rgba8_bytes", "Readback size via Render.gpu_required_rgba8_bytes."),
        gpu_live_tool("render:gpu_live_emf_upload_field", "EMF upload field", "Render.emf_upload_field", "Upload EMF grid via Render.emf_upload_field."),
        gpu_live_tool("render:gpu_live_emf_render_slice", "EMF render slice", "Render.emf_render_slice", "Render EMF slice via Render.emf_render_slice."),
        gpu_live_tool("render:gpu_live_emf_field_info", "EMF field info", "Render.emf_field_info", "Query EMF field via Render.emf_field_info."),
    ]
}
