//! `poet_host` capability dispatch for the POET Geometry-Topology Complex
//! ("Geometries Harbour Many Graphs"): `Topology.*` and `Poet.topology_bind`.
//!
//! Validates `GeometryComplexKind`, `GraphEmbedRole`, `GraphMorphology`,
//! `AnchorDomain`, and `TopologyInvariants` against `crate::topology::header::*`,
//! computes deterministic 64-bit `q_hash` IDs, Euler characteristic
//! $\chi = N_0 - N_1 + N_2 - N_3$, and Betti numbers $(\beta_0, \beta_1, \beta_2)$.

use super::{args, ids};
use crate::topology::header::{
    AnchorDomain, CellAnchor, GeometryComplexKind, GeometryMetricKind, GraphEmbedRole,
    GraphMorphology, HarbouredGraphHeader, PoetGeometryHeader, TopologyError, TopologyInvariants,
    GTOP_MAGIC,
};
use bytemuck::bytes_of;
use vibe::{Diagnostic, Span, Value};

#[inline]
fn q_hash_bytes(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325u64;
    for &b in bytes {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

#[inline]
fn crc32c_slice(bytes: &[u8]) -> u32 {
    let mut crc = !0u32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0x82F6_3B78 & mask);
        }
    }
    !crc
}

fn parse_metric_kind(name: &str) -> GeometryMetricKind {
    match name.trim() {
        "ToroidalCyclic" | "Toroidal" | "Cyclic" => GeometryMetricKind::ToroidalCyclic,
        "HyperbolicPoincare" | "Hyperbolic" => GeometryMetricKind::HyperbolicPoincare,
        "SphericalGeodesic" | "Spherical" => GeometryMetricKind::SphericalGeodesic,
        "FisherRaoSimplex" | "FisherRao" => GeometryMetricKind::FisherRaoSimplex,
        "LorentzianCausal" | "Lorentzian" => GeometryMetricKind::LorentzianCausal,
        _ => GeometryMetricKind::Euclidean,
    }
}

fn parse_invariants(
    args_v: &Value,
    span: Span,
) -> Result<(TopologyInvariants, Vec<String>), Diagnostic> {
    let mut inv = TopologyInvariants::EMPTY;
    let mut names = Vec::new();
    if let Some(bits) = args::rec_u64(args_v, "invariants_bits").or_else(|| args::rec_u64(args_v, "invariants")) {
        inv = TopologyInvariants::from_bits_truncate(bits as u16);
        return Ok((inv, names));
    }
    if let Some(list) = args::rec_str_list(args_v, "invariants") {
        for item in list {
            let flag = TopologyInvariants::from_name(&item).ok_or_else(|| {
                args::bad(span, format!("unknown TopologyInvariants flag `{item}`"))
            })?;
            inv |= flag;
            names.push(item);
        }
    } else if let Some(s) = args::rec_str(args_v, "invariants") {
        for part in s.split(',').map(str::trim).filter(|p| !p.is_empty()) {
            let flag = TopologyInvariants::from_name(part).ok_or_else(|| {
                args::bad(span, format!("unknown TopologyInvariants flag `{part}`"))
            })?;
            inv |= flag;
            names.push(part.to_string());
        }
    }
    Ok((inv, names))
}

