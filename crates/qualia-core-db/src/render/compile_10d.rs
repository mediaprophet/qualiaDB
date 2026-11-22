//! Compile an imported triangle `Mesh` into a sealed `.10d` container — the
//! **dense compiled-geometry** half of a geometry asset (see
//! `docs/manuals/standards/geometry-asset-ontology.md` §3). This is the
//! "mesh → `.10d`" step of the 3-D-anatomy asset pipeline: the renderer and the
//! anatomy layer read the `.10d` back with `decode_10d_mesh` instead of
//! reparsing the source GLB, and the q42 semantic manifest cites the container's
//! `compiled_digest`.
//!
//! **Scope (honest):** emits a `QuantizedMesh` section (u16-quantized
//! vertices in the bbox + u16/u32 indices). Optional `Tensor10DNodes` (D1),
//! provenance sidecars, and on native / `wasm-scientific` builds optional
//! `Topology` + `SpatialIndex` sections (C3) for scan-free picking. LOD chain
//! from `decimate_3` remains a separate pre-compile step.

use crate::container_10d::crc32c::crc32c;
use crate::container_10d::header::Container10dHeader;
use crate::container_10d::integrity::{compute_whole_file_crc32c, seal_whole_file_crc32c};
use crate::container_10d::mesh_section::{
    decode_mesh_section, encode_mesh_section, encoded_len, MeshSectionError,
};
use crate::container_10d::node_section::{
    parse_node_header, read_node, write_node_section_aos, NodeMiniHeader, NodeSectionError,
};
use crate::container_10d::normal_section::NormalSectionError;
use crate::container_10d::provenance_section::{
    encode_provenance_section, encoded_len as provenance_encoded_len, ProvenanceSectionError,
    ProvenanceSidecar,
};
use crate::container_10d::section::{
    encode_container, parse_section_table, AlignmentTier, SectionInput, SectionTableError,
    SectionType,
};
use crate::container_10d::{attach_material_section, MaterialContainerError};
use crate::render::assets::{
    import_asset_with_normals, mesh_to_nquins_with_dev, mesh_to_nquins_with_meta, AssetError, Mesh,
};
use crate::tensor::Tensor10D;
use crate::NQuin;
use std::collections::HashMap;

/// Optional extra sections for vision / recon seals (programme C3).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Compile10dExtras {
    /// Build half-edge Topology section from mesh triangles.
    pub topology: bool,
    /// Build BVH + kd-tree SpatialIndex over triangle AABBs / vertices.
    pub spatial_index: bool,
}

impl Compile10dExtras {
    /// Full vision recon package: topology + spatial index (when CG available).
    pub const VISION: Self = Self {
        topology: true,
        spatial_index: true,
    };
}

/// Failure modes for `.10d` compilation and read-back.
///
/// Not `Clone` — it wraps [`AssetError`], which is not `Clone`. Errors are consumed
/// on the failure path, not duplicated.
#[derive(Debug, PartialEq, Eq)]
pub enum Compile10dError {
    /// Decoding the source asset bytes (OBJ/STL/GLB) failed.
    Import(AssetError),
    /// The QuantizedMesh section encode/decode failed.
    Mesh(MeshSectionError),
    /// The optional vertex-normal field sidecar failed validation or encoding.
    Normals(NormalSectionError),
    /// Typed MID2 payload selection or codec admission failed.
    TypedPayload(crate::container_10d::TypedPayloadError),
    /// The container section table encode/decode failed.
    Section(SectionTableError),
    /// Encoding the provenance sidecar section failed.
    Provenance(ProvenanceSectionError),
    /// Encoding or reading the Tensor10DNodes section failed.
    Nodes(NodeSectionError),
    /// The optional MAT1 material/submesh section failed validation or encoding.
    Materials(MaterialContainerError),
    /// The optional UV01 texture-coordinate section failed validation or encoding.
    TextureCoordinates(crate::container_10d::TextureCoordinateContainerError),
    /// Stable material identities collided while scoping imported materials to an asset.
    MaterialIdentityCollision,
    /// A normal sidecar does not match the mesh vertex count.
    NormalVertexCountMismatch { expected: usize, got: usize },
    /// Topology / spatial-index extra section failed (C3).
    ExtraSection { kind: &'static str },
    /// The container parsed but held no `QuantizedMesh` section.
    NoMeshSection,
    /// The container parsed but held no `Tensor10DNodes` section.
    NoNodesSection,
    /// The 64-byte container header failed to parse on read-back.
    BadHeader,
    /// A section descriptor's byte range fell outside the container bytes.
    SectionOutOfBounds,
}

impl std::fmt::Display for Compile10dError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Import(e) => write!(f, ".10d compile: source import: {e}"),
            Self::Mesh(e) => write!(f, ".10d compile: mesh section: {e}"),
            Self::Normals(e) => write!(f, ".10d compile: normal field: {e}"),
            Self::TypedPayload(e) => write!(f, ".10d compile: {e}"),
            Self::Section(e) => write!(f, ".10d compile: section table: {e:?}"),
            Self::Provenance(e) => write!(f, ".10d compile: provenance section: {e}"),
            Self::Nodes(e) => write!(f, ".10d compile: Tensor10DNodes section: {e}"),
            Self::Materials(e) => write!(f, ".10d compile: MAT1 material section: {e:?}"),
            Self::TextureCoordinates(e) => {
                write!(f, ".10d compile: UV01 texture coordinates: {e:?}")
            }
            Self::MaterialIdentityCollision => {
                write!(f, ".10d compile: stable asset material identity collision")
            }
            Self::NormalVertexCountMismatch { expected, got } => write!(
                f,
                ".10d normal field: vertex count {got} does not match mesh count {expected}"
            ),
            Self::ExtraSection { kind } => write!(f, ".10d compile: extra section {kind} failed"),
            Self::NoMeshSection => write!(f, ".10d: no QuantizedMesh section in container"),
            Self::NoNodesSection => write!(f, ".10d: no Tensor10DNodes section in container"),
            Self::BadHeader => write!(f, ".10d: container header failed to parse"),
            Self::SectionOutOfBounds => write!(f, ".10d: section byte range outside container"),
        }
    }
}

impl std::error::Error for Compile10dError {}

/// Compile a mesh into a **sealed** `.10d` container holding one `QuantizedMesh`
/// section. The whole-file CRC-32C is written (the container is self-verifying).
///
/// Cold path (one-shot asset compilation) — allocates the output `Vec`; the hot
/// render/query paths operate zero-heap on the decoded section, not here.
/// Deterministic: identical input → byte-identical container (attestable).
pub fn compile_mesh_to_10d(mesh: &Mesh) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_provenance(mesh, None)
}

/// Compile a mesh into a sealed `.10d`, optionally **bundling a provenance sidecar
/// physically inside the container** (P1) — the immutable source bytes + licence
/// (+ optional VC) the asset was derived from, so context is byte-inseparable: a
/// `.10d` copied on its own still carries what it came from and under what licence.
/// The renderer's governance path treats the presence of this section as the
/// attestation that makes the mesh *citable* (see `render/portal/mod.rs`).
pub fn compile_mesh_to_10d_with_provenance(
    mesh: &Mesh,
    provenance: Option<&ProvenanceSidecar>,
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_nodes_and_provenance(mesh, &[], provenance)
}

/// Compile mesh + optional Tensor10D nodes (vision detections / σ paint fuel).
///
/// Nodes are encoded as `SectionType::Tensor10DNodes` (AoS). Empty `nodes` is
/// equivalent to [`compile_mesh_to_10d`].
pub fn compile_mesh_to_10d_with_nodes(
    mesh: &Mesh,
    nodes: &[Tensor10D],
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_nodes_and_provenance(mesh, nodes, None)
}

/// Full vision/recon seal: mesh + nodes + optional provenance (no topology extras).
pub fn compile_mesh_to_10d_with_nodes_and_provenance(
    mesh: &Mesh,
    nodes: &[Tensor10D],
    provenance: Option<&ProvenanceSidecar>,
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_extras(mesh, nodes, provenance, Compile10dExtras::default(), None)
}

/// Vision recon seal: mesh + nodes + Topology + SpatialIndex when CG is linked (C3).
///
/// On slim WASM portal builds without `wasm-scientific`, extras are silently
/// skipped (mesh+nodes still seal) so product paths stay portable.
pub fn compile_mesh_to_10d_vision(
    mesh: &Mesh,
    nodes: &[Tensor10D],
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_extras(mesh, nodes, None, Compile10dExtras::VISION, None)
}

/// Vision recon seal + in-envelope provenance (D4).
pub fn compile_mesh_to_10d_vision_with_provenance(
    mesh: &Mesh,
    nodes: &[Tensor10D],
    provenance: &ProvenanceSidecar,
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_extras(
        mesh,
        nodes,
        Some(provenance),
        Compile10dExtras::VISION,
        None,
    )
}

/// Seal mesh + provenance + a per-vertex surface colour reading.
/// The reading length must equal the vertex count. One organ albedo is not
/// a substitute: callers that have no reading use the provenance-only seal.
pub fn compile_mesh_to_10d_with_surface_reading(
    mesh: &Mesh,
    provenance: Option<&ProvenanceSidecar>,
    reading: &[[f32; 4]],
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_extras(
        mesh,
        &[],
        provenance,
        Compile10dExtras::default(),
        Some(reading),
    )
}

/// Full seal with explicit extras (topology / spatial index).
pub fn compile_mesh_to_10d_with_extras(
    mesh: &Mesh,
    nodes: &[Tensor10D],
    provenance: Option<&ProvenanceSidecar>,
    extras: Compile10dExtras,
    surface_reading: Option<&[[f32; 4]]>,
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_extras_and_frames(
        mesh,
        nodes,
        provenance,
        extras,
        surface_reading,
        None,
        None,
    )
}

/// Compile a mesh and preserve a vertex-aligned authored normal stream in an optional `.10d`
/// FieldSidecar. The existing QuantizedMesh payload remains unchanged.
pub fn compile_mesh_to_10d_with_normals(
    mesh: &Mesh,
    normals: &[[f32; 3]],
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_extras_and_frames(
        mesh,
        &[],
        None,
        Compile10dExtras::default(),
        None,
        Some(normals),
        None,
    )
}

/// Compile authored vertex normals and tangent handedness in an optional FieldSidecar.
pub fn compile_mesh_to_10d_with_frames(
    mesh: &Mesh,
    normals: Option<&[[f32; 3]]>,
    tangents: Option<&[[f32; 4]]>,
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_extras_and_frames(
        mesh,
        &[],
        None,
        Compile10dExtras::default(),
        None,
        normals,
        tangents,
    )
}

