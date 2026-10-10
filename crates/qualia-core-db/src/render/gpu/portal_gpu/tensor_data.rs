//! Tensor buffer ingestion and GPU particle field synchronization.

use super::super::{
    make_ambient_bind_group, make_projector_tensor_bind_group, particles_from_tensor, PortalGpu,
    MAX_AMBIENT_INSTANCES,
};
use crate::gpu_context::global_vram_ledger;
use crate::render::telemetry::ParticleInstance;
use crate::tensor::buffer_export::TENSOR_HEADER_BYTES;
use wgpu::util::DeviceExt;

impl PortalGpu {
    pub fn upload_tensor_buffer(&mut self, bytes: &[u8]) -> Result<u32, String> {
        let (header, _) =
            crate::tensor::buffer_export::parse_header(bytes).map_err(|e| e.to_string())?;
        let count = header.node_count;
        if count == 0 {
            return Ok(0);
        }

        let particles = particles_from_tensor(bytes, MAX_AMBIENT_INSTANCES)?;
        let instance_count = particles.len() as u32;

        // Admit the replacement as a peak allocation while the current field
        // remains resident. Both reservations drop automatically if validation
        // or GPU resource construction fails before the state swap.
        let particle_bytes = (particles.len() as u64)
            .checked_mul(std::mem::size_of::<ParticleInstance>() as u64)
            .ok_or_else(|| "tensor particle field byte size overflow".to_string())?;
        let particle_reservation = global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::FieldResidency,
                particle_bytes,
            )
            .map_err(|e| format!("tensor particle field admission failed: {e}"))?;
        let body = bytes
            .get(TENSOR_HEADER_BYTES..)
            .ok_or_else(|| "tensor buffer shorter than header".to_string())?;
        let tensor_bytes = body.len() as u64;
        let max_binding_bytes = u64::from(self.device.limits().max_storage_buffer_binding_size);
        let max_buffer_bytes = self.device.limits().max_buffer_size;
        if tensor_bytes == 0 || tensor_bytes > max_binding_bytes || tensor_bytes > max_buffer_bytes
        {
            return Err(format!(
                "tensor buffer body size {tensor_bytes} exceeds device storage limits (binding {max_binding_bytes}, buffer {max_buffer_bytes})"
            ));
        }
        let tensor_field_reservation = global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::FieldResidency,
                tensor_bytes,
            )
            .map_err(|e| format!("Tensor10D field admission failed: {e}"))?;

        let particle_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-tensor-particles"),
                contents: bytemuck::cast_slice(&particles),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        // Upload the SOA *body* only (skip the 32-byte header). WebGPU requires storage-buffer
        // binding offsets to be a multiple of minStorageBufferOffsetAlignment (256), so we cannot
        // bind at offset 32 the way native backends allow — start the buffer at the first record
        // and bind at offset 0.
        let tensor_raw_buf = self
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("portal-tensor-raw-soa"),
                contents: body,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            });

        let ambient_bind_group = make_ambient_bind_group(
            &self.device,
            &self.ambient_bind_group_layout,
            &self.uniform_buf,
            &self.telemetry_buf,
            &self.camera_buf,
            &self.observer_buf,
            &particle_buf,
        );

        let projector_tensor_bind = make_projector_tensor_bind_group(
            &self.device,
            &self.projector_tensor_layout,
            &tensor_raw_buf,
            count,
        )?;

        self.ambient_bind_group = ambient_bind_group;
        self.projector_tensor_bind = Some(projector_tensor_bind);
        self.particle_buf = particle_buf;
        self.tensor_raw_buf = Some(tensor_raw_buf);
        self._particle_reservation = particle_reservation;
        self._tensor_field_reservation = Some(tensor_field_reservation);
        self.tensor_node_count = count;
        self.particle_count = instance_count.max(1);
        // The particle field now carries real tensor nodes (not the
        // decorative random cloud), but it stays opt-in: showing it is an
        // explicit presentation choice (`set_ambient_enabled`), never forced
        // by an upload. Forcing it here resurrected the debug field after a
        // GPU re-init even when the host had disabled it.

        Ok(count)
    }

    pub fn tensor_node_count(&self) -> u32 {
        self.tensor_node_count
    }


    pub fn has_tensor_buffer(&self) -> bool {
        self.tensor_raw_buf.is_some()
    }

}