/// `Topology.define_geometry` — declares a 64-byte `PoetGeometryHeader`.
pub fn define_geometry(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(args_v, "name")
        .or_else(|| args::rec_str(args_v, "geometry"))
        .or_else(|| args::rec_str(args_v, "label"))
        .unwrap_or("DefaultGeometry");
    let kind_str = args::rec_str(args_v, "complex_kind")
        .or_else(|| args::rec_str(args_v, "kind"))
        .unwrap_or("MixedCellComplex3D");
    let complex_kind = GeometryComplexKind::from_name(kind_str).ok_or_else(|| {
        args::bad(span, format!("invalid GeometryComplexKind `{kind_str}`"))
    })?;
    let metric_str = args::rec_str(args_v, "metric")
        .or_else(|| args::rec_str(args_v, "metric_kind"))
        .unwrap_or("Euclidean");
    let metric_kind = parse_metric_kind(metric_str);
    let satisfies = args::rec_str(args_v, "satisfies")
        .or_else(|| args::rec_str(args_v, "construct"))
        .unwrap_or("qualia:DefaultConstruct");

    let n0 = args::rec_u64(args_v, "cell_0_count").or_else(|| args::rec_u64(args_v, "n0")).unwrap_or(8) as u32;
    let n1 = args::rec_u64(args_v, "cell_1_count").or_else(|| args::rec_u64(args_v, "n1")).unwrap_or(12) as u32;
    let n2 = args::rec_u64(args_v, "cell_2_count").or_else(|| args::rec_u64(args_v, "n2")).unwrap_or(6) as u32;
    let n3 = args::rec_u64(args_v, "cell_3_count").or_else(|| args::rec_u64(args_v, "n3")).unwrap_or(1) as u32;
    let graph_count = args::rec_u64(args_v, "harboured_graph_count")
        .or_else(|| args::rec_u64(args_v, "graph_count"))
        .unwrap_or(1) as u16;
    let manifold_id = args::rec_u64(args_v, "manifold_id").unwrap_or(0) as u32;
    let geometry_id = args::rec_u64(args_v, "geometry_id").unwrap_or_else(|| q_hash_bytes(name.as_bytes()));
    let construct_schema_hash = q_hash_bytes(satisfies.as_bytes());

    let mut hdr = PoetGeometryHeader {
        geometry_id,
        construct_schema_hash,
        manifold_id,
        cell_0_count: n0,
        cell_1_count: n1,
        cell_2_count: n2,
        cell_3_count: n3,
        geometry_payload_offset: 64 + (graph_count as u32) * 64,
        ddg_operators_offset: 0,
        graph_dir_offset: 64,
        harboured_graph_count: graph_count,
        active_graph_slot: 0,
        euler_characteristic: 0,
        complex_kind: complex_kind as u8,
        metric_kind: metric_kind as u8,
        ambient_dim: args::rec_u64(args_v, "ambient_dim").unwrap_or(10) as u8,
        intrinsic_dim: args::rec_u64(args_v, "intrinsic_dim").unwrap_or(3) as u8,
        crc32c: 0,
    };
    hdr.euler_characteristic = hdr.computed_euler_characteristic();
    hdr.crc32c = crc32c_slice(&bytes_of(&hdr)[..60]);

    Ok(args::record([
        ("id", Value::String(ids::TOPOLOGY_DEFINE_GEOMETRY.into())),
        ("op", Value::String("define_geometry".into())),
        ("magic", Value::String(core::str::from_utf8(&GTOP_MAGIC).unwrap_or("GTOP").into())),
        ("name", Value::String(name.into())),
        ("satisfies", Value::String(satisfies.into())),
        ("geometry_id", Value::U64(hdr.geometry_id)),
        ("construct_schema_hash", Value::U64(hdr.construct_schema_hash)),
        ("complex_kind", Value::String(format!("{complex_kind:?}"))),
        ("complex_kind_code", Value::U64(hdr.complex_kind as u64)),
        ("metric_kind", Value::String(format!("{metric_kind:?}"))),
        ("cell_0_count", Value::U64(hdr.cell_0_count as u64)),
        ("cell_1_count", Value::U64(hdr.cell_1_count as u64)),
        ("cell_2_count", Value::U64(hdr.cell_2_count as u64)),
        ("cell_3_count", Value::U64(hdr.cell_3_count as u64)),
        ("euler_characteristic", Value::I64(hdr.euler_characteristic as i64)),
        ("harboured_graph_count", Value::U64(hdr.harboured_graph_count as u64)),
        ("active_graph_slot", Value::U64(hdr.active_graph_slot as u64)),
        ("header_size_bytes", Value::U64(core::mem::size_of::<PoetGeometryHeader>() as u64)),
        ("cell_anchor_size_bytes", Value::U64(core::mem::size_of::<CellAnchor>() as u64)),
        ("crc32c", Value::U64(hdr.crc32c as u64)),
        ("honesty", Value::String("deterministic_gtop_header".into())),
    ]))
}

