//! Fixed WebGPU texture slots, bounded sampler cache and fallback texels for MAT3 materials.

use crate::container_10d::{
    TextureAddressMode, TextureFilterMode, TextureMinFilter, TextureSampler,
};

pub(super) const SAMPLER_VARIANTS: usize = 3 * 3 * 2 * 6;

pub(super) fn create_sampler(device: &wgpu::Device, state: TextureSampler) -> wgpu::Sampler {
    let address = |mode| match mode {
        TextureAddressMode::ClampToEdge => wgpu::AddressMode::ClampToEdge,
        TextureAddressMode::MirroredRepeat => wgpu::AddressMode::MirrorRepeat,
        TextureAddressMode::Repeat => wgpu::AddressMode::Repeat,
    };
    let (min_filter, mipmap_filter) = match state.min_filter {
        TextureMinFilter::Nearest => (wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Nearest),
        TextureMinFilter::Linear => (wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Nearest),
        TextureMinFilter::NearestMipmapNearest => {
            (wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Nearest)
        }
        TextureMinFilter::LinearMipmapNearest => {
            (wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Nearest)
        }
        TextureMinFilter::NearestMipmapLinear => {
            (wgpu::FilterMode::Nearest, wgpu::MipmapFilterMode::Linear)
        }
        TextureMinFilter::LinearMipmapLinear => {
            (wgpu::FilterMode::Linear, wgpu::MipmapFilterMode::Linear)
        }
    };
    device.create_sampler(&wgpu::SamplerDescriptor {
        label: Some("portal-material-sampler"),
        address_mode_u: address(state.wrap_s),
        address_mode_v: address(state.wrap_t),
        address_mode_w: wgpu::AddressMode::ClampToEdge,
        mag_filter: match state.mag_filter {
            TextureFilterMode::Nearest => wgpu::FilterMode::Nearest,
            TextureFilterMode::Linear => wgpu::FilterMode::Linear,
        },
        min_filter,
        mipmap_filter,
        ..Default::default()
    })
}

pub(super) struct MaterialTextureDefaults {
    _textures: [wgpu::Texture; 6],
    views: [wgpu::TextureView; 6],
    samplers: [wgpu::Sampler; SAMPLER_VARIANTS],
    _resident_reservation: crate::gpu_context::VramReservation<'static>,
}

impl MaterialTextureDefaults {
    pub fn view(&self, slot: usize) -> &wgpu::TextureView {
        &self.views[slot]
    }

    pub fn sampler(&self, state: TextureSampler) -> &wgpu::Sampler {
        &self.samplers[state.cache_index()]
    }

    pub fn filtering_sampler(&self, state: TextureSampler) -> &wgpu::Sampler {
        self.sampler(if state.needs_filtering() {
            state
        } else {
            state.filtering_variant()
        })
    }
}

pub(super) fn create_layout(device: &wgpu::Device) -> wgpu::BindGroupLayout {
    let mut entries = Vec::with_capacity(12);
    for binding in 0..6 {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
                sample_type: wgpu::TextureSampleType::Float { filterable: true },
                view_dimension: wgpu::TextureViewDimension::D2,
                multisampled: false,
            },
            count: None,
        });
    }
    for slot in 0..6 {
        entries.push(wgpu::BindGroupLayoutEntry {
            binding: 6 + slot,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
        });
    }
    device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("portal-material-texture-layout"),
        entries: &entries,
    })
}

pub(super) fn create_defaults(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
) -> Result<MaterialTextureDefaults, String> {
    // base colour=white, normal=(0.5,0.5,1), metallic-roughness=(0,1,0), AO=white,
    // emissive=black, stylized ramp=white. One pixel per semantic slot.
    const DEFAULTS: [[u8; 4]; 6] = [
        [255, 255, 255, 255],
        [128, 128, 255, 255],
        [0, 255, 0, 255],
        [255, 255, 255, 255],
        [0, 0, 0, 255],
        [255, 255, 255, 255],
    ];
    let resident = crate::gpu_context::global_vram_ledger()
        .try_reserve_graphics(crate::gpu_context::VramResourceClass::TextureResidency, 24)
        .map_err(|error| format!("material fallback texture reservation failed: {error}"))?;
    let staging = crate::gpu_context::global_vram_ledger()
        .try_reserve_graphics(crate::gpu_context::VramResourceClass::UploadStaging, 24)
        .map_err(|error| format!("material fallback upload reservation failed: {error}"))?;
    let mut textures = Vec::with_capacity(6);
    for (index, pixel) in DEFAULTS.iter().enumerate() {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("portal-material-semantic-fallback"),
            size: wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: if matches!(index, 0 | 4 | 5) {
                wgpu::TextureFormat::Rgba8UnormSrgb
            } else {
                wgpu::TextureFormat::Rgba8Unorm
            },
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            pixel,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            wgpu::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        textures.push(texture);
    }
    queue.submit(std::iter::empty());
    queue.on_submitted_work_done(move || drop(staging));
    let views: [wgpu::TextureView; 6] = textures
        .iter()
        .map(|texture| texture.create_view(&wgpu::TextureViewDescriptor::default()))
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "material fallback texture view count mismatch".to_string())?;
    let textures: [wgpu::Texture; 6] = textures
        .try_into()
        .map_err(|_| "material fallback texture count mismatch".to_string())?;
    const ADDRESSES: [TextureAddressMode; 3] = [
        TextureAddressMode::ClampToEdge,
        TextureAddressMode::MirroredRepeat,
        TextureAddressMode::Repeat,
    ];
    const MAG_FILTERS: [TextureFilterMode; 2] =
        [TextureFilterMode::Nearest, TextureFilterMode::Linear];
    const MIN_FILTERS: [TextureMinFilter; 6] = [
        TextureMinFilter::Nearest,
        TextureMinFilter::Linear,
        TextureMinFilter::NearestMipmapNearest,
        TextureMinFilter::LinearMipmapNearest,
        TextureMinFilter::NearestMipmapLinear,
        TextureMinFilter::LinearMipmapLinear,
    ];
    let samplers = std::array::from_fn(|index| {
        let min_filter = MIN_FILTERS[index % 6];
        let mag_filter = MAG_FILTERS[(index / 6) % 2];
        let wrap_t = ADDRESSES[(index / 12) % 3];
        let wrap_s = ADDRESSES[(index / 36) % 3];
        create_sampler(
            device,
            TextureSampler {
                wrap_s,
                wrap_t,
                mag_filter,
                min_filter,
            },
        )
    });
    Ok(MaterialTextureDefaults {
        _textures: textures,
        views,
        samplers,
        _resident_reservation: resident,
    })
}
