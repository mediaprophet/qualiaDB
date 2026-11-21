//! CPU decode for QualiaPortal `load_10d`.
//!
//! Lives outside the wasm-only `portal` facade so native tests can cover the
//! SurfaceReading colour buffer, UV01 stream, and MAT3/MAT4 records that
//! `load_10d` uploads.

use bytemuck::Zeroable;
use crate::container_10d;
use crate::container_10d::header::{Container10dHeader, FLAG_DEFAULT_DISPOSITION_REFUSE};
use crate::container_10d::{MaterialRecord, SubmeshRange};

/// Orbit-framed mesh buffers `load_10d` passes to GPU upload.
#[derive(Debug, Clone)]
pub struct Load10dGpuBuffers {
    /// Positions centred and scaled into the portal orbit frame.
    pub positions: Vec<[f32; 3]>,
    /// Per-vertex linear RGBA from SurfaceReading. Empty iff SurfaceReading is absent.
    pub colors: Vec<[f32; 4]>,
    /// Authored UV01 when the section is present and vertex-aligned.
    pub uv0: Option<Vec<[f32; 2]>>,
    /// Authored normals when the frame sidecar is present and vertex-aligned.
    pub normals: Option<Vec<[f32; 3]>>,
    /// Authored tangents when the frame sidecar is present and vertex-aligned.
    pub tangents: Option<Vec<[f32; 4]>>,
    /// Flat triangle-list indices.
    pub indices: Vec<u32>,
    /// MAT3/MAT4 records. Empty when the section is absent (colour path stays).
    pub materials: Vec<MaterialRecord>,
    /// Submesh ranges bound to [`Self::materials`]. Empty with missing MAT.
    pub ranges: Vec<SubmeshRange>,
    /// First Tensor10DNodes μ, or 0 when that section is absent.
    pub provenance_mu: f32,
    /// Header default-refuse with no attestation sidecar.
    pub governance_refused: bool,
    /// Actual field or geometry representation decoded (`"QuantizedMesh"`, `"HolographicMultiSegmentField"`, `"HolographicProxyBox"`).
    pub field_representation: &'static str,
    /// Number of `M64HoloRecord` segments decoded from `HOLO` (`0x0306`).
    pub holo_segment_count: u16,
    /// Selected scale-space harmonic detail cutoff $K \in [1, 12]$ (`0` if `HOLO` absent).
    pub active_harmonic_order: u8,
    pub vertex_count: u32,
    pub triangle_count: u32,
}

