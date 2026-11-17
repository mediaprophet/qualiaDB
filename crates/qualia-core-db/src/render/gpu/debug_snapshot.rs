//! Observable portal GPU occupancy for the WebGL2 habitat-missing-assets hunt.
use super::PortalGpu;
use crate::render::instance_culling::GpuInstanceRecord;
use serde_json::{json, Value};

impl PortalGpu {
    /// CPU-side occupancy of the live sky / water / mesh path. This is admission
    /// state, not proof that WebGL2 presented the texels.
    pub fn debug_scene_snapshot(&self) -> Value {
        let adapter = self.gpu_timing_adapter_info();
        let backend = adapter.map(|info| format!("{:?}", info.backend));
        let (groups, source, visible, authored_triangles) = self.mesh_instance_group_counts();
        let stride = std::mem::size_of::<GpuInstanceRecord>() as u64;
        let mut instance_origins = [[0.0f32; 3]; 4];
        let origin_count = self.mesh_instance_source.len().min(4);
        for i in 0..origin_count {
            let record = &self.mesh_instance_source[i];
            instance_origins[i] = [
                record.world_from_local[3][0],
                record.world_from_local[3][1],
                record.world_from_local[3][2],
            ];
        }
        let mesh = self.mesh.as_ref().map(|mesh| {
            let draws = mesh.material_gpu.draws.len();
            let missing_maps = mesh
                .material_gpu
                .draws
                .iter()
                .filter(|draw| draw.missing_material_maps)
                .count();
            let blend_draws = mesh
                .material_gpu
                .draws
                .iter()
                .filter(|draw| draw.opacity_mode == crate::container_10d::OpacityMode::Blend)
                .count();
            json!({
                "present": true,
                "cpu_vertices": mesh.cpu_positions.len(),
                "cpu_indices": mesh.cpu_indices.len(),
                "cpu_triangles": mesh.cpu_indices.len() / 3,
                "material_bind_groups": mesh.material_gpu.material_bind_groups.len(),
                "draws": draws,
                "blend_draws": blend_draws,
                "missing_material_maps": missing_maps,
                "transparent_draw_order": mesh.material_gpu.transparent_draw_order.len(),
            })
        });
        json!({
            "schema": "webizen.portal-scene-debug/v1",
            "backend": backend,
            "adapter_name": adapter.map(|info| info.name.clone()),
            "hdr_scene_available": self.hdr_scene_available(),
            "sky_enabled": self.sky_enabled,
            "sky_pipeline_hdr": self.sky_pipeline_hdr.is_some(),
            "mesh_pipeline_hdr": self.mesh_pipeline_hdr.is_some(),
            "ambient_enabled": self.ambient_enabled,
            "tensor_projection_enabled": self.tensor_projection_enabled,
            "mesh": mesh.unwrap_or_else(|| json!({"present": false})),
            "water": self.water.debug_snapshot(),
            "resident_textures": self.resident_textures.len(),
            "instances": {
                "stride_bytes": stride,
                "groups": groups,
                "source": source,
                "visible": visible,
                "authored_triangles": authored_triangles,
                "submitted_triangles": self.mesh_submitted_triangle_count(),
                "source_packed_bytes": source as u64 * stride,
                "visible_packed_bytes": visible as u64 * stride,
                "first_origins": &instance_origins[..origin_count],
            },
            "colour_path": {
                "habitat_mesh": "MAT1 factors plus hydrated PNG/JPEG/KTX2 via load_texture_encoded; scene colour is a neutral [1,1,1,1] multiplier",
                "native_field": "EMF [alpha, mu, sigma] converted by spectrum-area (CIE CMF x SPD) to display RGBA",
                "water": "WaterUniform.colour_alpha",
                "sky": "procedural sky pipeline, camera uniform, no mesh_instances storage",
            },
        })
    }
}