/// Full seal with optional color reading and vertex frame fields plus topology/spatial extras.
pub fn compile_mesh_to_10d_with_extras_and_frames(
    mesh: &Mesh,
    nodes: &[Tensor10D],
    provenance: Option<&ProvenanceSidecar>,
    extras: Compile10dExtras,
    surface_reading: Option<&[[f32; 4]]>,
    normals: Option<&[[f32; 3]]>,
    tangents: Option<&[[f32; 4]]>,
) -> Result<Vec<u8>, Compile10dError> {
    // 1. Encode the QuantizedMesh section payload.
    let mut payload = vec![0u8; encoded_len(mesh.vertex_count(), mesh.triangle_count())];
    let written = encode_mesh_section(mesh, &mut payload).map_err(Compile10dError::Mesh)?;
    payload.truncate(written);

    // 1b. Encode Tensor10DNodes (AoS) when present.
    let node_payload: Option<Vec<u8>> = if nodes.is_empty() {
        None
    } else {
        let mut buf = vec![0u8; NodeMiniHeader::payload_bytes(nodes.len())];
        let n = write_node_section_aos(nodes, &mut buf).map_err(Compile10dError::Nodes)?;
        buf.truncate(n);
        Some(buf)
    };

    // 1c. Encode the provenance sidecar payload, if bundling one.
    let prov_payload: Option<Vec<u8>> = match provenance {
        Some(p) => {
            let mut buf = vec![0u8; provenance_encoded_len(p)];
            let n = encode_provenance_section(p, &mut buf).map_err(Compile10dError::Provenance)?;
            buf.truncate(n);
            Some(buf)
        }
        None => None,
    };

    // 1d. Optional Topology + SpatialIndex (native / wasm-scientific only).
    let topo_payload = if extras.topology {
        encode_topology_for_mesh(mesh)?
    } else {
        None
    };
    let spatial_payload = if extras.spatial_index {
        encode_spatial_index_for_mesh(mesh)?
    } else {
        None
    };

    let surface_payload = match surface_reading {
        Some(reading) if reading.len() == mesh.vertex_count() && !reading.is_empty() => {
            Some(crate::container_10d::surface_reading::encode_surface_reading(reading))
        }
        Some(_) => {
            return Err(Compile10dError::ExtraSection {
                kind: "surface reading length",
            });
        }
        None => None,
    };

    let frame_payload = match (normals, tangents) {
        (None, None) => None,
        (Some(values), _) if values.len() != mesh.vertex_count() => {
            return Err(Compile10dError::ExtraSection {
                kind: "normal field length",
            });
        }
        (_, Some(values)) if values.len() != mesh.vertex_count() => {
            return Err(Compile10dError::ExtraSection {
                kind: "tangent field length",
            });
        }
        _ => {
            let count = mesh.vertex_count();
            let needed = crate::container_10d::normal_section::frame_encoded_len(
                count,
                normals.is_some(),
                tangents.is_some(),
            )
            .ok_or(Compile10dError::ExtraSection {
                kind: "mesh frame field size",
            })?;
            let mut payload = vec![0u8; needed];
            let written = crate::container_10d::normal_section::encode_mesh_frames(
                normals,
                tangents,
                &mut payload,
            )
            .map_err(Compile10dError::Normals)?;
            payload.truncate(written);
            Some(payload)
        }
    };

    // 2. Assemble the container. Writer canonical-orders by section type.
    let header = Container10dHeader::proposed();
    let mut inputs = vec![SectionInput {
        section_type: SectionType::QuantizedMesh,
        alignment_tier: AlignmentTier::Page,
        stride: 0,
        element_count: 0,
        payload: &payload,
    }];
    if let Some(np) = &node_payload {
        // stride/element_count stay 0: payload includes 16-byte NodeMiniHeader +
        // N×40 tensors (same contract as container_10d conformance golden).
        inputs.push(SectionInput {
            section_type: SectionType::Tensor10DNodes,
            alignment_tier: AlignmentTier::CacheLine,
            stride: 0,
            element_count: 0,
            payload: np,
        });
    }
    if let Some(pp) = &prov_payload {
        inputs.push(SectionInput {
            section_type: SectionType::ProvenanceSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: pp,
        });
    }
    if let Some(np) = &frame_payload {
        inputs.push(SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: np,
        });
    }
    if let Some(tp) = &topo_payload {
        inputs.push(SectionInput {
            section_type: SectionType::Topology,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: tp,
        });
    }
    if let Some(sp) = &spatial_payload {
        inputs.push(SectionInput {
            section_type: SectionType::SpatialIndex,
            alignment_tier: AlignmentTier::Page,
            stride: 0,
            element_count: 0,
            payload: sp,
        });
    }
    if let Some(sr) = &surface_payload {
        inputs.push(SectionInput {
            section_type: SectionType::SurfaceReading,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: sr,
        });
    }
    // Dry-run against an empty buffer to size the output exactly.
    let needed = match encode_container(&header, &inputs, &mut []) {
        Err(SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Ok(n) => n,
        Err(e) => return Err(Compile10dError::Section(e)),
    };
    let mut out = vec![0u8; needed];
    let total = encode_container(&header, &inputs, &mut out).map_err(Compile10dError::Section)?;
    out.truncate(total);

    // 3. Seal the whole-file CRC-32C (the `compiledDigest` source).
    seal_whole_file_crc32c(&mut out);
    Ok(out)
}

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
fn encode_topology_for_mesh(mesh: &Mesh) -> Result<Option<Vec<u8>>, Compile10dError> {
    use crate::container_10d::topology_section::{
        encode_topology_section, encoded_len as topo_len,
    };
    use crate::specialized_libs::computational_geometry::{
        build_triangle_half_edges, required_edge_slots, EdgeSlot, HalfEdge,
    };

    if mesh.triangle_count() == 0 || mesh.vertex_count() == 0 {
        return Ok(None);
    }
    let vc = mesh.vertex_count() as u32;
    let fc = mesh.triangle_count() as u32;
    let mut edges = vec![HalfEdge::default(); mesh.triangles.len().saturating_mul(3)];
    let mut slots = vec![EdgeSlot::default(); required_edge_slots(mesh.triangles.len())];
    build_triangle_half_edges(vc, &mesh.triangles, &mut edges, &mut slots).map_err(|_| {
        Compile10dError::ExtraSection {
            kind: "topology_half_edges",
        }
    })?;
    let need = topo_len(vc, fc, edges.len() as u32);
    let mut buf = vec![0u8; need];
    let n = encode_topology_section(vc, fc, &edges, &mut buf).map_err(|_| {
        Compile10dError::ExtraSection {
            kind: "topology_encode",
        }
    })?;
    buf.truncate(n);
    Ok(Some(buf))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
fn encode_topology_for_mesh(_mesh: &Mesh) -> Result<Option<Vec<u8>>, Compile10dError> {
    Ok(None)
}

#[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
fn encode_spatial_index_for_mesh(mesh: &Mesh) -> Result<Option<Vec<u8>>, Compile10dError> {
    use crate::container_10d::spatial_index_section::{
        encode_spatial_index_section, encoded_len as spatial_len,
    };
    use crate::specialized_libs::computational_geometry::{
        build_bvh_recursive, build_kd_tree_3d, Aabb, BvhNode, KdNode, Point3,
    };

    if mesh.triangle_count() == 0 || mesh.vertex_count() == 0 {
        return Ok(None);
    }

    let mut aabbs = Vec::with_capacity(mesh.triangles.len());
    for tri in &mesh.triangles {
        let p0 = mesh.positions[tri[0] as usize];
        let p1 = mesh.positions[tri[1] as usize];
        let p2 = mesh.positions[tri[2] as usize];
        let min = Point3::new(
            p0[0].min(p1[0]).min(p2[0]) as f64,
            p0[1].min(p1[1]).min(p2[1]) as f64,
            p0[2].min(p1[2]).min(p2[2]) as f64,
        );
        let max = Point3::new(
            p0[0].max(p1[0]).max(p2[0]) as f64,
            p0[1].max(p1[1]).max(p2[1]) as f64,
            p0[2].max(p1[2]).max(p2[2]) as f64,
        );
        aabbs.push(Aabb::new(min, max));
    }
    let n = aabbs.len();
    let mut bvh_nodes = vec![BvhNode::default(); 2 * n];
    let mut bvh_indices = vec![0u32; n];
    let mut bvh_codes = vec![0u64; n];
    let mut bvh_sort = vec![0u32; n];
    let (bvh_count, bvh_root) = build_bvh_recursive(
        &aabbs,
        &mut bvh_nodes,
        &mut bvh_indices,
        &mut bvh_codes,
        &mut bvh_sort,
    )
    .map_err(|_| Compile10dError::ExtraSection { kind: "bvh_build" })?;

    let points: Vec<[f64; 3]> = mesh
        .positions
        .iter()
        .map(|p| [p[0] as f64, p[1] as f64, p[2] as f64])
        .collect();
    let np = points.len();
    let mut kd_nodes = vec![KdNode::default(); np];
    let mut kd_indices = vec![0u32; np];
    let mut kd_codes = vec![0u64; np];
    let mut kd_sort = vec![0u32; np];
    let (kd_count, kd_root) = build_kd_tree_3d(
        &points,
        &mut kd_nodes,
        &mut kd_indices,
        &mut kd_codes,
        &mut kd_sort,
    )
    .map_err(|_| Compile10dError::ExtraSection { kind: "kd_build" })?;

    let need = spatial_len(bvh_count as u32, kd_count as u32, n as u32, np as u32);
    let mut buf = vec![0u8; need];
    encode_spatial_index_section(
        &bvh_nodes[..bvh_count],
        &bvh_indices,
        bvh_root as u32,
        &kd_nodes[..kd_count],
        &kd_indices,
        kd_root as u32,
        &mut buf,
    )
    .map_err(|_| Compile10dError::ExtraSection {
        kind: "spatial_encode",
    })?;
    Ok(Some(buf))
}

#[cfg(not(any(not(target_arch = "wasm32"), feature = "wasm-scientific")))]
fn encode_spatial_index_for_mesh(_mesh: &Mesh) -> Result<Option<Vec<u8>>, Compile10dError> {
    Ok(None)
}

/// The `compiledDigest` a q42 asset manifest cites: the container's whole-file
/// CRC-32C (computed with the header's own CRC field zeroed, so it equals the
/// sealed header value). Deterministic; changes on any geometry-byte change.
#[inline]
pub fn compiled_digest(container_10d: &[u8]) -> u32 {
    compute_whole_file_crc32c(container_10d)
}

/// Read a `.10d` container back into a dequantized `Mesh` — the renderer /
/// anatomy path that avoids reparsing the source GLB. Extracts the first
/// `QuantizedMesh` section.
pub fn decode_10d_mesh(container_10d: &[u8]) -> Result<Mesh, Compile10dError> {
    let header =
        Container10dHeader::parse(container_10d).map_err(|_| Compile10dError::BadHeader)?;
    let descs = parse_section_table(container_10d, &header).map_err(Compile10dError::Section)?;
    for d in descs {
        if d.typ() == Some(SectionType::QuantizedMesh) {
            let start = d.byte_offset as usize;
            let end = start
                .checked_add(d.byte_length as usize)
                .ok_or(Compile10dError::SectionOutOfBounds)?;
            let payload = container_10d
                .get(start..end)
                .ok_or(Compile10dError::SectionOutOfBounds)?;
            return decode_mesh_section(payload).map_err(Compile10dError::Mesh);
        }
    }
    Err(Compile10dError::NoMeshSection)
}

/// Read an optional vertex-aligned normal sidecar from a `.10d` container. Missing sidecars
/// return `Ok(None)` so existing mesh-only assets retain their established generated-normal path.
pub fn decode_10d_normals(
    container_10d: &[u8],
    expected_vertex_count: usize,
) -> Result<Option<Vec<[f32; 3]>>, Compile10dError> {
    Ok(decode_10d_mesh_frames(container_10d, expected_vertex_count)?.and_then(|f| f.normals))
}

/// Read an optional vertex-aligned tangent frame stream from a `.10d` container.
pub fn decode_10d_tangents(
    container_10d: &[u8],
    expected_vertex_count: usize,
) -> Result<Option<Vec<[f32; 4]>>, Compile10dError> {
    Ok(decode_10d_mesh_frames(container_10d, expected_vertex_count)?.and_then(|f| f.tangents))
}

/// Read an optional vertex-aligned UV0 stream into caller-owned storage.
pub fn decode_10d_texture_coordinates_into(
    container_10d: &[u8],
    expected_vertex_count: usize,
    out: &mut [[f32; 2]],
) -> Result<Option<usize>, Compile10dError> {
    let header =
        Container10dHeader::parse(container_10d).map_err(|_| Compile10dError::BadHeader)?;
    let descriptors =
        parse_section_table(container_10d, &header).map_err(Compile10dError::Section)?;
    if !descriptors
        .iter()
        .any(|descriptor| descriptor.typ() == Some(SectionType::TextureCoordinates))
    {
        return Ok(None);
    }
    let count = crate::container_10d::read_texture_coordinate_section_into(
        container_10d,
        expected_vertex_count,
        out,
    )
    .map_err(Compile10dError::TextureCoordinates)?;
    Ok(Some(count))
}

/// Decode supported normal/tangent field data from the optional mesh FieldSidecar.
pub fn decode_10d_mesh_frames(
    container_10d: &[u8],
    expected_vertex_count: usize,
) -> Result<Option<crate::container_10d::MeshFrameStreams>, Compile10dError> {
    // Selection/codec admission lives in the focused shared SDK; this oversized
    // legacy compiler retains only compatibility error projection (see SDK note).
    crate::container_10d::read_container_mesh_frames_with_limits(
        container_10d,
        expected_vertex_count,
        crate::container_10d::MeshFrameDecodeLimits::COMPATIBILITY,
    )
    .map_err(|error| match error {
        crate::container_10d::TypedPayloadError::Header(_) => Compile10dError::BadHeader,
        crate::container_10d::TypedPayloadError::Sections(e) => Compile10dError::Section(e),
        crate::container_10d::TypedPayloadError::Frames(e) => Compile10dError::Normals(e),
        crate::container_10d::TypedPayloadError::VertexCount { expected, got } => {
            Compile10dError::NormalVertexCountMismatch { expected, got }
        }
        other => Compile10dError::TypedPayload(other),
    })
}

/// Read Tensor10D nodes from a sealed `.10d` (first Tensor10DNodes section).
///
/// Returns the node count written into `out` (caller buffer; truncated to
/// `out.len()`). Fail-closed if the section is missing or malformed.
pub fn decode_10d_nodes(
    container_10d: &[u8],
    out: &mut [Tensor10D],
) -> Result<usize, Compile10dError> {
    let header =
        Container10dHeader::parse(container_10d).map_err(|_| Compile10dError::BadHeader)?;
    let descs = parse_section_table(container_10d, &header).map_err(Compile10dError::Section)?;
    for d in descs {
        if d.typ() == Some(SectionType::Tensor10DNodes) {
            let start = d.byte_offset as usize;
            let end = start
                .checked_add(d.byte_length as usize)
                .ok_or(Compile10dError::SectionOutOfBounds)?;
            let payload = container_10d
                .get(start..end)
                .ok_or(Compile10dError::SectionOutOfBounds)?;
            let (nh, _) = parse_node_header(payload).map_err(Compile10dError::Nodes)?;
            let count = (nh.node_count as usize).min(out.len());
            for i in 0..count {
                out[i] = read_node(payload, i).map_err(Compile10dError::Nodes)?;
            }
            return Ok(count);
        }
    }
    Err(Compile10dError::NoNodesSection)
}

/// A fully compiled geometry asset: the source-imported `Mesh`, the sealed `.10d`
/// container, its two content digests, and the q42 semantic manifest that cites the
/// container by `compiledDigest` (geometry-asset-ontology §1 two-layer model).
///
/// This is the whole "GLB → `.10d` + manifest" pipeline output. The `container_10d`
/// bytes are the on-disk sidecar; `quins`/`lexicon` are the portable q42 facts that
/// travel in the graph and point at the container by hash.
#[derive(Debug, Clone)]
pub struct CompiledAsset {
    /// The imported triangle mesh (positions + indices + bbox).
    pub mesh: Mesh,
    /// The sealed `.10d` container (dense compiled geometry).
    pub container_10d: Vec<u8>,
    /// Content-addressed source images referenced by MAT1, ready for separate HMC packaging;
    /// asset bytes are intentionally not placed in the Sentinel arena.
    pub texture_dependencies: Vec<crate::render::assets::TextureDependency>,
    /// Whole-file CRC-32C of `container_10d` — the manifest's `compiledDigest`.
    pub compiled_digest: u32,
    /// CRC-32C of the immutable source bytes — the manifest's `sourceDigest`.
    pub source_digest: u32,
    /// The q42 manifest facts, including both digests (via `mesh_to_nquins_with_digests()`).
    pub quins: Vec<NQuin>,
    /// Object-lexicon for the string-valued facts in `quins`.
    pub lexicon: HashMap<u64, String>,
}

/// The end-to-end asset-compile step: source asset bytes → [`CompiledAsset`].
///
/// Runs the full pipeline — import the source ([`import_asset`]), compile the dense
/// `.10d` ([`compile_mesh_to_10d`]), hash both layers, then emit the q42 manifest that
/// binds them (`mesh_to_nquins_with_digests()`). Deterministic: identical
/// `(source_bytes, asset_uri, source_format)` → byte-identical container and identical
/// digests, so the manifest→container citation is attestable.
///
/// `hint` is the source-format hint forwarded to `import_asset` (e.g. `Some("glb")`);
/// `source_format` is the value recorded in the manifest and should agree with the real
/// format of `source_bytes`.
pub fn compile_asset(
    source_bytes: &[u8],
    hint: Option<&str>,
    asset_uri: &str,
    source_format: &str,
) -> Result<CompiledAsset, Compile10dError> {
    compile_organ_asset(
        source_bytes,
        hint,
        asset_uri,
        source_format,
        None,
        None,
        None,
    )
}

/// Like [`compile_asset`] but also binds the compiled asset to a 3D-body organ: its `body_system`
/// (which of the 17 systems colours it by burden) and `anatomy_model` (`"male"` / `"female"`, from the
/// user's declared XY/XX basis). This is the compile step for an anatomy organ mesh — the manifest it
/// emits carries `geo:bodySystem` + `geo:anatomyModel` so the renderer can look the organ up in the
/// per-system percept table (S5.1) and both colour and sonify it.
#[allow(clippy::too_many_arguments)]
pub fn compile_organ_asset(
    source_bytes: &[u8],
    hint: Option<&str>,
    asset_uri: &str,
    source_format: &str,
    body_system: Option<&str>,
    anatomy_model: Option<&str>,
    provenance: Option<&ProvenanceSidecar>,
) -> Result<CompiledAsset, Compile10dError> {
    let imported =
        import_asset_with_normals(source_bytes, hint).map_err(Compile10dError::Import)?;
    let mesh = imported.mesh;
    let texture_dependencies = imported.texture_dependencies;
    let texture_coordinates_0 = imported.texture_coordinates_0;
    // When provenance is supplied it is sealed into the `.10d` as a ProvenanceSidecar section, so the
    // asset is attested (the renderer's fail-closed governance gate treats an unattested asset with the
    // default-refuse disposition as REFUSE).
    let mut container_10d = compile_mesh_to_10d_with_extras_and_frames(
        &mesh,
        &[],
        provenance,
        Compile10dExtras::default(),
        None,
        imported.normals.as_deref(),
        imported.tangents.as_deref(),
    )?;
    if let Some(coordinates) = texture_coordinates_0.as_deref() {
        container_10d =
            crate::container_10d::attach_texture_coordinate_section(&container_10d, coordinates)
                .map_err(Compile10dError::TextureCoordinates)?;
    }
    if !imported.submeshes.is_empty() {
        let (materials, ranges) = crate::render::material_compile::scope_material_bindings(
            asset_uri,
            &imported.materials,
            &imported.submeshes,
        )
        .map_err(|_| Compile10dError::MaterialIdentityCollision)?;
        let mesh_index_count = u32::try_from(mesh.triangle_count().saturating_mul(3))
            .map_err(|_| Compile10dError::MaterialIdentityCollision)?;
        container_10d =
            attach_material_section(&container_10d, &materials, &ranges, mesh_index_count)
                .map_err(Compile10dError::Materials)?;
    }
    let compiled = compiled_digest(&container_10d);
    let source = crc32c(source_bytes);
    let (quins, lexicon) = mesh_to_nquins_with_meta(
        &mesh,
        asset_uri,
        source_format,
        source,
        compiled,
        body_system,
        anatomy_model,
    );
    Ok(CompiledAsset {
        mesh,
        container_10d,
        texture_dependencies,
        compiled_digest: compiled,
        source_digest: source,
        quins,
        lexicon,
    })
}

/// Like [`compile_asset`] but binds the compiled asset to a point on the developmental **`t`-axis** — its
/// `gestational_age_days` (postfertilization) and `carnegie_stage`. This is the compile step for a fetal/
/// embryonic stage: consecutive stages, ordered by gestational age, form a 4-D developmental body (the
/// maternal–fetal dyad's fetal side, reproductive-continuum plan §2).
pub fn compile_developmental_asset(
    source_bytes: &[u8],
    hint: Option<&str>,
    asset_uri: &str,
    source_format: &str,
    gestational_age_days: u16,
    carnegie_stage: u8,
) -> Result<CompiledAsset, Compile10dError> {
    let imported =
        import_asset_with_normals(source_bytes, hint).map_err(Compile10dError::Import)?;
    let mesh = imported.mesh;
    let texture_dependencies = imported.texture_dependencies;
    let texture_coordinates_0 = imported.texture_coordinates_0;
    let mut container_10d = compile_mesh_to_10d_with_extras_and_frames(
        &mesh,
        &[],
        None,
        Compile10dExtras::default(),
        None,
        imported.normals.as_deref(),
        imported.tangents.as_deref(),
    )?;
    if let Some(coordinates) = texture_coordinates_0.as_deref() {
        container_10d =
            crate::container_10d::attach_texture_coordinate_section(&container_10d, coordinates)
                .map_err(Compile10dError::TextureCoordinates)?;
    }
    if !imported.submeshes.is_empty() {
        let (materials, ranges) = crate::render::material_compile::scope_material_bindings(
            asset_uri,
            &imported.materials,
            &imported.submeshes,
        )
        .map_err(|_| Compile10dError::MaterialIdentityCollision)?;
        let mesh_index_count = u32::try_from(mesh.triangle_count().saturating_mul(3))
            .map_err(|_| Compile10dError::MaterialIdentityCollision)?;
        container_10d =
            attach_material_section(&container_10d, &materials, &ranges, mesh_index_count)
                .map_err(Compile10dError::Materials)?;
    }
    let compiled = compiled_digest(&container_10d);
    let source = crc32c(source_bytes);
    let (quins, lexicon) = mesh_to_nquins_with_dev(
        &mesh,
        asset_uri,
        source_format,
        source,
        compiled,
        gestational_age_days,
        carnegie_stage,
    );
    Ok(CompiledAsset {
        mesh,
        container_10d,
        texture_dependencies,
        compiled_digest: compiled,
        source_digest: source,
        quins,
        lexicon,
    })
}

/// Compile a primary 10D computational or Mode-B Archetype-Instance `.10d` container
/// without requiring an inline `QuantizedMesh` section.
///
/// Supports:
/// - **Profile 0 (`ComputationalOnly`):** `identity` (`MID2`) + optional `nodes` (`Tensor10DNodes`),
///   `strata` (`MSTR` `0x0304`, `L0`..=`L6`), and `symbolic_cbor_ld` (`CBLD` `0x0305`).
/// - **Profile 2 / Mode B (`ArchetypeInstance`):** Adds `archetype_ref` (`AREF` `0x0301`)
///   referencing a shared Mode-A Archetype `.10d` by its 256-bit digest while overriding
///   10D coordinates `[q, v, w, x, y, z, t, α, μ, σ]`, multi-scale strata, and symbolic RDF state.
pub fn compile_computational_instance_to_10d(
    identity: &crate::container_10d::ManifoldIdentityV2,
    archetype_ref: Option<&crate::container_10d::ArchetypeRefRecord>,
    strata: &[crate::container_10d::NestedStratumNodeRecord],
    coupling_rule_graph_hash: u64,
    symbolic_cbor_ld: Option<&crate::container_10d::SymbolicRdfCborLdPayload>,
    nodes: &[Tensor10D],
) -> Result<Vec<u8>, Compile10dError> {
    use crate::container_10d::{
        encode_archetype_ref_record, encode_manifold_identity_v2, encode_multiscale_strata_section,
        encode_symbolic_rdf_cbor_ld, manifold_identity_v2_encoded_len,
        multiscale_strata_encoded_len, TypedFieldRecord, ARCHETYPE_REF_RECORD_SIZE,
        FIELD_KIND_ARCHETYPE_REF, FIELD_KIND_MULTISCALE_STRATA, FIELD_KIND_SYMBOLIC_RDF_CBOR_LD,
    };

    let mut sidecar_bytes: Vec<u8> = Vec::new();
    let mut fields: Vec<TypedFieldRecord> = Vec::new();
    let mut next_field_id = 1u64;

    if let Some(aref) = archetype_ref {
        let offset = sidecar_bytes.len() as u64;
        let mut buf = [0u8; ARCHETYPE_REF_RECORD_SIZE];
        let written = encode_archetype_ref_record(aref, &mut buf)
            .map_err(|_| Compile10dError::ExtraSection { kind: "AREF" })?;
        sidecar_bytes.extend_from_slice(&buf[..written]);
        fields.push(TypedFieldRecord {
            field_id: next_field_id,
            kind: FIELD_KIND_ARCHETYPE_REF,
            flags: 0,
            unit_id: 0,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: offset,
            payload_length: written as u64,
        });
        next_field_id += 1;
    }

    if !strata.is_empty() {
        let offset = sidecar_bytes.len() as u64;
        let needed = multiscale_strata_encoded_len(strata.len())
            .ok_or(Compile10dError::ExtraSection { kind: "MSTR" })?;
        let mut buf = vec![0u8; needed];
        let written = encode_multiscale_strata_section(strata, coupling_rule_graph_hash, &mut buf)
            .map_err(|_| Compile10dError::ExtraSection { kind: "MSTR" })?;
        sidecar_bytes.extend_from_slice(&buf[..written]);
        fields.push(TypedFieldRecord {
            field_id: next_field_id,
            kind: FIELD_KIND_MULTISCALE_STRATA,
            flags: 0,
            unit_id: 0,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: offset,
            payload_length: written as u64,
        });
        next_field_id += 1;
    }

    if let Some(cbld) = symbolic_cbor_ld {
        let offset = sidecar_bytes.len() as u64;
        let encoded = encode_symbolic_rdf_cbor_ld(cbld)
            .map_err(|_| Compile10dError::ExtraSection { kind: "CBLD" })?;
        let written = encoded.len();
        sidecar_bytes.extend_from_slice(&encoded);
        fields.push(TypedFieldRecord {
            field_id: next_field_id,
            kind: FIELD_KIND_SYMBOLIC_RDF_CBOR_LD,
            flags: 0,
            unit_id: 0,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: offset,
            payload_length: written as u64,
        });
    }

    let mid2_len = manifold_identity_v2_encoded_len(fields.len())
        .map_err(|_| Compile10dError::ExtraSection { kind: "MID2" })?;
    let mut mid2_bytes = vec![0u8; mid2_len];
    let mid2_written = encode_manifold_identity_v2(identity, &fields, &mut mid2_bytes)
        .map_err(|_| Compile10dError::ExtraSection { kind: "MID2" })?;
    mid2_bytes.truncate(mid2_written);

    let node_payload: Option<Vec<u8>> = if nodes.is_empty() {
        None
    } else {
        let mut buf = vec![0u8; NodeMiniHeader::payload_bytes(nodes.len())];
        let n = write_node_section_aos(nodes, &mut buf).map_err(Compile10dError::Nodes)?;
        buf.truncate(n);
        Some(buf)
    };

    let mut sections: Vec<SectionInput<'_>> = Vec::new();
    if let Some(ref np) = node_payload {
        sections.push(SectionInput {
            section_type: SectionType::Tensor10DNodes,
            alignment_tier: AlignmentTier::Page,
            stride: 0,
            element_count: 0,
            payload: np,
        });
    }
    if !sidecar_bytes.is_empty() {
        sections.push(SectionInput {
            section_type: SectionType::FieldSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &sidecar_bytes,
        });
    }
    sections.push(SectionInput {
        section_type: SectionType::ManifoldIdentityV2,
        alignment_tier: AlignmentTier::Page,
        stride: 0,
        element_count: 0,
        payload: &mid2_bytes,
    });

    let total_payload: usize = sections.iter().map(|s| s.payload.len() + 64).sum();
    let mut out = vec![0u8; 256 + sections.len() * 24 + total_payload];
    let header = Container10dHeader::proposed();
    let total_written =
        encode_container(&header, &sections, &mut out).map_err(Compile10dError::Section)?;
    out.truncate(total_written);
    let _ = seal_whole_file_crc32c(&mut out);
    Ok(out)
}

/// Losslessly upgrade an existing B-Rep `.10d` container (v1 or v2) into a full
/// **Holographic Phase-Space + Multi-Scale Computational `.10d` v2 container**.
///
/// - Preserves every existing section (`QuantizedMesh`, `Materials`, `TextureCoordinates`,
///   `SurfaceReading`, `Topology`, `SpatialIndex`, `SkeletalAnimation`, `EntityOntology`,
///   `ProvenanceSidecar`, etc.) byte-for-byte.
/// - Fits a 64-byte [`crate::q42::m64_matter::M64HoloRecord`] (`sdf_harmonics: [i8; 12]`,
///   Hamiltonian eigen-frequencies, damping, allometric bounds, 6-band spectral SPD) and
///   12-triangle proxy bounding lens (`HolographicPhaseHeader`, `max_raymarch_steps = 32`)
///   from the mesh and surface reading, encoding them as `FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE` (`0x0306`, `HOLO`).
/// - Attaches `SectionType::ProceduralSynthesis` (`PROC`, Section `18`) and
///   `SectionType::ManifoldIdentityV2` (`MID2`, Section `15`) if not already present,
///   preserving any existing `FieldSidecar` (`NRM1`/`FRM1` normals/tangents or `MRP1` morphs)
///   as coexisting `MID2` typed field slices.
pub fn upgrade_legacy_10d_to_holographic_v2(
    bytes: &[u8],
    entity_uri: &str,
    domain_id: u64,
    coordinates_10d: [f32; 10],
) -> Result<Vec<u8>, Compile10dError> {
    use crate::container_10d::{
        decode_manifold_identity_v2, decode_mesh_section, encode_holographic_phase_section,
        encode_manifold_identity_v2, encode_procedural_synthesis_section,
        fit_m64_holo_segments_from_mesh_zero_heap, holographic_phase_encoded_len,
        is_normal_section, manifold_identity_v2_encoded_len,
        surface_reading::decode_surface_reading, ManifoldIdentityV2, ProceduralSynthesisHeader,
        StableEntityId, TypedFieldRecord, FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE,
        FIELD_KIND_MORPH_TARGETS, FIELD_KIND_SURFACE_FRAMES, FLAG_ALLOMETRIC_GROWTH,
        FLAG_CELLULAR_VORONOI, FLAG_IMPLICIT_FILLET, FLAG_PGA_MOTOR_DYNAMICS, MAGIC_PROC,
        MAX_FITTED_HOLO_SEGMENTS, PROCEDURAL_SYNTHESIS_HEADER_SIZE,
        PROCEDURAL_SYNTHESIS_SECTION_VERSION,
    };
    use crate::container_10d::{
        decode_symbolic_rdf_cbor_ld, encode_multiscale_strata_section,
        encode_symbolic_rdf_cbor_ld, multiscale_strata_encoded_len, MultilingualLexicalTerm,
        NestedStratumNodeRecord, SymbolicRdfCborLdPayload, FIELD_KIND_MULTISCALE_STRATA,
        FIELD_KIND_SYMBOLIC_RDF_CBOR_LD, MANIFOLD_IDENTITY_V2_MAX_FIELDS,
    };
    use crate::q42::m64_matter::M64HoloRecord;
    use crate::bundle::semantic_model::{holo_address_payload, validated_holo_header, HOLO_ADDRESS_CONTEXT};
    use crate::q_hash;

    const GEOMETRY_CONTEXT: &str = "https://qualiadb.org/contexts/10d-geometry-v1";
    const ASSET_CLASS: &str = "https://qualiadb.org/ontology/hmc#SpatialAsset";
    const RDF_TYPE: &str = "http://www.w3.org/1999/02/22-rdf-syntax-ns#type";
    let parsed_header = Container10dHeader::parse(bytes).map_err(|_| Compile10dError::BadHeader)?;
    let descs = parse_section_table(bytes, &parsed_header).map_err(Compile10dError::Section)?;

    let mut mesh_opt = None;
    let mut surface_opt = None;
    let mut existing_sidecar: Option<&[u8]> = None;
    let mut existing_mid2: Option<&[u8]> = None;
    let mut has_proc = false;
    let mut has_provenance = false;
    let mut preserved_sections: Vec<(SectionType, AlignmentTier, u32, u32, &[u8])> = Vec::new();

    for d in descs {
        let st = SectionType::from_u8(d.section_type).ok_or(Compile10dError::BadHeader)?;
        let tier = AlignmentTier::from_u8(d.alignment_tier).unwrap_or(AlignmentTier::Word);
        let start = d.byte_offset as usize;
        let end = start
            .checked_add(d.byte_length as usize)
            .ok_or(Compile10dError::SectionOutOfBounds)?;
        let slice = bytes
            .get(start..end)
            .ok_or(Compile10dError::SectionOutOfBounds)?;

        match st {
            SectionType::QuantizedMesh => {
                mesh_opt = Some(decode_mesh_section(slice).map_err(Compile10dError::Mesh)?);
                preserved_sections.push((st, tier, d.stride, d.element_count, slice));
            }
            SectionType::SurfaceReading => {
                surface_opt = decode_surface_reading(slice);
                preserved_sections.push((st, tier, d.stride, d.element_count, slice));
            }
            SectionType::FieldSidecar => {
                existing_sidecar = Some(slice);
            }
            SectionType::ManifoldIdentityV2 => {
                existing_mid2 = Some(slice);
            }
            SectionType::ProceduralSynthesis => {
                has_proc = true;
                preserved_sections.push((st, tier, d.stride, d.element_count, slice));
            }
            SectionType::ProvenanceSidecar => {
                has_provenance = true;
                preserved_sections.push((st, tier, d.stride, d.element_count, slice));
            }
            _ => {
                preserved_sections.push((st, tier, d.stride, d.element_count, slice));
            }
        }
    }

    let subj_hash = q_hash(entity_uri);
    let prov_hash = q_hash(&format!("{entity_uri}#provenance"));
    if subj_hash == 0 || !crate::bundle::semantic_model::valid_iri(entity_uri) {
        return Err(Compile10dError::ExtraSection { kind: "CBLD identity" });
    }
    let metric_variety = if domain_id == 1 { 2 } else { 0 };

    let proxy_mesh = match mesh_opt {
        Some(ref m) => m.clone(),
        None => crate::render::portal_load_10d::synthesize_holographic_proxy_box_mesh(
            [-0.5, 0.0, -0.5],
            [0.5, 1.0, 0.5],
        ),
    };

    let mut holo_segments = [M64HoloRecord::default(); MAX_FITTED_HOLO_SEGMENTS];
    let (holo_hdr, holo_seg_count) = fit_m64_holo_segments_from_mesh_zero_heap(
        &proxy_mesh,
        surface_opt.as_deref(),
        subj_hash,
        prov_hash,
        metric_variety,
        &mut holo_segments,
    );

    let mut sidecar_combined: Vec<u8> = Vec::new();
    let mut fields: Vec<TypedFieldRecord> = Vec::new();
    let mut next_field_id = 1u64;
    let mut has_geometry_cbld = false;

    if let Some(mid2_bytes) = existing_mid2 {
        let mut decoded_fields = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
        let (prior_identity, fcount) =
            decode_manifold_identity_v2(mid2_bytes, &[], &mut decoded_fields)
                .map_err(|_| Compile10dError::ExtraSection { kind: "MID2" })?;
            // Only short-circuit if there is no mesh to re-fit HOLO from and all v2 sections exist.
            if mesh_opt.is_none()
                && decoded_fields[..fcount]
                    .iter()
                    .any(|f| f.kind == FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE)
                && has_proc
                && has_provenance
                && decoded_fields[..fcount].iter().any(|f| f.kind == FIELD_KIND_SYMBOLIC_RDF_CBOR_LD)
                && decoded_fields[..fcount].iter().any(|f| f.kind == FIELD_KIND_MULTISCALE_STRATA)
            {
                return append_holo_addresses_to_existing_10d(
                    bytes, &parsed_header, &prior_identity, &decoded_fields[..fcount], entity_uri,
                );
            }
            let mut old_holo_addresses = Vec::new();
            let mut expected_holo_addresses = Vec::new();
            for f in &decoded_fields[..fcount] {
                if f.kind == FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE {
                    if f.payload_section_id == SectionType::FieldSidecar as u32 {
                        let offset = usize::try_from(f.payload_offset)
                            .map_err(|_| Compile10dError::ExtraSection { kind: "HOLO" })?;
                        let length = usize::try_from(f.payload_length)
                            .map_err(|_| Compile10dError::ExtraSection { kind: "HOLO" })?;
                        let end = offset.checked_add(length)
                            .ok_or(Compile10dError::ExtraSection { kind: "HOLO" })?;
                        let old = existing_sidecar.and_then(|sidecar| sidecar.get(offset..end))
                            .ok_or(Compile10dError::ExtraSection { kind: "HOLO" })?;
                        let prior = validated_holo_header(old)
                            .map_err(|_| Compile10dError::ExtraSection { kind: "HOLO" })?;
                        if prior.nquin_subject_hash == subj_hash {
                            expected_holo_addresses.push(holo_address_payload(
                                entity_uri, f.field_id, prior.holo_segment_count,
                            ).map_err(|_| Compile10dError::ExtraSection { kind: "CBLD HOLO addresses" })?);
                        }
                    }
                    continue;
                }
                let mut f_copy = *f;
                if f.payload_section_id == SectionType::FieldSidecar as u32 {
                    if let Some(sc) = existing_sidecar {
                        let s = f.payload_offset as usize;
                        let e = s.saturating_add(f.payload_length as usize);
                        if let Some(sub) = sc.get(s..e) {
                            if f.kind == FIELD_KIND_SYMBOLIC_RDF_CBOR_LD {
                                let payload = decode_symbolic_rdf_cbor_ld(sub)
                                    .map_err(|_| Compile10dError::ExtraSection { kind: "CBLD" })?;
                                if payload.context_iri == HOLO_ADDRESS_CONTEXT {
                                    old_holo_addresses.push(payload);
                                    continue;
                                }
                                if payload.context_iri == GEOMETRY_CONTEXT {
                                    if !payload.lexicon.iter().any(|term| {
                                        term.term_hash == subj_hash && term.iri_utf8 == entity_uri
                                    }) {
                                        return Err(Compile10dError::ExtraSection {
                                            kind: "CBLD identity",
                                        });
                                    }
                                    has_geometry_cbld = true;
                                }
                            }
                            let new_off = sidecar_combined.len() as u64;
                            sidecar_combined.extend_from_slice(sub);
                            f_copy.payload_offset = new_off;
                        }
                    }
                }
                fields.push(f_copy);
                if f_copy.field_id >= next_field_id {
                    next_field_id = f_copy.field_id.checked_add(1)
                        .ok_or(Compile10dError::ExtraSection { kind: "MID2" })?;
                }
            }
            for old in old_holo_addresses {
                let matched = expected_holo_addresses.iter().position(|expected| expected == &old)
                    .ok_or(Compile10dError::ExtraSection { kind: "CBLD HOLO addresses" })?;
                expected_holo_addresses.swap_remove(matched);
            }
    } else if let Some(sc) = existing_sidecar {
        sidecar_combined.extend_from_slice(sc);
        if fields.is_empty() && !sc.is_empty() {
            if is_normal_section(sc) {
                fields.push(TypedFieldRecord {
                    field_id: next_field_id,
                    kind: FIELD_KIND_SURFACE_FRAMES,
                    flags: 0,
                    unit_id: 0,
                    payload_section_id: SectionType::FieldSidecar as u32,
                    payload_offset: 0,
                    payload_length: sc.len() as u64,
                });
                next_field_id += 1;
            } else if sc.len() >= 4 && &sc[0..4] == b"MRP1" {
                fields.push(TypedFieldRecord {
                    field_id: next_field_id,
                    kind: FIELD_KIND_MORPH_TARGETS,
                    flags: 0,
                    unit_id: 0,
                    payload_section_id: SectionType::FieldSidecar as u32,
                    payload_offset: 0,
                    payload_length: sc.len() as u64,
                });
                next_field_id += 1;
            }
        }
    }

    let prov_iri = format!("{entity_uri}#provenance");
    let predicate = q_hash(RDF_TYPE);
    let object = q_hash(ASSET_CLASS);
    let context = q_hash(GEOMETRY_CONTEXT);
    let mut extra_cbld_iris: Vec<String> = Vec::new();
    let mut rewritten_mat_buf: Vec<u8> = Vec::new();
    let mut rewritten_eont_buf: Vec<u8> = Vec::new();
    if existing_mid2.is_none() {
        if let Some(ref mesh) = mesh_opt {
            let index_count = (mesh.triangle_count() * 3) as u32;
            for entry in &mut preserved_sections {
                if entry.0 == SectionType::Materials {
                    use crate::container_10d::material_section::{
                        decode_material_section_into, encode_material_section,
                        encoded_len as mat_encoded_len, material_section_counts, MaterialRecord,
                        SubmeshRange,
                    };
                    if let Ok((m_count, r_count)) = material_section_counts(entry.4) {
                        let mut mats = vec![MaterialRecord::legacy_default(); m_count];
                        let mut rngs = vec![
                            SubmeshRange {
                                first_index: 0,
                                index_count: 0,
                                material_id: 0,
                                semantic_id: 0,
                            };
                            r_count
                        ];
                        if decode_material_section_into(entry.4, index_count, &mut mats, &mut rngs)
                            .is_ok()
                        {
                            for (idx, rng) in rngs.iter_mut().enumerate() {
                                let sub_iri = format!("{entity_uri}#submesh-{idx}");
                                rng.semantic_id = q_hash(&sub_iri);
                                extra_cbld_iris.push(sub_iri);
                            }
                            if let Some(needed) = mat_encoded_len(mats.len(), rngs.len()) {
                                rewritten_mat_buf = vec![0u8; needed];
                                if let Ok(written) = encode_material_section(
                                    &mats,
                                    &rngs,
                                    index_count,
                                    &mut rewritten_mat_buf,
                                ) {
                                    rewritten_mat_buf.truncate(written);
                                } else {
                                    rewritten_mat_buf.clear();
                                }
                            }
                        }
                    }
                } else if entry.0 == SectionType::EntityOntology {
                    use crate::container_10d::{
                        AffordanceBinaryRecord, EntityOntologyMiniHeader, AFFORDANCE_RECORD_SIZE,
                        ENTITY_ONTOLOGY_MINI_HEADER_SIZE, MAX_AFFORDANCES_PER_SECTION,
                    };
                    if entry.4.len() >= ENTITY_ONTOLOGY_MINI_HEADER_SIZE {
                        let hdr: EntityOntologyMiniHeader = bytemuck::pod_read_unaligned(
                            &entry.4[..ENTITY_ONTOLOGY_MINI_HEADER_SIZE],
                        );
                        let count = hdr.affordance_count as usize;
                        let needed_min =
                            ENTITY_ONTOLOGY_MINI_HEADER_SIZE + count * AFFORDANCE_RECORD_SIZE;
                        if count <= MAX_AFFORDANCES_PER_SECTION && entry.4.len() >= needed_min {
                            rewritten_eont_buf = entry.4.to_vec();
                            for idx in 0..count {
                                let off = ENTITY_ONTOLOGY_MINI_HEADER_SIZE
                                    + idx * AFFORDANCE_RECORD_SIZE;
                                let mut rec: AffordanceBinaryRecord = bytemuck::pod_read_unaligned(
                                    &rewritten_eont_buf[off..off + AFFORDANCE_RECORD_SIZE],
                                );
                                let act_iri = format!("{entity_uri}#affordance-action-{idx}");
                                rec.action_id = q_hash(&act_iri);
                                extra_cbld_iris.push(act_iri);
                                if rec.required_capability_hash != 0 {
                                    let cap_iri =
                                        format!("{entity_uri}#affordance-capability-{idx}");
                                    rec.required_capability_hash = q_hash(&cap_iri);
                                    extra_cbld_iris.push(cap_iri);
                                }
                                if rec.emitted_topic_hash != 0 {
                                    let top_iri = format!("{entity_uri}#affordance-topic-{idx}");
                                    rec.emitted_topic_hash = q_hash(&top_iri);
                                    extra_cbld_iris.push(top_iri);
                                }
                                rewritten_eont_buf[off..off + AFFORDANCE_RECORD_SIZE]
                                    .copy_from_slice(bytemuck::bytes_of(&rec));
                            }
                        }
                    }
                }
            }
        }
    }
    let mut lexicon: Vec<MultilingualLexicalTerm> =
        [entity_uri, &prov_iri, RDF_TYPE, ASSET_CLASS, GEOMETRY_CONTEXT]
            .into_iter()
            .map(|iri| MultilingualLexicalTerm {
                term_hash: q_hash(iri),
                iri_utf8: iri.to_owned(),
                lang_bcp47: None,
                label_utf8: String::new(),
            })
            .collect();
    for iri in &extra_cbld_iris {
        let term_hash = q_hash(iri);
        if !lexicon.iter().any(|t| t.term_hash == term_hash) {
            lexicon.push(MultilingualLexicalTerm {
                term_hash,
                iri_utf8: iri.clone(),
                lang_bcp47: None,
                label_utf8: String::new(),
            });
        }
    }
    let payload = SymbolicRdfCborLdPayload {
        context_iri: GEOMETRY_CONTEXT.into(),
        lexicon,
        nquins: vec![[
            subj_hash, predicate, object, context, 0,
            crate::NQuin::calculate_parity(subj_hash, predicate, object, context, 0),
        ]],
        n3_logic_rules_utf8: Vec::new(),
        shacl_shape_iris_utf8: Vec::new(),
    };
    let cbld = encode_symbolic_rdf_cbor_ld(&payload)
        .map_err(|_| Compile10dError::ExtraSection { kind: "CBLD" })?;
    let has_strata = fields.iter().any(|field| field.kind == FIELD_KIND_MULTISCALE_STRATA);
    let mut add_field = |kind, payload: &[u8]| -> Result<(), Compile10dError> {
        let current_id = next_field_id;
        next_field_id = current_id.checked_add(1)
            .ok_or(Compile10dError::ExtraSection { kind: "MID2" })?;
        fields.push(TypedFieldRecord {
            field_id: current_id,
            kind,
            flags: 0,
            unit_id: 0,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: sidecar_combined.len() as u64,
            payload_length: payload.len() as u64,
        });
        sidecar_combined.extend_from_slice(payload);
        Ok(())
    };
    // An existing stratum hierarchy remains authored data. Legacy geometry has
    // only enough evidence for one root stratum.
    if !has_geometry_cbld {
        add_field(FIELD_KIND_SYMBOLIC_RDF_CBOR_LD, &cbld)?;
    }
    if !has_strata {
        let mut root = <NestedStratumNodeRecord as bytemuck::Zeroable>::zeroed();
        root.host_anchor.entity_hash = subj_hash;
        root.host_anchor.anchor_domain = 0xff;
        root.parent_node_index = u16::MAX;
        root.taxon_or_ontology_iri_hash = object;
        root.embodiment_state = crate::container_10d::EmbodimentResolutionState::InlineEmbodied as u8;
        let mut strata = vec![0; multiscale_strata_encoded_len(1)
            .ok_or(Compile10dError::ExtraSection { kind: "MSTR" })?];
        let written = encode_multiscale_strata_section(&[root], 0, &mut strata)
            .map_err(|_| Compile10dError::ExtraSection { kind: "MSTR" })?;
        add_field(FIELD_KIND_MULTISCALE_STRATA, &strata[..written])?;
    }

    // Align HOLO payload start to 64 bytes inside FieldSidecar
    while sidecar_combined.len() % 64 != 0 {
        sidecar_combined.push(0);
    }
    let holo_offset = sidecar_combined.len() as u64;
    let holo_len = holographic_phase_encoded_len(holo_seg_count, 0).unwrap();
    let mut holo_buf = vec![0u8; holo_len];
    let holo_written = encode_holographic_phase_section(
        holo_hdr,
        &holo_segments[..holo_seg_count],
        &[],
        &mut holo_buf,
    )
    .map_err(|_| Compile10dError::ExtraSection { kind: "HOLO" })?;
    sidecar_combined.extend_from_slice(&holo_buf[..holo_written]);

    let holo_field_id = next_field_id;
    fields.push(TypedFieldRecord {
        field_id: holo_field_id,
        kind: FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE,
        flags: 0,
        unit_id: 0,
        payload_section_id: SectionType::FieldSidecar as u32,
        payload_offset: holo_offset,
        payload_length: holo_written as u64,
    });
    let sealed_holo = validated_holo_header(&holo_buf[..holo_written])
        .map_err(|_| Compile10dError::ExtraSection { kind: "HOLO" })?;
    let projection = holo_address_payload(entity_uri, holo_field_id, sealed_holo.holo_segment_count)
        .map_err(|_| Compile10dError::ExtraSection { kind: "CBLD HOLO addresses" })?;
    let projection = encode_symbolic_rdf_cbor_ld(&projection)
        .map_err(|_| Compile10dError::ExtraSection { kind: "CBLD HOLO addresses" })?;
    fields.push(TypedFieldRecord {
        field_id: holo_field_id.checked_add(1).ok_or(Compile10dError::ExtraSection { kind: "MID2" })?,
        kind: FIELD_KIND_SYMBOLIC_RDF_CBOR_LD,
        flags: 0,
        unit_id: 0,
        payload_section_id: SectionType::FieldSidecar as u32,
        payload_offset: sidecar_combined.len() as u64,
        payload_length: projection.len() as u64,
    });
    sidecar_combined.extend_from_slice(&projection);

    let mut entity_id_bytes = [0u8; 16];
    entity_id_bytes[0..8].copy_from_slice(&subj_hash.to_le_bytes());
    entity_id_bytes[8..16].copy_from_slice(&prov_hash.to_le_bytes());
    if entity_id_bytes.iter().all(|&b| b == 0) {
        entity_id_bytes[0] = 1;
    }

    let identity = ManifoldIdentityV2 {
        entity_id: StableEntityId(entity_id_bytes),
        profile_id: 1,
        domain_id: domain_id.max(1),
        coordinate_convention_id: 1,
        coordinates: coordinates_10d,
        external_digest: None,
    };

    let mid2_needed = manifold_identity_v2_encoded_len(fields.len())
        .map_err(|_| Compile10dError::ExtraSection { kind: "MID2" })?;
    let mut mid2_buf = vec![0u8; mid2_needed];
    let mid2_written = encode_manifold_identity_v2(&identity, &fields, &mut mid2_buf)
        .map_err(|_| Compile10dError::ExtraSection { kind: "MID2" })?;
    mid2_buf.truncate(mid2_written);

    let mut proc_buf = [0u8; PROCEDURAL_SYNTHESIS_HEADER_SIZE];
    if !has_proc {
        let proc_hdr = ProceduralSynthesisHeader {
            magic: MAGIC_PROC,
            version: PROCEDURAL_SYNTHESIS_SECTION_VERSION,
            flags: (FLAG_CELLULAR_VORONOI
                | FLAG_IMPLICIT_FILLET
                | FLAG_ALLOMETRIC_GROWTH
                | FLAG_PGA_MOTOR_DYNAMICS) as u16,
            seed: subj_hash,
            voronoi_frequency: 12.0,
            voronoi_jitter: 0.85,
            fillet_blend_radius: holo_hdr.sdf_smooth_union_radius,
            fillet_threshold: 0.02,
            allometry_exponent_k: 0.75,
            allometry_base_b: 1.0,
            spiral_growth_rate_k: 0.22,
            spiral_initial_radius_a: 0.05,
            motor_frequency_hz: 1.2,
            motor_damping_ratio: 0.35,
            critical_stress_threshold: 45.0,
            reserved_f32: 0.0,
        };
        let _ = encode_procedural_synthesis_section(&proc_hdr, &mut proc_buf)
            .map_err(|_| Compile10dError::ExtraSection { kind: "PROC" })?;
    }

    let mut auto_prov_buf: Vec<u8> = Vec::new();
    if !has_provenance {
        let digest_bytes: [u8; 32] = *blake3::hash(bytes).as_bytes();
        let default_prv = ProvenanceSidecar::new(
            digest_bytes.to_vec(),
            "application/x-qualia-10d-v2",
            "CC-BY-NC-ND-4.0",
        )
        .with_metadata(Vec::new(), 1_700_000_000, digest_bytes);
        auto_prov_buf.resize(provenance_encoded_len(&default_prv), 0);
        let written = encode_provenance_section(&default_prv, &mut auto_prov_buf)
            .map_err(Compile10dError::Provenance)?;
        auto_prov_buf.truncate(written);
    }

    let mut sections: Vec<SectionInput<'_>> = preserved_sections
        .iter()
        .map(|&(section_type, alignment_tier, stride, element_count, payload)| {
            let actual_payload = if section_type == SectionType::Materials
                && !rewritten_mat_buf.is_empty()
            {
                &rewritten_mat_buf[..]
            } else if section_type == SectionType::EntityOntology
                && !rewritten_eont_buf.is_empty()
            {
                &rewritten_eont_buf[..]
            } else {
                payload
            };
            SectionInput {
                section_type,
                alignment_tier,
                stride,
                element_count,
                payload: actual_payload,
            }
        })
        .collect();

    if !has_provenance && !auto_prov_buf.is_empty() {
        sections.push(SectionInput {
            section_type: SectionType::ProvenanceSidecar,
            alignment_tier: AlignmentTier::Word,
            stride: 0,
            element_count: 0,
            payload: &auto_prov_buf,
        });
    }

    sections.push(SectionInput {
        section_type: SectionType::FieldSidecar,
        alignment_tier: AlignmentTier::Page,
        stride: 0,
        element_count: 0,
        payload: &sidecar_combined,
    });
    sections.push(SectionInput {
        section_type: SectionType::ManifoldIdentityV2,
        alignment_tier: AlignmentTier::Page,
        stride: 0,
        element_count: 0,
        payload: &mid2_buf,
    });
    if !has_proc {
        sections.push(SectionInput {
            section_type: SectionType::ProceduralSynthesis,
            alignment_tier: AlignmentTier::Page,
            stride: 0,
            element_count: 0,
            payload: &proc_buf,
        });
    }

    let total_payload: usize = sections.iter().map(|s| s.payload.len() + 64).sum();
    let mut out = vec![0u8; 256 + sections.len() * 24 + total_payload];
    let header = Container10dHeader::proposed();
    let total_written =
        encode_container(&header, &sections, &mut out).map_err(Compile10dError::Section)?;
    out.truncate(total_written);
    let _ = seal_whole_file_crc32c(&mut out);
    Ok(out)
}

fn append_holo_addresses_to_existing_10d(
    bytes: &[u8],
    header: &Container10dHeader,
    identity: &crate::container_10d::ManifoldIdentityV2,
    fields: &[crate::container_10d::TypedFieldRecord],
    root: &str,
) -> Result<Vec<u8>, Compile10dError> {
    use crate::bundle::semantic_model::{holo_address_payload, validated_holo_header, HOLO_ADDRESS_CONTEXT};
    use crate::container_10d::{
        decode_symbolic_rdf_cbor_ld, encode_manifold_identity_v2, encode_symbolic_rdf_cbor_ld,
        manifold_identity_v2_encoded_len, TypedFieldRecord, TypedPayloadResolver,
        FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE, FIELD_KIND_SYMBOLIC_RDF_CBOR_LD,
        MANIFOLD_IDENTITY_V2_MAX_FIELDS,
    };

    let error = || Compile10dError::ExtraSection { kind: "CBLD HOLO addresses" };
    let mut scratch = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let mut decoded = [TypedFieldRecord::default(); MANIFOLD_IDENTITY_V2_MAX_FIELDS];
    let view = TypedPayloadResolver::new(bytes, &[], &mut scratch, &mut decoded)
        .map_err(|_| error())?;
    let mut pending = Vec::new();
    for field in view.payloads_of_kind(FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE) {
        let field = field.map_err(|_| error())?;
        let holo = validated_holo_header(field.bytes).map_err(|_| error())?;
        if holo.nquin_subject_hash != crate::q_hash(root) {
            return Err(error());
        }
        pending.push(holo_address_payload(root, field.record.field_id, holo.holo_segment_count)
            .map_err(|_| error())?);
    }
    for field in view.payloads_of_kind(FIELD_KIND_SYMBOLIC_RDF_CBOR_LD) {
        let field = field.map_err(|_| error())?;
        let payload = decode_symbolic_rdf_cbor_ld(field.bytes).map_err(|_| error())?;
        if payload.context_iri == HOLO_ADDRESS_CONTEXT {
            let index = pending.iter().position(|expected| expected == &payload).ok_or_else(error)?;
            pending.remove(index);
        }
    }
    if pending.is_empty() {
        return Ok(bytes.to_vec());
    }

    let mut sidecar = view.section_payload(SectionType::FieldSidecar).unwrap_or_default().to_vec();
    let mut fields = fields.to_vec();
    for payload in pending {
        let encoded = encode_symbolic_rdf_cbor_ld(&payload).map_err(|_| error())?;
        let field_id = fields.last().map_or(0, |field| field.field_id).checked_add(1)
            .ok_or_else(error)?;
        fields.push(TypedFieldRecord {
            field_id,
            kind: FIELD_KIND_SYMBOLIC_RDF_CBOR_LD,
            flags: 0,
            unit_id: 0,
            payload_section_id: SectionType::FieldSidecar as u32,
            payload_offset: sidecar.len() as u64,
            payload_length: encoded.len() as u64,
        });
        sidecar.extend_from_slice(&encoded);
    }
    let needed = manifold_identity_v2_encoded_len(fields.len()).map_err(|_| error())?;
    let mut new_identity = vec![0; needed];
    let written = encode_manifold_identity_v2(identity, &fields, &mut new_identity).map_err(|_| error())?;
    new_identity.truncate(written);

    let mut sections = Vec::new();
    for descriptor in parse_section_table(bytes, header).map_err(Compile10dError::Section)? {
        let kind = descriptor.typ().ok_or(Compile10dError::BadHeader)?;
        let mut section = SectionInput {
            section_type: kind,
            alignment_tier: AlignmentTier::from_u8(descriptor.alignment_tier)
                .ok_or(Compile10dError::BadHeader)?,
            stride: descriptor.stride,
            element_count: descriptor.element_count,
            payload: view.section_payload(kind).ok_or(Compile10dError::SectionOutOfBounds)?,
        };
        match kind {
            SectionType::FieldSidecar => section.payload = &sidecar,
            SectionType::ManifoldIdentityV2 => section.payload = &new_identity,
            _ => {}
        }
        sections.push(section);
    }
    if !sections.iter().any(|section| section.section_type == SectionType::FieldSidecar) {
        sections.push(SectionInput {
            section_type: SectionType::FieldSidecar, alignment_tier: AlignmentTier::Page,
            stride: 0, element_count: 0, payload: &sidecar,
        });
    }
    let needed = match encode_container(header, &sections, &mut []) {
        Err(crate::container_10d::SectionTableError::OutputBufferTooSmall { needed, .. }) => needed,
        Err(other) => return Err(Compile10dError::Section(other)),
        Ok(_) => return Err(error()),
    };
    if needed > 128 * 1024 * 1024 {
        return Err(Compile10dError::ExtraSection { kind: "HOLO projection exceeds the 128 MiB 10D ring" });
    }
    let mut out = vec![0; needed];
    let written = encode_container(header, &sections, &mut out).map_err(Compile10dError::Section)?;
    out.truncate(written);
    let _ = seal_whole_file_crc32c(&mut out);
    Ok(out)
}

/// Source-licensing & provenance governance Tier for converted 3D assets and scenes
/// (`core-ontologies/source-asset-licensing.n3`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLicenseTier {
    /// `CC-BY-NC-ND-4.0` (Author Moral & Material Rights / UDHR Art. 27 / ICESCR Art. 15 / Timothy Charles Holborn).
    AuthorMoralRightsQualiaCcByNcNd,
    /// `PC-PAYDOWN-1.0` / `PermissiveCommonsPaydown` (`pc:Work` cost-recovery to true commons).
    PermissiveCommonsPaydown,
    /// `PerpetualCreativeRoyalty` (Ongoing artist/creator revenue share).
    PerpetualCreativeRoyalty,
    /// `SPC` / `SanctuaryProtected` (Inalienable biosphere / human-dignity protection).
    SanctuaryProtected,
    /// Permissive upstream open licenses (`CC0-1.0`, `CC-BY-4.0`, `MIT`, `Apache-2.0`, `BSD-3-Clause`).
    UpstreamPermissiveOpen,
    /// Upstream ShareAlike or NonCommercial (`CC-BY-SA-4.0`, `CC-BY-NC-4.0`, `CC-BY-NC-SA-4.0`).
    UpstreamShareAlikeOrNc,
    /// Upstream custom commercial or proprietary license.
    UpstreamProprietaryRestricted,
}