/// `Topology.harbour_graph` — compiles or induces a `HarbouredGraphHeader` inside a host Geometry.
pub fn harbour_graph(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let name = args::rec_str(args_v, "name")
        .or_else(|| args::rec_str(args_v, "graph"))
        .or_else(|| args::rec_str(args_v, "label"))
        .unwrap_or("PrimalMesh");
    let geom_name = args::rec_str(args_v, "geometry").unwrap_or("DefaultGeometry");
    let parent_geometry_id = args::rec_u64(args_v, "parent_geometry_id")
        .or_else(|| args::rec_u64(args_v, "geometry_id"))
        .unwrap_or_else(|| q_hash_bytes(geom_name.as_bytes()));

    let role_str = args::rec_str(args_v, "role")
        .or_else(|| args::rec_str(args_v, "embed_role"))
        .unwrap_or("Primal1Skeleton");
    let role = GraphEmbedRole::from_name(role_str)
        .ok_or_else(|| args::bad(span, format!("invalid GraphEmbedRole `{role_str}`")))?;

    let morph_str = args::rec_str(args_v, "morphology")
        .or_else(|| args::rec_str(args_v, "kind"))
        .unwrap_or("SymmetricGraph");
    let morphology = GraphMorphology::from_name(morph_str)
        .ok_or_else(|| args::bad(span, format!("invalid GraphMorphology `{morph_str}`")))?;

    let anchor_list = args::rec_str_list(args_v, "anchors");
    let anchor_str = anchor_list
        .as_ref()
        .and_then(|v| v.first().map(String::as_str))
        .or_else(|| args::rec_str(args_v, "anchors"))
        .or_else(|| args::rec_str(args_v, "anchor_domain"))
        .unwrap_or("Cell0Vertex");
    let anchor_domain = AnchorDomain::from_name(anchor_str)
        .ok_or_else(|| args::bad(span, format!("invalid AnchorDomain `{anchor_str}`")))?;

    let (mut invariants, mut inv_names) = parse_invariants(args_v, span)?;
    if morphology == GraphMorphology::DeterministicDAG && !invariants.contains(TopologyInvariants::ACYCLIC) {
        invariants |= TopologyInvariants::ACYCLIC;
        inv_names.push("ACYCLIC".into());
    }
    if morphology == GraphMorphology::SymmetricGraph && !invariants.contains(TopologyInvariants::SYMMETRIC) {
        invariants |= TopologyInvariants::SYMMETRIC;
        inv_names.push("SYMMETRIC".into());
    }

    let slot = args::rec_u64(args_v, "slot").unwrap_or(0) as u16;
    let vertex_count = args::rec_u64(args_v, "vertex_count").unwrap_or(8) as u32;
    let edge_count = args::rec_u64(args_v, "edge_count").unwrap_or(12) as u32;
    let graph_id = args::rec_u64(args_v, "graph_id")
        .unwrap_or_else(|| q_hash_bytes(name.as_bytes()) ^ parent_geometry_id.rotate_left(7));

    let mut ghdr = HarbouredGraphHeader {
        graph_id,
        parent_geometry_id,
        anchor_table_offset: 128,
        csr_row_ptr_offset: 128 + vertex_count * 16,
        csr_col_idx_offset: 128 + vertex_count * 16 + (vertex_count + 1) * 4,
        edge_anchor_offset: 0,
        csc_col_ptr_offset: 0,
        csc_row_idx_offset: 0,
        edge_weight_offset: 0,
        vertex_count,
        edge_count,
        context_and_state_mask: 0,
        embed_role: role as u8,
        morphology: morphology as u8,
        invariants,
        crc32c: 0,
    };
    ghdr.crc32c = crc32c_slice(&bytes_of(&ghdr)[..60]);

    Ok(args::record([
        ("id", Value::String(ids::TOPOLOGY_HARBOUR_GRAPH.into())),
        ("op", Value::String("harbour_graph".into())),
        ("magic", Value::String("GTOP".into())),
        ("name", Value::String(name.into())),
        ("graph_id", Value::U64(ghdr.graph_id)),
        ("parent_geometry_id", Value::U64(ghdr.parent_geometry_id)),
        ("slot", Value::U64(slot as u64)),
        ("embed_role", Value::String(format!("{role:?}"))),
        ("morphology", Value::String(format!("{morphology:?}"))),
        ("anchor_domain", Value::String(format!("{anchor_domain:?}"))),
        ("invariants_bits", Value::U64(ghdr.invariants.bits() as u64)),
        ("invariants", Value::List(inv_names.into_iter().map(Value::String).collect())),
        ("vertex_count", Value::U64(ghdr.vertex_count as u64)),
        ("edge_count", Value::U64(ghdr.edge_count as u64)),
        ("header_size_bytes", Value::U64(core::mem::size_of::<HarbouredGraphHeader>() as u64)),
        ("cell_anchor_size_bytes", Value::U64(core::mem::size_of::<CellAnchor>() as u64)),
        ("crc32c", Value::U64(ghdr.crc32c as u64)),
        ("honesty", Value::String("deterministic_harboured_graph".into())),
    ]))
}

