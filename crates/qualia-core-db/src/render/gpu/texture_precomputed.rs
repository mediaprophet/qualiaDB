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
        if !mip_semantic_matches(color_space, mip_semantic) {
            return Err(TextureUploadError::InvalidMipSemantic);
        }
        let (mip_count, resident_bytes) = validate_precomputed_chain(
            width,
            height,
            self.device.limits().max_texture_dimension_2d,
            levels,
        )?;
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
                resident_bytes,
            )
            .map_err(|_| TextureUploadError::GpuBudgetRefused)?;
        let upload_reservation = crate::gpu_context::global_vram_ledger()
            .try_reserve_graphics(
                crate::gpu_context::VramResourceClass::UploadStaging,
                resident_bytes,
            )
            .map_err(|_| TextureUploadError::UploadBudgetRefused)?;
        let format = match color_space {
            TextureColorSpace::Srgb => wgpu::TextureFormat::Rgba8UnormSrgb,
            TextureColorSpace::Linear => wgpu::TextureFormat::Rgba8Unorm,
        };
        let texture = self.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("qualia-resident-precomputed-mip-texture"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: mip_count,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let mut level_width = width;
        let mut level_height = height;
        for (mip_level, pixels) in levels.iter().enumerate() {
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
