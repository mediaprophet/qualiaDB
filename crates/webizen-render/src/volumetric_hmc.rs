//! HMC asset and texture streaming admission for the volumetric renderer.

use crate::volumetric_texture::coarse_texture_level;
use qualia_core_db::container_10d::{
    decode_material_section_into, decode_mesh_section, material_section_counts, Container10dHeader,
    MaterialRecord, OpacityMode, SectionDescriptor, SectionType, SubmeshRange,
};
use qualia_core_db::render::gpu::{TextureColorSpace, TextureMipSemantic, TextureUploadError};
use qualia_core_db::render::texture_ktx2::Ktx2Document;
use qualia_core_db::render::texture_stream_policy::{
    plan_texture_residency_partial, TextureStreamAction, TextureStreamActionKind,
    TextureStreamBudget, TextureStreamDemand,
};
use qualia_core_db::render::texture_streaming_plan::{
    decode_rgba8_ktx2_mip_into, rgba8_ktx2_mip_candidate, reserve_texture_mip,
    select_texture_mip, Ktx2MipCandidateError, TextureMipBackendCost, TextureMipBudget,
    TextureMipCandidate,
};
use std::collections::{BTreeMap, BTreeSet};

#[path = "volumetric_hmc_stream.rs"]
mod volumetric_hmc_stream;

pub use self::volumetric_hmc_stream::{
    HmcTextureResidencyRequest, HmcTextureStreamApplyRequest, HmcTextureStreamPlan,
};
use self::volumetric_hmc_stream::HmcTextureStreamEntry;
use self::volumetric_hmc_stream::{
    HMC_TEXTURE_STREAM_MAX_ENTRIES, HMC_TEXTURE_STREAM_STATE_MAX_BYTES,
};
pub(crate) use self::volumetric_hmc_stream::HmcTextureStreamState;

const HMC_KTX2_MAX_DECODED_BYTES: u64 = 256 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct TextureUse {
    pub(crate) color_space: TextureColorSpace,
    pub(crate) mip_semantic: TextureMipSemantic,
}

#[derive(Clone, Copy)]
pub(crate) struct HmcTexturePlanEntry {
    pub(crate) semantic_id: u64,
    pub(crate) digest: [u8; 32],
    pub(crate) texture_use: TextureUse,
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) mip_count: u8,
    pub(crate) supported_mips: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ktx2HmcFallback {
    Parse,
    Candidate(Ktx2MipCandidateError),
    ColorSpaceMismatch,
    NoAffordableLevel,
    Decode,
}

fn note_ktx2_fallback(
    report: &mut super::HmcTextureAdmissionReport,
    fallback: Ktx2HmcFallback,
    mip_bytes: u64,
) {
    report.deferred_interpretations += 1;
    if matches!(
        fallback,
        Ktx2HmcFallback::Parse
            | Ktx2HmcFallback::Candidate(_)
            | Ktx2HmcFallback::ColorSpaceMismatch
    ) {
        report.capability_refusals += 1;
    }
    if matches!(fallback, Ktx2HmcFallback::NoAffordableLevel | Ktx2HmcFallback::Decode) {
        report.deferred_mips += 1;
        report.deferred_upload_bytes = report.deferred_upload_bytes.saturating_add(mip_bytes);
    }
}

