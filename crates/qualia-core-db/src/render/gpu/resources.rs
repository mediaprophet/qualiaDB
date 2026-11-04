//! GPU resource builders — depth/picking textures, bind-group layouts/groups, target states.
use super::*;

#[cfg(test)]
mod tests {
    use super::{checked_padded_bytes_per_row, frame_target_vram_bytes, readback_buffer_bytes};

    #[test]
    fn portal_target_estimate_counts_depth_picking_and_offscreen_color() {
        assert_eq!(frame_target_vram_bytes(1920, 1080, false), Some(16_588_800));
        assert_eq!(frame_target_vram_bytes(1920, 1080, true), Some(24_883_200));
        assert_eq!(frame_target_vram_bytes(u32::MAX, u32::MAX, true), None);
    }

    #[test]
    fn readback_staging_size_uses_checked_row_pitch_and_height() {
        assert_eq!(checked_padded_bytes_per_row(1), Some(256));
        assert_eq!(readback_buffer_bytes(1920, 1080), Some(8_294_400));
        assert_eq!(readback_buffer_bytes(u32::MAX, u32::MAX), None);
    }
}

/// Logical residency estimate for the portal's persistent depth and picking
/// targets, plus its RGBA8 offscreen target when present. Surface swapchain
/// colour storage is driver-owned and is not included here.
pub(super) fn frame_target_vram_bytes(width: u32, height: u32, offscreen: bool) -> Option<u64> {
    let pixels = u64::from(width.max(1)).checked_mul(u64::from(height.max(1)))?;
    pixels.checked_mul(if offscreen { 12 } else { 8 })
}

/// `reserve_view_resources`.
pub(super) fn reserve_view_resources(
    width: u32,
    height: u32,
    offscreen: bool,
) -> Result<
    (
        u32,
        u32,
        crate::gpu_context::VramReservation<'static>,
        Option<crate::gpu_context::VramReservation<'static>>,
    ),
    String,
> {
    let (mut width, mut height) = (width.max(1), height.max(1));
    let requested = (width, height);
    for _ in 0..33 {
        if let Some(bytes) = frame_target_vram_bytes(width, height, offscreen) {
            if let Ok(reservation) = crate::gpu_context::global_vram_ledger()
                .try_reserve_graphics(crate::gpu_context::VramResourceClass::FrameTarget, bytes)
            {
                let readback_reservation = if offscreen {
                    let Some(readback_bytes) = readback_buffer_bytes(width, height) else {
                        if width == 1 && height == 1 {
                            break;
                        }
                        width = width.div_ceil(2);
                        height = height.div_ceil(2);
                        continue;
                    };
                    let Ok(staging) = crate::gpu_context::global_vram_ledger()
                        .try_reserve_graphics(
                            crate::gpu_context::VramResourceClass::UploadStaging,
                            readback_bytes,
                        )
                    else {
                        if width == 1 && height == 1 {
                            break;
                        }
                        width = width.div_ceil(2);
                        height = height.div_ceil(2);
                        continue;
                    };
                    Some(staging)
                } else {
                    None
                };
                return Ok((width, height, reservation, readback_reservation));
            }
        }
        if !offscreen || (width == 1 && height == 1) {
            break;
        }
        width = width.div_ceil(2);
        height = height.div_ceil(2);
    }
    Err(format!(
        "frame targets refused by graphics budget for requested extent {}x{}",
        requested.0, requested.1
    ))
}

pub(super) use etalon_render::targets::*;

/// `uniform_128_bind_entry`.
pub(super) fn uniform_128_bind_entry(
    binding: u32,
    stages: wgpu::ShaderStages,
) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: stages,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: std::num::NonZeroU64::new(128),
        },
        count: None,
    }
}

/// `tensor_storage_bind_entry`.
pub(super) fn tensor_storage_bind_entry() -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding: 0,
        visibility: wgpu::ShaderStages::VERTEX,
        ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        },
        count: None,
    }
}

/// `ambient_bind_entries`.
pub(super) fn ambient_bind_entries() -> [wgpu::BindGroupLayoutEntry; 5] {
    [
        wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        wgpu::BindGroupLayoutEntry {
            binding: 2,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            count: None,
        },
        uniform_128_bind_entry(3, wgpu::ShaderStages::VERTEX),
        uniform_128_bind_entry(4, wgpu::ShaderStages::VERTEX_FRAGMENT),
    ]
}

/// Makes `ambient-bind-group`.
pub(super) fn make_ambient_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    uniform_buf: &wgpu::Buffer,
    telemetry_buf: &wgpu::Buffer,
    camera_buf: &wgpu::Buffer,
    observer_buf: &wgpu::Buffer,
    particle_buf: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("portal-ambient-bind"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: telemetry_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: particle_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: camera_buf.as_entire_binding(),
            },
            wgpu::BindGroupEntry {
                binding: 4,
                resource: observer_buf.as_entire_binding(),
            },
        ],
    })
}

/// Makes `projector-tensor-bind-group`.
pub(super) fn make_projector_tensor_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    tensor_raw_buf: &wgpu::Buffer,
    node_count: u32,
) -> Result<wgpu::BindGroup, String> {
    let body_bytes = (node_count as u64)
        .checked_mul(TENSOR_STRIDE as u64)
        .ok_or_else(|| "tensor body size overflow".to_string())?;
    let size =
        std::num::NonZeroU64::new(body_bytes).ok_or_else(|| "empty tensor body".to_string())?;
    Ok(device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("portal-projector-tensor-bind"),
        layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::Buffer(wgpu::BufferBinding {
                buffer: tensor_raw_buf,
                // Buffer already starts at the first record (header stripped at upload); offset 0
                // satisfies the 256-byte minStorageBufferOffsetAlignment that Dawn enforces.
                offset: 0,
                size: Some(size),
            }),
        }],
    }))
}
