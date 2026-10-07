//! Bounded RGBA8 texture residency with semantic GPU-generated mip chains.

use super::texture_mips::AlphaCoverageDiagnostics;
use super::PortalGpu;
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum TextureColorSpace {
    Srgb,
    Linear,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct TextureIdentity {
    pub source_digest: [u8; 32],
    pub color_space: TextureColorSpace,
    pub mip_semantic: super::texture_mips::TextureMipSemantic,
}

pub(super) struct ResidentTexture {
    pub(super) _texture: wgpu::Texture,
    pub(super) view: wgpu::TextureView,
    pub(super) _reservation: crate::gpu_context::VramReservation<'static>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TextureUploadError {
    InvalidDimensions,
    InvalidRgba8Length,
    TextureDimensionLimit,
    ByteSizeOverflow,
    GpuBudgetRefused,
    UploadBudgetRefused,
    InvalidMipSemantic,
    InvalidMipLevelCount,
    InvalidMipLevelLength { level: usize },
    TooManyMipLevels,
}

impl std::fmt::Display for TextureUploadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "texture upload: {self:?}")
    }
}

impl std::error::Error for TextureUploadError {}

pub(super) fn validate_upload(
    width: u32,
    height: u32,
    adapter_limit: u32,
    rgba8_len: usize,
) -> Result<(u64, u32), TextureUploadError> {
    if width == 0 || height == 0 {
        return Err(TextureUploadError::InvalidDimensions);
    }
    if width > adapter_limit || height > adapter_limit {
        return Err(TextureUploadError::TextureDimensionLimit);
    }
    let byte_len = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(TextureUploadError::ByteSizeOverflow)?;
    let bytes_per_row = width
        .checked_mul(4)
        .ok_or(TextureUploadError::ByteSizeOverflow)?;
    if rgba8_len != byte_len {
        return Err(TextureUploadError::InvalidRgba8Length);
    }
    Ok((
        u64::try_from(byte_len).map_err(|_| TextureUploadError::ByteSizeOverflow)?,
        bytes_per_row,
    ))
}

