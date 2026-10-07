//! Caller-supplied complete RGBA8 mip-chain upload.
//!
//! Offline pipelines can avoid runtime render passes and alpha-plan scratch by delivering a
//! validated full chain. This module owns only the cold upload lifecycle; mip generation remains
//! in `texture_mips.rs` for runtime-generated chains.

use super::texture_mips::{AlphaCoverageDiagnostics, TextureMipSemantic};
use super::texture_residency::{
    mip_chain_layout, mip_semantic_matches, validate_upload, ResidentTexture, TextureColorSpace,
    TextureIdentity, TextureUploadError,
};
use super::PortalGpu;

fn validate_precomputed_chain(
    width: u32,
    height: u32,
    adapter_limit: u32,
    levels: &[&[u8]],
) -> Result<(u32, u64), TextureUploadError> {
    let Some(base_level) = levels.first() else {
        return Err(TextureUploadError::InvalidMipLevelCount);
    };
    validate_upload(width, height, adapter_limit, base_level.len())?;
    let (expected_count, resident_bytes) = mip_chain_layout(width, height)?;
    if levels.len() != expected_count as usize {
        return Err(TextureUploadError::InvalidMipLevelCount);
    }
    if levels.len() > 32 {
        return Err(TextureUploadError::TooManyMipLevels);
    }

    let mut level_width = width;
    let mut level_height = height;
    let mut summed_bytes = 0u64;
    for (level, pixels) in levels.iter().enumerate() {
        let expected_bytes = u64::from(level_width)
            .checked_mul(u64::from(level_height))
            .and_then(|count| count.checked_mul(4))
            .ok_or(TextureUploadError::ByteSizeOverflow)?;
        if u64::try_from(pixels.len()).map_err(|_| TextureUploadError::ByteSizeOverflow)?
            != expected_bytes
        {
            return Err(TextureUploadError::InvalidMipLevelLength { level });
        }
        summed_bytes = summed_bytes
            .checked_add(expected_bytes)
            .ok_or(TextureUploadError::ByteSizeOverflow)?;
        if level + 1 < levels.len() {
            level_width = (level_width / 2).max(1);
            level_height = (level_height / 2).max(1);
        }
    }
    if summed_bytes != resident_bytes {
        return Err(TextureUploadError::ByteSizeOverflow);
    }
    Ok((expected_count, resident_bytes))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ResidentMipWindow {
    first_mip: u32,
    width: u32,
    height: u32,
    mip_count: u32,
    resident_bytes: u64,
}

fn resident_mip_window(
    width: u32,
    height: u32,
    levels: &[&[u8]],
    first_resident_mip: u32,
) -> Result<ResidentMipWindow, TextureUploadError> {
    let mip_count =
        u32::try_from(levels.len()).map_err(|_| TextureUploadError::ByteSizeOverflow)?;
    if first_resident_mip >= mip_count {
        return Err(TextureUploadError::InvalidMipLevelCount);
    }

    let mut resident_bytes = 0u64;
    for pixels in levels.iter().skip(first_resident_mip as usize) {
        let bytes =
            u64::try_from(pixels.len()).map_err(|_| TextureUploadError::ByteSizeOverflow)?;
        resident_bytes = resident_bytes
            .checked_add(bytes)
            .ok_or(TextureUploadError::ByteSizeOverflow)?;
    }

    // Match the validated floor-halving chain, including a one-pixel axis that remains 1.
    let mut base_width = width;
    let mut base_height = height;
    for _ in 0..first_resident_mip {
        base_width = (base_width / 2).max(1);
        base_height = (base_height / 2).max(1);
    }
    Ok(ResidentMipWindow {
        first_mip: first_resident_mip,
        width: base_width,
        height: base_height,
        mip_count: mip_count - first_resident_mip,
        resident_bytes,
    })
}

fn coarse_base_dimensions(
    source_width: u32,
    source_height: u32,
    first_resident_mip: u32,
) -> Result<(u32, u32), TextureUploadError> {
    let (source_mip_count, _) = mip_chain_layout(source_width, source_height)?;
    if first_resident_mip >= source_mip_count {
        return Err(TextureUploadError::InvalidMipLevelCount);
    }
    let mut width = source_width;
    let mut height = source_height;
    for _ in 0..first_resident_mip {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    Ok((width, height))
}

fn coverage(pixels: &[u8], cutoff: f32) -> f32 {
    let covered = pixels
        .chunks_exact(4)
        .filter(|pixel| f32::from(pixel[3]) / 255.0 >= cutoff)
        .count();
    (covered as f64 / (pixels.len() / 4) as f64) as f32
}

fn alpha_diagnostics(levels: &[&[u8]], cutoff: f32) -> AlphaCoverageDiagnostics {
    let base_coverage = coverage(levels[0], cutoff);
    let mut signed_residuals = [0.0; 32];
    for (index, pixels) in levels.iter().enumerate().skip(1) {
        signed_residuals[index] = coverage(pixels, cutoff) - base_coverage;
    }
    AlphaCoverageDiagnostics {
        base_coverage,
        level_count: levels.len(),
        signed_residuals,
    }
}

impl PortalGpu {
    /// Upload an already-downsampled coarse base and generate only its remaining mips on-GPU.
    ///
    /// `coarse_rgba8` must contain exactly the RGBA8 image at
    /// `(max(source_width >> first_resident_mip, 1),
    /// max(source_height >> first_resident_mip, 1))`, using floor-halving at each source level.
    /// The physical texture starts at that extent and owns only the remaining mip chain. This is
    /// suitable when an offline or streaming decoder provides the coarse image but not a complete
    /// precomputed suffix. An existing `TextureIdentity` is a cache hit: it is returned unchanged,
    /// and this call does not upgrade or refine its resident mip window.
    pub fn upload_resident_texture_rgba8_at_mip_with_mips(
        &mut self,
        source_digest: [u8; 32],
        color_space: TextureColorSpace,
        mip_semantic: TextureMipSemantic,
        source_width: u32,
        source_height: u32,
        first_resident_mip: u32,
        coarse_rgba8: &[u8],
    ) -> Result<Option<AlphaCoverageDiagnostics>, TextureUploadError> {
        if !mip_semantic_matches(color_space, mip_semantic) {
            return Err(TextureUploadError::InvalidMipSemantic);
        }
        let (width, height) =
            coarse_base_dimensions(source_width, source_height, first_resident_mip)?;
        let (base_bytes, bytes_per_row) = validate_upload(
            width,
            height,
            self.device.limits().max_texture_dimension_2d,
            coarse_rgba8.len(),
        )?;
        let (mip_level_count, resident_bytes) = mip_chain_layout(width, height)?;
        let key = TextureIdentity {
            source_digest,
            color_space,
            mip_semantic,
        };
        let alpha_plan = mip_semantic.alpha_cutoff().map(|cutoff| {
            super::texture_mips::alpha_coverage_plan(coarse_rgba8, width, height, cutoff)
        });
        let diagnostics = alpha_plan.map(AlphaCoverageDiagnostics::from);
        if self.resident_textures.contains_key(&key) {
            return Ok(diagnostics);
        }

        let resident_reservation = crate::gpu_context::global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::TextureResidency,
                resident_bytes,
            )
            .map_err(|_| TextureUploadError::GpuBudgetRefused)?;
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
            label: Some("qualia-resident-coarse-base-mip-texture"),
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
                | wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            coarse_rgba8,
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
        self.queue
            .on_submitted_work_done(move || drop(upload_reservation));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.resident_textures.insert(
            key,
            ResidentTexture {
                _texture: texture,
                view,
                _reservation: resident_reservation,
            },
        );
        Ok(diagnostics)
    }

    /// Upload an offline-generated, complete RGBA8 mip chain without running GPU mip passes.
    ///
    /// Levels must follow the exact floor-halving dimensions down to 1×1. The supplied bytes
    /// remain caller-owned; resident texture bytes and transient upload bytes are separately
    /// reserved before any GPU resource is created. Alpha-mask diagnostics measure the supplied
    /// quantized levels against their own base-level coverage.
    pub fn upload_resident_texture_mip_chain_rgba8(
        &mut self,
        source_digest: [u8; 32],
        color_space: TextureColorSpace,
        mip_semantic: TextureMipSemantic,
        width: u32,
        height: u32,
        levels: &[&[u8]],
    ) -> Result<Option<AlphaCoverageDiagnostics>, TextureUploadError> {
        self.upload_resident_texture_mip_window_rgba8(
            source_digest,
            color_space,
            mip_semantic,
            width,
            height,
            levels,
            0,
        )
    }

    /// Upload a suffix of an offline-generated, complete RGBA8 mip chain.
    ///
    /// The full supplied chain is validated, but only levels starting at
    /// `first_resident_mip` are allocated and uploaded. The coarser source level becomes texture
    /// mip zero, so both the physical texture and its residency/upload reservations cover only
    /// the selected suffix. This provides a coarse-first initial allocation; it does not refine
    /// or replace a texture that is already resident under the same `TextureIdentity`.
    ///
    /// Alpha diagnostics, when applicable, still describe the full supplied chain. A cache hit
    /// returns those diagnostics without uploading or changing the resident mip window.
    pub fn upload_resident_texture_mip_window_rgba8(
        &mut self,
        source_digest: [u8; 32],
        color_space: TextureColorSpace,
        mip_semantic: TextureMipSemantic,
        width: u32,
        height: u32,
        levels: &[&[u8]],
        first_resident_mip: u32,
    ) -> Result<Option<AlphaCoverageDiagnostics>, TextureUploadError> {
        if !mip_semantic_matches(color_space, mip_semantic) {
            return Err(TextureUploadError::InvalidMipSemantic);
        }
        validate_precomputed_chain(
            width,
            height,
            self.device.limits().max_texture_dimension_2d,
            levels,
        )?;
        let window = resident_mip_window(width, height, levels, first_resident_mip)?;
        let key = TextureIdentity {
            source_digest,
            color_space,
            mip_semantic,
        };
        let diagnostics = mip_semantic
            .alpha_cutoff()
            .map(|cutoff| alpha_diagnostics(levels, cutoff));
        if self.resident_textures.contains_key(&key) {
            return Ok(diagnostics);
        }

        let resident_reservation = crate::gpu_context::global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::TextureResidency,
                window.resident_bytes,
            )
            .map_err(|_| TextureUploadError::GpuBudgetRefused)?;
        let upload_reservation = crate::gpu_context::global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::UploadStaging,
                window.resident_bytes,
            )
            .map_err(|_| TextureUploadError::UploadBudgetRefused)?;
        let format = match color_space {
            TextureColorSpace::Srgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureColorSpace::Linear => wgpu::TextureFormat::Rgba8Unorm,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qualia-resident-precomputed-mip-texture"),
            size: wgpu::Extent3d {
                width: window.width,
                height: window.height,
                depth_or_array_layers: 1,
            },
            mip_level_count: window.mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut level_width = window.width;
        let mut level_height = window.height;
        for (mip_level, pixels) in levels.iter().skip(window.first_mip as usize).enumerate() {
            self.queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    texture: &texture,
                    mip_level: mip_level as u32,
                    origin: wgpu::Origin3d::ZERO,
                    aspect: wgpu::TextureAspect::All,
                },
                pixels,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(level_width * 4),
                    rows_per_image: Some(level_height),
                },
                wgpu::Extent3d {
                    width: level_width,
                    height: level_height,
                    depth_or_array_layers: 1,
                },
            );
            level_width = (level_width / 2).max(1);
            level_height = (level_height / 2).max(1);
        }
        self.queue.submit(std::iter::empty());
        self.queue
            .on_submitted_work_done(move || drop(upload_reservation));
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        self.resident_textures.insert(
            key,
            ResidentTexture {
                _texture: texture,
                view,
                _reservation: resident_reservation,
            },
        );
        Ok(diagnostics)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn precomputed_chain_requires_every_expected_level_and_byte_count() {
        let levels = [vec![0u8; 4 * 2 * 4], vec![0u8; 2 * 1 * 4], vec![0u8; 4]];
        let borrowed = levels.iter().map(Vec::as_slice).collect::<Vec<_>>();
        assert_eq!(validate_precomputed_chain(4, 2, 8, &borrowed), Ok((3, 44)));
        assert_eq!(
            validate_precomputed_chain(4, 2, 8, &borrowed[..2]),
            Err(TextureUploadError::InvalidMipLevelCount)
        );
        let invalid = [borrowed[0], &borrowed[1][..7], borrowed[2]];
        assert_eq!(
            validate_precomputed_chain(4, 2, 8, &invalid),
            Err(TextureUploadError::InvalidMipLevelLength { level: 1 })
        );
    }

    #[test]
    fn precomputed_window_accounts_only_selected_suffix() {
        let levels = [vec![0u8; 32], vec![0u8; 8], vec![0u8; 4]];
        let borrowed = levels.iter().map(Vec::as_slice).collect::<Vec<_>>();
        assert_eq!(validate_precomputed_chain(4, 2, 8, &borrowed), Ok((3, 44)));
        assert_eq!(
            resident_mip_window(4, 2, &borrowed, 1),
            Ok(ResidentMipWindow {
                first_mip: 1,
                width: 2,
                height: 1,
                mip_count: 2,
                resident_bytes: 12,
            })
        );
        assert_eq!(
            resident_mip_window(4, 2, &borrowed, 2),
            Ok(ResidentMipWindow {
                first_mip: 2,
                width: 1,
                height: 1,
                mip_count: 1,
                resident_bytes: 4,
            })
        );
        assert_eq!(
            resident_mip_window(4, 2, &borrowed, 3),
            Err(TextureUploadError::InvalidMipLevelCount)
        );
    }

    #[test]
    fn coarse_base_dimensions_and_generated_chain_budget_start_at_selected_mip() {
        // Repeated floor-halving gives source mips 17x9, 8x4, 4x2, 2x1, 1x1.
        let (width, height) = coarse_base_dimensions(17, 9, 2).unwrap();
        assert_eq!((width, height), (4, 2));
        let (base_bytes, _) = validate_upload(width, height, 64, 4 * 2 * 4).unwrap();
        let (mip_count, resident_bytes) = mip_chain_layout(width, height).unwrap();
        assert_eq!(base_bytes, 32);
        assert_eq!(mip_count, 3);
        assert_eq!(resident_bytes, 44);

        assert_eq!(
            coarse_base_dimensions(17, 9, 5),
            Err(TextureUploadError::InvalidMipLevelCount)
        );
        assert_eq!(
            validate_upload(width, height, 64, 31),
            Err(TextureUploadError::InvalidRgba8Length)
        );
    }

    #[test]
    fn precomputed_alpha_diagnostics_report_quantized_level_residuals() {
        let base = [
            255u8, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 0, 0, 0, 0,
        ];
        let mip1 = [255u8, 255, 255, 255];
        let mip2 = [255u8, 255, 255, 255];
        let report = alpha_diagnostics(&[&base, &mip1, &mip2], 0.5);
        assert_eq!(report.level_count, 3);
        assert_eq!(report.base_coverage, 0.75);
        assert_eq!(report.signed_residuals[1], 0.25);
        assert_eq!(report.signed_residuals[2], 0.25);
    }
}