fn ktx2_backend_cost(
    document: &Ktx2Document<'_>,
    level_index: usize,
) -> Result<TextureMipBackendCost, Ktx2MipCandidateError> {
    let mut width = document
        .pixel_width
        .checked_shr(level_index as u32)
        .unwrap_or(0)
        .max(1);
    let mut height = document
        .pixel_height
        .checked_shr(level_index as u32)
        .unwrap_or(0)
        .max(1);
    let mut resident_bytes = 0u64;
    loop {
        let level_bytes = u64::from(width)
            .checked_mul(u64::from(height))
            .and_then(|pixels| pixels.checked_mul(4))
            .ok_or(Ktx2MipCandidateError::SizeOverflow)?;
        resident_bytes = resident_bytes
            .checked_add(level_bytes)
            .ok_or(Ktx2MipCandidateError::SizeOverflow)?;
        if width == 1 && height == 1 {
            break;
        }
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    let upload_staging_bytes = document
        .level(level_index)
        .ok_or(Ktx2MipCandidateError::InvalidLevel)?
        .bytes
        .len() as u64;
    Ok(TextureMipBackendCost {
        gpu_resident_bytes: resident_bytes,
        upload_staging_bytes,
    })
}

fn ktx2_rgba8_candidates(
    document: &Ktx2Document<'_>,
) -> Result<Vec<TextureMipCandidate>, Ktx2MipCandidateError> {
    let mut candidates = Vec::new();
    candidates
        .try_reserve_exact(document.level_count())
        .map_err(|_| Ktx2MipCandidateError::SizeOverflow)?;
    for level_index in 0..document.level_count() {
        let backend_cost = ktx2_backend_cost(document, level_index)?;
        if let Ok(candidate) = rgba8_ktx2_mip_candidate(document, level_index, backend_cost) {
            candidates.push(candidate);
        }
    }
    if candidates.is_empty() {
        return Err(rgba8_ktx2_mip_candidate(
            document,
            0,
            TextureMipBackendCost {
                gpu_resident_bytes: 0,
                upload_staging_bytes: 0,
            },
        )
        .err()
        .unwrap_or(Ktx2MipCandidateError::InvalidLevel));
    }
    Ok(candidates)
}

fn select_hmc_ktx2_level(
    document: &Ktx2Document<'_>,
    texture_use: TextureUse,
    request: HmcTextureStreamRequest,
    remaining: TextureMipBudget,
) -> Result<TextureMipCandidate, Ktx2HmcFallback> {
    let candidates = ktx2_rgba8_candidates(document).map_err(Ktx2HmcFallback::Candidate)?;
    let is_srgb = document.vk_format == 43;
    if (texture_use.color_space == TextureColorSpace::Srgb) != is_srgb {
        return Err(Ktx2HmcFallback::ColorSpaceMismatch);
    }
    let projected_width = if request.projected_width == 0 {
        document.pixel_width
    } else {
        request.projected_width
    };
    let projected_height = if request.projected_height == 0 {
        document.pixel_height.max(1)
    } else {
        request.projected_height
    };
    select_texture_mip(
        &candidates,
        projected_width,
        projected_height,
        remaining,
    )
    .map(|selection| selection.candidate)
    .ok_or(Ktx2HmcFallback::NoAffordableLevel)
}

/// Request-bearing parameters for texture streaming and view-footprint selection.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct HmcTextureStreamRequest {
    pub budget: TextureStreamBudget,
    /// Projected on-screen sampling footprint width (0 = unconstrained, finest authored).
    pub projected_width: u32,
    /// Projected on-screen sampling footprint height (0 = unconstrained, finest authored).
    pub projected_height: u32,
}

impl Default for HmcTextureStreamRequest {
    fn default() -> Self {
        Self {
            budget: TextureStreamBudget {
                max_resident_bytes: u64::MAX,
                max_upload_bytes: u64::MAX,
            },
            projected_width: 0,
            projected_height: 0,
        }
    }
}

pub(crate) fn is_ktx2_mime(mime: &str) -> bool {
    mime.split(';')
        .next()
        .map(str::trim)
        .is_some_and(|m| m.eq_ignore_ascii_case("image/ktx2"))
}

const HMC_WATER_MAX_VERTICES: usize = 1_048_576;
const HMC_WATER_MAX_INDICES: usize = 3_145_728;
const HMC_WATER_DEFAULT_MAX_GEOMETRY_BYTES: u64 = 64 * 1024 * 1024;

/// Admission and presentation policy for a persistent HMC water surface.
/// Geometry admission is independent from texture fallback: a refused water
/// surface must never evict or replace an already resident surface.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HmcWaterLoadPolicy {
    pub max_geometry_bytes: u64,
    /// 0 = lower tier, 1 = balanced, 2 = cinematic bounded response.
    pub preferred_quality: u32,
}

impl Default for HmcWaterLoadPolicy {
    fn default() -> Self {
        Self {
            max_geometry_bytes: HMC_WATER_DEFAULT_MAX_GEOMETRY_BYTES,
            preferred_quality: 2,
        }
    }
}

pub(crate) fn validate_hmc_water_geometry(
    positions: &[[f32; 3]],
    triangles: &[[u32; 3]],
    max_bytes: u64,
) -> Result<u64, String> {
    if positions.is_empty()
        || triangles.is_empty()
        || positions.len() > HMC_WATER_MAX_VERTICES
        || triangles.len().saturating_mul(3) > HMC_WATER_MAX_INDICES
    {
        return Err("HMC water geometry exceeds bounded admission limits".to_string());
    }
    if positions
        .iter()
        .any(|position| position.iter().any(|value| !value.is_finite()))
    {
        return Err("HMC water geometry contains non-finite positions".to_string());
    }
    if triangles.iter().any(|triangle| {
        triangle
            .iter()
            .any(|&index| index as usize >= positions.len())
    }) {
        return Err("HMC water geometry contains an out-of-range index".to_string());
    }
    let bytes = (positions.len() as u64)
        .checked_mul(20)
        .and_then(|value| value.checked_add((triangles.len() as u64).checked_mul(3 * 4)?))
        .ok_or_else(|| "HMC water geometry byte size overflow".to_string())?;
    if bytes > max_bytes {
        return Err(format!(
            "HMC water geometry exceeds admission budget: {bytes} > {max_bytes} bytes"
        ));
    }
    Ok(bytes)
}

