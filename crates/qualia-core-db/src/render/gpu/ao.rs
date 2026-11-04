//! Half-resolution, depth/normal screen-space ambient visibility.
//!
//! This portable baseline borrows CACAO's adaptive-resolution strategy but keeps a bounded
//! single-ring kernel suitable for WebGPU/WASM. It is an indirect-light approximation, never a
//! replacement for sun shadows. All targets are explicitly budgeted and optional.

use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};

#[path = "ao_depth.rs"]
mod depth;
#[path = "ao_math.rs"]
mod math;
#[cfg(test)]
pub(super) use depth::{decode_depth, encode_depth};
pub(super) use etalon_render::ao::AoUniform;
pub(super) use math::make_uniform;

pub(super) struct AoTargets(etalon_render::ao::AoTargets<VramReservation<'static>>);
impl std::ops::Deref for AoTargets {
    type Target = etalon_render::ao::AoTargets<VramReservation<'static>>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl AoTargets {
    pub fn extent(&self) -> (u32, u32) {
        self.0.extent()
    }

    pub fn try_new(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        camera_layout: &wgpu::BindGroupLayout,
        model_layout: &wgpu::BindGroupLayout,
        instance_layout: &wgpu::BindGroupLayout,
        material_layout: &wgpu::BindGroupLayout,
        texture_layout: &wgpu::BindGroupLayout,
        uniform_buf: &wgpu::Buffer,
    ) -> Option<Self> {
        etalon_render::ao::AoTargets::try_new(
            device,
            width,
            height,
            camera_layout,
            model_layout,
            instance_layout,
            material_layout,
            texture_layout,
            uniform_buf,
            cfg!(all(test, not(target_arch = "wasm32"))),
            |bytes| {
                global_vram_ledger()
                    .try_reserve_graphics(VramResourceClass::FrameTarget, bytes)
                    .ok()
            },
        )
        .map(Self)
    }
    pub fn record(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        mesh: &super::MeshGpu,
        camera_bind: &wgpu::BindGroup,
        model_bind: &wgpu::BindGroup,
        instance_bind: &wgpu::BindGroup,
        instance_count: u32,
    ) {
        self.0.record(encoder, |prepass| {
            prepass.set_bind_group(0, camera_bind, &[]);
            prepass.set_bind_group(1, model_bind, &[]);
            prepass.set_bind_group(3, instance_bind, &[]);
            prepass.set_vertex_buffer(0, mesh.vertex_buf.slice(..));
            prepass.set_vertex_buffer(1, mesh.normal_buf.slice(..));
            prepass.set_vertex_buffer(2, mesh.color_buf.slice(..));
            prepass.set_vertex_buffer(3, mesh.uv_buf.slice(..));
            prepass.set_index_buffer(mesh.index_buf.slice(..), wgpu::IndexFormat::Uint32);
            for draw in &mesh.material_gpu.draws {
                if draw.opacity_mode == crate::container_10d::OpacityMode::Blend {
                    continue;
                }
                prepass.set_bind_group(
                    2,
                    &mesh.material_gpu.material_bind_groups[draw.material_index],
                    &[draw.material_offset],
                );
                prepass.draw_indexed(
                    draw.first_index..draw.first_index + draw.index_count,
                    0,
                    0..instance_count,
                );
            }
        });
    }
}
