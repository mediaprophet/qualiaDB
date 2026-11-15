//! Attach existing `.10d` sections (VOX0, FEMC, MRP1, FIEL, MID2). Does not fork the container.

use qualia_core_db::container_10d::correspondence_section::{
    correspondence_encoded_len, encode_correspondence_section, CorrespondenceMiniHeader,
    VertexTetBinding, CORRESPONDENCE_SECTION_VERSION, FLAG_FEMC_CONFORMING_BARYCENTRIC,
    FLAG_FEMC_HAS_NORMAL_ROTATION, MAGIC_FEMC,
};
use qualia_core_db::container_10d::field_section::{
    encode_field_section, FieldEntry, FieldRepresentationKind, FieldSupportKind, FieldTypeKind,
    FIELD_DESCRIPTOR_SIZE, FIELD_SECTION_HEADER_SIZE,
};
use qualia_core_db::container_10d::header::{Container10dHeader, MAX_SECTION_COUNT};
use qualia_core_db::container_10d::integrity::seal_whole_file_crc32c;
use qualia_core_db::container_10d::manifold_identity_v2::{
    encode_manifold_identity_v2, ManifoldIdentityV2, StableEntityId, TypedFieldRecord,
    FIELD_KIND_ELECTROMAGNETIC, MANIFOLD_IDENTITY_V2_MAX_BYTES,
};
use qualia_core_db::container_10d::morph_section::{
    encode_morph_section, morph_section_encoded_len, quantize_morph_delta, MorphSectionHeader,
    QuantizedMorphDelta, FLAG_MORPH_HAS_NORMALS, FLAG_MORPH_SPARSE, MAGIC_MRP1,
    MORPH_SECTION_VERSION,
};
use qualia_core_db::container_10d::section::{
    encode_container, parse_section_table, AlignmentTier, SectionInput, SectionTableError,
    SectionType,
};
use qualia_core_db::container_10d::voxel_section::{
    encode_voxel_section, Vox0DagNode, VoxelSectionMiniHeader, MAGIC_VOX0, VOXEL_SECTION_VERSION,
};
use qualia_core_db::q_hash;

use super::cardiac::{heartbeat_envelope, BEAT_DURATION};
use super::occupancy::OccupancyGrid;
use super::{TriangleMesh, SealError};

const LONGITUDINAL: f32 = 0.15;
const RADIAL: f32 = 0.22;

pub fn attach_volume_fields(
    compiled: &[u8],
    mesh: &TriangleMesh,
    grid: &OccupancyGrid,
) -> Result<Vec<u8>, SealError> {
    let vox = encode_vox0(grid)?;
    let packed = attach_raw_section(
        compiled,
        SectionType::VoxelLattice,
        AlignmentTier::CacheLine,
        0,
        0,
        &vox,
    )?;
    let femc = encode_femc(mesh, grid)?;
    let packed = attach_raw_section(
        &packed,
        SectionType::CorrespondenceMap,
        AlignmentTier::CacheLine,
        0,
        0,
        &femc,
    )?;
    let morph = encode_systole_morph(mesh)?;
    let packed = attach_raw_section(
        &packed,
        SectionType::Reconstruction,
        AlignmentTier::CacheLine,
        0,
        0,
        &morph,
    )?;
    let fiel = encode_emf_fiel()?;
    let fiel_len = fiel.len() as u64;
    let packed = attach_raw_section(
        &packed,
        SectionType::HyperspectralCube,
        AlignmentTier::CacheLine,
        0,
        0,
        &fiel,
    )?;
    let mid2 = encode_mid2_emf(mesh, fiel_len)?;
    attach_raw_section(
        &packed,
        SectionType::ManifoldIdentityV2,
        AlignmentTier::Page,
        0,
        0,
        &mid2,
    )
}

fn encode_vox0(grid: &OccupancyGrid) -> Result<Vec<u8>, SealError> {
    let size = grid.size();
    let n = grid.resolution.max(1) as f32;
    let mut node = Vox0DagNode::default();
    node.child_mask = 0;
    node.leaf_mask = 0xFF;
    node.primary_material_lut = majority_material(grid);
    node.secondary_material_lut = 1;
    node.child_ptr_or_density = occupied_density(grid);
    node.moisture_u8 = 90;
    node.compaction_u8 = 140;
    node.update_crc();
    let header = VoxelSectionMiniHeader {
        magic: MAGIC_VOX0,
        version: VOXEL_SECTION_VERSION,
        flags: 0,
        node_count: 1,
        stride: 16,
        root_node_index: 0,
        max_depth: 1,
        _pad: 0,
        aabb_min: [grid.min[0] * 10.0, grid.min[1] * 10.0, grid.min[2] * 10.0],
        aabb_max: [grid.max[0] * 10.0, grid.max[1] * 10.0, grid.max[2] * 10.0],
        voxel_resolution_mm: (size[0] / n * 10.0).max(0.1),
        _reserved: [0; 12],
    };
    let mut buf = vec![0u8; 64 + 16];
    let written = encode_voxel_section(&header, &[node], &mut buf)
        .map_err(|e| SealError::Attach(format!("VOX0: {e:?}")))?;
    buf.truncate(written);
    Ok(buf)
}