impl SourceLicenseTier {
    pub fn classify_spdx(spdx: &str) -> Self {
        let trimmed = spdx.trim();
        match trimmed {
            "CC-BY-NC-ND-4.0" => Self::AuthorMoralRightsQualiaCcByNcNd,
            "PC-PAYDOWN-1.0" | "PermissiveCommonsPaydown" => Self::PermissiveCommonsPaydown,
            "PerpetualCreativeRoyalty" | "PERPETUAL-ROYALTY-1.0" => Self::PerpetualCreativeRoyalty,
            "SPC" | "SanctuaryProtected" => Self::SanctuaryProtected,
            "CC0-1.0" | "CC0" | "CC-BY-4.0" | "MIT" | "Apache-2.0" | "BSD-3-Clause"
            | "BSD-2-Clause" => Self::UpstreamPermissiveOpen,
            "CC-BY-SA-4.0" | "CC-BY-NC-4.0" | "CC-BY-NC-SA-4.0" => Self::UpstreamShareAlikeOrNc,
            _ => Self::UpstreamProprietaryRestricted,
        }
    }

    pub fn iri(self) -> &'static str {
        match self {
            Self::AuthorMoralRightsQualiaCcByNcNd => {
                "https://ns.webcivics.net/source-asset-licensing/AuthorMoralRightsQualiaCcByNcNd"
            }
            Self::PermissiveCommonsPaydown => {
                "https://ns.webcivics.net/source-asset-licensing/PermissiveCommonsPaydownTier"
            }
            Self::PerpetualCreativeRoyalty => {
                "https://ns.webcivics.net/source-asset-licensing/PerpetualCreativeRoyaltyTier"
            }
            Self::SanctuaryProtected => {
                "https://ns.webcivics.net/source-asset-licensing/SanctuaryProtectedTier"
            }
            Self::UpstreamPermissiveOpen => {
                "https://ns.webcivics.net/source-asset-licensing/UpstreamPermissiveOpen"
            }
            Self::UpstreamShareAlikeOrNc => {
                "https://ns.webcivics.net/source-asset-licensing/UpstreamShareAlikeOrNc"
            }
            Self::UpstreamProprietaryRestricted => {
                "https://ns.webcivics.net/source-asset-licensing/UpstreamProprietaryRestricted"
            }
        }
    }

    pub fn permits_uncompensated_commercial_use(self) -> bool {
        matches!(self, Self::UpstreamPermissiveOpen)
    }
}

