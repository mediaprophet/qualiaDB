//! Cold mesh upload, validation, material admission and GPU buffer construction.

use super::*;
use crate::container_10d::{MaterialRecord, SubmeshRange};
use crate::render::gpu::materials::create_material_gpu;

const IDENTITY_MAT4: [[f32; 4]; 4] = [
    [1.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0],
    [0.0, 0.0, 0.0, 1.0],
];

impl PortalGpu {
    /// Replace the shared mesh instance source stream. Camera-facing forward and AO passes are
    /// culled automatically from the active camera; the full source stream remains available to
    /// shadow cascades so off-camera casters are not dropped. Empty input restores the legacy
    /// single identity instance. Transparent multi-instance draws are refused until per-instance
    /// back-to-front ordering is available.
    pub fn set_mesh_instances(
        &mut self,
        records: &[crate::render::instance_culling::GpuInstanceRecord],
    ) -> Result<(), MeshInstanceUploadError> {
        self.set_mesh_instances_packed(bytemuck::cast_slice(records))
    }

    /// Replace instances from the stable byte ABI without requiring caller-pointer alignment.
    pub fn set_mesh_instances_packed(
        &mut self,
        packed_records: &[u8],
    ) -> Result<(), MeshInstanceUploadError> {
        let record_size = std::mem::size_of::<crate::render::instance_culling::GpuInstanceRecord>();
        if packed_records.len() % record_size != 0 {
            return Err(MeshInstanceUploadError::InvalidTransform { index: 0 });
        }
        let record_count = packed_records.len() / record_size;
        if record_count > 1
            && self.mesh.as_ref().is_some_and(|mesh| {
                mesh.material_gpu
                    .draws
                    .iter()
                    .any(|draw| draw.opacity_mode == crate::container_10d::OpacityMode::Blend)
            })
        {
            return Err(MeshInstanceUploadError::TransparentInstancesRequireSortedSubmission);
        }
        self.visible_mesh_instances.replace_packed(
            &self.device,
            &self.queue,
            &self.mesh_instance_layout,
            packed_records,
        )?;
        if let Err(error) = self.mesh_instances.replace_packed(
            &self.device,
            &self.queue,
            &self.mesh_instance_layout,
            packed_records,
        ) {
            let previous = bytemuck::cast_slice(&self.mesh_instance_source);
            let _ = self.visible_mesh_instances.replace_packed(
                &self.device,
                &self.queue,
                &self.mesh_instance_layout,
                previous,
            );
            return Err(error);
        }
        self.mesh_instance_source.clear();
        if packed_records.is_empty() {
            self.mesh_instance_source.push(
                crate::render::instance_culling::GpuInstanceRecord::new(IDENTITY_MAT4, 0),
            );
        } else {
            let record_size =
                std::mem::size_of::<crate::render::instance_culling::GpuInstanceRecord>();
            for bytes in packed_records.chunks_exact(record_size) {
                self.mesh_instance_source
                    .push(bytemuck::pod_read_unaligned(bytes));
            }
        }
        self.last_visibility_key = None;
        self.shadow_map_dirty = true;
        Ok(())
    }

    /// Upload an imported triangle mesh (Phase 1.2). `positions` are model-space `f32x3` (the caller
    /// centres + scales them to the orbit frame); `indices` is a flat triangle list (`tris * 3`).
    /// Returns the triangle count; clears any prior mesh when empty.
    pub fn upload_mesh(&mut self, positions: &[[f32; 3]], indices: &[u32]) -> u32 {
        self.upload_mesh_colored(positions, &[], indices)
    }

    /// Upload a triangle mesh with per-vertex linear RGBA colours. When `colors` is empty the
    /// engine's neutral blue-grey material is used; any non-empty slice must match `positions`.
    pub fn upload_mesh_colored(
        &mut self,
        positions: &[[f32; 3]],
        colors: &[[f32; 4]],
        indices: &[u32],
    ) -> u32 {
        self.upload_mesh_colored_with_normals(positions, colors, None, indices)
    }

    /// Upload a mesh while preserving a valid authored normal stream when supplied. The generated
    /// workspace remains resident for later deformation updates, where normals are recomputed from
    /// the changed geometry.
    pub fn upload_mesh_colored_with_normals(
        &mut self,
        positions: &[[f32; 3]],
        colors: &[[f32; 4]],
        authored_normals: Option<&[[f32; 3]]>,
        indices: &[u32],
    ) -> u32 {
        self.upload_mesh_colored_with_frames(positions, colors, authored_normals, None, indices)
    }

