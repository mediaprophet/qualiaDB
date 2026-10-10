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
use qualia_core_db::render::texture_streaming_plan::decode_rgba8_ktx2_mip_into;
use std::collections::{BTreeMap, BTreeSet};

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

/// Load an HMC `.10d` asset while admitting texture mips according to footprint and budget.
pub fn load_hmc_asset_with_texture_request(
    renderer: &mut super::VolumetricRenderer,
    hmc_bytes: &[u8],
    asset_key: &str,
    request: HmcTextureStreamRequest,
) -> Result<((u32, u32, f32), super::HmcTextureAdmissionReport), String> {
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
    let mut demands = Vec::new();
    let mut next_semantic_id = 1u64;
    let mut report = super::HmcTextureAdmissionReport::default();
    for (digest, uses) in &texture_uses {
        let resource =
            qualia_core_db::render::asset_package::resolve_hmc_texture_resource(&bundle, digest)
                .map_err(|e| format!("HMC texture resolution: {e}"))?;
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

            // KTX2 Direct Mip Path
            if is_ktx2_mime(resource.mime_type) {
                if let Ok(doc) = Ktx2Document::parse(resource.bytes) {
                    let is_srgb = doc.vk_format == 43;
                    let mut all_uploaded = true;
                    for (entry, first_mip, mip_bytes, mip_count) in selected.clone() {
                        if (entry.texture_use.color_space == TextureColorSpace::Srgb) != is_srgb {
                            note_refusal(&mut report, mip_count, mip_bytes);
                            continue;
                        }
                        let mip_idx = first_mip as usize;
                        if doc.level(mip_idx).is_some() {
                            let coarse_w = (entry.width >> first_mip).max(1);
                            let coarse_h = (entry.height >> first_mip).max(1);
                            let req_len = (coarse_w as usize)
                                .saturating_mul(coarse_h as usize)
                                .saturating_mul(4);
                            let mut coarse_buf = vec![0u8; req_len];
                            if decode_rgba8_ktx2_mip_into(&doc, mip_idx, &mut coarse_buf).is_ok() {
                                let upload = if first_mip == 0 {
                                    renderer.inner.upload_resident_texture_rgba8_with_mips(
                                        entry.digest,
                                        entry.texture_use.color_space,
                                        entry.texture_use.mip_semantic,
                                        entry.width,
                                        entry.height,
                                        &coarse_buf,
                                    )
                                } else {
                                    renderer
                                        .inner
                                        .upload_resident_texture_rgba8_at_mip_with_mips(
                                            entry.digest,
                                            entry.texture_use.color_space,
                                            entry.texture_use.mip_semantic,
                                            entry.width,
                                            entry.height,
                                            u32::from(first_mip),
                                            &coarse_buf,
                                        )
                                        .map(|_| ())
                                };
                                match upload {
                                    Ok(_) => {
                                        newly_resident.push((entry.digest, entry.texture_use));
                                        report.resident_interpretations += 1;
                                    }
                                    Err(TextureUploadError::GpuBudgetRefused)
                                    | Err(TextureUploadError::UploadBudgetRefused) => {
                                        note_refusal(&mut report, mip_count, mip_bytes);
                                    }
                                    Err(e) => return Err(format!("HMC texture upload: {e}")),
                                }
                                continue;
                            }
                        }
                        all_uploaded = false;
                    }
                    if all_uploaded {
                        continue;
                    }
                }
            }

            // Fallback: Full base decode + CPU downsample for PNG, JPEG, or missing KTX2 mips
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
    match renderer.load_10d_asset(asset_bytes) {
        Ok(result) => Ok((result, report)),
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

/// Load an HMC `.10d` geometry asset into the renderer's persistent water
/// surface slot. Water is presentation-only: it shares the renderer model
/// transform and scene depth, but does not become a pickable/entity mesh.
pub fn load_hmc_water_asset(
    renderer: &mut super::VolumetricRenderer,
    hmc_bytes: &[u8],
    asset_key: &str,
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
    let mut indices = Vec::with_capacity(mesh.triangles.len().saturating_mul(3));
    for triangle in &mesh.triangles {
        indices.extend_from_slice(triangle);
    }
    let vertex_count = u32::try_from(mesh.positions.len())
        .map_err(|_| "HMC water vertex count exceeds u32".to_string())?;
    let triangle_count = u32::try_from(mesh.triangles.len())
        .map_err(|_| "HMC water triangle count exceeds u32".to_string())?;
    renderer
        .inner
        .upload_water_geometry(&mesh.positions, &indices)?;
    Ok((vertex_count, triangle_count))
}