pub(super) fn mip_chain_layout(width: u32, height: u32) -> Result<(u32, u64), TextureUploadError> {
    if width == 0 || height == 0 {
        return Err(TextureUploadError::InvalidDimensions);
    }
    let mut level_width = width;
    let mut level_height = height;
    let mut levels = 1u32;
    let mut total = 0u64;
    loop {
        let level_bytes = u64::from(level_width)
            .checked_mul(u64::from(level_height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(TextureUploadError::ByteSizeOverflow)?;
        total = total
            .checked_add(level_bytes)
            .ok_or(TextureUploadError::ByteSizeOverflow)?;
        if level_width == 1 && level_height == 1 {
            break;
        }
        level_width = (level_width / 2).max(1);
        level_height = (level_height / 2).max(1);
        levels = levels
            .checked_add(1)
            .ok_or(TextureUploadError::ByteSizeOverflow)?;
    }
    Ok((levels, total))
}

fn alpha_mip_scratch_bytes(width: u32, height: u32) -> Result<u64, TextureUploadError> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(TextureUploadError::ByteSizeOverflow)
}

pub(super) fn mip_semantic_matches(
    color_space: TextureColorSpace,
    semantic: super::texture_mips::TextureMipSemantic,
) -> bool {
    use super::texture_mips::TextureMipSemantic::{AlphaMask, Color, LinearData, Normal};
    match (color_space, semantic) {
        (TextureColorSpace::Srgb, Color) => true,
        (TextureColorSpace::Srgb, AlphaMask { .. }) => semantic.alpha_cutoff().is_some(),
        (TextureColorSpace::Linear, LinearData | Normal) => true,
        _ => false,
    }
}

impl PortalGpu {
    /// Upload a decoded RGBA8 base level and generate a complete mip chain on the GPU.
    ///
    /// This compatibility entry uses sRGB colour mips for sRGB textures and linear-data mips for
    /// linear textures. Use the semantic variant for normal maps so their mips are renormalized.
    pub fn upload_resident_texture_rgba8(
        &mut self,
        source_digest: [u8; 32],
        color_space: TextureColorSpace,
        width: u32,
        height: u32,
        rgba8: &[u8],
    ) -> Result<(), TextureUploadError> {
        let mip_semantic = match color_space {
            TextureColorSpace::Srgb => super::texture_mips::TextureMipSemantic::Color,
            TextureColorSpace::Linear => super::texture_mips::TextureMipSemantic::LinearData,
        };
        self.upload_resident_texture_rgba8_with_mips(
            source_digest,
            color_space,
            mip_semantic,
            width,
            height,
            rgba8,
        )
    }

    /// Upload with a semantic mip filter: linear-light colour, linear data, or renormalized normals.
    pub fn upload_resident_texture_rgba8_with_mips(
        &mut self,
        source_digest: [u8; 32],
        color_space: TextureColorSpace,
        mip_semantic: super::texture_mips::TextureMipSemantic,
        width: u32,
        height: u32,
        rgba8: &[u8],
    ) -> Result<(), TextureUploadError> {
        self.upload_resident_texture_rgba8_with_mips_and_diagnostics(
            source_digest,
            color_space,
            mip_semantic,
            width,
            height,
            rgba8,
        )
        .map(|_| ())
    }

    /// Upload a texture and return the bounded CPU prediction for alpha-mask mip coverage.
    ///
    /// `None` is returned for non-alpha mip semantics. Diagnostics are returned by value instead
    /// of retained in the residency map, so long-lived metadata does not grow per texture.
    pub fn upload_resident_texture_rgba8_with_mips_and_diagnostics(
        &mut self,
        source_digest: [u8; 32],
        color_space: TextureColorSpace,
        mip_semantic: super::texture_mips::TextureMipSemantic,
        width: u32,
        height: u32,
        rgba8: &[u8],
    ) -> Result<Option<AlphaCoverageDiagnostics>, TextureUploadError> {
        if !mip_semantic_matches(color_space, mip_semantic) {
            return Err(TextureUploadError::InvalidMipSemantic);
        }
        let (base_bytes, bytes_per_row) = validate_upload(
            width,
            height,
            self.device.limits().max_texture_dimension_2d,
            rgba8.len(),
        )?;
        let (mip_level_count, resident_bytes) = mip_chain_layout(width, height)?;
        let key = TextureIdentity {
            source_digest,
            color_space,
            mip_semantic,
        };
        if self.resident_textures.contains_key(&key) {
            let alpha_scratch_reservation = if mip_semantic.alpha_cutoff().is_some() {
                Some(
                    crate::gpu_context::global_vram_ledger()
                        .try_reserve_graphics(
                            crate::gpu_context::VramResourceClass::UploadStaging,
                            alpha_mip_scratch_bytes(width, height)?,
                        )
                        .map_err(|_| TextureUploadError::UploadBudgetRefused)?,
                )
            } else {
                None
            };
            let diagnostics = mip_semantic
                .alpha_cutoff()
                .map(|cutoff| {
                    super::texture_mips::alpha_coverage_plan(rgba8, width, height, cutoff)
                })
                .map(AlphaCoverageDiagnostics::from);
            drop(alpha_scratch_reservation);
            return Ok(diagnostics);
        }
        let reservation = crate::gpu_context::global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::TextureResidency,
                resident_bytes,
            )
            .map_err(|_| TextureUploadError::GpuBudgetRefused)?;
        let alpha_scratch_reservation = if mip_semantic.alpha_cutoff().is_some() {
            Some(
                crate::gpu_context::global_vram_ledger()
                    .try_reserve_graphics(
                        crate::gpu_context::VramResourceClass::UploadStaging,
                        alpha_mip_scratch_bytes(width, height)?,
                    )
                    .map_err(|_| TextureUploadError::UploadBudgetRefused)?,
            )
        } else {
            None
        };
        let alpha_plan = mip_semantic
            .alpha_cutoff()
            .map(|cutoff| super::texture_mips::alpha_coverage_plan(rgba8, width, height, cutoff));
        let diagnostics = alpha_plan.map(AlphaCoverageDiagnostics::from);
        let upload_reservation = crate::gpu_context::global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::UploadStaging,
                base_bytes,
            )
            .map_err(|_| TextureUploadError::UploadBudgetRefused)?;
        let format = match color_space {
            TextureColorSpace::Srgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureColorSpace::Linear => wgpu::TextureFormat::Rgba8Unorm,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qualia-resident-material-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING
                | wgpu::TextureUsages::COPY_DST
                | wgpu::TextureUsages::RENDER_ATTACHMENT
                | if cfg!(test) {
                    wgpu::TextureUsages::COPY_SRC
                } else {
                    wgpu::TextureUsages::empty()
                },
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            rgba8,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(bytes_per_row),
                rows_per_image: Some(height),
            },
            wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
        );
        self.mip_generator.generate(
            &self.device,
            &self.queue,
            &texture,
            mip_level_count,
            mip_semantic,
            alpha_plan.as_ref(),
        );
        drop(alpha_scratch_reservation);
        self.queue
            .on_submitted_work_done(move || drop(upload_reservation));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.resident_textures.insert(
            key,
            ResidentTexture {
                _texture: texture,
                view,
                _reservation: reservation,
            },
        );
        Ok(diagnostics)
    }

    /// Borrow the resident view and shared default sampler for `(digest, colour-space)`.
    pub fn resident_texture_binding(
        &self,
        source_digest: &[u8; 32],
        color_space: TextureColorSpace,
    ) -> Option<(&wgpu::TextureView, &wgpu::Sampler)> {
        let mip_semantic = match color_space {
            TextureColorSpace::Srgb => super::texture_mips::TextureMipSemantic::Color,
            TextureColorSpace::Linear => super::texture_mips::TextureMipSemantic::LinearData,
        };
        self.resident_texture_binding_with_mips(source_digest, color_space, mip_semantic)
    }

    pub fn resident_texture_binding_with_mips(
        &self,
        source_digest: &[u8; 32],
        color_space: TextureColorSpace,
        mip_semantic: super::texture_mips::TextureMipSemantic,
    ) -> Option<(&wgpu::TextureView, &wgpu::Sampler)> {
        let key = TextureIdentity {
            source_digest: *source_digest,
            color_space,
            mip_semantic,
        };
        self.resident_textures.get(&key).map(|resident| {
            (
                &resident.view,
                self.material_texture_defaults
                    .sampler(crate::container_10d::TextureSampler::GLTF_DEFAULT),
            )
        })
    }

    /// Evict one interpretation of a source image and release its resident-byte reservation.
    pub fn evict_resident_texture(
        &mut self,
        source_digest: &[u8; 32],
        color_space: TextureColorSpace,
    ) -> bool {
        let previous_len = self.resident_textures.len();
        self.resident_textures.retain(|identity, _| {
            identity.source_digest != *source_digest || identity.color_space != color_space
        });
        self.resident_textures.len() != previous_len
    }

    pub fn evict_resident_texture_with_mips(
        &mut self,
        source_digest: &[u8; 32],
        color_space: TextureColorSpace,
        mip_semantic: super::texture_mips::TextureMipSemantic,
    ) -> bool {
        self.resident_textures
            .remove(&TextureIdentity {
                source_digest: *source_digest,
                color_space,
                mip_semantic,
            })
            .is_some()
    }
}