pub(crate) fn select_hmc_water_quality(
    triangle_count: usize,
    geometry_bytes: u64,
    preferred_quality: u32,
) -> u32 {
    let preferred_quality = preferred_quality.min(2);
    if preferred_quality == 0 {
        return 0;
    }
    // Larger surfaces retain authoritative depth and stable normals but use
    // the lower-cost bounded reflection/foam response.
    if triangle_count > 750_000 || geometry_bytes > 32 * 1024 * 1024 {
        0
    } else if triangle_count > 250_000 || geometry_bytes > 12 * 1024 * 1024 {
        preferred_quality.min(1)
    } else {
        preferred_quality
    }
}

fn extract_texture_uses(
    asset_bytes: &[u8],
    descs: &[SectionDescriptor],
) -> Result<BTreeMap<[u8; 32], BTreeSet<TextureUse>>, String> {
    let mut uses = BTreeMap::<[u8; 32], BTreeSet<TextureUse>>::new();
    let mat_desc = descs
        .iter()
        .find(|d| d.typ() == Some(SectionType::Materials));
    let Some(descriptor) = mat_desc else {
        return Ok(uses);
    };
    let start = descriptor.byte_offset as usize;
    let end = start
        .checked_add(descriptor.byte_length as usize)
        .filter(|&end| end <= asset_bytes.len())
        .ok_or_else(|| "10d MAT1 section is outside asset bytes".to_string())?;
    let payload = &asset_bytes[start..end];
    let (mat_count, range_count) =
        material_section_counts(payload).map_err(|e| format!("10d MAT1 header: {e}"))?;
    let mut materials = vec![MaterialRecord::legacy_default(); mat_count];
    let first_mat_id = materials
        .first()
        .ok_or_else(|| "10d MAT1 section has no materials".to_string())?
        .id;
    let mut ranges = vec![
        SubmeshRange {
            first_index: 0,
            index_count: 3,
            material_id: first_mat_id,
            semantic_id: 0,
        };
        range_count
    ];
    let mesh_index_count = descs
        .iter()
        .find(|d| d.typ() == Some(SectionType::QuantizedMesh))
        .map(|d| {
            let start = d.byte_offset as usize;
            let end = start + d.byte_length as usize;
            decode_mesh_section(&asset_bytes[start..end])
                .map(|m| m.triangles.len().saturating_mul(3) as u32)
                .map_err(|e| format!("10d mesh decode: {e}"))
        })
        .transpose()?
        .ok_or_else(|| "10d asset has no QuantizedMesh section".to_string())?;

    decode_material_section_into(payload, mesh_index_count, &mut materials, &mut ranges)
        .map_err(|e| format!("10d MAT1 decode: {e}"))?;

    let mut add_use =
        |digest: [u8; 32], color_space: TextureColorSpace, mip_semantic: TextureMipSemantic| {
            if digest != [0; 32] {
                uses.entry(digest).or_default().insert(TextureUse {
                    color_space,
                    mip_semantic,
                });
            }
        };

    for mat in &materials {
        let base_mip = if mat.opacity_mode == OpacityMode::Mask {
            TextureMipSemantic::alpha_mask(mat.alpha_cutoff).unwrap_or(TextureMipSemantic::Color)
        } else {
            TextureMipSemantic::Color
        };
        add_use(mat.base_color_texture, TextureColorSpace::Srgb, base_mip);
        add_use(
            mat.emissive_texture,
            TextureColorSpace::Srgb,
            TextureMipSemantic::Color,
        );
        add_use(
            mat.normal_texture,
            TextureColorSpace::Linear,
            TextureMipSemantic::Normal,
        );
        add_use(
            mat.metallic_roughness_texture,
            TextureColorSpace::Linear,
            TextureMipSemantic::LinearData,
        );
        add_use(
            mat.occlusion_texture,
            TextureColorSpace::Linear,
            TextureMipSemantic::LinearData,
        );
        add_use(
            mat.stylized_ramp_texture,
            TextureColorSpace::Srgb,
            TextureMipSemantic::Color,
        );
    }
    Ok(uses)
}

fn note_refusal(report: &mut super::HmcTextureAdmissionReport, mip_count: usize, mip_bytes: u64) {
    report.admitted_mips = report.admitted_mips.saturating_sub(mip_count);
    report.deferred_mips += mip_count;
    report.admitted_upload_bytes = report.admitted_upload_bytes.saturating_sub(mip_bytes);
    report.deferred_upload_bytes = report.deferred_upload_bytes.saturating_add(mip_bytes);
    report.deferred_interpretations += 1;
}