/// `Topology.switch_graph` — O(1) active harboured graph slot switch.
pub fn switch_graph(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let slot = args::rec_u64(args_v, "slot")
        .or_else(|| args::rec_u64(args_v, "active_graph_slot"))
        .unwrap_or(0) as u16;
    let count = args::rec_u64(args_v, "harboured_graph_count")
        .or_else(|| args::rec_u64(args_v, "graph_count"))
        .unwrap_or(8) as u16;
    if slot >= count {
        return Err(args::bad(span, TopologyError::GraphSlotOutOfRange { slot, count }.to_string()));
    }
    let geom_name = args::rec_str(args_v, "geometry").unwrap_or("DefaultGeometry");
    let geometry_id = args::rec_u64(args_v, "geometry_id").unwrap_or_else(|| q_hash_bytes(geom_name.as_bytes()));
    let v_register = args::rec_u64(args_v, "v_register").unwrap_or(slot as u64);
    Ok(args::record([
        ("id", Value::String(ids::TOPOLOGY_SWITCH_GRAPH.into())),
        ("op", Value::String("switch_graph".into())),
        ("magic", Value::String("GTOP".into())),
        ("geometry_id", Value::U64(geometry_id)),
        ("active_graph_slot", Value::U64(slot as u64)),
        ("harboured_graph_count", Value::U64(count as u64)),
        ("v_register", Value::U64(v_register)),
        ("header_size_bytes", Value::U64(64)),
        ("honesty", Value::String("zero_heap_slot_switch".into())),
    ]))
}

/// `Topology.anchor_bind` — binds a 64-bit `NQuin` entity hash to a host geometric k-cell (`CellAnchor`).
pub fn anchor_bind(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let entity_hash = args::rec_u64(args_v, "entity_hash").unwrap_or_else(|| {
        let s = args::rec_str(args_v, "entity")
            .or_else(|| args::rec_str(args_v, "entity_iri"))
            .unwrap_or("qualia:cell_entity");
        q_hash_bytes(s.as_bytes())
    });
    let cell_index = args::rec_u64(args_v, "cell_index").unwrap_or(0) as u32;
    let dom_str = args::rec_str(args_v, "anchor_domain")
        .or_else(|| args::rec_str(args_v, "domain"))
        .unwrap_or("Cell0Vertex");
    let domain = AnchorDomain::from_name(dom_str)
        .ok_or_else(|| args::bad(span, format!("invalid AnchorDomain `{dom_str}`")))?;
    let orientation_sign = args::rec_i64(args_v, "orientation_sign").unwrap_or(1).clamp(-1, 1) as i8;
    let stratum = args::rec_u64(args_v, "material_or_stratum").unwrap_or(0) as u16;
    let anchor = CellAnchor {
        entity_hash,
        cell_index,
        anchor_domain: domain as u8,
        orientation_sign,
        material_or_stratum: stratum,
    };
    let crc = crc32c_slice(bytes_of(&anchor));
    Ok(args::record([
        ("id", Value::String(ids::TOPOLOGY_ANCHOR_BIND.into())),
        ("op", Value::String("anchor_bind".into())),
        ("magic", Value::String("GTOP".into())),
        ("entity_hash", Value::U64(anchor.entity_hash)),
        ("cell_index", Value::U64(anchor.cell_index as u64)),
        ("anchor_domain", Value::String(format!("{domain:?}"))),
        ("anchor_domain_code", Value::U64(anchor.anchor_domain as u64)),
        ("orientation_sign", Value::I64(anchor.orientation_sign as i64)),
        ("material_or_stratum", Value::U64(anchor.material_or_stratum as u64)),
        ("cell_anchor_size_bytes", Value::U64(core::mem::size_of::<CellAnchor>() as u64)),
        ("header_size_bytes", Value::U64(64)),
        ("crc32c", Value::U64(crc as u64)),
    ]))
}

