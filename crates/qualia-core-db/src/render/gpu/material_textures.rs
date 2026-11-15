//! Fixed WebGPU texture slots, bounded sampler cache and fallback texels for MAT4 materials.

use crate::container_10d::{
    TextureAddressMode, TextureFilterMode, TextureMinFilter, TextureSampler,
};

pub(super) const SAMPLER_VARIANTS: usize = 3 * 3 * 2 * 6;

/// Creates `sampler`.
pub(super) fn create_sampler(device: &etalon::Device, state: TextureSampler) -> etalon::Sampler {
    let address = |mode| match mode {
        TextureAddressMode::ClampToEdge => etalon::AddressMode::ClampToEdge,
        TextureAddressMode::MirroredRepeat => etalon::AddressMode::MirrorRepeat,
        TextureAddressMode::Repeat => etalon::AddressMode::Repeat,
    };
    let (min_filter, mipmap_filter) = match state.min_filter {
        TextureMinFilter::Nearest => (
            etalon::FilterMode::Nearest,
            etalon::MipmapFilterMode::Nearest,
        ),
        TextureMinFilter::Linear => (
            etalon::FilterMode::Linear,
            etalon::MipmapFilterMode::Nearest,
        ),
        TextureMinFilter::NearestMipmapNearest => (
            etalon::FilterMode::Nearest,
            etalon::MipmapFilterMode::Nearest,
        ),
        TextureMinFilter::LinearMipmapNearest => (
            etalon::FilterMode::Linear,
            etalon::MipmapFilterMode::Nearest,
        ),
        TextureMinFilter::NearestMipmapLinear => (
            etalon::FilterMode::Nearest,
            etalon::MipmapFilterMode::Linear,
        ),
        TextureMinFilter::LinearMipmapLinear => {
            (etalon::FilterMode::Linear, etalon::MipmapFilterMode::Linear)
        }
    };
    device.create_sampler(&etalon::SamplerDescriptor {
        label: Some("portal-material-sampler"),
        address_mode_u: address(state.wrap_s),
        address_mode_v: address(state.wrap_t),
        address_mode_w: etalon::AddressMode::ClampToEdge,
        mag_filter: match state.mag_filter {
            TextureFilterMode::Nearest => etalon::FilterMode::Nearest,
            TextureFilterMode::Linear => etalon::FilterMode::Linear,
        },
        min_filter,
        mipmap_filter,
        ..Default::default()
    })
}

pub(super) struct MaterialTextureDefaults {
    _textures: [etalon::Texture; 6],
    views: [etalon::TextureView; 6],
    samplers: [etalon::Sampler; SAMPLER_VARIANTS],
    _resident_reservation: crate::gpu_context::VramReservation<'static>,
}

impl MaterialTextureDefaults {
    /// Default texture view for `slot`.
    pub fn view(&self, slot: usize) -> &etalon::TextureView {
        &self.views[slot]
    }

    /// Default sampler for `state`.
    pub fn sampler(&self, state: TextureSampler) -> &etalon::Sampler {
        &self.samplers[state.cache_index()]
    }

    /// Filtering sampler for `state`.
    pub fn filtering_sampler(&self, state: TextureSampler) -> &etalon::Sampler {
        self.sampler(if state.needs_filtering() {
            state
        } else {
            state.filtering_variant()
        })
    }
}

/// Creates `layout`.
pub(super) fn create_layout(device: &etalon::Device) -> etalon::BindGroupLayout {
    let entries = layout_entries(0);
    device.create_bind_group_layout(&etalon::BindGroupLayoutDescriptor {
        label: Some("portal-material-texture-layout"),
        entries: &entries,
    })
}

/// Build the fixed texture/sampler slots at a caller-selected binding offset. The mesh renderer
/// places these after its dynamic material uniform so a draw uses one material bind group.
pub(super) fn layout_entries(binding_offset: u32) -> Vec<etalon::BindGroupLayoutEntry> {
    etalon_render::materials::layout_entries(binding_offset).to_vec()
}

/// Creates `defaults`.
pub(super) fn create_defaults(
    device: &etalon::Device,
    queue: &etalon::Queue,
) -> Result<MaterialTextureDefaults, String> {
    // base colour=white, normal=(0.5,0.5,1), metallic-roughness=(0,1,1), AO=white,
    // G/B are neutral multipliers for authored roughness/metallic factors.
    // emissive=black, stylized ramp=white. One pixel per semantic slot.
    const DEFAULTS: [[u8; 4]; 6] = [
        [255, 255, 255, 255],
        [128, 128, 255, 255],
        [0, 255, 255, 255],
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
        let texture = device.create_texture(&etalon::TextureDescriptor {
            label: Some("portal-material-semantic-fallback"),
            size: etalon::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: etalon::TextureDimension::D2,
            format: if matches!(index, 0 | 4 | 5) {
                etalon::TextureFormat::Rgba8UnormSrgb
            } else {
                etalon::TextureFormat::Rgba8Unorm
            },
            usage: etalon::TextureUsages::TEXTURE_BINDING | etalon::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        queue.write_texture(
            etalon::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: etalon::Origin3d::ZERO,
                aspect: etalon::TextureAspect::All,
            },
            pixel,
            etalon::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4),
                rows_per_image: Some(1),
            },
            etalon::Extent3d {
                width: 1,
                height: 1,
                depth_or_array_layers: 1,
            },
        );
        textures.push(texture);
    }
    queue.submit(std::iter::empty());
    queue.on_submitted_work_done(move || drop(staging));
    let views: [etalon::TextureView; 6] = textures
        .iter()
        .map(|texture| texture.create_view(&etalon::TextureViewDescriptor::default()))
        .collect::<Vec<_>>()
        .try_into()
        .map_err(|_| "material fallback texture view count mismatch".to_string())?;
    let textures: [etalon::Texture; 6] = textures
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