    /// Upload a mesh with optional authored normal and tangent frame streams.
    pub fn upload_mesh_colored_with_frames(
        &mut self,
        positions: &[[f32; 3]],
        colors: &[[f32; 4]],
        authored_normals: Option<&[[f32; 3]]>,
        authored_tangents: Option<&[[f32; 4]]>,
        indices: &[u32],
    ) -> u32 {
        let Ok(index_count) = u32::try_from(indices.len()) else {
            return 0;
        };
        let material = MaterialRecord::legacy_default();
        let range = SubmeshRange {
            first_index: 0,
            index_count,
            material_id: material.id,
            semantic_id: 0,
        };
        self.upload_mesh_colored_with_frames_and_materials(
            positions,
            colors,
            authored_normals,
            authored_tangents,
            indices,
            &[material],
            &[range],
        )
        .unwrap_or(0)
    }

    /// Upload a triangle mesh with per-submesh MAT1 shading and validated alpha coverage.
    pub fn upload_mesh_colored_with_frames_and_materials(
        &mut self,
        positions: &[[f32; 3]],
        colors: &[[f32; 4]],
        authored_normals: Option<&[[f32; 3]]>,
        authored_tangents: Option<&[[f32; 4]]>,
        indices: &[u32],
        materials: &[MaterialRecord],
        ranges: &[SubmeshRange],
    ) -> Result<u32, String> {
        self.upload_mesh_colored_with_frames_and_materials_and_uv0(
            positions,
            colors,
            authored_normals,
            authored_tangents,
            None,
            indices,
            materials,
            ranges,
        )
    }

