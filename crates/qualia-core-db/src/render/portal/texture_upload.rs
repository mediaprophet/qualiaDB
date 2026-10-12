//! Cold, bounded image hydration for the same portal that submits the scene.
//! Codec, digest checks, mip semantics and residency remain engine-owned.

use super::QualiaPortal;
use crate::render::asset_package::HmcTextureResource;
use crate::render::gpu::{TextureColorSpace, TextureMipSemantic};
use crate::render::texture_decode::{
    decode_hmc_texture_rgba8_into, inspect_hmc_texture_requirements, TextureDecodeLimits,
};
use sha2::{Digest, Sha256};
use wasm_bindgen::prelude::*;

#[wasm_bindgen]
impl QualiaPortal {
    /// Hydrate one content-addressed PNG/JPEG/KTX2 dependency. Role 0 is sRGB
    /// base colour, 1 a linear tangent-space normal, 2 linear packed data and
    /// 3 sRGB alpha-masked colour. Returns false for an unavailable GPU backend.
    /// Call before scene submission so the material bind groups see residency.
    pub fn load_texture_encoded(
        &mut self, digest: &[u8], mime: &str, encoded: &[u8], role: u8, cutoff: f32,
    ) -> Result<bool, JsValue> {
        if digest.len() != 32 || encoded.len() > 4 * 1024 * 1024 {
            return Err(JsValue::from_str("texture input exceeds portal bounds"));
        }
        let identity: [u8; 32] = digest.try_into().map_err(|_| JsValue::from_str("texture digest"))?;
        let actual: [u8; 32] = Sha256::digest(encoded).into();
        if identity != actual { return Err(JsValue::from_str("texture SHA-256 mismatch")); }
        let (space, semantic) = match role {
            0 => (TextureColorSpace::Srgb, TextureMipSemantic::Color),
            1 => (TextureColorSpace::Linear, TextureMipSemantic::Normal),
            2 => (TextureColorSpace::Linear, TextureMipSemantic::LinearData),
            3 => (TextureColorSpace::Srgb, TextureMipSemantic::alpha_mask(cutoff)
                .ok_or_else(|| JsValue::from_str("invalid alpha cutoff"))?),
            _ => return Err(JsValue::from_str("unknown texture role")),
        };
        let resource = HmcTextureResource { digest: identity, mime_type: mime, bytes: encoded };
        let limits = TextureDecodeLimits { max_decoded_bytes: 4 * 1024 * 1024,
            max_encoded_bytes: 4 * 1024 * 1024, ..TextureDecodeLimits::default() };
        let (info, scratch_len) = inspect_hmc_texture_requirements(&resource, limits)
            .map_err(|error| JsValue::from_str(&format!("texture admission: {error:?}")))?;
        self.adopt_pending_gpu();
        let Some(gpu) = self.gpu.as_mut() else { return Ok(false); };
        if gpu.resident_texture_binding_with_mips(&identity, space, semantic).is_some() {
            return Ok(true);
        }
        let mut rgba = vec![0; info.rgba8_bytes];
        let mut scratch = vec![0; scratch_len];
        decode_hmc_texture_rgba8_into(&resource, limits, &mut rgba, &mut scratch)
            .map_err(|error| JsValue::from_str(&format!("texture decode: {error:?}")))?;
        gpu.upload_resident_texture_rgba8_with_mips(identity, space, semantic,
            info.width, info.height, &rgba)
            .map_err(|error| JsValue::from_str(&format!("texture upload: {error:?}")))?;
        Ok(true)
    }
}
