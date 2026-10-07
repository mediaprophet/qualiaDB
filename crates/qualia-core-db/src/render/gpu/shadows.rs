//! Portable cascaded-sun shadow resources and bounded GPU target ownership.

use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(super) struct ShadowUniform {
    pub light_view_projection: [[[f32; 4]; 4]; SHADOW_CASCADE_COUNT],
    /// enabled, inverse resolution, constant receiver bias, camera split depth
    pub params: [f32; 4],
}

pub(super) const SHADOW_CASCADE_COUNT: usize = 2;

pub(super) struct ShadowTarget {
    pub _textures: [wgpu::Texture; SHADOW_CASCADE_COUNT],
    pub views: [wgpu::TextureView; SHADOW_CASCADE_COUNT],
    pub _reservation: VramReservation<'static>,
    pub resolution: u32,
}

impl ShadowTarget {
    /// Try quality tiers from high to low. If even the smallest allocation is refused, return
    /// `None` so the material shader uses its explicit unshadowed fallback.
    pub fn try_new(device: &wgpu::Device) -> Option<Self> {
        for resolution in SHADOW_RESOLUTION_CANDIDATES {
            let bytes = u64::from(resolution)
                .checked_mul(u64::from(resolution))?
                .checked_mul(4)?
                .checked_mul(SHADOW_CASCADE_COUNT as u64)?;
            let Ok(reservation) =
                global_vram_ledger().try_reserve_graphics(VramResourceClass::FrameTarget, bytes)
            else {
                continue;
            };
            let textures = std::array::from_fn(|cascade| {
                device.create_texture(&wgpu::TextureDescriptor {
                    label: Some(if cascade == 0 {
                        "qualia-sun-shadow-near-depth"
                    } else {
                        "qualia-sun-shadow-far-depth"
                    }),
                    size: wgpu::Extent3d {
                        width: resolution,
                        height: resolution,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format: wgpu::TextureFormat::Depth32Float,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT
                        | wgpu::TextureUsages::TEXTURE_BINDING,
                    view_formats: &[],
                })
            });
            let views = std::array::from_fn(|cascade| {
                textures[cascade].create_view(&wgpu::TextureViewDescriptor::default())
            });
            return Some(Self {
                _textures: textures,
                views,
                _reservation: reservation,
                resolution,
            });
        }
        None
    }
}

pub(super) fn sample_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("qualia-sun-shadow-sample-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Depth,
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 2,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Comparison),
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 3,
                visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: std::num::NonZeroU64::new(
                        std::mem::size_of::<ShadowUniform>() as u64,
                    ),
                },
                count: None,
            },
        ],
    })
}

pub(super) fn compare_sampler(device: &wgpu::Device) -> wgpu::Sampler {
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("qualia-sun-shadow-compare"),
        address_mode_u: wgpu::AddressMode::ClampToEdge,
        address_mode_v: wgpu::AddressMode::ClampToEdge,
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        compare: Some(wgpu::CompareFunction::LessEqual),
        ..Default::default()
    })
}

pub(super) fn sample_bind_group(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    views: [&wgpu::TextureView; SHADOW_CASCADE_COUNT],
    sampler: &wgpu::Sampler,
    uniform: &wgpu::Buffer,
) -> wgpu::BindGroup {
    device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("qualia-sun-shadow-sample"),
        layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(views[0]),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::TextureView(views[1]),
            },
            wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::Sampler(sampler),
            },
            wgpu::BindGroupEntry {
                binding: 3,
                resource: uniform.as_entire_binding(),
            },
        ],
    })
}

pub(super) fn matrix_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("qualia-sun-shadow-matrix-layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::VERTEX,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: std::num::NonZeroU64::new(
                    std::mem::size_of::<ShadowUniform>() as u64
                ),
            },
            count: None,
        }],
    })
}

#[path = "shadow_math.rs"]
mod shadow_math;
pub(super) use shadow_math::{
    aabb_outside_shadow_clip, camera_range_sun_view_projections, SHADOW_RESOLUTION_CANDIDATES,
};
