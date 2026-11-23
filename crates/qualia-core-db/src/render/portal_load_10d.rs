//! CPU decode for QualiaPortal `load_10d`.
//!
//! Lives outside the wasm-only `portal` facade so native tests can cover the
//! SurfaceReading colour buffer, UV01 stream, and MAT3/MAT4 records that
//! `load_10d` uploads.

use bytemuck::Zeroable;
use crate::container_10d;
use crate::container_10d::header::{Container10dHeader, FLAG_DEFAULT_DISPOSITION_REFUSE};
use crate::container_10d::{MaterialRecord, SubmeshRange};

#[path = "sdf_brick_surface.rs"]
mod sdf_brick_surface;
pub(crate) use sdf_brick_surface::decode_sdfb_mesh;
pub use sdf_brick_surface::{
    decode_sdfb_cutaway, decode_sdfb_cutaway_with_emf, SdfCutawaySurface, SectionPlane,
};
#[path = "portal_assembly_mesh.rs"]
mod assembly_mesh;
pub use assembly_mesh::{
    assembly_residency_digest, decode_hmc_leaf_assembly, decode_hmc_leaf_assembly_with_emf,
    decode_hmc_leaf_cutaway, decode_hmc_leaf_cutaway_with_emf, resolve_hmc_section_iris,
    LoadAssemblyCutaway, LoadAssemblyGpuBuffers, SectionLeafOccurrence,
};
use crate::render::portal_emf_binding::{sample_optical_surface_colors, BoundLeafEmfCompanion};

/// Two-bit geometric region carried with a semantic surface pick.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum PickSurfaceRegion {
    UncutSurface = 0b00,
    CutTissueWall = 0b01,
    InternalCavityLumen = 0b10,
}

impl PickSurfaceRegion {
    pub const fn from_u8(region: u8) -> Option<Self> {
        match region {
            0 => Some(Self::UncutSurface),
            1 => Some(Self::CutTissueWall),
            2 => Some(Self::InternalCavityLumen),
            _ => None,
        }
    }
}

/// GPU draw identity captured before asynchronous visibility can change.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SemanticSurfacePick {
    pub instance_slot: u32,
    pub semantic_id: u64,
    pub surface_region: PickSurfaceRegion,
}

/// A verified leaf reference and its authored transform, borrowed from an HMC.
pub struct AdmittedAssemblyLeaf<'a> {
    pub slot: usize,
    pub semantic_id: u64,
    pub key: &'a str,
    pub bytes: &'a [u8],
    pub overrides: container_10d::InstanceOverrideHeader,
    pub coefficients: Vec<container_10d::SsmInstanceCoeff>,
}

