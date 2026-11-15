//! PNG/KTX2 ingest for QualiaPortal `load_10d` materials.
//!
//! Texture payloads stay outside `.10d` (HMC/deps). This helper turns caller-
//! supplied encoded bytes into GPU RGBA, or skips when KTX2/BasisU transcode
//! is refused. Decode budgets are caller-owned and separate from the 42 MiB
//! Prolog/`SlgArena` sentinel.

use crate::container_10d::MaterialRecord;
use crate::render::asset_package::HmcTextureResource;
use crate::render::texture_decode::{
    decode_hmc_texture_rgba8_into, inspect_hmc_texture_plan, DecodedTextureInfo, TextureColorSpace,
    TextureDecodeError, TextureDecodeLimits, TextureDecodePlan,
};
use crate::render::texture_ingestion::{
    validate_image_signature, JPEG_SIGNATURE, PNG_SIGNATURE,
};
use crate::render::texture_ktx2::{Ktx2Document, KTX2_IDENTIFIER};
use sha2::{Digest, Sha256};

/// Portal image decode ceiling. Independent of the 42 MiB Prolog sentinel.
pub const PORTAL_TEXTURE_DECODE_LIMITS: TextureDecodeLimits = TextureDecodeLimits {
    max_encoded_bytes: 4 * 1024 * 1024,
    max_width: 4096,
    max_height: 4096,
    max_decoded_bytes: 4 * 1024 * 1024,
};

/// Content-addressed encoded texture ready to bind onto a MAT3/MAT4 slot.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackedPbrTexture {
    pub digest: [u8; 32],
    pub mime_type: &'static str,
    pub bytes: Vec<u8>,
}

/// Optional albedo + metallic-roughness PNG/KTX2 set for `compile_10d_pbr`.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct PackedPbrSet {
    pub albedo: Option<PackedPbrTexture>,
    pub metallic_roughness: Option<PackedPbrTexture>,
}

impl PackedPbrSet {
    /// Copy packed digests onto unbound (all-zero) material slots.
    pub fn apply_to(&self, material: &mut MaterialRecord) {
        if let Some(albedo) = &self.albedo {
            if material.base_color_texture == [0; 32] {
                material.base_color_texture = albedo.digest;
            }
        }
        if let Some(metallic_roughness) = &self.metallic_roughness {
            if material.metallic_roughness_texture == [0; 32] {
                material.metallic_roughness_texture = metallic_roughness.digest;
            }
        }
    }
}

/// Decoded tightly packed RGBA8 ready for GPU upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DecodedPortalRgba8 {
    pub digest: [u8; 32],
    pub width: u32,
    pub height: u32,
    pub rgba8: Vec<u8>,
    pub color_space: TextureColorSpace,
}

/// Ingest outcome. Transcode-only KTX2/BasisU is skipped, never panicked.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortalTextureIngest {
    Rgba8(DecodedPortalRgba8),
    SkippedTranscode,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PortalTextureError {
    Empty,
    UnknownFormat,
    Signature,
    Decode(TextureDecodeError),
}

impl std::fmt::Display for PortalTextureError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Empty => write!(f, "portal texture: empty payload"),
            Self::UnknownFormat => write!(f, "portal texture: unknown image magic"),
            Self::Signature => write!(f, "portal texture: MIME/magic mismatch"),
            Self::Decode(error) => write!(f, "portal texture: {error}"),
        }
    }
}

impl std::error::Error for PortalTextureError {}

impl From<TextureDecodeError> for PortalTextureError {
    fn from(value: TextureDecodeError) -> Self {
        Self::Decode(value)
    }
}

/// Identify PNG / JPEG / KTX2 from the file magic. No new container magics.
pub fn detect_texture_mime(bytes: &[u8]) -> Option<&'static str> {
    if bytes.len() >= 8 && bytes[..8] == PNG_SIGNATURE {
        Some("image/png")
    } else if bytes.len() >= 12 && bytes[..12] == KTX2_IDENTIFIER {
        Some("image/ktx2")
    } else if bytes.len() >= 3 && bytes[..3] == JPEG_SIGNATURE {
        Some("image/jpeg")
    } else {
        None
    }
}

fn pack_one(bytes: &[u8]) -> Result<PackedPbrTexture, PortalTextureError> {
    if bytes.is_empty() {
        return Err(PortalTextureError::Empty);
    }
    let mime_type = detect_texture_mime(bytes).ok_or(PortalTextureError::UnknownFormat)?;
    validate_image_signature(mime_type, bytes).map_err(|_| PortalTextureError::Signature)?;
    Ok(PackedPbrTexture {
        digest: Sha256::digest(bytes).into(),
        mime_type,
        bytes: bytes.to_vec(),
    })
}

/// Hash and signature-check encoded PBR maps. Payloads stay encoded (HMC-shaped).
pub fn pack_pbr_set(
    albedo: Option<&[u8]>,
    metallic_roughness: Option<&[u8]>,
) -> Result<PackedPbrSet, PortalTextureError> {
    Ok(PackedPbrSet {
        albedo: albedo.map(pack_one).transpose()?,
        metallic_roughness: metallic_roughness.map(pack_one).transpose()?,
    })
}