pub(super) type ResidentTextureMap = BTreeMap<TextureIdentity, ResidentTexture>;

pub(super) fn resident_view<'a>(
    textures: &'a ResidentTextureMap,
    digest: &[u8; 32],
    color_space: TextureColorSpace,
    mip_semantic: super::texture_mips::TextureMipSemantic,
) -> Option<&'a wgpu::TextureView> {
    textures
        .get(&TextureIdentity {
            source_digest: *digest,
            color_space,
            mip_semantic,
        })
        .map(|resident| &resident.view)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn upload_admission_checks_dimensions_and_exact_rgba_bytes() {
        assert_eq!(validate_upload(2, 3, 8, 24), Ok((24, 8)));
        assert_eq!(
            validate_upload(0, 3, 8, 24),
            Err(TextureUploadError::InvalidDimensions)
        );
        assert_eq!(
            validate_upload(9, 3, 8, 108),
            Err(TextureUploadError::TextureDimensionLimit)
        );
        assert_eq!(
            validate_upload(2, 3, 8, 23),
            Err(TextureUploadError::InvalidRgba8Length)
        );
    }

    #[test]
    fn mip_chain_budget_covers_odd_edges_and_rejects_colour_space_mismatch() {
        assert_eq!(mip_chain_layout(1, 1), Ok((1, 4)));
        assert_eq!(mip_chain_layout(2, 2), Ok((2, 20)));
        assert_eq!(mip_chain_layout(3, 1), Ok((2, 16)));
        assert_eq!(mip_chain_layout(4, 4), Ok((3, 84)));
        assert!(mip_semantic_matches(
            TextureColorSpace::Srgb,
            super::super::texture_mips::TextureMipSemantic::Color
        ));
        assert!(!mip_semantic_matches(
            TextureColorSpace::Srgb,
            super::super::texture_mips::TextureMipSemantic::Normal
        ));
        assert!(!mip_semantic_matches(
            TextureColorSpace::Linear,
            super::super::texture_mips::TextureMipSemantic::Color
        ));
        assert!(mip_semantic_matches(
            TextureColorSpace::Srgb,
            super::super::texture_mips::TextureMipSemantic::alpha_mask(0.37).unwrap()
        ));
        assert!(!mip_semantic_matches(
            TextureColorSpace::Linear,
            super::super::texture_mips::TextureMipSemantic::alpha_mask(0.37).unwrap()
        ));
        assert_eq!(alpha_mip_scratch_bytes(5, 3), Ok(60));
    }
}