/// Parse a sealed `.10d` into the GPU upload arguments `QualiaPortal::load_10d` uses.
pub fn decode_load_10d(bytes: &[u8]) -> Result<Load10dGpuBuffers, String> {
    let mut bytes_mut = bytes.to_vec();
    let header = Container10dHeader::parse(&bytes_mut).map_err(|e| format!("10d header: {e}"))?;

    container_10d::verify_whole_file_crc32c(&mut bytes_mut).map_err(|e| format!("10d CRC: {e}"))?;

    let descs = container_10d::parse_section_table(&bytes_mut, &header)
        .map_err(|e| format!("10d section table: {e}"))?;

    let mut mesh = None;
    let mut provenance_mu: f32 = 0.0;
    let mut has_attestation = false;
    let mut has_holographic_or_computational = false;
    let mut proxy_min = [-0.5f32, -0.5, -0.5];
    let mut proxy_max = [0.5f32, 0.5, 0.5];
    let mut decoded_holo_segments: Vec<crate::q42::m64_matter::M64HoloRecord> = Vec::new();
    let mut holo_max_harmonic_order: u8 = 0;

    for desc in descs.iter() {
        let st = container_10d::SectionType::from_u8(desc.section_type)
            .ok_or_else(|| "10d: unknown section type".to_string())?;
        let off = desc.byte_offset as usize;
        let len = desc.byte_length as usize;
        let payload = &bytes_mut[off..off + len];

        match st {
            container_10d::SectionType::QuantizedMesh => {
                mesh = Some(
                    container_10d::decode_mesh_section(payload)
                        .map_err(|e| format!("10d mesh decode: {e}"))?,
                );
            }
            container_10d::SectionType::Tensor10DNodes => {
                has_holographic_or_computational = true;
                if let Ok(t) = container_10d::read_node(payload, 0) {
                    provenance_mu = t.mu;
                }
            }
            container_10d::SectionType::ManifoldIdentityV2 => {
                has_holographic_or_computational = true;
                let mut fields = [container_10d::TypedFieldRecord::default(); 32];
                if let Ok((id, count)) =
                    container_10d::decode_manifold_identity_v2(payload, &[], &mut fields)
                {
                    if provenance_mu == 0.0 {
                        provenance_mu = id.coordinates[8];
                    }
                    // Check if a HOLO phase-space header supplies explicit proxy lens bounds and segments
                    for f in &fields[..count] {
                        if f.kind == container_10d::FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE
                            && f.payload_section_id == container_10d::SectionType::FieldSidecar as u32
                        {
                            if let Some(sidecar_desc) = descs.iter().find(|d| {
                                container_10d::SectionType::from_u8(d.section_type)
                                    == Some(container_10d::SectionType::FieldSidecar)
                            }) {
                                let s_off = sidecar_desc.byte_offset as usize
                                    + (f.payload_offset as usize);
                                let s_len = f.payload_length as usize;
                                if s_off + s_len <= bytes_mut.len()
                                    && s_len >= container_10d::HOLOGRAPHIC_PHASE_HEADER_SIZE
                                {
                                    let holo_slice = &bytes_mut[s_off..s_off + s_len];
                                    let hdr: container_10d::HolographicPhaseHeader =
                                        bytemuck::pod_read_unaligned(
                                            &holo_slice
                                                [..container_10d::HOLOGRAPHIC_PHASE_HEADER_SIZE],
                                        );
                                    if hdr.magic == container_10d::HOLOGRAPHIC_PHASE_TAG {
                                        proxy_min = hdr.proxy_aabb_min;
                                        proxy_max = hdr.proxy_aabb_max;
                                        holo_max_harmonic_order = hdr.max_harmonic_order;
                                        let mut seg_buf = [crate::q42::m64_matter::M64HoloRecord::default();
                                            container_10d::MAX_FITTED_HOLO_SEGMENTS];
                                        let mut wf_buf =
                                            [container_10d::EulerianWavefieldRecord::zeroed(); 4];
                                        if let Ok((_, sc, _)) =
                                            container_10d::decode_holographic_phase_section_into(
                                                holo_slice,
                                                &mut seg_buf,
                                                &mut wf_buf,
                                            )
                                        {
                                            decoded_holo_segments
                                                .extend_from_slice(&seg_buf[..sc]);
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            container_10d::SectionType::ProceduralSynthesis
            | container_10d::SectionType::EntityOntology => {
                has_holographic_or_computational = true;
            }
            container_10d::SectionType::ProvenanceSidecar => {
                if let Ok(view) =
                    crate::container_10d::provenance_section::decode_provenance_section(payload)
                {
                    if crate::container_10d::provenance_section::validate_provenance(&view).is_ok()
                    {
                        has_attestation = true;
                    }
                }
            }
            _ => {}
        }
    }

    let had_explicit_mesh = mesh.is_some();
    let active_harmonic_order = if holo_max_harmonic_order > 0 {
        container_10d::select_scale_space_harmonic_cutoff_zero_heap(1.5, holo_max_harmonic_order)
            as u8
    } else {
        0
    };
    let field_representation = if had_explicit_mesh {
        "QuantizedMesh"
    } else if !decoded_holo_segments.is_empty() {
        "HolographicMultiSegmentField"
    } else {
        "HolographicProxyBox"
    };
    let holo_segment_count = decoded_holo_segments.len() as u16;

    let mesh = match mesh {
        Some(m) => m,
        None if !decoded_holo_segments.is_empty() => synthesize_holographic_segments_mesh(
            &decoded_holo_segments,
            proxy_min,
            proxy_max,
            active_harmonic_order.max(1),
        ),
        None if has_holographic_or_computational => {
            synthesize_holographic_proxy_box_mesh(proxy_min, proxy_max)
        }
        None => return Err("10d: no mesh or holographic phase-space section".to_string()),
    };
    let frames = if had_explicit_mesh {
        container_10d::read_container_mesh_frames_with_limits(
            &bytes_mut,
            mesh.vertex_count(),
            container_10d::MeshFrameDecodeLimits {
                max_vertices: container_10d::MAX_NORMAL_COUNT,
                max_payload_bytes: 64 * 1024 * 1024,
                max_decoded_bytes: 256 * 1024 * 1024,
            },
        )
        .map_err(|e| format!("10d frames decode: {e}"))?
    } else {
        None
    };
    let (authored_normals, authored_tangents) = frames
        .map(|frames| (frames.normals, frames.tangents))
        .unwrap_or((None, None));
    if authored_normals
        .as_ref()
        .is_some_and(|normals: &Vec<[f32; 3]>| normals.len() != mesh.vertex_count())
    {
        return Err("10d normal field vertex count does not match mesh".into());
    }
    if authored_tangents
        .as_ref()
        .is_some_and(|tangents: &Vec<[f32; 4]>| tangents.len() != mesh.vertex_count())
    {
        return Err("10d tangent field vertex count does not match mesh".into());
    }

    let mut surface = None;
    if had_explicit_mesh {
        for desc in descs.iter() {
            if container_10d::SectionType::from_u8(desc.section_type)
                != Some(container_10d::SectionType::SurfaceReading)
            {
                continue;
            }
            let off = desc.byte_offset as usize;
            let len = desc.byte_length as usize;
            if off.saturating_add(len) > bytes_mut.len() {
                continue;
            }
            surface =
                container_10d::surface_reading::decode_surface_reading(&bytes_mut[off..off + len]);
        }
    }
    let colors = match surface {
        Some(reading) if reading.len() == mesh.vertex_count() => reading,
        Some(reading) => {
            return Err(format!(
                "10d SurfaceReading vertex count {} does not match mesh count {}",
                reading.len(),
                mesh.vertex_count()
            ));
        }
        None => Vec::new(),
    };

    let uv0 = if had_explicit_mesh {
        let mut uv0 = vec![[0.0; 2]; mesh.vertex_count()];
        match container_10d::read_texture_coordinate_section_into(
            &bytes_mut,
            mesh.vertex_count(),
            &mut uv0,
        ) {
            Ok(_) => Some(uv0),
            Err(container_10d::TextureCoordinateContainerError::Missing) => None,
            Err(container_10d::TextureCoordinateContainerError::CountMismatch {
                expected,
                got,
            }) => {
                return Err(format!(
                    "10d UV01 vertex count {got} does not match mesh count {expected}"
                ));
            }
            Err(e) => return Err(format!("10d UV01: {e:?}")),
        }
    } else {
        None
    };

    let mesh_index_count = u32::try_from(mesh.triangles.len().saturating_mul(3))
        .map_err(|_| "10d index count exceeds u32".to_string())?;
    let (materials, ranges) = if had_explicit_mesh {
        decode_load_10d_materials(&bytes_mut, mesh_index_count)?
    } else {
        (Vec::new(), Vec::new())
    };

    let governance_refused =
        (header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE) != 0 && !has_attestation;

    let c = mesh.centroid();
    let ext = [
        mesh.max[0] - mesh.min[0],
        mesh.max[1] - mesh.min[1],
        mesh.max[2] - mesh.min[2],
    ];
    let span = ext[0].max(ext[1]).max(ext[2]).max(1e-6);
    let s = 1.6 / span;
    let positions: Vec<[f32; 3]> = mesh
        .positions
        .iter()
        .map(|p| [(p[0] - c[0]) * s, (p[1] - c[1]) * s, (p[2] - c[2]) * s])
        .collect();
    let indices: Vec<u32> = mesh
        .triangles
        .iter()
        .flat_map(|t| [t[0], t[1], t[2]])
        .collect();

    Ok(Load10dGpuBuffers {
        vertex_count: mesh.positions.len() as u32,
        triangle_count: mesh.triangles.len() as u32,
        positions,
        colors,
        uv0,
        normals: authored_normals,
        tangents: authored_tangents,
        indices,
        materials,
        ranges,
        provenance_mu,
        governance_refused,
        field_representation,
        holo_segment_count,
        active_harmonic_order,
    })
}

/// Reconstruct a cavity-preserving multi-segment Holographic Phase-Space mesh directly from
/// a slice of 64-byte [`crate::q42::m64_matter::M64HoloRecord`] segments at harmonic cutoff `max_order`.
///
/// When a single segment is present, falls back to [`synthesize_holographic_proxy_box_mesh`]
/// over `[proxy_min, proxy_max]`. When multiple segments are present (e.g. a perforated cervical
/// vertebra where the central vertebral foramen cell and lateral transverse foramina are unoccupied),
/// emits a harmonic-modulated local sub-volume for each occupied `M64HoloRecord` segment so all
/// anatomical canals and apertures remain open across scale-space harmonic cutoffs $K \in \{2, 4, 8, 12\}$.
pub fn synthesize_holographic_segments_mesh(
    segments: &[crate::q42::m64_matter::M64HoloRecord],
    proxy_min: [f32; 3],
    proxy_max: [f32; 3],
    max_order: u8,
) -> crate::render::assets::Mesh {
    use half::f16;
    if segments.len() <= 1 {
        return synthesize_holographic_proxy_box_mesh(proxy_min, proxy_max);
    }

    let mut positions: Vec<[f32; 3]> = Vec::with_capacity(segments.len() * 8);
    let mut triangles: Vec<[u32; 3]> = Vec::with_capacity(segments.len() * 12);
    let mut gmin = [f32::INFINITY; 3];
    let mut gmax = [f32::NEG_INFINITY; 3];

    let k_active = (max_order.clamp(1, 12)) as usize;
    let box_tris: [[u32; 3]; 12] = [
        [0, 3, 2],
        [0, 2, 1],
        [4, 5, 6],
        [4, 6, 7],
        [0, 1, 5],
        [0, 5, 4],
        [3, 7, 6],
        [3, 6, 2],
        [0, 4, 7],
        [0, 7, 3],
        [1, 2, 6],
        [1, 6, 5],
    ];

    for seg in segments {
        let cx = f16::from_bits(seg.local_origin[0]).to_f32();
        let cy = f16::from_bits(seg.local_origin[1]).to_f32();
        let cz = f16::from_bits(seg.local_origin[2]).to_f32();
        let half_wx = f16::from_bits(seg.scale_bounds[0]).to_f32().clamp(0.001, 100.0);
        let half_dz = f16::from_bits(seg.scale_bounds[1]).to_f32().clamp(0.001, 100.0);
        let half_hy_raw = f16::from_bits(seg.mass_amplitude).to_f32();
        let half_hy = if half_hy_raw.is_finite() && half_hy_raw > 0.0005 && half_hy_raw < 100.0 {
            half_hy_raw
        } else {
            0.5 * (proxy_max[1] - proxy_min[1]).abs().max(0.01)
        };

        let mut h_mod = 0.0f32;
        for m in 0..k_active {
            h_mod += (seg.sdf_harmonics[m] as f32 / 127.0) * 0.025 / ((m + 1) as f32);
        }
        let mod_factor = h_mod.clamp(-0.10, 0.10);
        let rx = (half_wx * (1.0 + mod_factor)).max(0.001);
        let ry = half_hy.max(0.001);
        let rz = (half_dz * (1.0 - mod_factor)).max(0.001);

        let [x0, y0, z0] = [cx - rx, cy - ry, cz - rz];
        let [x1, y1, z1] = [cx + rx, cy + ry, cz + rz];
        let base = positions.len() as u32;
        let corners = [
            [x0, y0, z0],
            [x1, y0, z0],
            [x1, y1, z0],
            [x0, y1, z0],
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y1, z1],
            [x0, y1, z1],
        ];
        for c in corners {
            for axis in 0..3 {
                gmin[axis] = gmin[axis].min(c[axis]);
                gmax[axis] = gmax[axis].max(c[axis]);
            }
            positions.push(c);
        }
        for t in box_tris {
            triangles.push([base + t[0], base + t[1], base + t[2]]);
        }
    }

    crate::render::assets::Mesh {
        positions,
        triangles,
        min: gmin,
        max: gmax,
    }
}

/// Synthesize the 12-triangle (8-vertex) Holographic Proxy Bounding Lens (`Mesh`)
/// for `.10d` containers that operate in pure Holographic Phase-Space / SDF raymarching
/// or Mode-B Archetype-Reference mode without an inline B-Rep `QuantizedMesh`.
pub fn synthesize_holographic_proxy_box_mesh(
    min: [f32; 3],
    max: [f32; 3],
) -> crate::render::assets::Mesh {
    let [x0, y0, z0] = min;
    let [x1, y1, z1] = [
        max[0].max(x0 + 1e-3),
        max[1].max(y0 + 1e-3),
        max[2].max(z0 + 1e-3),
    ];
    crate::render::assets::Mesh {
        positions: vec![
            [x0, y0, z0],
            [x1, y0, z0],
            [x1, y1, z0],
            [x0, y1, z0],
            [x0, y0, z1],
            [x1, y0, z1],
            [x1, y1, z1],
            [x0, y1, z1],
        ],
        triangles: vec![
            [0, 3, 2],
            [0, 2, 1],
            [4, 5, 6],
            [4, 6, 7],
            [0, 1, 5],
            [0, 5, 4],
            [3, 7, 6],
            [3, 6, 2],
            [0, 4, 7],
            [0, 7, 3],
            [1, 2, 6],
            [1, 6, 5],
        ],
        min: [x0, y0, z0],
        max: [x1, y1, z1],
    }
}

fn decode_load_10d_materials(
    bytes: &[u8],
    mesh_index_count: u32,
) -> Result<(Vec<MaterialRecord>, Vec<SubmeshRange>), String> {
    match container_10d::material_container_counts(bytes) {
        Ok((material_count, range_count)) => {
            if material_count == 0 || range_count == 0 {
                return Err("10d MAT section is empty".into());
            }
            if material_count > container_10d::MAX_MATERIALS
                || range_count > container_10d::MAX_SUBMESH_RANGES
            {
                return Err("10d MAT section exceeds portal limits".into());
            }
            let mut materials = vec![MaterialRecord::legacy_default(); material_count];
            let mut ranges = vec![
                SubmeshRange {
                    first_index: 0,
                    index_count: 0,
                    material_id: 0,
                    semantic_id: 0,
                };
                range_count
            ];
            match container_10d::read_material_section_into(
                bytes,
                mesh_index_count,
                &mut materials,
                &mut ranges,
            ) {
                Ok((got_materials, got_ranges))
                    if got_materials == material_count && got_ranges == range_count =>
                {
                    Ok((materials, ranges))
                }
                Ok(_) => Err("10d MAT section count mismatch".into()),
                Err(error) => Err(format!("10d MAT: {error:?}")),
            }
        }
        Err(container_10d::MaterialContainerError::Missing) => Ok((Vec::new(), Vec::new())),
        Err(error) => Err(format!("10d MAT: {error:?}")),
    }
}

/// Upload the decoded `load_10d` buffers. Authored MAT3/MAT4 is used when present;
/// otherwise UV0 uses the legacy default material. Empty `colors` is the GPU grey default.
#[cfg(feature = "gpu-runtime")]
pub fn upload_load_10d_to_gpu(
    gpu: &mut crate::render::gpu::PortalGpu,
    decoded: &Load10dGpuBuffers,
) -> Result<u32, String> {
    upload_load_10d_to_gpu_with_textures(gpu, decoded, &[])
}

/// Upload mesh plus optional PNG/KTX2 albedo (or other MAT maps). Texture
/// failures skip the map; the mesh and vertex colour still draw.
#[cfg(feature = "gpu-runtime")]
pub fn upload_load_10d_to_gpu_with_textures(
    gpu: &mut crate::render::gpu::PortalGpu,
    decoded: &Load10dGpuBuffers,
    texture_bytes: &[&[u8]],
) -> Result<u32, String> {
    let use_legacy_colour = decoded.materials.is_empty()
        && decoded.uv0.is_none()
        && texture_bytes.is_empty();
    if use_legacy_colour {
        return Ok(gpu.upload_mesh_colored_with_frames(
            &decoded.positions,
            &decoded.colors,
            decoded.normals.as_deref(),
            decoded.tangents.as_deref(),
            &decoded.indices,
        ));
    }

    let (mut materials, ranges) = if decoded.materials.is_empty() {
        let material = MaterialRecord::legacy_default();
        let material_id = material.id;
        let index_count = u32::try_from(decoded.indices.len())
            .map_err(|_| "10d index count exceeds u32".to_string())?;
        (
            vec![material],
            vec![SubmeshRange {
                first_index: 0,
                index_count,
                material_id,
                semantic_id: 0,
            }],
        )
    } else {
        (decoded.materials.clone(), decoded.ranges.clone())
    };

    bind_and_upload_textures(gpu, &mut materials, texture_bytes);

    gpu.upload_mesh_colored_with_frames_and_materials_and_uv0(
        &decoded.positions,
        &decoded.colors,
        decoded.normals.as_deref(),
        decoded.tangents.as_deref(),
        decoded.uv0.as_deref(),
        &decoded.indices,
        &materials,
        &ranges,
    )
}

#[cfg(feature = "gpu-runtime")]
fn bind_and_upload_textures(
    gpu: &mut crate::render::gpu::PortalGpu,
    materials: &mut [MaterialRecord],
    texture_bytes: &[&[u8]],
) {
    use crate::render::portal_load_textures::{
        ingest_portal_texture, material_references_digest, upload_portal_texture_rgba8,
        PortalTextureIngest,
    };
    for bytes in texture_bytes {
        let tex = match ingest_portal_texture(None, bytes) {
            Ok(PortalTextureIngest::Rgba8(decoded)) => decoded,
            Ok(PortalTextureIngest::SkippedTranscode) | Err(_) => continue,
        };
        if !materials
            .iter()
            .any(|material| material_references_digest(material, tex.digest))
        {
            if let Some(material) = materials
                .iter_mut()
                .find(|material| material.base_color_texture == [0; 32])
            {
                material.base_color_texture = tex.digest;
            }
        }
        let _ = upload_portal_texture_rgba8(gpu, materials, &tex);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "portal_load_10d_tests.rs"]
mod tests;