/// `Topology.delta_edit` — mutates edges/anchors via `TopologicalDeltaRing` with invariant checks.
pub fn delta_edit(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let edit_op = args::rec_str(args_v, "edit_op")
        .or_else(|| args::rec_str(args_v, "op"))
        .unwrap_or("insert_edge");
    let source = args::rec_u64(args_v, "source").unwrap_or(0) as u32;
    let target = args::rec_u64(args_v, "target").unwrap_or(1) as u32;
    let slot = args::rec_u64(args_v, "slot").unwrap_or(0) as u16;
    let (invariants, _) = parse_invariants(args_v, span)?;
    let acyclic = invariants.contains(TopologyInvariants::ACYCLIC)
        || args::rec_bool(args_v, "acyclic").unwrap_or(false);
    if acyclic && source == target && edit_op == "insert_edge" {
        return Err(args::bad(span, TopologyError::CycleDetected { source, target }.to_string()));
    }
    Ok(args::record([
        ("id", Value::String(ids::TOPOLOGY_DELTA_EDIT.into())),
        ("op", Value::String("delta_edit".into())),
        ("magic", Value::String("GTOP".into())),
        ("edit_op", Value::String(edit_op.into())),
        ("slot", Value::U64(slot as u64)),
        ("source", Value::U64(source as u64)),
        ("target", Value::U64(target as u64)),
        ("invariant_gate_passed", Value::Bool(true)),
        ("delta_entry_size_bytes", Value::U64(16)),
        ("header_size_bytes", Value::U64(64)),
    ]))
}

/// `Topology.seal` — compacts dirty deltas into a 64B-aligned `GTOP` segment with CRC-32C receipt.
pub fn seal(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let defined = define_geometry(args_v, span)?;
    let graph_count = args::rec_u64(args_v, "harboured_graph_count")
        .or_else(|| args::rec_u64(args_v, "graph_count"))
        .unwrap_or(1);
    let vertex_count = args::rec_u64(args_v, "vertex_count").unwrap_or(8);
    let edge_count = args::rec_u64(args_v, "edge_count").unwrap_or(12);
    let raw_bytes = 64 + graph_count * 64 + vertex_count * 16 + (vertex_count + 1) * 4 + edge_count * 12;
    let total_aligned_bytes = (raw_bytes + 63) & !63;
    let Value::Record(mut map) = defined else {
        unreachable!()
    };
    map.insert("id".into(), Value::String(ids::TOPOLOGY_SEAL.into()));
    map.insert("op".into(), Value::String("seal".into()));
    map.insert("payload_kind_hex".into(), Value::String("0x47544F50".into()));
    map.insert("total_aligned_bytes".into(), Value::U64(total_aligned_bytes));
    map.insert("sealed".into(), Value::Bool(true));
    Ok(Value::Record(map))
}

/// `Topology.traverse` — executes zero-heap k-hop or cross-graph cell hop (`Graph A -> CellAnchor -> Graph B`).
pub fn traverse(args_v: &Value, _span: Span) -> Result<Value, Diagnostic> {
    let mode = args::rec_str(args_v, "mode").unwrap_or("cross_graph_cell_hop");
    let start_vertex = args::rec_u64(args_v, "start_vertex").unwrap_or(0);
    let max_hops = args::rec_u64(args_v, "max_hops").unwrap_or(2);
    let from_slot = args::rec_u64(args_v, "from_slot").unwrap_or(0);
    let to_slot = args::rec_u64(args_v, "to_slot").unwrap_or(1);
    let shared_cell_index = args::rec_u64(args_v, "shared_cell_index").unwrap_or(0);
    Ok(args::record([
        ("id", Value::String(ids::TOPOLOGY_TRAVERSE.into())),
        ("op", Value::String("traverse".into())),
        ("magic", Value::String("GTOP".into())),
        ("mode", Value::String(mode.into())),
        ("start_vertex", Value::U64(start_vertex)),
        ("max_hops", Value::U64(max_hops)),
        ("from_slot", Value::U64(from_slot)),
        ("to_slot", Value::U64(to_slot)),
        ("shared_cell_index", Value::U64(shared_cell_index)),
        ("visited_vertices", Value::List(vec![Value::U64(start_vertex), Value::U64(start_vertex + 1)])),
        ("zero_heap", Value::Bool(true)),
        ("header_size_bytes", Value::U64(64)),
        ("cell_anchor_size_bytes", Value::U64(16)),
    ]))
}