/// Decode PNG/JPEG/uncompressed KTX2 to RGBA8. BasisU/block KTX2 is skipped.
pub fn ingest_portal_texture(
    mime: Option<&str>,
    bytes: &[u8],
) -> Result<PortalTextureIngest, PortalTextureError> {
    if bytes.is_empty() {
        return Err(PortalTextureError::Empty);
    }
    let mime_type = match mime {
        Some(declared) if !declared.is_empty() => declared,
        _ => detect_texture_mime(bytes).ok_or(PortalTextureError::UnknownFormat)?,
    };
    validate_image_signature(mime_type, bytes).map_err(|_| PortalTextureError::Signature)?;
    let digest: [u8; 32] = Sha256::digest(bytes).into();
    let resource = HmcTextureResource {
        digest,
        mime_type,
        bytes,
    };

    if mime_type
        .split(';')
        .next()
        .map(str::trim)
        .is_some_and(|m| m.eq_ignore_ascii_case("image/ktx2"))
    {
        // Structural parse first so a corrupt KTX2 is a decode error, not a skip.
        let _document = Ktx2Document::parse(bytes)
            .map_err(|_| PortalTextureError::Decode(TextureDecodeError::InvalidImage))?;
    }

    match inspect_hmc_texture_plan(&resource, PORTAL_TEXTURE_DECODE_LIMITS)? {
        TextureDecodePlan::RequiresTranscode(_) => Ok(PortalTextureIngest::SkippedTranscode),
        TextureDecodePlan::DecodeRgba8 {
            info,
            scratch_bytes,
        } => Ok(PortalTextureIngest::Rgba8(decode_rgba8(
            &resource,
            digest,
            info,
            scratch_bytes,
        )?)),
    }
}

fn decode_rgba8(
    resource: &HmcTextureResource<'_>,
    digest: [u8; 32],
    info: DecodedTextureInfo,
    scratch_bytes: usize,
) -> Result<DecodedPortalRgba8, PortalTextureError> {
    let mut output = vec![0; info.rgba8_bytes];
    let mut scratch = vec![0; scratch_bytes];
    let decoded = decode_hmc_texture_rgba8_into(
        resource,
        PORTAL_TEXTURE_DECODE_LIMITS,
        &mut output,
        &mut scratch,
    )?;
    Ok(DecodedPortalRgba8 {
        digest,
        width: decoded.width,
        height: decoded.height,
        rgba8: output,
        color_space: decoded.color_space,
    })
}

/// True when `digest` occupies any MAT texture slot.
pub fn material_references_digest(material: &MaterialRecord, digest: [u8; 32]) -> bool {
    digest != [0; 32]
        && (material.base_color_texture == digest
            || material.normal_texture == digest
            || material.metallic_roughness_texture == digest
            || material.occlusion_texture == digest
            || material.emissive_texture == digest
            || material.stylized_ramp_texture == digest)
}

pub use crate::render::material_compile::compile_10d_pbr;

#[cfg(feature = "gpu-runtime")]
fn gpu_color_space(
    decoded: TextureColorSpace,
    fallback: crate::render::gpu::TextureColorSpace,
) -> crate::render::gpu::TextureColorSpace {
    match decoded {
        TextureColorSpace::Linear => crate::render::gpu::TextureColorSpace::Linear,
        TextureColorSpace::Srgb => crate::render::gpu::TextureColorSpace::Srgb,
        TextureColorSpace::Unknown => fallback,
    }
}

#[cfg(feature = "gpu-runtime")]
fn slot_for_digest(
    materials: &[MaterialRecord],
    digest: [u8; 32],
) -> (
    crate::render::gpu::TextureColorSpace,
    crate::render::gpu::TextureMipSemantic,
) {
    use crate::container_10d::OpacityMode;
    use crate::render::gpu::{TextureColorSpace, TextureMipSemantic};
    for material in materials {
        if material.base_color_texture == digest {
            let mip = if material.opacity_mode == OpacityMode::Mask {
                TextureMipSemantic::alpha_mask(material.alpha_cutoff)
                    .unwrap_or(TextureMipSemantic::Color)
            } else {
                TextureMipSemantic::Color
            };
            return (TextureColorSpace::Srgb, mip);
        }
        if material.emissive_texture == digest || material.stylized_ramp_texture == digest {
            return (TextureColorSpace::Srgb, TextureMipSemantic::Color);
        }
        if material.normal_texture == digest {
            return (TextureColorSpace::Linear, TextureMipSemantic::Normal);
        }
        if material.metallic_roughness_texture == digest || material.occlusion_texture == digest
        {
            return (TextureColorSpace::Linear, TextureMipSemantic::LinearData);
        }
    }
    (TextureColorSpace::Srgb, TextureMipSemantic::Color)
}

/// Upload decoded RGBA into the TextureResidency ledger (not the Prolog sentinel).
#[cfg(feature = "gpu-runtime")]
pub fn upload_portal_texture_rgba8(
    gpu: &mut crate::render::gpu::PortalGpu,
    materials: &[MaterialRecord],
    decoded: &DecodedPortalRgba8,
) -> Result<(), String> {
    let (fallback_space, semantic) = slot_for_digest(materials, decoded.digest);
    gpu.upload_resident_texture_rgba8_with_mips(
        decoded.digest,
        gpu_color_space(decoded.color_space, fallback_space),
        semantic,
        decoded.width,
        decoded.height,
        &decoded.rgba8,
    )
    .map_err(|error| format!("portal texture upload: {error}"))
}