fn rgba8_resident_bytes(width: u32, height: u32, first_mip: u8) -> Result<u64, String> {
    let mut width = width.max(1);
    let mut height = height.max(1);
    for _ in 0..first_mip {
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
    let mut total = 0u64;
    loop {
        total = total
            .checked_add(
                u64::from(width)
                    .checked_mul(u64::from(height))
                    .and_then(|pixels| pixels.checked_mul(4))
                    .ok_or_else(|| "texture resident byte size overflow".to_string())?,
            )
            .ok_or_else(|| "texture resident byte size overflow".to_string())?;
        if width == 1 && height == 1 {
            return Ok(total);
        }
        width = (width / 2).max(1);
        height = (height / 2).max(1);
    }
}

fn build_texture_stream_state(
    hmc_bytes: &[u8],
    asset_bytes: &[u8],
    entries: &[HmcTexturePlanEntry],
    loaded_residency: &BTreeMap<([u8; 32], TextureUse), (u8, u64)>,
) -> Result<HmcTextureStreamState, String> {
    if entries.len() > HMC_TEXTURE_STREAM_MAX_ENTRIES {
        return Err(format!(
            "HMC texture stream entry count exceeds bounded limit: {} > {}",
            entries.len(), HMC_TEXTURE_STREAM_MAX_ENTRIES
        ));
    }
    let retained_bytes = hmc_bytes
        .len()
        .checked_add(asset_bytes.len())
        .ok_or_else(|| "HMC texture stream state size overflow".to_string())?;
    if retained_bytes > HMC_TEXTURE_STREAM_STATE_MAX_BYTES {
        return Err(format!(
            "HMC texture stream state exceeds bounded retention limit: {} > {} bytes",
            retained_bytes, HMC_TEXTURE_STREAM_STATE_MAX_BYTES
        ));
    }
    let mut retained_bundle = Vec::new();
    retained_bundle
        .try_reserve_exact(hmc_bytes.len())
        .map_err(|_| "HMC texture stream state allocation refused".to_string())?;
    retained_bundle.extend_from_slice(hmc_bytes);
    let mut retained_asset = Vec::new();
    retained_asset
        .try_reserve_exact(asset_bytes.len())
        .map_err(|_| "HMC texture asset retention allocation refused".to_string())?;
    retained_asset.extend_from_slice(asset_bytes);
    let mut state_entries = Vec::new();
    state_entries
        .try_reserve_exact(entries.len())
        .map_err(|_| "HMC texture stream entry allocation refused".to_string())?;
    for entry in entries {
        let (resident_finest_mip, resident_bytes) = loaded_residency
            .get(&(entry.digest, entry.texture_use))
            .copied()
            .map_or((None, 0), |(mip, bytes)| (Some(mip), bytes));
        state_entries.push(HmcTextureStreamEntry {
            semantic_id: entry.semantic_id,
            digest: entry.digest,
            texture_use: entry.texture_use,
            width: entry.width,
            height: entry.height,
            mip_count: entry.mip_count,
            supported_mips: entry.supported_mips,
            resident_finest_mip,
            resident_bytes,
        });
    }
    Ok(HmcTextureStreamState {
        bundle_bytes: retained_bundle,
        asset_bytes: retained_asset,
        entries: state_entries,
        generation: 0,
    })
}

fn evict_replaced_asset_textures(
    renderer: &mut super::VolumetricRenderer,
    next_entries: &[HmcTexturePlanEntry],
) {
    let Some(previous_entries) = renderer
        .texture_stream_state
        .as_ref()
        .map(|state| state.entries.clone())
    else {
        return;
    };
    for entry in &previous_entries {
        if entry.resident_finest_mip.is_some()
            && !next_entries.iter().any(|next| {
                next.digest == entry.digest && next.texture_use == entry.texture_use
            })
        {
            renderer.inner.evict_resident_texture_with_mips(
                &entry.digest,
                entry.texture_use.color_space,
                entry.texture_use.mip_semantic,
            );
        }
    }
}

/// Load an HMC `.10d` asset while admitting texture mips according to footprint and budget.
pub fn load_hmc_asset_with_texture_request(
    renderer: &mut super::VolumetricRenderer,
    hmc_bytes: &[u8],
    asset_key: &str,
    request: HmcTextureStreamRequest,
) -> Result<((u32, u32, f32), super::HmcTextureAdmissionReport), String> {
    if hmc_bytes.len() > HMC_TEXTURE_STREAM_STATE_MAX_BYTES {
        return Err(format!(
            "HMC texture stream state exceeds bounded retention limit: {} > {} bytes",
            hmc_bytes.len(), HMC_TEXTURE_STREAM_STATE_MAX_BYTES
        ));
    }
    let bundle = qualia_core_db::bundle::BundleReader::parse(hmc_bytes)
        .map_err(|e| format!("HMC bundle: {e}"))?;
    let asset_bytes = bundle
        .get(asset_key)
        .ok_or_else(|| format!("HMC asset is missing: {asset_key}"))?;
    let retained_bytes = hmc_bytes
        .len()
        .checked_add(asset_bytes.len())
        .ok_or_else(|| "HMC texture stream state size overflow".to_string())?;
    if retained_bytes > HMC_TEXTURE_STREAM_STATE_MAX_BYTES {
        return Err(format!(
            "HMC texture stream state exceeds bounded retention limit: {} > {} bytes",
            retained_bytes, HMC_TEXTURE_STREAM_STATE_MAX_BYTES
        ));
    }
    if !bundle.verify_entry(asset_key) {
        return Err(format!("HMC asset digest check failed: {asset_key}"));
    }
    let entry = bundle
        .entry(asset_key)
        .ok_or_else(|| format!("HMC asset index entry is missing: {asset_key}"))?;
    if entry.kind != "10d" {
        return Err(format!(
            "HMC asset {asset_key} has kind {:?}, expected 10d",
            entry.kind
        ));
    }

    let header = Container10dHeader::parse(asset_bytes).map_err(|e| format!("10d header: {e}"))?;
    if qualia_core_db::container_10d::compute_whole_file_crc32c(asset_bytes) != header.header_crc32c
    {
        return Err("10d whole-file integrity check failed".to_string());
    }
    let descs = qualia_core_db::container_10d::parse_section_table(asset_bytes, &header)
        .map_err(|e| format!("10d section table: {e}"))?;
    let texture_uses = extract_texture_uses(asset_bytes, &descs)?;

    let mut plan_entries = Vec::new();
    let mut ktx2_entries = Vec::new();
    let mut demands = Vec::new();
    let mut next_semantic_id = 1u64;
    let mut report = super::HmcTextureAdmissionReport::default();
    let mut loaded_residency = BTreeMap::<([u8; 32], TextureUse), (u8, u64)>::new();
    for (digest, uses) in &texture_uses {
        let resource =
            qualia_core_db::render::asset_package::resolve_hmc_texture_resource(&bundle, digest)
                .map_err(|e| format!("HMC texture resolution: {e}"))?;

        // KTX2 resources are admitted from their indexed authored levels. Do not route them
        // through the generic image inspector: that inspector intentionally validates level zero,
        // while coarse-first streaming must be able to start from a later valid level.
        if is_ktx2_mime(resource.mime_type) {
            let document = match Ktx2Document::parse(resource.bytes) {
                Ok(document) => document,
                Err(_) => {
                    report.requested_interpretations += uses.len();
                    for _ in uses {
                        note_ktx2_fallback(&mut report, Ktx2HmcFallback::Parse, 0);
                    }
                    continue;
                }
            };
            let candidates = ktx2_rgba8_candidates(&document);
            let candidate_error = candidates.as_ref().err().copied();
            let supported_mips = candidates.as_ref().ok().map(|candidates| {
                candidates.iter().fold(0u32, |mask, candidate| {
                    mask | (1u32 << candidate.level)
                })
            });
            for texture_use in uses {
                report.requested_interpretations += 1;
                if renderer
                    .inner
                    .resident_texture_binding_with_mips(
                        digest,
                        texture_use.color_space,
                        texture_use.mip_semantic,
                    )
                    .is_some()
                {
                    report.resident_interpretations += 1;
                    continue;
                }
                if let Some(error) = candidate_error {
                    note_ktx2_fallback(
                        &mut report,
                        Ktx2HmcFallback::Candidate(error),
                        0,
                    );
                    continue;
                }
                if (texture_use.color_space == TextureColorSpace::Srgb)
                    != (document.vk_format == 43)
                {
                    note_ktx2_fallback(&mut report, Ktx2HmcFallback::ColorSpaceMismatch, 0);
                    continue;
                }
                let semantic_id = next_semantic_id;
                next_semantic_id = next_semantic_id
                    .checked_add(1)
                    .ok_or_else(|| "overflow".to_string())?;
                ktx2_entries.push(HmcTexturePlanEntry {
                    semantic_id,
                    digest: *digest,
                    texture_use: *texture_use,
                    width: document.pixel_width,
                    height: document.pixel_height.max(1),
                    mip_count: u8::try_from(document.level_count())
                        .map_err(|_| "KTX2 mip count exceeds u8".to_string())?,
                    supported_mips: supported_mips.unwrap_or(0),
                });
            }
            continue;
        }

        let (info, _) =
            match qualia_core_db::render::texture_decode::inspect_hmc_texture_requirements(
                &resource,
                qualia_core_db::render::texture_decode::TextureDecodeLimits::default(),
            ) {
                Ok(reqs) => reqs,
                Err(_) => {
                    report.requested_interpretations += uses.len();
                    report.deferred_interpretations += uses.len();
                    continue;
                }
            };
        let max_dim = info.width.max(info.height);
        let mip_count = (u32::BITS - max_dim.leading_zeros()) as u8;

        let desired_finest_mip = if request.projected_width > 0 && request.projected_height > 0 {
            let mut level = 0u8;
            while level + 1 < mip_count {
                let next_w = (info.width >> (level + 1)).max(1);
                let next_h = (info.height >> (level + 1)).max(1);
                if next_w >= request.projected_width && next_h >= request.projected_height {
                    level += 1;
                } else {
                    break;
                }
            }
            level
        } else {
            0
        };

        for texture_use in uses {
            report.requested_interpretations += 1;
            if renderer
                .inner
                .resident_texture_binding_with_mips(
                    digest,
                    texture_use.color_space,
                    texture_use.mip_semantic,
                )
                .is_some()
            {
                report.resident_interpretations += 1;
                continue;
            }
            let semantic_id = next_semantic_id;
            next_semantic_id = next_semantic_id
                .checked_add(1)
                .ok_or_else(|| "overflow".to_string())?;
            let importance = match texture_use.mip_semantic {
                TextureMipSemantic::Color => 250,
                TextureMipSemantic::AlphaMask { .. } => 240,
                TextureMipSemantic::Normal => 220,
                TextureMipSemantic::LinearData => 200,
            };
            demands.push(TextureStreamDemand {
                semantic_id,
                width: info.width,
                height: info.height,
                mip_count,
                block_width: 1,
                block_height: 1,
                bytes_per_block: 4,
                desired_finest_mip,
                resident_finest_mip: None,
                resident_bytes: 0,
                requested: true,
                pinned: false,
                visible: true,
                importance,
                distance_key: 0,
                last_used_frame: 0,
            });
            plan_entries.push(HmcTexturePlanEntry {
                semantic_id,
                digest: *digest,
                texture_use: *texture_use,
                width: info.width,
                height: info.height,
                mip_count,
                supported_mips: if mip_count == 32 {
                    u32::MAX
                } else {
                    (1u32 << mip_count) - 1
                },
            });
        }
    }

    let action_capacity = demands.iter().try_fold(0usize, |total, demand| {
        total
            .checked_add(demand.mip_count as usize)
            .ok_or_else(|| "overflow".to_string())
    })?;
    let mut actions = Vec::new();
    actions
        .try_reserve_exact(action_capacity)
        .map_err(|_| "OOM".to_string())?;
    actions.resize(
        action_capacity,
        TextureStreamAction {
            semantic_id: 0,
            kind: TextureStreamActionKind::RequestMip,
            mip_level: 0,
            bytes: 0,
        },
    );
    let plan = plan_texture_residency_partial(&demands, request.budget, &mut actions)
        .map_err(|e| format!("HMC texture budget plan: {e:?}"))?;
    report.admitted_mips = plan.admitted_mip_count;
    report.deferred_mips = plan.deferred_mip_count;
    report.admitted_upload_bytes = plan.admitted_upload_bytes;
    report.deferred_upload_bytes = plan.deferred_upload_bytes;
    let actions = &actions[..plan.action_count];

    let mut newly_resident = Vec::new();
    let upload_result = (|| -> Result<(), String> {
        // KTX2 uses the indexed-level planner directly. This keeps the selected authored level
        // independent of the generic coarse-prefix policy and, importantly, makes the decode
        // operate on only the level that was admitted by footprint and budget.
        // The indexed-level planner shares the same per-load budgets as the legacy image path.
        // Account for already-admitted non-KTX2 requests before selecting any KTX2 level so a
        // mixed asset cannot independently spend the full budget twice.
        let mut ktx2_budget = TextureMipBudget {
            decoded_cpu_bytes: HMC_KTX2_MAX_DECODED_BYTES,
            gpu_resident_bytes: request
                .budget
                .max_resident_bytes
                .saturating_sub(plan.projected_resident_bytes),
            upload_staging_bytes: request
                .budget
                .max_upload_bytes
                .saturating_sub(plan.admitted_upload_bytes),
            per_frame_upload_bytes: request
                .budget
                .max_upload_bytes
                .saturating_sub(plan.admitted_upload_bytes),
        };
        for entry in ktx2_entries.iter().copied() {
            let resource = qualia_core_db::render::asset_package::resolve_hmc_texture_resource(
                &bundle,
                &entry.digest,
            )
            .map_err(|e| format!("HMC texture resolution: {e}"))?;
            let document = Ktx2Document::parse(resource.bytes)
                .map_err(|_| format!("HMC KTX2 fallback: {:?}", Ktx2HmcFallback::Parse))?;
            let candidate = match select_hmc_ktx2_level(
                &document,
                entry.texture_use,
                request,
                ktx2_budget,
            ) {
                Ok(candidate) => candidate,
                Err(fallback) => {
                    note_ktx2_fallback(&mut report, fallback, 0);
                    continue;
                }
            };
            let mut candidate_budget = ktx2_budget;
            if !reserve_texture_mip(candidate, &mut candidate_budget) {
                note_ktx2_fallback(
                    &mut report,
                    Ktx2HmcFallback::NoAffordableLevel,
                    candidate.upload_bytes,
                );
                continue;
            }
            let output_len = usize::try_from(candidate.decoded_cpu_bytes)
                .map_err(|_| "HMC KTX2 selected level is too large".to_string())?;
            let mut selected_level = vec![0u8; output_len];
            if decode_rgba8_ktx2_mip_into(&document, candidate.level as usize, &mut selected_level)
                .is_err()
            {
                note_ktx2_fallback(
                    &mut report,
                    Ktx2HmcFallback::Decode,
                    candidate.upload_bytes,
                );
                continue;
            }
            ktx2_budget = candidate_budget;
            match renderer
                .inner
                .upload_resident_texture_rgba8_at_mip_with_mips(
                    entry.digest,
                    entry.texture_use.color_space,
                    entry.texture_use.mip_semantic,
                    entry.width,
                    entry.height,
                    candidate.level,
                    &selected_level,
                )
            {
                Ok(_) => {
                    newly_resident.push((entry.digest, entry.texture_use));
                    loaded_residency.insert(
                        (entry.digest, entry.texture_use),
                        (candidate.level as u8, candidate.gpu_resident_bytes),
                    );
                    report.resident_interpretations += 1;
                    report.admitted_mips += 1;
                    report.admitted_upload_bytes = report
                        .admitted_upload_bytes
                        .saturating_add(candidate.upload_bytes);
                }
                Err(TextureUploadError::GpuBudgetRefused)
                | Err(TextureUploadError::UploadBudgetRefused) => {
                    note_ktx2_fallback(
                        &mut report,
                        Ktx2HmcFallback::NoAffordableLevel,
                        candidate.upload_bytes,
                    );
                }
                Err(e) => return Err(format!("HMC texture upload: {e}")),
            }
        }

        for digest in texture_uses.keys() {
            let mut selected = Vec::new();
            for entry in plan_entries.iter().filter(|e| e.digest == *digest) {
                let mut first_mip = None;
                let mut mip_bytes = 0u64;
                let mut mip_count = 0usize;
                for action in actions.iter().filter(|a| {
                    a.semantic_id == entry.semantic_id
                        && a.kind == TextureStreamActionKind::RequestMip
                }) {
                    first_mip =
                        Some(first_mip.map_or(action.mip_level, |c: u8| c.min(action.mip_level)));
                    mip_bytes = mip_bytes.saturating_add(action.bytes);
                    mip_count += 1;
                }
                if let Some(first_mip) = first_mip {
                    selected.push((*entry, first_mip, mip_bytes, mip_count));
                } else {
                    report.deferred_interpretations += 1;
                }
            }
            if selected.is_empty() {
                continue;
            }
            let resource = qualia_core_db::render::asset_package::resolve_hmc_texture_resource(
                &bundle, digest,
            )
            .map_err(|e| format!("HMC texture resolution: {e}"))?;

            // Fallback: Full base decode + CPU downsample for PNG and JPEG.
            let (rgba8, _) = match qualia_core_db::render::texture_decode::decode_hmc_texture_rgba8(
                &resource,
                qualia_core_db::render::texture_decode::TextureDecodeLimits::default(),
            ) {
                Ok(decoded) => decoded,
                Err(_) => {
                    for (_, _, mip_bytes, mip_count) in &selected {
                        note_refusal(&mut report, *mip_count, *mip_bytes);
                    }
                    continue;
                }
            };
            for (entry, first_mip, mip_bytes, mip_count) in selected {
                let upload = if first_mip == 0 {
                    renderer.inner.upload_resident_texture_rgba8_with_mips(
                        entry.digest,
                        entry.texture_use.color_space,
                        entry.texture_use.mip_semantic,
                        entry.width,
                        entry.height,
                        &rgba8,
                    )
                } else {
                    let (coarse, cw, ch) = coarse_texture_level(
                        &rgba8,
                        entry.width,
                        entry.height,
                        first_mip,
                        entry.texture_use.mip_semantic,
                    )?;
                    if cw == 0 || ch == 0 {
                        return Err("HMC coarse texture dimensions are invalid".to_string());
                    }
                    renderer
                        .inner
                        .upload_resident_texture_rgba8_at_mip_with_mips(
                            entry.digest,
                            entry.texture_use.color_space,
                            entry.texture_use.mip_semantic,
                            entry.width,
                            entry.height,
                            u32::from(first_mip),
                            &coarse,
                        )
                        .map(|_| ())
                };
                match upload {
                    Ok(_) => {
                        newly_resident.push((entry.digest, entry.texture_use));
                        loaded_residency.insert(
                            (entry.digest, entry.texture_use),
                            (
                                first_mip,
                                rgba8_resident_bytes(entry.width, entry.height, first_mip)?,
                            ),
                        );
                        report.resident_interpretations += 1;
                    }
                    Err(TextureUploadError::GpuBudgetRefused)
                    | Err(TextureUploadError::UploadBudgetRefused) => {
                        note_refusal(&mut report, mip_count, mip_bytes);
                    }
                    Err(e) => return Err(format!("HMC texture upload: {e}")),
                }
            }
        }
        Ok(())
    })();
    if let Err(error) = upload_result {
        for (digest, texture_use) in newly_resident {
            renderer.inner.evict_resident_texture_with_mips(
                &digest,
                texture_use.color_space,
                texture_use.mip_semantic,
            );
        }
        return Err(error);
    }
    let mut all_plan_entries = plan_entries;
    all_plan_entries.extend(ktx2_entries);
    let stream_state = match build_texture_stream_state(
        hmc_bytes,
        asset_bytes,
        &all_plan_entries,
        &loaded_residency,
    ) {
        Ok(state) => state,
        Err(error) => {
            for (digest, texture_use) in newly_resident {
                renderer.inner.evict_resident_texture_with_mips(
                    &digest,
                    texture_use.color_space,
                    texture_use.mip_semantic,
                );
            }
            return Err(error);
        }
    };
    match renderer.load_10d_asset(asset_bytes) {
        Ok(result) => {
            evict_replaced_asset_textures(renderer, &all_plan_entries);
            renderer.texture_stream_state = Some(stream_state);
            Ok((result, report))
        }
        Err(error) => {
            for (digest, texture_use) in newly_resident {
                renderer.inner.evict_resident_texture_with_mips(
                    &digest,
                    texture_use.color_space,
                    texture_use.mip_semantic,
                );
            }
            Err(error)
        }
    }
}

pub fn request_texture_stream(
    renderer: &super::VolumetricRenderer,
    request: HmcTextureStreamApplyRequest<'_>,
) -> Result<HmcTextureStreamPlan, String> {
    volumetric_hmc_stream::request_texture_stream(renderer, request)
}

pub fn apply_texture_stream(
    renderer: &mut super::VolumetricRenderer,
    plan: HmcTextureStreamPlan,
) -> Result<(), String> {
    volumetric_hmc_stream::apply_texture_stream(renderer, plan)
}

/// Load an HMC `.10d` geometry asset into the renderer's persistent water
/// surface slot. Water is presentation-only: it shares the renderer model
/// transform and scene depth, but does not become a pickable/entity mesh.
pub fn load_hmc_water_asset(
    renderer: &mut super::VolumetricRenderer,
    hmc_bytes: &[u8],
    asset_key: &str,
) -> Result<(u32, u32), String> {
    load_hmc_water_asset_with_policy(renderer, hmc_bytes, asset_key, HmcWaterLoadPolicy::default())
}

/// Policy-bearing HMC water load. Validation and admission happen before any
/// GPU replacement, so failed loads leave the previous water surface intact.
pub fn load_hmc_water_asset_with_policy(
    renderer: &mut super::VolumetricRenderer,
    hmc_bytes: &[u8],
    asset_key: &str,
    policy: HmcWaterLoadPolicy,
) -> Result<(u32, u32), String> {
    let bundle = qualia_core_db::bundle::BundleReader::parse(hmc_bytes)
        .map_err(|e| format!("HMC bundle: {e}"))?;
    let asset_bytes = bundle
        .get(asset_key)
        .ok_or_else(|| format!("HMC asset is missing: {asset_key}"))?;
    if !bundle.verify_entry(asset_key) {
        return Err(format!("HMC asset digest check failed: {asset_key}"));
    }
    let entry = bundle
        .entry(asset_key)
        .ok_or_else(|| format!("HMC asset index entry is missing: {asset_key}"))?;
    if entry.kind != "10d" {
        return Err(format!(
            "HMC water asset {asset_key} has kind {:?}, expected 10d",
            entry.kind
        ));
    }
    let header = Container10dHeader::parse(asset_bytes).map_err(|e| format!("10d header: {e}"))?;
    if qualia_core_db::container_10d::compute_whole_file_crc32c(asset_bytes) != header.header_crc32c
    {
        return Err("10d whole-file integrity check failed".to_string());
    }
    let descs = qualia_core_db::container_10d::parse_section_table(asset_bytes, &header)
        .map_err(|e| format!("10d section table: {e}"))?;
    let descriptor = descs
        .iter()
        .find(|descriptor| descriptor.typ() == Some(SectionType::QuantizedMesh))
        .ok_or_else(|| "HMC water asset has no QuantizedMesh section".to_string())?;
    let start = descriptor.byte_offset as usize;
    let end = start
        .checked_add(descriptor.byte_length as usize)
        .filter(|&end| end <= asset_bytes.len())
        .ok_or_else(|| "HMC water mesh section is outside asset bytes".to_string())?;
    let mesh = decode_mesh_section(&asset_bytes[start..end])
        .map_err(|e| format!("10d water mesh decode: {e}"))?;
    let geometry_bytes = validate_hmc_water_geometry(
        &mesh.positions,
        &mesh.triangles,
        policy.max_geometry_bytes,
    )?;
    let mut indices = Vec::new();
    indices
        .try_reserve_exact(mesh.triangles.len().saturating_mul(3))
        .map_err(|_| "HMC water index staging allocation refused".to_string())?;
    for triangle in &mesh.triangles {
        indices.extend_from_slice(triangle);
    }
    let vertex_count = u32::try_from(mesh.positions.len())
        .map_err(|_| "HMC water vertex count exceeds u32".to_string())?;
    let triangle_count = u32::try_from(mesh.triangles.len())
        .map_err(|_| "HMC water triangle count exceeds u32".to_string())?;
    renderer
        .inner
        .upload_water_geometry_with_budget(&mesh.positions, &indices, policy.max_geometry_bytes)?;
    renderer.inner.set_water_quality(select_hmc_water_quality(
        mesh.triangles.len(),
        geometry_bytes,
        policy.preferred_quality,
    ));
    Ok((vertex_count, triangle_count))
}