/// Attested source-licensing and provenance descriptor required when converting
/// existing 3D formatted assets or scenes into Holographic `.10d` v2 + `.q42`.
#[derive(Debug, Clone)]
pub struct SourceLicenseAttribution {
    /// Non-empty SPDX or Qualia license identifier (e.g. `"CC-BY-NC-ND-4.0"`, `"CC-BY-4.0"`, `"PC-PAYDOWN-1.0"`).
    pub spdx_license: String,
    /// Original creator / author name or DID (UTF-8).
    pub creator_name_or_did: String,
    /// Mandatory UTF-8 attribution and moral-rights notice (travels with artifact via `spc:ArtifactAttachedRule`).
    pub attribution_notice_utf8: String,
    /// Optional origin URL, DOI, or catalogue identifier.
    pub origin_uri: Option<String>,
    /// Harvest or conversion UNIX epoch timestamp (seconds).
    pub timestamp_epoch_s: u64,
    /// Whether to embed the full raw `source_bytes` inside `.10d` `PRV1` (`true`) or a compact digest-bound header.
    pub embed_full_source_bytes: bool,
}

/// Result of converting an external 3D asset or multi-node 3D scene into Holographic `.10d` v2 + `.q42`.
#[derive(Debug, Clone)]
pub struct Converted3dScenePackage {
    /// The primary Mode-A Archetype `.10d` v2 container (with `QuantizedMesh`, `HOLO` `0x0306`,
    /// `PROC` `18`, `MID2` `15`, `CBLD` `0x0305`, `PRV1` `7`, and any `SKL1`/`MRP1`/`MAT4`/`UV01`).
    pub archetype_10d: Vec<u8>,
    /// Whole-file CRC-32C of `archetype_10d`.
    pub archetype_compiled_digest: u32,
    /// 256-bit BLAKE3 digest of `archetype_10d` used by Mode-B scene node instances (`AREF` `0x0301`).
    pub archetype_blake3_digest: [u8; 32],
    /// CRC-32C of the original 3D source bytes.
    pub source_digest: u32,
    /// Classified source-licensing tier.
    pub license_tier: SourceLicenseTier,
    /// Mode-B instance `.10d` v2 containers for multi-node scene hierarchies (referencing `archetype_10d` via `AREF`).
    pub instance_10d_containers: Vec<(String, Vec<u8>)>,
    /// Content-addressed textures extracted from the source 3D file (`.glb`).
    pub texture_dependencies: Vec<crate::render::assets::TextureDependency>,
    /// Emitted 48-byte `NQuin` semantic manifest + source-licensing & provenance facts.
    pub quins: Vec<NQuin>,
    /// UTF-8 lexicon mapping `NQuin` hashes to their terms.
    pub lexicon: HashMap<u64, String>,
}

