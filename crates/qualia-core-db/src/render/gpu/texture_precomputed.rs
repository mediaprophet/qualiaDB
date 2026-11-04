//! Universe-admitted coarse and precomputed texture upload adapters.
use super::{
    texture_mips::{AlphaCoverageDiagnostics, TextureMipSemantic},
    texture_residency::{TextureColorSpace, TextureUploadError},
    PortalGpu,
};
fn reserve(
    class: etalon_render::texture_residency::TextureAllocation,
    bytes: u64,
) -> Option<crate::gpu_context::VramReservation<'static>> {
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
}
impl PortalGpu {
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
        etalon_render::texture_precomputed::TextureUploader {
            device: &self.device,
            queue: &self.queue,
            mip_generator: &self.mip_generator,
            resident_textures: &mut self.resident_textures,
            reserve,
        }
        .upload_resident_texture_rgba8_at_mip_with_mips(
            source_digest,
            color_space,
            mip_semantic,
            source_width,
            source_height,
            first_resident_mip,
            coarse_rgba8,
        )
    }
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
        etalon_render::texture_precomputed::TextureUploader {
            device: &self.device,
            queue: &self.queue,
            mip_generator: &self.mip_generator,
            resident_textures: &mut self.resident_textures,
            reserve,
        }
        .upload_resident_texture_mip_window_rgba8(
            source_digest,
            color_space,
            mip_semantic,
            width,
            height,
            levels,
            first_resident_mip,
        )
    }
}