/// Verify the selected assembly, semantic model and leaf digests before resolving
/// the ordered AREF/IOVR pairs and INST identities for the renderer.
pub fn admit_leaf_assembly<'a>(
    bundle: &'a crate::bundle::BundleReader<'a>,
    assembly_key: &str,
) -> Result<Vec<AdmittedAssemblyLeaf<'a>>, String> {
    use container_10d::{
        decode_archetype_ref_record, decode_instance_batch_into, ArchetypeDigestAlg,
        InstanceBatchRecord, SsmInstanceCoeff, TypedFieldRecord, TypedPayloadResolver,
        FIELD_KIND_ARCHETYPE_REF, FIELD_KIND_INSTANCE_BATCH, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
        MAX_INSTANCE_SSM_COEFFS,
    };

    let assembly = bundle.get(assembly_key).ok_or("Assembly entry missing from HMC")?;
    if bundle.entry(assembly_key).is_none_or(|entry| entry.kind != "10d")
        || !bundle.verify_entry(assembly_key)
    {
        return Err("HMC assembly entry failed its 10D kind or SHA-256 check".into());
    }
    let header = Container10dHeader::parse(assembly).map_err(|error| error.to_string())?;
    if header.header_crc32c != container_10d::compute_whole_file_crc32c(assembly) {
        return Err("HMC leaf assembly has a mismatched 10D checksum".into());
    }
    let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut records = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let view = TypedPayloadResolver::new(assembly, &[], &mut scratch, &mut records)
        .map_err(|error| error.to_string())?;
    if header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE != 0 {
        use container_10d::provenance_section::{decode_provenance_section, validate_provenance};
        let payload = view.section_payload(container_10d::SectionType::ProvenanceSidecar)
            .ok_or("HMC leaf assembly has no required root attestation")?;
        let provenance = decode_provenance_section(payload).map_err(|error| format!("{error:?}"))?;
        validate_provenance(&provenance).map_err(|error| format!("{error:?}"))?;
    }
    let count = view.instance_override_count().map_err(|error| error.to_string())?;
    if count == 0 || view.payloads_of_kind(FIELD_KIND_ARCHETYPE_REF).count() != count {
        return Err("HMC leaf assembly requires an AREF for each IOVR".into());
    }
    let mut batches = view.payloads_of_kind(FIELD_KIND_INSTANCE_BATCH);
    let batch = batches.next().ok_or("HMC leaf assembly requires INST")?
        .map_err(|error| error.to_string())?;
    if batches.next().is_some() {
        return Err("HMC leaf assembly requires one ordered INST batch".into());
    }
    let mut instances = vec![InstanceBatchRecord::default(); count];
    let (_, instance_count) = decode_instance_batch_into(batch.bytes, &mut instances)
        .map_err(|error| format!("Invalid INST batch: {error:?}"))?;
    if instance_count != count {
        return Err("INST record count differs from AREF and IOVR".into());
    }

    let model = crate::bundle::semantic_model::open_semantic_model(bundle)
        .map_err(|error| error.to_string())?;
    let mut page = vec![0; 1024 * 1024];
    let mut lexical = [0; 4096];
    let mut verified_semantics = std::collections::HashSet::new();
    let mut require_iri = |hash| -> Result<(), String> {
        if verified_semantics.contains(&hash) {
            return Ok(());
        }
        let len = model.lookup_lexicon_hash_into(hash, &mut page, &mut lexical)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Unresolved assembly semantic identity {hash:#018x}"))?;
        let iri = std::str::from_utf8(&lexical[..len]).map_err(|error| error.to_string())?;
        if hash == 0 || !crate::bundle::semantic_model::valid_iri(iri) || crate::q_hash(iri) != hash {
            return Err(format!("Invalid assembly semantic binding for {hash:#018x}"));
        }
        verified_semantics.insert(hash);
        Ok(())
    };

    let mut admitted = Vec::with_capacity(count);
    let mut coefficient_scratch = [SsmInstanceCoeff::default(); MAX_INSTANCE_SSM_COEFFS];
    let mut leaf_ids = std::collections::HashSet::with_capacity(count);
    let mut verified_leaves = std::collections::HashSet::with_capacity(count);
    for (slot, (reference, instance)) in view.payloads_of_kind(FIELD_KIND_ARCHETYPE_REF)
        .zip(instances).enumerate()
    {
        let reference = reference.map_err(|error| error.to_string())?;
        let aref = decode_archetype_ref_record(reference.bytes)
            .map_err(|error| format!("Invalid AREF at slot {slot}: {error:?}"))?;
        require_iri(aref.taxon_or_class_iri_hash)?;
        require_iri(instance.entity_id_hash)?;
        if !leaf_ids.insert(instance.entity_id_hash) {
            return Err(format!("Duplicate assembly instance identity at slot {slot}"));
        }
        let entry_selector = crate::bundle::semantic_model::hmc_entry_key_iri;
        let mut matches = bundle.entries().iter().filter(|entry| {
            crate::q_hash(&entry.key) == aref.subresource_key_hash
                || crate::q_hash(&entry_selector(&entry.key)) == aref.subresource_key_hash
        });
        let entry = matches.next().ok_or_else(|| format!("Missing AREF leaf at slot {slot}"))?;
        if matches.next().is_some()
            || entry.key == assembly_key
            || entry.kind != "10d"
            || aref.digest_alg != ArchetypeDigestAlg::Sha256 as u8
            || entry.sha256.as_slice() != aref.archetype_digest.as_slice()
        {
            return Err(format!("Invalid or ambiguous SHA-256 AREF leaf at slot {slot}"));
        }
        if crate::q_hash(&entry_selector(&entry.key)) == aref.subresource_key_hash {
            require_iri(aref.subresource_key_hash)?;
        }
        let bytes = bundle.get(&entry.key).ok_or("AREF leaf range is missing")?;
        let first_use = verified_leaves.insert(entry.key.as_str());
        if first_use && !bundle.verify_entry(&entry.key) {
            return Err(format!("AREF leaf SHA-256 check failed at slot {slot}"));
        }
        let mut leaf_scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let mut leaf_fields = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let leaf = TypedPayloadResolver::new(bytes, &[], &mut leaf_scratch, &mut leaf_fields)
            .map_err(|error| error.to_string())?;
        if leaf.identity().map(|identity| identity.entity_id.0) != Some(aref.archetype_entity_id) {
            return Err(format!("AREF entity identity mismatch at slot {slot}"));
        }
        if first_use {
            if let Some(sdf) = container_10d::sdf_brick_section::read_container_sdfb(bytes)? {
                for brick in sdf.bricks() {
                    require_iri(brick.wall_semantic_id())?;
                    require_iri(brick.lumen_semantic_id())?;
                }
            }
        }
        let bound = view.instance_override_into(slot, &mut coefficient_scratch)
            .map_err(|error| error.to_string())?
            .ok_or("IOVR disappeared after assembly admission")?;
        admitted.push(AdmittedAssemblyLeaf {
            slot,
            semantic_id: instance.entity_id_hash,
            key: &entry.key,
            bytes,
            overrides: bound.header,
            coefficients: coefficient_scratch[..bound.coefficient_count].to_vec(),
        });
    }
    Ok(admitted)
}

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
    /// Actual field or geometry representation decoded.
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
    decode_load_10d_in_frame(bytes, true)
}