fn encode_femc(mesh: &TriangleMesh, grid: &OccupancyGrid) -> Result<Vec<u8>, SealError> {
    let n = mesh.positions.len();
    let header = CorrespondenceMiniHeader {
        magic: MAGIC_FEMC,
        version: CORRESPONDENCE_SECTION_VERSION,
        flags: FLAG_FEMC_CONFORMING_BARYCENTRIC | FLAG_FEMC_HAS_NORMAL_ROTATION,
        binding_count: n as u32,
        tet_cell_count: 1,
    };
    let bindings: Vec<VertexTetBinding> = mesh
        .positions
        .iter()
        .enumerate()
        .map(|(i, p)| {
            let normal = mesh
                .normals
                .get(i)
                .copied()
                .unwrap_or_else(|| outward(*p, grid.min, grid.max));
            VertexTetBinding::from_barycentric(0, [1.0, 0.0, 0.0, 0.0], normal)
        })
        .collect();
    let needed = correspondence_encoded_len(bindings.len())
        .ok_or_else(|| SealError::Attach("FEMC length overflow".into()))?;
    let mut buf = vec![0u8; needed];
    let written = encode_correspondence_section(&header, &bindings, &mut buf)
        .map_err(|e| SealError::Attach(format!("FEMC: {e:?}")))?;
    buf.truncate(written);
    Ok(buf)
}

fn encode_systole_morph(mesh: &TriangleMesh) -> Result<Vec<u8>, SealError> {
    let n = mesh.positions.len() as u32;
    if n == 0 {
        return Err(SealError::EmptyMesh);
    }
    let (min, max) = super::bounds(&mesh.positions);
    let size = [
        (max[0] - min[0]).max(1.0e-3),
        (max[1] - min[1]).max(1.0e-3),
        (max[2] - min[2]).max(1.0e-3),
    ];
    let centroid = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let env = heartbeat_envelope(0.06) as f32;
    let mut deltas: Vec<QuantizedMorphDelta> = Vec::with_capacity(mesh.positions.len());
    let mut max_disp = 1.0e-4f32;
    let mut raw: Vec<[f32; 3]> = Vec::with_capacity(mesh.positions.len());
    for p in &mesh.positions {
        let toward = [
            centroid[0] - p[0],
            centroid[1] - p[1],
            centroid[2] - p[2],
        ];
        let delta = [
            toward[0] * RADIAL * env,
            toward[1] * RADIAL * env + size[1] * LONGITUDINAL * env,
            toward[2] * RADIAL * env,
        ];
        let mag = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        max_disp = max_disp.max(mag);
        raw.push(delta);
    }
    for (i, delta) in raw.iter().enumerate() {
        let mag = (delta[0] * delta[0] + delta[1] * delta[1] + delta[2] * delta[2]).sqrt();
        if mag < 1.0e-5 {
            continue;
        }
        deltas.push(quantize_morph_delta(
            *delta,
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            max_disp,
            0,
            i as u32,
        ));
    }
    if deltas.is_empty() {
        deltas.push(quantize_morph_delta(
            [0.0, size[1] * LONGITUDINAL * env, 0.0],
            [0.0, 0.0, 0.0],
            [0.0, 0.0, 0.0],
            max_disp.max(size[1] * LONGITUDINAL),
            0,
            0,
        ));
    }
    let header = MorphSectionHeader {
        magic: MAGIC_MRP1,
        version: MORPH_SECTION_VERSION,
        target_count: 1,
        vertex_count: n,
        flags: FLAG_MORPH_SPARSE | FLAG_MORPH_HAS_NORMALS,
        max_displacement: max_disp,
        ontogenetic_bind_mask: 0,
        reserved: [0u8; 8],
    };
    let needed = morph_section_encoded_len(deltas.len())
        .ok_or_else(|| SealError::Attach("MRP1 length overflow".into()))?;
    let mut buf = vec![0u8; needed];
    let written = encode_morph_section(&header, &deltas, &mut buf)
        .map_err(|e| SealError::Attach(format!("MRP1: {e:?}")))?;
    buf.truncate(written);
    let _ = BEAT_DURATION;
    Ok(buf)
}

fn encode_emf_fiel() -> Result<Vec<u8>, SealError> {
    let entries = [
        FieldEntry::new(
            q_hash("electromagnetic.e_field"),
            q_hash("http://qudt.org/vocab/unit/V-PER-M"),
            FieldTypeKind::Vector,
            FieldSupportKind::Region,
            FieldRepresentationKind::Grid,
        ),
        FieldEntry::new(
            q_hash("electromagnetic.spectrum"),
            q_hash("sigma"),
            FieldTypeKind::Spectral,
            FieldSupportKind::Region,
            FieldRepresentationKind::Grid,
        ),
    ];
    let needed = FIELD_SECTION_HEADER_SIZE + entries.len() * FIELD_DESCRIPTOR_SIZE;
    let mut buf = vec![0u8; needed];
    let written = encode_field_section(&entries, &mut buf)
        .map_err(|e| SealError::Attach(format!("FIEL: {e:?}")))?;
    buf.truncate(written);
    Ok(buf)
}

