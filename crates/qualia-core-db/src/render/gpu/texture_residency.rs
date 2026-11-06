//! Bounded RGBA8 texture residency with semantic GPU-generated mip chains.

use super::texture_mips::AlphaCoverageDiagnostics;
use super::PortalGpu;
use std::collections::BTreeMap;

pub(super) use etalon_render::texture_layout::TextureIdentity;
#[cfg(test)]
use etalon_render::texture_layout::{mip_chain_layout,mip_semantic_matches,validate_upload};
pub use etalon_render::texture_layout::{TextureColorSpace, TextureUploadError};

pub(super) type ResidentTexture =
    etalon_render::texture_residency::ResidentTexture<crate::gpu_context::VramReservation<'static>>;

#[cfg(test)]
fn alpha_mip_scratch_bytes(width: u32, height: u32) -> Result<u64, TextureUploadError> {
    u64::from(width)
        .checked_mul(u64::from(height))
        .and_then(|pixels| pixels.checked_mul(4))
        .ok_or(TextureUploadError::ByteSizeOverflow)
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
        etalon_render::texture_residency::upload_rgba8(
            &self.device,
            &self.queue,
            &self.mip_generator,
            &mut self.resident_textures,
            source_digest,
            color_space,
            mip_semantic,
            width,
            height,
            rgba8,
            cfg!(test),
            |class, bytes| {
                let class = match class {
                    etalon_render::texture_residency::TextureAllocation::TextureResidency => {
                        crate::gpu_context::VramResourceClass::TextureResidency
                    }
                    etalon_render::texture_residency::TextureAllocation::UploadStaging => {
                        crate::gpu_context::VramResourceClass::UploadStaging
                    }
                };
                crate::gpu_context::global_vram_ledger()
                    .try_reserve_graphics(class, bytes)
                    .ok()
            },
        )
    }

    /// Borrow the resident view and shared default sampler for `(digest, colour-space)`.
    pub fn resident_texture_binding(
        &self,
        source_digest: &[u8; 32],
        color_space: TextureColorSpace,
    ) -> Option<(&etalon::TextureView, &etalon::Sampler)> {
        let mip_semantic = match color_space {
            TextureColorSpace::Srgb => super::texture_mips::TextureMipSemantic::Color,
            TextureColorSpace::Linear => super::texture_mips::TextureMipSemantic::LinearData,
        };
        self.resident_texture_binding_with_mips(source_digest, color_space, mip_semantic)
    }

    /// Binding for a resident texture with mips.
    pub fn resident_texture_binding_with_mips(
        &self,
        source_digest: &[u8; 32],
        color_space: TextureColorSpace,
        mip_semantic: super::texture_mips::TextureMipSemantic,
    ) -> Option<(&etalon::TextureView, &etalon::Sampler)> {
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

    /// Evicts a resident texture with mips.
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

/// `resident_view`.
pub(super) use etalon_render::texture_residency::resident_view;

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
