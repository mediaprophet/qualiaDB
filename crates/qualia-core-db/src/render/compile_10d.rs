//! Compile an imported triangle [`Mesh`] into a sealed `.10d` container — the
//! **dense compiled-geometry** half of a geometry asset (see
//! `docs/manuals/standards/geometry-asset-ontology.md` §3). This is the
//! "mesh → `.10d`" step of the 3-D-anatomy asset pipeline: the renderer and the
//! anatomy layer read the `.10d` back with [`decode_10d_mesh`] instead of
//! reparsing the source GLB, and the q42 semantic manifest cites the container's
//! [`compiled_digest`].
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
use crate::container_10d::provenance_section::{
    encode_provenance_section, encoded_len as provenance_encoded_len, ProvenanceSectionError,
    ProvenanceSidecar,
};
use crate::container_10d::section::{
    encode_container, parse_section_table, AlignmentTier, SectionInput, SectionTableError,
    SectionType,
};
use crate::render::assets::{
    import_asset, mesh_to_nquins_with_dev, mesh_to_nquins_with_meta, AssetError, Mesh,
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
    /// The container section table encode/decode failed.
    Section(SectionTableError),
    /// Encoding the provenance sidecar section failed.
    Provenance(ProvenanceSectionError),
    /// Encoding or reading the Tensor10DNodes section failed.
    Nodes(NodeSectionError),
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
            Self::Section(e) => write!(f, ".10d compile: section table: {e:?}"),
            Self::Provenance(e) => write!(f, ".10d compile: provenance section: {e}"),
            Self::Nodes(e) => write!(f, ".10d compile: Tensor10DNodes section: {e}"),
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
    compile_mesh_to_10d_with_extras(mesh, nodes, provenance, Compile10dExtras::default())
}

/// Vision recon seal: mesh + nodes + Topology + SpatialIndex when CG is linked (C3).
///
/// On slim WASM portal builds without `wasm-scientific`, extras are silently
/// skipped (mesh+nodes still seal) so product paths stay portable.
pub fn compile_mesh_to_10d_vision(
    mesh: &Mesh,
    nodes: &[Tensor10D],
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_extras(mesh, nodes, None, Compile10dExtras::VISION)
}

/// Vision recon seal + in-envelope provenance (D4).
pub fn compile_mesh_to_10d_vision_with_provenance(
    mesh: &Mesh,
    nodes: &[Tensor10D],
    provenance: &ProvenanceSidecar,
) -> Result<Vec<u8>, Compile10dError> {
    compile_mesh_to_10d_with_extras(mesh, nodes, Some(provenance), Compile10dExtras::VISION)
}

/// Full seal with explicit extras (topology / spatial index).
pub fn compile_mesh_to_10d_with_extras(
    mesh: &Mesh,
    nodes: &[Tensor10D],
    provenance: Option<&ProvenanceSidecar>,
    extras: Compile10dExtras,
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

/// Read a `.10d` container back into a dequantized [`Mesh`] — the renderer /
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

/// A fully compiled geometry asset: the source-imported [`Mesh`], the sealed `.10d`
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
    /// Whole-file CRC-32C of `container_10d` — the manifest's `compiledDigest`.
    pub compiled_digest: u32,
    /// CRC-32C of the immutable source bytes — the manifest's `sourceDigest`.
    pub source_digest: u32,
    /// The q42 manifest facts, including both digests (via [`mesh_to_nquins_with_digests`]).
    pub quins: Vec<NQuin>,
    /// Object-lexicon for the string-valued facts in `quins`.
    pub lexicon: HashMap<u64, String>,
}

/// The end-to-end asset-compile step: source asset bytes → [`CompiledAsset`].
///
/// Runs the full pipeline — import the source ([`import_asset`]), compile the dense
/// `.10d` ([`compile_mesh_to_10d`]), hash both layers, then emit the q42 manifest that
/// binds them ([`mesh_to_nquins_with_digests`]). Deterministic: identical
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
    let mesh = import_asset(source_bytes, hint).map_err(Compile10dError::Import)?;
    // When provenance is supplied it is sealed into the `.10d` as a ProvenanceSidecar section, so the
    // asset is attested (the renderer's fail-closed governance gate treats an unattested asset with the
    // default-refuse disposition as REFUSE).
    let container_10d = compile_mesh_to_10d_with_provenance(&mesh, provenance)?;
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
    let mesh = import_asset(source_bytes, hint).map_err(Compile10dError::Import)?;
    let container_10d = compile_mesh_to_10d(&mesh)?;
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
        compiled_digest: compiled,
        source_digest: source,
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
}