fn encode_mid2_emf(mesh: &TriangleMesh, fiel_len: u64) -> Result<Vec<u8>, SealError> {
    let (min, max) = super::bounds(&mesh.positions);
    let cx = (min[0] + max[0]) * 0.5;
    let cy = (min[1] + max[1]) * 0.5;
    let cz = (min[2] + max[2]) * 0.5;
    let mut entity = [0u8; 16];
    entity[..14].copy_from_slice(b"anatomy.heart\0");
    let identity = ManifoldIdentityV2 {
        entity_id: StableEntityId(entity),
        profile_id: 1,
        domain_id: 1,
        coordinate_convention_id: 1,
        coordinates: [0.0, 0.0, 0.0, cx, cy, cz, 0.0, 0.70, 1.0, 0.12],
        external_digest: None,
    };
    let fields = [TypedFieldRecord {
        field_id: 1,
        kind: FIELD_KIND_ELECTROMAGNETIC,
        flags: 0,
        unit_id: q_hash("http://qudt.org/vocab/unit/V-PER-M"),
        payload_section_id: SectionType::HyperspectralCube as u32,
        payload_offset: 0,
        payload_length: fiel_len,
    }];
    let mut buf = vec![0u8; MANIFOLD_IDENTITY_V2_MAX_BYTES];
    let written = encode_manifold_identity_v2(&identity, &fields, &mut buf)
        .map_err(|e| SealError::Attach(format!("MID2: {e:?}")))?;
    buf.truncate(written);
    Ok(buf)
}

fn attach_raw_section(
    container: &[u8],
    section_type: SectionType,
    alignment_tier: AlignmentTier,
    stride: u32,
    element_count: u32,
    payload: &[u8],
) -> Result<Vec<u8>, SealError> {
    let header = Container10dHeader::parse(container)
        .map_err(|e| SealError::Attach(format!("10d header: {e:?}")))?;
    let descriptors = parse_section_table(container, &header)
        .map_err(|e| SealError::Attach(format!("section table: {e:?}")))?;
    if descriptors.len() >= MAX_SECTION_COUNT as usize {
        return Err(SealError::Attach("MAX_SECTION_COUNT reached".into()));
    }
    let mut inputs = Vec::with_capacity(descriptors.len() + 1);
    for descriptor in descriptors {
        let start = descriptor.byte_offset as usize;
        let end = start
            .checked_add(descriptor.byte_length as usize)
            .filter(|&end| end <= container.len())
            .ok_or_else(|| SealError::Attach("section payload out of bounds".into()))?;
        inputs.push(SectionInput {
            section_type: descriptor
                .typ()
                .ok_or_else(|| SealError::Attach("undefined section type".into()))?,
            alignment_tier: descriptor
                .tier()
                .ok_or_else(|| SealError::Attach("undefined alignment tier".into()))?,
            stride: descriptor.stride,
            element_count: descriptor.element_count,
            payload: &container[start..end],
        });
    }
    inputs.push(SectionInput {
        section_type,
        alignment_tier,
        stride,
        element_count,
        payload,
    });
    let needed = match encode_container(&header, &inputs, &mut []) {
        Err(SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Ok(size) => size,
        Err(error) => return Err(SealError::Attach(format!("encode dry-run: {error:?}"))),
    };
    let mut output = vec![0u8; needed];
    let written = encode_container(&header, &inputs, &mut output)
        .map_err(|e| SealError::Attach(format!("encode: {e:?}")))?;
    output.truncate(written);
    seal_whole_file_crc32c(&mut output);
    Ok(output)
}

fn majority_material(grid: &OccupancyGrid) -> u8 {
    let mut counts = [0u32; 4];
    for &m in &grid.material {
        if (m as usize) < 4 {
            counts[m as usize] += 1;
        }
    }
    let mut best = 1u8;
    let mut best_n = 0u32;
    for (i, &c) in counts.iter().enumerate().skip(1) {
        if c > best_n {
            best_n = c;
            best = i as u8;
        }
    }
    best
}

fn occupied_density(grid: &OccupancyGrid) -> u32 {
    let occupied = grid.material.iter().filter(|&&m| m != 0).count() as u32;
    let total = grid.material.len().max(1) as u32;
    ((occupied as u64 * 255) / total as u64) as u32
}

fn outward(p: [f32; 3], min: [f32; 3], max: [f32; 3]) -> [f32; 3] {
    let c = [
        (min[0] + max[0]) * 0.5,
        (min[1] + max[1]) * 0.5,
        (min[2] + max[2]) * 0.5,
    ];
    let d = [p[0] - c[0], p[1] - c[1], p[2] - c[2]];
    let len = (d[0] * d[0] + d[1] * d[1] + d[2] * d[2]).sqrt();
    if len > 1.0e-6 {
        [d[0] / len, d[1] / len, d[2] / len]
    } else {
        [0.0, 1.0, 0.0]
    }
}