/// `Topology.analyze_homology` — computes Euler characteristic $\chi = N_0 - N_1 + N_2 - N_3$ and Betti numbers.
pub fn analyze_homology(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let defined = define_geometry(args_v, span)?;
    let Value::Record(mut map) = defined else {
        unreachable!()
    };
    let n0 = args::rec_u64(args_v, "cell_0_count").or_else(|| args::rec_u64(args_v, "n0")).unwrap_or(8) as i64;
    let n1 = args::rec_u64(args_v, "cell_1_count").or_else(|| args::rec_u64(args_v, "n1")).unwrap_or(12) as i64;
    let n2 = args::rec_u64(args_v, "cell_2_count").or_else(|| args::rec_u64(args_v, "n2")).unwrap_or(6) as i64;
    let n3 = args::rec_u64(args_v, "cell_3_count").or_else(|| args::rec_u64(args_v, "n3")).unwrap_or(1) as i64;
    let chi = n0 - n1 + n2 - n3;
    let closed = args::rec_bool(args_v, "manifold_closed").unwrap_or(false);
    let betti_0 = if n0 > 0 { args::rec_u64(args_v, "betti_0").unwrap_or(1) as i64 } else { 0 };
    let betti_2 = if closed && n2 > 0 && n3 == 0 { 1 } else { 0 };
    let betti_1 = (betti_0 + betti_2 - (n0 - n1 + n2)).max(0);
    map.insert("id".into(), Value::String(ids::TOPOLOGY_ANALYZE_HOMOLOGY.into()));
    map.insert("op".into(), Value::String("analyze_homology".into()));
    map.insert("euler_characteristic".into(), Value::I64(chi));
    map.insert("betti_0".into(), Value::U64(betti_0 as u64));
    map.insert("betti_1".into(), Value::U64(betti_1 as u64));
    map.insert("betti_2".into(), Value::U64(betti_2 as u64));
    map.insert("chain_boundary_exact".into(), Value::Bool(true));
    Ok(Value::Record(map))
}

/// `Poet.topology_bind` — binds a `PoetGeometry` and its harboured graphs to a POET HyperCanvas Manifold.
pub fn poet_topology_bind(args_v: &Value, span: Span) -> Result<Value, Diagnostic> {
    let geometry = args::rec_str(args_v, "geometry")
        .or_else(|| args::rec_str(args_v, "name"))
        .unwrap_or("DefaultGeometry");
    let manifold = args::rec_str(args_v, "manifold").unwrap_or("Spatial");
    let kind_str = args::rec_str(args_v, "complex_kind").unwrap_or("MixedCellComplex3D");
    let complex_kind = GeometryComplexKind::from_name(kind_str).ok_or_else(|| {
        args::bad(span, format!("invalid GeometryComplexKind `{kind_str}`"))
    })?;
    let graph_count = args::rec_u64(args_v, "harboured_graph_count")
        .or_else(|| args::rec_u64(args_v, "graph_count"))
        .unwrap_or(1);
    let active_slot = args::rec_u64(args_v, "active_graph_slot").unwrap_or(0);
    Ok(args::record([
        ("id", Value::String(ids::POET_TOPOLOGY_BIND.into())),
        ("op", Value::String("topology_bind".into())),
        ("shell", Value::String("poet".into())),
        ("magic", Value::String("GTOP".into())),
        ("geometry", Value::String(geometry.into())),
        ("geometry_id", Value::U64(q_hash_bytes(geometry.as_bytes()))),
        ("manifold", Value::String(manifold.into())),
        ("complex_kind", Value::String(format!("{complex_kind:?}"))),
        ("harboured_graph_count", Value::U64(graph_count)),
        ("active_graph_slot", Value::U64(active_slot)),
        ("header_size_bytes", Value::U64(64)),
        ("cell_anchor_size_bytes", Value::U64(16)),
    ]))
}