    /// Upload a mesh with optional vertex-aligned glTF TEXCOORD_0 values.
    pub fn upload_mesh_colored_with_frames_and_materials_and_uv0(
        &mut self,
        positions: &[[f32; 3]],
        colors: &[[f32; 4]],
        authored_normals: Option<&[[f32; 3]]>,
        authored_tangents: Option<&[[f32; 4]]>,
        texture_coordinates_0: Option<&[[f32; 2]]>,
        indices: &[u32],
        materials: &[MaterialRecord],
        ranges: &[SubmeshRange],
    ) -> Result<u32, String> {
        self.last_visibility_key = None;
        if positions.is_empty() || indices.len() < 3 {
            self.mesh = None;
            self.mesh_reservation = None;
            self.shadow_map_dirty = true;
            return Ok(0);
        }
        if positions.len() > u32::MAX as usize || indices.len() > u32::MAX as usize {
            return Err("mesh vertex or index count exceeds u32".to_string());
        }
        if !colors.is_empty() && colors.len() != positions.len() {
            return Err("mesh colour count does not match vertex count".to_string());
        }
        if materials.len() > crate::container_10d::MAX_MATERIALS
            || ranges.len() > materials::MAX_MATERIAL_DRAWS
        {
            return Err("mesh material or draw count exceeds renderer limits".to_string());
        }
        if self.mesh_instances.count() > 1
            && materials
                .iter()
                .any(|material| material.opacity_mode == crate::container_10d::OpacityMode::Blend)
        {
            return Err("transparent meshes require sorted per-instance submission".to_string());
        }
        if authored_normals.is_some_and(|normals| {
            normals.len() != positions.len()
                || normals.iter().flatten().any(|value| !value.is_finite())
                || normals.iter().any(|n| {
                    let length2 = n[0] * n[0] + n[1] * n[1] + n[2] * n[2];
                    !length2.is_finite() || length2 <= f32::EPSILON
                })
        }) {
            return Err("mesh authored normal stream is invalid".to_string());
        }
        if authored_tangents.is_some_and(|tangents| {
            tangents.len() != positions.len()
                || tangents.iter().flatten().any(|value| !value.is_finite())
                || tangents.iter().any(|t| {
                    let direction2 = t[0] * t[0] + t[1] * t[1] + t[2] * t[2];
                    !direction2.is_finite()
                        || direction2 <= f32::EPSILON
                        || (t[3].abs() - 1.0).abs() > 1e-3
                })
        }) {
            return Err("mesh authored tangent stream is invalid".to_string());
        }
        if texture_coordinates_0.is_some_and(|coordinates| {
            coordinates.len() != positions.len()
                || coordinates
                    .iter()
                    .flatten()
                    .any(|value| !value.is_finite() || value.abs() > 1_000_000.0)
        }) {
            return Err("mesh TEXCOORD_0 stream is invalid".to_string());
        }
        let normal_workspace = mesh_normals::MeshNormalWorkspace::new(positions, indices)
            .map_err(|error| error.to_string())?;
        let index_count =
            u32::try_from(indices.len()).map_err(|_| "mesh index count exceeds u32".to_string())?;
        crate::container_10d::validate_material_bindings(materials, ranges, index_count)
            .map_err(|error| format!("material ranges are invalid: {error}"))?;
        let material_bytes = (materials.len() as u64)
            .checked_mul(materials::MATERIAL_UNIFORM_STRIDE)
            .ok_or_else(|| "mesh material-uniform size overflow".to_string())?;
        let position_bytes = (positions.len() as u64).checked_mul(12);
        let normal_bytes = (positions.len() as u64).checked_mul(12);
        let tangent_bytes = (positions.len() as u64).checked_mul(16);
        let color_bytes = (positions.len() as u64).checked_mul(16);
        let uv_bytes = (positions.len() as u64).checked_mul(8);
        let index_bytes = (indices.len() as u64).checked_mul(4);
        let Some(mesh_bytes) = position_bytes
            .zip(normal_bytes)
            .zip(tangent_bytes)
            .zip(color_bytes)
            .zip(index_bytes)
            .zip(uv_bytes)
            .and_then(
                |(((((positions, normals), tangents), colors), indices), uvs)| {
                    positions
                        .checked_add(normals)?
                        .checked_add(tangents)?
                        .checked_add(colors)?
                        .checked_add(indices)?
                        .checked_add(material_bytes)
                        .and_then(|bytes| bytes.checked_add(uvs))
                },
            )
        else {
            return Err("mesh GPU byte size overflow".to_string());
        };
        // Reserve the peak before allocating GPU buffers. The old mesh remains
        // alive until the replacement has been constructed and installed.
        let Ok(mesh_reservation) = global_vram_ledger()
            .try_reserve_graphics(crate::gpu_context::VramResourceClass::Geometry, mesh_bytes)
        else {
            return Err("mesh GPU geometry reservation refused".to_string());
        };
        let material_gpu = create_material_gpu(
            &self.device,
            &self.mesh_material_layout,
            &self.mesh_texture_layout,
            &self.material_texture_defaults,
            &self.resident_textures,
            materials,
            ranges,
            positions,
            indices,
            index_count,
        )?;
        let shadow_draws = material_gpu
            .draws
            .iter()
            .copied()
            .filter(|draw| {
                materials[draw.material_index].casts_shadow
                    && draw.opacity_mode != crate::container_10d::OpacityMode::Blend
            })
            .collect();
        let receives_shadows = materials.iter().any(|material| material.receives_shadow);
        let default_colors;
        let colors = if colors.is_empty() {
            default_colors = vec![[0.50, 0.60, 0.82, 1.0]; positions.len()];
            default_colors.as_slice()
        } else {
            colors
        };
        let vertex_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-mesh-verts"),
                contents: bytemuck::cast_slice(positions),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let color_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-mesh-colors"),
                contents: bytemuck::cast_slice(colors),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let normal_data = authored_normals.unwrap_or_else(|| normal_workspace.normals());
        let normal_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-mesh-normals"),
                contents: bytemuck::cast_slice(normal_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let generated_tangents;
        let tangent_data = if let Some(tangents) = authored_tangents {
            tangents
        } else {
            generated_tangents = normal_data
                .iter()
                .copied()
                .map(default_gpu_tangent)
                .collect::<Vec<_>>();
            generated_tangents.as_slice()
        };
        let tangent_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-mesh-tangents"),
                contents: bytemuck::cast_slice(tangent_data),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let index_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-mesh-indices"),
                contents: bytemuck::cast_slice(indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            });
        let default_uvs;
        let texture_coordinates_0 = if let Some(coordinates) = texture_coordinates_0 {
            coordinates
        } else {
            default_uvs = vec![[0.0, 0.0]; positions.len()];
            default_uvs.as_slice()
        };
        let uv_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-mesh-uv0"),
                contents: bytemuck::cast_slice(texture_coordinates_0),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            });
        let index_count = indices.len() as u32;
        self.mesh = Some(MeshGpu {
            vertex_buf,
            color_buf,
            normal_buf,
            tangent_buf,
            uv_buf,
            index_buf,
            vertex_count: positions.len() as u32,
            material_gpu,
            shadow_draws,
            receives_shadows,
            cpu_positions: positions.to_vec(),
            cpu_indices: indices.to_vec(),
            normal_workspace,
        });
        self.mesh_reservation = Some(mesh_reservation);
        self.mesh_base_aabb = Aabb::from_points(positions); // for Phase 2 admission
        self.last_visibility_key = None;
        self.shadow_map_dirty = true;
        self.last_admitted = Motor::identity();
        self.last_refused = false;
        Ok(index_count / 3)
    }
}