/// Convert an existing 3D formatted asset or scene (`.glb`/`.gltf` static or living scene with skins/anims/morphs,
/// `.obj`, `.stl`, or legacy `.10d`) into a Holographic `.10d` v2 package with mandatory source-licensing
/// & provenance validation (`core-ontologies/source-asset-licensing.n3`).
///
/// Fails closed with [`Compile10dError::Provenance`] if `license.spdx_license` or
/// `license.attribution_notice_utf8` is empty.
pub fn convert_3d_asset_or_scene_with_license(
    source_bytes: &[u8],
    format_hint: Option<&str>,
    asset_uri: &str,
    domain_id: u32,
    coordinates_10d: [f32; 10],
    license: &SourceLicenseAttribution,
) -> Result<Converted3dScenePackage, Compile10dError> {
    use crate::container_10d::{
        encode_symbolic_rdf_cbor_ld, ArchetypeDigestAlg, ArchetypeRefRecord, ArchetypeTargetKind,
        ManifoldIdentityV2, MultilingualLexicalTerm, StableEntityId, SymbolicRdfCborLdPayload,
        OVERRIDE_FLAG_COORDS_10D,
    };
    use crate::q_hash;

    if license.spdx_license.trim().is_empty() || license.attribution_notice_utf8.trim().is_empty() {
        return Err(Compile10dError::Provenance(
            ProvenanceSectionError::MissingLicence,
        ));
    }
    let license_tier = SourceLicenseTier::classify_spdx(&license.spdx_license);
    let source_crc = crc32c(source_bytes);
    let source_blake3: [u8; 32] = *blake3::hash(source_bytes).as_bytes();

    // Detect if input is already a legacy `.10d` file
    let is_legacy_10d = format_hint == Some("10d")
        || (source_bytes.len() >= 64 && &source_bytes[0..4] == b"10d\0");

    // Build Symbolic RDF CBOR-LD payload (CBLD 0x0305) carrying UTF-8 provenance & license terms
    let subj_hash = q_hash(asset_uri);
    let creator_iri = format!("urn:qualia:agent:{}", license.creator_name_or_did.trim());
    let creator_hash = q_hash(&creator_iri);
    let tier_hash = q_hash(license_tier.iri());
    let p_derived_from = q_hash("http://www.w3.org/ns/prov#wasDerivedFrom");
    let p_attributed_to = q_hash("https://ns.webcivics.net/permissive-commons/attributedTo");
    let p_license_tier = q_hash("https://ns.webcivics.net/source-asset-licensing/licenseTier");
    let p_spdx = q_hash("https://ns.webcivics.net/source-asset-licensing/spdxIdentifier");
    let p_src_crc = q_hash("https://ns.webcivics.net/source-asset-licensing/sourceCrc32c");
    let spdx_hash = q_hash(license.spdx_license.trim());
    let src_node_iri = format!("{asset_uri}#source");
    let src_node_hash = q_hash(&src_node_iri);
    let ctx_hash = q_hash("https://ns.webcivics.net/source-asset-licensing/");

    let mk_q = |s: u64, p: u64, o: u64| NQuin {
        subject: s,
        predicate: p,
        object: o,
        context: ctx_hash,
        metadata: 0,
        parity: NQuin::calculate_parity(s, p, o, ctx_hash, 0),
    };

    let license_quins = vec![
        mk_q(subj_hash, p_derived_from, src_node_hash),
        mk_q(subj_hash, p_attributed_to, creator_hash),
        mk_q(subj_hash, p_license_tier, tier_hash),
        mk_q(src_node_hash, p_spdx, spdx_hash),
        mk_q(src_node_hash, p_src_crc, source_crc as u64),
    ];

    let cbld_payload = SymbolicRdfCborLdPayload {
        context_iri: "https://qualiadb.org/contexts/10d-symbolic-v2.cbor-ld".to_string(),
        lexicon: vec![
            MultilingualLexicalTerm {
                term_hash: subj_hash,
                iri_utf8: asset_uri.to_string(),
                lang_bcp47: Some("en".to_string()),
                label_utf8: license.attribution_notice_utf8.clone(),
            },
            MultilingualLexicalTerm {
                term_hash: creator_hash,
                iri_utf8: creator_iri.clone(),
                lang_bcp47: None,
                label_utf8: license.creator_name_or_did.clone(),
            },
            MultilingualLexicalTerm {
                term_hash: spdx_hash,
                iri_utf8: format!("https://spdx.org/licenses/{}", license.spdx_license.trim()),
                lang_bcp47: None,
                label_utf8: license.spdx_license.trim().to_string(),
            },
        ],
        nquins: license_quins
            .iter()
            .map(|q| [q.subject, q.predicate, q.object, q.context, q.metadata, q.parity])
            .collect(),
        n3_logic_rules_utf8: vec![
            "{ ?emb prov:wasDerivedFrom ?src . ?src src-lic:originalCreator ?c } => { ?emb pc:attributedTo ?c } .".to_string(),
        ],
        shacl_shape_iris_utf8: vec![
            "https://ns.webcivics.net/source-asset-licensing/SourceAssetProvenanceShape".to_string(),
        ],
    };
    let cbld_bytes = encode_symbolic_rdf_cbor_ld(&cbld_payload).unwrap_or_default();

    let media_type = match format_hint {
        Some("glb") | Some("gltf") => "model/gltf-binary",
        Some("obj") => "model/obj",
        Some("stl") => "model/stl",
        Some("step") | Some("stp") => "model/step",
        Some("dxf") => "image/vnd.dxf",
        Some("10d") => "application/x-qualia-10d",
        _ if is_legacy_10d => "application/x-qualia-10d",
        _ if source_bytes.starts_with(b"glTF") => "model/gltf-binary",
        _ => "application/octet-stream",
    };

    let carried_source = if license.embed_full_source_bytes {
        source_bytes.to_vec()
    } else {
        source_blake3.to_vec()
    };
    let prv_sidecar = ProvenanceSidecar::new(
        carried_source,
        media_type,
        license.spdx_license.trim(),
    )
    .with_metadata(cbld_bytes, license.timestamp_epoch_s, source_blake3);

    if is_legacy_10d {
        let upgraded = upgrade_legacy_10d_to_holographic_v2(
            source_bytes,
            asset_uri,
            domain_id as u64,
            coordinates_10d,
        )?;
        let mesh = decode_10d_mesh(&upgraded).unwrap_or_else(|_| {
            crate::render::portal_load_10d::synthesize_holographic_proxy_box_mesh(
                [-0.5, 0.0, -0.5],
                [0.5, 1.0, 0.5],
            )
        });
        let compiled_crc = compiled_digest(&upgraded);
        let archetype_blake3: [u8; 32] = *blake3::hash(&upgraded).as_bytes();
        let (mut quins, mut lexicon) =
            crate::render::assets::mesh_to_nquins_with_digests(&mesh, asset_uri, "10d", source_crc, compiled_crc);
        quins.extend(license_quins);
        lexicon.insert(creator_hash, license.creator_name_or_did.clone());
        lexicon.insert(spdx_hash, license.spdx_license.trim().to_string());
        lexicon.insert(tier_hash, license_tier.iri().to_string());
        return Ok(Converted3dScenePackage {
            archetype_10d: upgraded,
            archetype_compiled_digest: compiled_crc,
            archetype_blake3_digest: archetype_blake3,
            source_digest: source_crc,
            license_tier,
            instance_10d_containers: Vec::new(),
            texture_dependencies: Vec::new(),
            quins,
            lexicon,
        });
    }

    // Standard 3D format (GLB, OBJ, STL)
    let fmt_str = format_hint.unwrap_or(if source_bytes.starts_with(b"glTF") {
        "glb"
    } else {
        "obj"
    });
    let compiled_organ = compile_organ_asset(
        source_bytes,
        format_hint,
        asset_uri,
        fmt_str,
        None,
        None,
        Some(&prv_sidecar),
    )?;
    let upgraded_10d = upgrade_legacy_10d_to_holographic_v2(
        &compiled_organ.container_10d,
        asset_uri,
        domain_id as u64,
        coordinates_10d,
    )?;
    let compiled_crc = compiled_digest(&upgraded_10d);
    let archetype_blake3: [u8; 32] = *blake3::hash(&upgraded_10d).as_bytes();

    // If this is a multi-node GLB scene, also generate Mode-B Instance .10d containers for scene nodes!
    let mut instance_10d_containers = Vec::new();
    if source_bytes.starts_with(b"glTF") {
        if let Ok(living_scene) = crate::render::assets::parse_glb_living_scene(source_bytes) {
            if living_scene.nodes.len() > 1 {
                let mut arch_id_bytes = [0u8; 16];
                arch_id_bytes[0..8].copy_from_slice(&subj_hash.to_le_bytes());
                arch_id_bytes[8..16].copy_from_slice(&(compiled_crc as u64).to_le_bytes());

                for (idx, node) in living_scene.nodes.iter().enumerate() {
                    let node_name = node
                        .name
                        .clone()
                        .unwrap_or_else(|| format!("node-{idx:04}"));
                    let node_uri = format!("{asset_uri}#node-{node_name}");
                    let node_hash = q_hash(&node_uri);
                    let mut node_id_bytes = [0u8; 16];
                    node_id_bytes[0..8].copy_from_slice(&node_hash.to_le_bytes());
                    node_id_bytes[8..16].copy_from_slice(&(idx as u64 + 1).to_le_bytes());

                    let (_, t) = node.local_motor.to_rotation_translation();
                    let mut node_coords = coordinates_10d;
                    node_coords[3] += t[0] as f32;
                    node_coords[4] += t[1] as f32;
                    node_coords[5] += t[2] as f32;

                    let node_identity = ManifoldIdentityV2 {
                        entity_id: StableEntityId(node_id_bytes),
                        profile_id: 2,
                        domain_id: (domain_id as u64).max(1),
                        coordinate_convention_id: 1,
                        coordinates: node_coords,
                        external_digest: None,
                    };
                    let aref = ArchetypeRefRecord::new(
                        ArchetypeDigestAlg::Blake3_256,
                        ArchetypeTargetKind::Archetype10D,
                        OVERRIDE_FLAG_COORDS_10D,
                        archetype_blake3,
                        arch_id_bytes,
                        subj_hash,
                        idx as u64,
                    );
                    if let Ok(inst_bytes) = compile_computational_instance_to_10d(
                        &node_identity,
                        Some(&aref),
                        &[],
                        subj_hash,
                        Some(&cbld_payload),
                        &[],
                    ) {
                        instance_10d_containers.push((node_name, inst_bytes));
                    }
                }
            }
        }
    }

    let mut quins = compiled_organ.quins;
    let mut lexicon = compiled_organ.lexicon;
    quins.extend(license_quins);
    lexicon.insert(creator_hash, license.creator_name_or_did.clone());
    lexicon.insert(spdx_hash, license.spdx_license.trim().to_string());
    lexicon.insert(tier_hash, license_tier.iri().to_string());

    Ok(Converted3dScenePackage {
        archetype_10d: upgraded_10d,
        archetype_compiled_digest: compiled_crc,
        archetype_blake3_digest: archetype_blake3,
        source_digest: source_crc,
        license_tier,
        instance_10d_containers,
        texture_dependencies: compiled_organ.texture_dependencies,
        quins,
        lexicon,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::container_10d::integrity::verify_whole_file_crc32c;

    /// Unit cube, 8 vertices / 12 triangles — a valid `render::Mesh`.
    fn cube() -> Mesh {
        Mesh {
            positions: vec![
                [0.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [1.0, 1.0, 0.0],
                [0.0, 1.0, 0.0],
                [0.0, 0.0, 1.0],
                [1.0, 0.0, 1.0],
                [1.0, 1.0, 1.0],
                [0.0, 1.0, 1.0],
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
            min: [0.0, 0.0, 0.0],
            max: [1.0, 1.0, 1.0],
        }
    }

    #[test]
    fn container_is_well_formed_and_sealed() {
        let mut bytes = compile_mesh_to_10d(&cube()).unwrap();
        assert!(
            bytes.len() > 64,
            "container must be larger than the 64-byte header"
        );
        // The seal verifies (whole-file CRC matches the header) — proves it's well-formed.
        verify_whole_file_crc32c(&mut bytes).expect(".10d whole-file CRC must verify");
    }

    #[test]
    fn optional_normal_field_round_trips_with_mesh_without_changing_mesh_payload() {
        let mesh = cube();
        let normals = vec![[0.0, 0.0, 1.0]; mesh.vertex_count()];
        let bytes = compile_mesh_to_10d_with_normals(&mesh, &normals).unwrap();
        assert_eq!(
            decode_10d_mesh(&bytes).unwrap().triangle_count(),
            mesh.triangle_count()
        );
        let decoded = decode_10d_normals(&bytes, mesh.vertex_count())
            .unwrap()
            .unwrap();
        assert_eq!(decoded.len(), normals.len());
        assert!(decoded.iter().all(|normal| normal[2] > 0.9999));
        assert_eq!(
            decode_10d_normals(&compile_mesh_to_10d(&mesh).unwrap(), mesh.vertex_count()).unwrap(),
            None
        );
    }

    #[test]
    fn optional_normal_field_requires_vertex_alignment() {
        let mesh = cube();
        assert!(matches!(
            compile_mesh_to_10d_with_normals(&mesh, &[[0.0, 0.0, 1.0]]),
            Err(Compile10dError::ExtraSection {
                kind: "normal field length"
            })
        ));
    }

    #[test]
    fn optional_tangent_frames_round_trip_through_the_field_sidecar() {
        let mesh = cube();
        let normals = vec![[0.0, 0.0, 1.0]; mesh.vertex_count()];
        let tangents = vec![[1.0, 0.0, 0.0, -1.0]; mesh.vertex_count()];
        let bytes =
            compile_mesh_to_10d_with_frames(&mesh, Some(&normals), Some(&tangents)).unwrap();
        assert_eq!(
            decode_10d_normals(&bytes, mesh.vertex_count())
                .unwrap()
                .unwrap(),
            normals
        );
        assert_eq!(
            decode_10d_tangents(&bytes, mesh.vertex_count())
                .unwrap()
                .unwrap(),
            tangents
        );

        let tangent_only = compile_mesh_to_10d_with_frames(&mesh, None, Some(&tangents)).unwrap();
        assert_eq!(
            decode_10d_normals(&tangent_only, mesh.vertex_count()).unwrap(),
            None
        );
        assert_eq!(
            decode_10d_tangents(&tangent_only, mesh.vertex_count())
                .unwrap()
                .unwrap(),
            tangents
        );
    }

    #[test]
    fn bundles_provenance_physically_and_still_decodes_the_mesh() {
        use crate::container_10d::provenance_section::{
            decode_provenance_section, validate_provenance,
        };

        let source = b"<the original source GLB bytes for this organ>".to_vec();
        let sidecar = ProvenanceSidecar::new(source.clone(), "model/gltf-binary", "CC-BY-4.0");
        let mut bytes = compile_mesh_to_10d_with_provenance(&cube(), Some(&sidecar)).unwrap();

        // The whole-file seal still verifies with the extra section.
        verify_whole_file_crc32c(&mut bytes).expect(".10d whole-file CRC must verify");
        // The mesh is still decodable (the provenance section does not disturb it).
        let mesh = decode_10d_mesh(&bytes).unwrap();
        assert_eq!(mesh.triangle_count(), cube().triangle_count());

        // The provenance sidecar is physically present in the container and passes its gate.
        let header = Container10dHeader::parse(&bytes).unwrap();
        let descs = parse_section_table(&bytes, &header).unwrap();
        let prov = descs
            .iter()
            .find(|d| d.typ() == Some(SectionType::ProvenanceSidecar))
            .expect("provenance section bundled in the .10d");
        let payload = &bytes[prov.byte_offset as usize..][..prov.byte_length as usize];
        let view = decode_provenance_section(payload).unwrap();
        validate_provenance(&view).expect("bundled provenance validates before use");
        assert_eq!(view.licence(), "CC-BY-4.0");
        assert_eq!(view.source_bytes(), source.as_slice());
    }

    #[test]
    fn round_trips_the_mesh_within_quantization_tolerance() {
        let m = cube();
        let bytes = compile_mesh_to_10d(&m).unwrap();
        let back = decode_10d_mesh(&bytes).unwrap();
        assert_eq!(back.vertex_count(), m.vertex_count());
        assert_eq!(back.triangle_count(), m.triangle_count());
        assert_eq!(back.triangles, m.triangles, "indices are exact");
        // Positions survive within the u16-in-bbox quantization bound (extent/65535).
        let tol = 1.0 / 65535.0 * 1.001;
        for (a, b) in m.positions.iter().zip(back.positions.iter()) {
            for k in 0..3 {
                assert!((a[k] - b[k]).abs() <= tol, "vertex {a:?} vs {b:?} axis {k}");
            }
        }
    }

    #[test]
    fn compilation_is_deterministic() {
        let m = cube();
        let a = compile_mesh_to_10d(&m).unwrap();
        let b = compile_mesh_to_10d(&m).unwrap();
        assert_eq!(a, b, "identical mesh → byte-identical .10d");
        assert_eq!(compiled_digest(&a), compiled_digest(&b));
    }

    #[test]
    fn digest_changes_when_geometry_changes() {
        let a = compile_mesh_to_10d(&cube()).unwrap();
        let mut m2 = cube();
        m2.positions[6] = [1.0, 1.0, 0.5]; // move one vertex
        let b = compile_mesh_to_10d(&m2).unwrap();
        assert_ne!(compiled_digest(&a), compiled_digest(&b));
    }

    #[test]
    fn decode_rejects_garbage() {
        assert_eq!(decode_10d_mesh(&[0u8; 8]), Err(Compile10dError::BadHeader));
    }

    #[test]
    fn mesh_with_nodes_round_trips_sigma() {
        use crate::tensor::Tensor10D;
        let mesh = cube();
        let nodes = [
            Tensor10D::ground_truth(0.0, 0.0, 0.5, 0.5, 0.0, 1.0, 0.9, 0.0, 0.42),
            Tensor10D::parallel_context(1.0, 0.0, 0.0, 0.1, 0.2, 0.0, 2.0, 0.5, 0.0, 0.7),
        ];
        let mut bytes = compile_mesh_to_10d_with_nodes(&mesh, &nodes).unwrap();
        verify_whole_file_crc32c(&mut bytes).expect("seal");
        let back = decode_10d_mesh(&bytes).unwrap();
        assert_eq!(back.triangle_count(), mesh.triangle_count());
        let mut out = [Tensor10D::default(); 4];
        let n = decode_10d_nodes(&bytes, &mut out).unwrap();
        assert_eq!(n, 2);
        assert!((out[0].sigma - 0.42).abs() < 1e-5);
        assert!((out[0].x - 0.5).abs() < 1e-5);
        assert!((out[1].q - 1.0).abs() < 1e-5);
        assert!((out[1].sigma - 0.7).abs() < 1e-5);
        // Mesh-only compile must not invent a nodes section.
        let plain = compile_mesh_to_10d(&mesh).unwrap();
        assert!(matches!(
            decode_10d_nodes(&plain, &mut out),
            Err(Compile10dError::NoNodesSection)
        ));
    }

    #[test]
    fn vision_seal_includes_topology_and_spatial_when_cg() {
        use crate::tensor::Tensor10D;
        let mesh = cube();
        let nodes = [Tensor10D::ground_truth(
            0.0, 0.0, 0.5, 0.5, 0.0, 0.0, 1.0, 0.0, 0.33,
        )];
        let mut bytes = compile_mesh_to_10d_vision(&mesh, &nodes).unwrap();
        verify_whole_file_crc32c(&mut bytes).expect("seal");
        let header = Container10dHeader::parse(&bytes).unwrap();
        let descs = parse_section_table(&bytes, &header).unwrap();
        let types: Vec<_> = descs.iter().filter_map(|d| d.typ()).collect();
        assert!(types.contains(&SectionType::QuantizedMesh));
        assert!(types.contains(&SectionType::Tensor10DNodes));
        #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
        {
            assert!(
                types.contains(&SectionType::Topology),
                "expected Topology section, got {types:?}"
            );
            assert!(
                types.contains(&SectionType::SpatialIndex),
                "expected SpatialIndex section, got {types:?}"
            );
        }
        let back = decode_10d_mesh(&bytes).unwrap();
        assert_eq!(back.triangle_count(), mesh.triangle_count());
    }

    /// A single OBJ triangle — the smallest valid source asset.
    const TRI_OBJ: &[u8] = b"v 0 0 0\nv 1 0 0\nv 0 1 0\nf 1 2 3\n";

    #[test]
    fn compile_asset_binds_manifest_to_its_container() {
        let a = compile_asset(TRI_OBJ, Some("obj"), "urn:asset:tri", "obj").unwrap();

        // The container round-trips back to the mesh the manifest describes.
        let back = decode_10d_mesh(&a.container_10d).unwrap();
        assert_eq!(back.triangle_count(), 1);
        assert_eq!(back.vertex_count(), 3);

        // compiled_digest field == the container's real whole-file CRC (the citation is honest).
        assert_eq!(a.compiled_digest, compiled_digest(&a.container_10d));
        // source_digest == CRC of the source bytes.
        assert_eq!(a.source_digest, crc32c(TRI_OBJ));

        // Both digests appear as manifest facts (object-side), binding the two layers.
        let objs: Vec<u64> = a.quins.iter().map(|q| q.object).collect();
        assert!(
            objs.contains(&(a.compiled_digest as u64)),
            "manifest cites compiledDigest"
        );
        assert!(
            objs.contains(&(a.source_digest as u64)),
            "manifest cites sourceDigest"
        );
    }

    #[test]
    fn compile_asset_is_deterministic() {
        let a = compile_asset(TRI_OBJ, Some("obj"), "urn:asset:tri", "obj").unwrap();
        let b = compile_asset(TRI_OBJ, Some("obj"), "urn:asset:tri", "obj").unwrap();
        assert_eq!(a.container_10d, b.container_10d, "byte-identical container");
        assert_eq!(a.compiled_digest, b.compiled_digest);
        assert_eq!(a.source_digest, b.source_digest);
    }

    #[test]
    fn compile_asset_surfaces_import_errors() {
        // Not a recognisable asset in any supported format.
        let err = compile_asset(b"\x00\x01\x02\x03", None, "urn:asset:junk", "obj");
        assert!(matches!(err, Err(Compile10dError::Import(_))));
    }

    #[test]
    fn compile_organ_asset_binds_body_system_and_model() {
        let plain = compile_asset(TRI_OBJ, Some("obj"), "urn:asset:organ", "obj").unwrap();
        let organ = compile_organ_asset(
            TRI_OBJ,
            Some("obj"),
            "urn:asset:organ",
            "obj",
            Some("respiratory"),
            Some("male"),
            None,
        )
        .unwrap();
        // Same geometry → identical container + digests; only the manifest gains the anatomy facts.
        assert_eq!(organ.container_10d, plain.container_10d);
        assert_eq!(organ.compiled_digest, plain.compiled_digest);
        assert_eq!(
            organ.quins.len(),
            plain.quins.len() + 2,
            "two anatomy facts added"
        );
        // bodySystem + anatomyModel strings are carried in the lexicon.
        let vals: Vec<&str> = organ.lexicon.values().map(String::as_str).collect();
        assert!(vals.contains(&"respiratory"), "bodySystem fact present");
        assert!(vals.contains(&"male"), "anatomyModel fact present");
        // None/None path is exactly compile_asset — no phantom facts.
        let none = compile_organ_asset(
            TRI_OBJ,
            Some("obj"),
            "urn:asset:organ",
            "obj",
            None,
            None,
            None,
        )
        .unwrap();
        assert_eq!(none.quins.len(), plain.quins.len());
    }

    #[test]
    fn compile_developmental_asset_binds_the_t_axis_coordinate() {
        let plain = compile_asset(TRI_OBJ, Some("obj"), "urn:asset:fetal", "obj").unwrap();
        // Carnegie stage 18 ≈ 44 postfertilization days.
        let dev =
            compile_developmental_asset(TRI_OBJ, Some("obj"), "urn:asset:fetal", "obj", 44, 18)
                .unwrap();
        // Same geometry → identical container; only the manifest gains the two developmental facts.
        assert_eq!(dev.container_10d, plain.container_10d);
        assert_eq!(
            dev.quins.len(),
            plain.quins.len() + 2,
            "gestationalAgeDays + carnegieStage"
        );
        // The t-axis coordinate (44 days) and the stage (18) are present as u64 fact objects (a 3-vertex,
        // 1-triangle mesh has no other facts with those values, so this is unambiguous).
        assert!(
            dev.quins.iter().any(|q| q.object == 44),
            "gestationalAgeDays=44"
        );
        assert!(dev.quins.iter().any(|q| q.object == 18), "carnegieStage=18");
    }

    #[test]
    fn compile_computational_instance_to_10d_seals_mid2_aref_mstr_and_cbor_ld_without_mesh() {
        use crate::container_10d::{
            decode_archetype_ref_record, decode_container_manifold_identity,
            decode_multiscale_strata_section_into, decode_symbolic_rdf_cbor_ld, ArchetypeDigestAlg,
            ArchetypeRefRecord, ArchetypeTargetKind, ComputationalStratumLevel,
            EmbodimentResolutionState, ManifoldIdentityV2, MultilingualLexicalTerm,
            NestedStratumNodeRecord, StableEntityId, StratumConstituentKind,
            StratumInteractionRegime, SymbolicRdfCborLdPayload, TypedFieldRecord,
            FIELD_KIND_ARCHETYPE_REF, FIELD_KIND_MULTISCALE_STRATA,
            FIELD_KIND_SYMBOLIC_RDF_CBOR_LD, OVERRIDE_FLAG_COORDS_10D,
            OVERRIDE_FLAG_MULTISCALE_STRATA, STRATUM_NO_PARENT,
        };
        use crate::topology::header::{AnchorDomain, CellAnchor, TopologyInvariants};

        let identity = ManifoldIdentityV2 {
            entity_id: StableEntityId(*b"tree-inst-000042"),
            profile_id: 2,
            domain_id: 1,
            coordinate_convention_id: 1,
            coordinates: [1.0, 3.0, 2.0, -23.698, 133.880, 582.4, 100.0, 0.9, 0.99, 0.55],
            external_digest: None,
        };
        let aref = ArchetypeRefRecord::new(
            ArchetypeDigestAlg::Blake3_256,
            ArchetypeTargetKind::Archetype10D,
            OVERRIDE_FLAG_COORDS_10D | OVERRIDE_FLAG_MULTISCALE_STRATA,
            [0xAB; 32],
            *b"corymbia-arch-01",
            0x1234_5678_9ABC_DEF0,
            0,
        );
        let stratum = NestedStratumNodeRecord {
            host_anchor: CellAnchor {
                entity_hash: 0x1111_2222,
                cell_index: 0,
                anchor_domain: AnchorDomain::SemanticQuin as u8,
                orientation_sign: 0,
                material_or_stratum: 6,
            },
            parent_node_index: STRATUM_NO_PARENT,
            stratum_level: ComputationalStratumLevel::L6QuantumChemistry as u8,
            constituent_kind: StratumConstituentKind::DftExchangeCorrelationGrid as u8,
            interaction_regime: StratumInteractionRegime::ElectronicExcitation as u8,
            embodiment_state: EmbodimentResolutionState::ComputationalOnly as u8,
            quantum_basis_and_xc: (4 << 4) | 2,
            characteristic_scale_log10_m_q12: -120,
            taxon_or_ontology_iri_hash: 0x3333_4444,
            target_archetype_or_qpu_nquin_hash: 0x5555_6666,
            density_or_abundance_per_m3: 1.0e20,
            metabolic_flux_or_hartree_energy: -2934.1284,
            coupled_metric_a_f16: 0,
            coupled_metric_b_f16: 0,
            logic_invariants: TopologyInvariants::REIFIED_EDGES,
            belnap_and_spin_u16: 1,
            orbital_or_coupling_flags: 136,
            record_crc32c: 0,
        };
        let cbld = SymbolicRdfCborLdPayload {
            context_iri: "https://qualiadb.org/contexts/10d-symbolic-v2.cbor-ld".to_string(),
            lexicon: vec![MultilingualLexicalTerm {
                term_hash: 0x1234_5678_9ABC_DEF0,
                iri_utf8: "https://intenteco.org/taxa/Corymbia_aparrerinja".to_string(),
                lang_bcp47: Some("rer".to_string()),
                label_utf8: "Ilwempe".to_string(),
            }],
            nquins: Vec::new(),
            n3_logic_rules_utf8: Vec::new(),
            shacl_shape_iris_utf8: Vec::new(),
        };

        let mut bytes = compile_computational_instance_to_10d(
            &identity,
            Some(&aref),
            &[stratum],
            0x9999,
            Some(&cbld),
            &[],
        )
        .unwrap();
        verify_whole_file_crc32c(&mut bytes).expect("valid whole-file CRC-32C");

        let header = Container10dHeader::parse(&bytes).unwrap();
        let mut scratch = [TypedFieldRecord::default(); 8];
        let mut fields = [TypedFieldRecord::default(); 8];
        let (decoded_id, fcount) =
            decode_container_manifold_identity(&bytes, &header, &[], &mut scratch, &mut fields)
                .unwrap();
        assert_eq!(decoded_id, identity);
        assert_eq!(fcount, 3);
        assert_eq!(fields[0].kind, FIELD_KIND_ARCHETYPE_REF);
        assert_eq!(fields[1].kind, FIELD_KIND_MULTISCALE_STRATA);
        assert_eq!(fields[2].kind, FIELD_KIND_SYMBOLIC_RDF_CBOR_LD);

        let descs = parse_section_table(&bytes, &header).unwrap();
        let sidecar_desc = descs
            .iter()
            .find(|d| d.typ() == Some(SectionType::FieldSidecar))
            .unwrap();
        let sidecar =
            &bytes[sidecar_desc.byte_offset as usize..][..sidecar_desc.byte_length as usize];

        let decoded_aref = decode_archetype_ref_record(
            &sidecar[fields[0].payload_offset as usize..][..fields[0].payload_length as usize],
        )
        .unwrap();
        assert_eq!(decoded_aref, aref);

        let mut decoded_strata = [bytemuck::Zeroable::zeroed(); 2];
        let (_, scount) = decode_multiscale_strata_section_into(
            &sidecar[fields[1].payload_offset as usize..][..fields[1].payload_length as usize],
            &mut decoded_strata,
        )
        .unwrap();
        assert_eq!(scount, 1);
        assert_eq!(decoded_strata[0].stratum_level, 6);

        let decoded_cbld = decode_symbolic_rdf_cbor_ld(
            &sidecar[fields[2].payload_offset as usize..][..fields[2].payload_length as usize],
        )
        .unwrap();
        assert_eq!(decoded_cbld, cbld);
    }

    #[test]
    fn upgrade_legacy_10d_to_holographic_v2_preserves_mesh_and_embeds_holo_proc_mid2() {
        use crate::container_10d::{
            decode_container_manifold_identity, decode_holographic_phase_section_into,
            decode_procedural_synthesis_section, EulerianWavefieldRecord, TypedFieldRecord,
            FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE,
        };
        use crate::q42::m64_matter::M64HoloRecord;

        let legacy_bytes = compile_mesh_to_10d(&cube()).unwrap();
        let mut upgraded = upgrade_legacy_10d_to_holographic_v2(
            &legacy_bytes,
            "https://intenteco.org/taxa/Corymbia_aparrerinja",
            1,
            [0.0, 3.0, 2.0, -23.698, 133.880, 582.4, 0.0, 0.9, 0.99, 0.55],
        )
        .unwrap();
        verify_whole_file_crc32c(&mut upgraded).expect("valid whole-file CRC-32C after upgrade");

        // Existing B-Rep QuantizedMesh remains intact
        let back_mesh = decode_10d_mesh(&upgraded).unwrap();
        assert_eq!(back_mesh.vertex_count(), 8);
        assert_eq!(back_mesh.triangle_count(), 12);

        // ManifoldIdentityV2 and HOLO (0x0306) are present and valid
        let header = Container10dHeader::parse(&upgraded).unwrap();
        let mut scratch = [TypedFieldRecord::default(); 8];
        let mut fields = [TypedFieldRecord::default(); 8];
        let (_id, fcount) = decode_container_manifold_identity(
            &upgraded,
            &header,
            &[],
            &mut scratch,
            &mut fields,
        )
        .unwrap();
        assert!(fcount >= 1);
        let holo_field = fields[..fcount]
            .iter()
            .find(|f| f.kind == FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE)
            .expect("HOLO field registered in MID2");

        let descs = parse_section_table(&upgraded, &header).unwrap();
        let sidecar_desc = descs
            .iter()
            .find(|d| d.typ() == Some(SectionType::FieldSidecar))
            .unwrap();
        let sidecar =
            &upgraded[sidecar_desc.byte_offset as usize..][..sidecar_desc.byte_length as usize];
        let holo_slice = &sidecar[holo_field.payload_offset as usize..]
            [..holo_field.payload_length as usize];

        let mut holo_recs = [bytemuck::Zeroable::zeroed(); 32];
        let mut wave_recs: [EulerianWavefieldRecord; 1] = [bytemuck::Zeroable::zeroed(); 1];
        let (holo_hdr, h_count, w_count) =
            decode_holographic_phase_section_into(holo_slice, &mut holo_recs, &mut wave_recs)
                .unwrap();
        assert!(h_count >= 1);
        assert_eq!(w_count, 0);
        assert!(holo_hdr.max_raymarch_steps <= 32);
        let rec: M64HoloRecord = holo_recs[0];
        assert_eq!(rec.metric_variety, 2);

        // ProceduralSynthesis (PROC = 18) is also present
        let proc_desc = descs
            .iter()
            .find(|d| d.typ() == Some(SectionType::ProceduralSynthesis))
            .expect("PROC section attached");
        let proc_slice =
            &upgraded[proc_desc.byte_offset as usize..][..proc_desc.byte_length as usize];
        let proc_hdr = decode_procedural_synthesis_section(proc_slice).unwrap();
        assert!(proc_hdr.voronoi_frequency > 0.0);

        // Calling upgrade a second time is idempotent
        let idempotent = upgrade_legacy_10d_to_holographic_v2(
            &upgraded,
            "https://intenteco.org/taxa/Corymbia_aparrerinja",
            1,
            [0.0, 3.0, 2.0, -23.698, 133.880, 582.4, 0.0, 0.9, 0.99, 0.55],
        )
        .unwrap();
        assert_eq!(upgraded, idempotent);
    }

    #[test]
    fn upgrade_real_existing_ecointent_10d_asset_to_holographic_v2() {
        use crate::container_10d::{
            decode_container_manifold_identity, decode_holographic_phase_section_into,
            EulerianWavefieldRecord, TypedFieldRecord, FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE,
        };
        use crate::q42::m64_matter::M64HoloRecord;
        use std::path::Path;

        let path = Path::new(r"C:\github\eco-intent\acacia-pycnantha\dist\lods\acacia-pycnantha-lod2.10d");
        if !path.exists() {
            return;
        }
        let raw_bytes = std::fs::read(path).expect("read existing .10d asset");
        let orig_mesh = decode_10d_mesh(&raw_bytes).expect("decode existing mesh");

        let mut upgraded = upgrade_legacy_10d_to_holographic_v2(
            &raw_bytes,
            "https://intenteco.org/taxa/Acacia_pycnantha",
            1,
            [0.0, 3.0, 2.0, -35.2809, 149.1300, 580.0, 0.0, 0.92, 0.99, 0.61],
        )
        .expect("upgrade real eco-intent .10d asset to holographic v2");

        verify_whole_file_crc32c(&mut upgraded).expect("upgraded real .10d CRC-32C verifies");
        let up_mesh = decode_10d_mesh(&upgraded).expect("decode upgraded mesh");
        assert_eq!(up_mesh.vertex_count(), orig_mesh.vertex_count());
        assert_eq!(up_mesh.triangle_count(), orig_mesh.triangle_count());

        let header = Container10dHeader::parse(&upgraded).unwrap();
        let mut scratch = [TypedFieldRecord::default(); 16];
        let mut fields = [TypedFieldRecord::default(); 16];
        let (_id, fcount) = decode_container_manifold_identity(
            &upgraded,
            &header,
            &[],
            &mut scratch,
            &mut fields,
        )
        .unwrap();
        let holo_field = fields[..fcount]
            .iter()
            .find(|f| f.kind == FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE)
            .expect("HOLO 0x0306 embedded in upgraded real asset");

        let descs = parse_section_table(&upgraded, &header).unwrap();
        let sidecar_desc = descs
            .iter()
            .find(|d| d.typ() == Some(SectionType::FieldSidecar))
            .unwrap();
        let sidecar =
            &upgraded[sidecar_desc.byte_offset as usize..][..sidecar_desc.byte_length as usize];
        let holo_slice = &sidecar[holo_field.payload_offset as usize..]
            [..holo_field.payload_length as usize];

        let mut holo_recs: [M64HoloRecord; 32] = [bytemuck::Zeroable::zeroed(); 32];
        let mut wave_recs: [EulerianWavefieldRecord; 1] = [bytemuck::Zeroable::zeroed(); 1];
        let (holo_hdr, h_count, _) =
            decode_holographic_phase_section_into(holo_slice, &mut holo_recs, &mut wave_recs)
                .unwrap();
        assert!(h_count >= 1);
        assert!(holo_hdr.max_raymarch_steps <= 32);
        assert_eq!(holo_recs[0].metric_variety, 2);
        assert!(holo_recs[0].scale_bounds[0] > 0);
    }

    #[test]
    fn convert_3d_asset_or_scene_with_license_enforces_provenance_and_converts_obj_and_10d() {
        use crate::container_10d::{
            decode_container_manifold_identity, TypedFieldRecord,
            FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE,
        };

        // Missing license fails closed
        let empty_license = SourceLicenseAttribution {
            spdx_license: String::new(),
            creator_name_or_did: "Creator".into(),
            attribution_notice_utf8: "Copyright Creator".into(),
            origin_uri: None,
            timestamp_epoch_s: 1700000000,
            embed_full_source_bytes: true,
        };
        assert!(matches!(
            convert_3d_asset_or_scene_with_license(
                TRI_OBJ,
                Some("obj"),
                "urn:test:converted:tri",
                1,
                [0.0; 10],
                &empty_license
            ),
            Err(Compile10dError::Provenance(
                ProvenanceSectionError::MissingLicence
            ))
        ));

        // Valid CC-BY-NC-ND-4.0 conversion succeeds and seals PRV1 + HOLO + MID2
        let moral_rights_license = SourceLicenseAttribution {
            spdx_license: "CC-BY-NC-ND-4.0".into(),
            creator_name_or_did: "Timothy Charles Holborn".into(),
            attribution_notice_utf8:
                "Copyright (c) 2026 Timothy Charles Holborn — CC BY-NC-ND 4.0".into(),
            origin_uri: Some("https://github.com/mediaprophet/qualiaDB".into()),
            timestamp_epoch_s: 1700000000,
            embed_full_source_bytes: true,
        };
        let mut pkg = convert_3d_asset_or_scene_with_license(
            TRI_OBJ,
            Some("obj"),
            "urn:test:converted:tri",
            1,
            [0.0, 3.0, 2.0, 1.0, 2.0, 3.0, 0.0, 1.0, 1.0, 0.5],
            &moral_rights_license,
        )
        .expect("convert OBJ with CC-BY-NC-ND-4.0 license");

        assert_eq!(pkg.license_tier, SourceLicenseTier::AuthorMoralRightsQualiaCcByNcNd);
        assert!(!pkg.license_tier.permits_uncompensated_commercial_use());
        verify_whole_file_crc32c(&mut pkg.archetype_10d).expect("valid converted .10d CRC-32C");

        let header = Container10dHeader::parse(&pkg.archetype_10d).unwrap();
        let descs = parse_section_table(&pkg.archetype_10d, &header).unwrap();
        assert!(
            descs.iter().any(|d| d.typ() == Some(SectionType::ProvenanceSidecar)),
            "PRV1 ProvenanceSidecar must be sealed in converted .10d"
        );
        let mut scratch = [TypedFieldRecord::default(); 8];
        let mut fields = [TypedFieldRecord::default(); 8];
        let (_, fcount) = decode_container_manifold_identity(
            &pkg.archetype_10d,
            &header,
            &[],
            &mut scratch,
            &mut fields,
        )
        .unwrap();
        assert!(fields[..fcount]
            .iter()
            .any(|f| f.kind == FIELD_KIND_HOLOGRAPHIC_PHASE_SPACE));
    }
}