/// Parse a sealed `.10d` with an optional bound optical `.emf` companion evaluated in
/// the leaf's authored metre frame before viewer normalization.
pub fn decode_load_10d_with_emf(
    bytes: &[u8],
    orbit_fit: bool,
    companion: Option<&BoundLeafEmfCompanion<'_>>,
) -> Result<Load10dGpuBuffers, String> {
    decode_load_10d_in_frame_with_emf(bytes, orbit_fit, companion)
}

pub(crate) fn decode_load_10d_in_frame(bytes: &[u8], orbit_fit: bool) -> Result<Load10dGpuBuffers, String> {
    decode_load_10d_in_frame_with_emf(bytes, orbit_fit, None)
}

pub(crate) fn decode_load_10d_in_frame_with_emf(
    bytes: &[u8],
    orbit_fit: bool,
    companion: Option<&BoundLeafEmfCompanion<'_>>,
) -> Result<Load10dGpuBuffers, String> {
    let mut bytes_mut = bytes.to_vec();
    let header = Container10dHeader::parse(&bytes_mut).map_err(|e| format!("10d header: {e}"))?;

    container_10d::verify_whole_file_crc32c(&mut bytes_mut).map_err(|e| format!("10d CRC: {e}"))?;
    validate_bound_instance_overrides(&bytes_mut)?;
    let sdf = decode_sdfb_mesh(&bytes_mut)?;
    let has_sdf = sdf.is_some();

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
                if !has_sdf {
                    mesh = Some(
                        container_10d::decode_mesh_section(payload)
                            .map_err(|e| format!("10d mesh decode: {e}"))?,
                    );
                }
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
    let field_representation = if has_sdf {
        "SignedDistanceBrickSurface"
    } else if had_explicit_mesh {
        "QuantizedMesh"
    } else if !decoded_holo_segments.is_empty() {
        "HolographicMultiSegmentField"
    } else {
        "HolographicProxyBox"
    };
    let holo_segment_count = decoded_holo_segments.len() as u16;

    let (mesh, sdf_ranges) = if let Some(surface) = sdf {
        (surface.mesh, Some(surface.ranges))
    } else {
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
        (mesh, None)
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
    let mut colors = match surface {
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
    let (mut materials, mut ranges) = if let Some(ranges) = sdf_ranges {
        (vec![MaterialRecord::legacy_default()], ranges)
    } else if had_explicit_mesh {
        decode_load_10d_materials(&bytes_mut, mesh_index_count)?
    } else {
        (Vec::new(), Vec::new())
    };

    if let Some(bound) = companion {
        let base_tissue = if has_sdf {
            [0.69, 0.58, 0.49]
        } else {
            materials
                .first()
                .map(|m| [m.base_color[0], m.base_color[1], m.base_color[2]])
                .unwrap_or([0.69, 0.58, 0.49])
        };
        // Evaluate optical colour in the authored metre frame BEFORE orbit_fit viewer normalization.
        colors = sample_optical_surface_colors(&mesh.positions, base_tissue, bound);
        if materials.is_empty() && mesh_index_count > 0 {
            let mut mat = MaterialRecord::legacy_default();
            mat.base_color = [1.0, 1.0, 1.0, 1.0];
            mat.multiply_vertex_color = true;
            let mat_id = mat.id;
            materials.push(mat);
            ranges.push(SubmeshRange {
                first_index: 0,
                index_count: mesh_index_count,
                material_id: mat_id,
                semantic_id: 0,
            });
        } else {
            for mat in &mut materials {
                mat.base_color = [1.0, 1.0, 1.0, mat.base_color[3]];
                mat.multiply_vertex_color = true;
            }
        }
    }

    let governance_refused =
        (header.flags & FLAG_DEFAULT_DISPOSITION_REFUSE) != 0 && !has_attestation;

    let vertex_count = mesh.positions.len() as u32;
    let positions = if orbit_fit {
        let c = mesh.centroid();
        let span = (0..3).map(|axis| mesh.max[axis] - mesh.min[axis]).fold(1e-6, f32::max);
        let s = 1.6 / span;
        mesh.positions.iter()
            .map(|p| [(p[0] - c[0]) * s, (p[1] - c[1]) * s, (p[2] - c[2]) * s])
            .collect()
    } else {
        mesh.positions
    };
    let indices: Vec<u32> = mesh
        .triangles
        .iter()
        .flat_map(|t| [t[0], t[1], t[2]])
        .collect();

    Ok(Load10dGpuBuffers {
        vertex_count,
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

pub(crate) fn validate_bound_instance_overrides(bytes: &[u8]) -> Result<(), String> {
    use container_10d::{TypedFieldRecord, TypedPayloadResolver, MANIFOLD_IDENTITY_V2_MAX_FIELDS};

    let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut fields = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    TypedPayloadResolver::new(bytes, &[], &mut scratch, &mut fields)
        .and_then(|view| view.validate_instance_overrides())
        .map_err(|error| error.to_string())
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

    let triangles = gpu.upload_mesh_colored_with_frames_and_materials_and_uv0(
        &decoded.positions,
        &decoded.colors,
        decoded.normals.as_deref(),
        decoded.tangents.as_deref(),
        decoded.uv0.as_deref(),
        &decoded.indices,
        &materials,
        &ranges,
    )?;
    if decoded.field_representation == "SignedDistanceBrickSurface" {
        // Each indexed chunk owns one identity transform. The existing native
        // group pass picks the visible chunk's full 64-bit wall hash.
        use crate::render::instance_culling::GpuInstanceRecord;
        const IDENTITY: [[f32; 4]; 4] = [
            [1.0, 0.0, 0.0, 0.0],
            [0.0, 1.0, 0.0, 0.0],
            [0.0, 0.0, 1.0, 0.0],
            [0.0, 0.0, 0.0, 1.0],
        ];
        let mut groups = Vec::with_capacity(ranges.len() * 4);
        let mut instances = Vec::with_capacity(ranges.len());
        for (slot, range) in ranges.iter().enumerate() {
            groups.extend([range.first_index, range.index_count, slot as u32, 1]);
            instances.push(GpuInstanceRecord::new(IDENTITY, range.semantic_id));
        }
        gpu.set_mesh_instance_groups_packed(&groups, bytemuck::cast_slice(&instances))?;
    }
    Ok(triangles)
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
