//! Comprehensive unit & integration test suite for the POET Geometry-Topology Complex
//! ("Geometries Harbour Many Graphs"):
//! - 64B `PoetGeometryHeader`, 64B `HarbouredGraphHeader`, 16B `CellAnchor`, 16B `TopologicalDeltaEntry`
//! - `HarbouredCsrView`, `PoetGeometryView`, cross-graph `hop_via_cell_anchor`, RDF-Star `TAG_EMBEDDED`
//! - `TopologicalDeltaRing` & `MergedGraphView`
//! - Non-recursive `invariants` (`ACYCLIC`, `TEMPORAL_MONOTONIC`, `DEONTIC_GUARDED`,
//!   `PARACONSISTENT_ISOLATED`, `SENSITIVITY_MONOTONIC`, Euler $\chi$, Betti $\beta_0, \beta_1$)
//! - `traverse_10d_pullback` & T16 `project_t16_epistemic_10d`
//! - Intrinsic graph induction (`induce_primal_and_dual_from_half_edges`, `induce_mixed_cell_dual_graph`,
//!   `induce_simplicial_boundary_incidence`, `induce_combinatorial_dart_graph`, `induce_metric_proximity_10d`)
//! - `compile_geometry_segment` & `seal_harboured_graph_deltas` (`GTOP` segment round-trip)

use crate::frame_layout::DEFEATER_BIT;
use crate::modalities::logic::deontic::OP_FORBID;
use crate::modalities::manifold::ManifoldCoordinate10D;
use crate::specialized_libs::computational_geometry::mixed_cell_topology::MixedCell;
use crate::specialized_libs::computational_geometry::{
    build_triangle_half_edges, EdgeSlot, HalfEdge,
};
use crate::tensor::topology::TopologyClass;
use crate::tensor::Tensor10D;
use crate::topology::*;

#[repr(C, align(64))]
struct AlignedBuf([u8; 16384]);

#[test]
fn header_and_anchor_sizes_and_alignment_are_exact() {
    assert_eq!(core::mem::size_of::<PoetGeometryHeader>(), 64);
    assert_eq!(core::mem::align_of::<PoetGeometryHeader>(), 64);
    assert_eq!(core::mem::size_of::<HarbouredGraphHeader>(), 64);
    assert_eq!(core::mem::align_of::<HarbouredGraphHeader>(), 64);
    assert_eq!(core::mem::size_of::<CellAnchor>(), 16);
    assert_eq!(core::mem::align_of::<CellAnchor>(), 16);
    assert_eq!(core::mem::size_of::<TopologicalDeltaEntry>(), 16);
    assert_eq!(core::mem::align_of::<TopologicalDeltaEntry>(), 16);
}

#[test]
fn csr_view_queries_and_reified_edge_resolution() {
    let q0 = crate::NQuin {
        subject: 11,
        predicate: 22,
        object: 33,
        context: 0,
        metadata: 0,
        parity: 11 ^ 22 ^ 33,
    };
    let reified_vid = crate::query::rdf_star::create_virtual_id(11, 22, 33);

    let header = HarbouredGraphHeader {
        graph_id: 1,
        parent_geometry_id: 1,
        vertex_count: 3,
        edge_count: 2,
        embed_role: GraphEmbedRole::SemanticOverlay as u8,
        morphology: GraphMorphology::HyperGraph as u8,
        invariants: TopologyInvariants::REIFIED_EDGES,
        ..Default::default()
    };
    let anchors = [
        CellAnchor {
            entity_hash: 100,
            cell_index: 5,
            anchor_domain: AnchorDomain::Cell2Face as u8,
            orientation_sign: 1,
            material_or_stratum: 0,
        },
        CellAnchor {
            entity_hash: 200,
            cell_index: 6,
            anchor_domain: AnchorDomain::Cell2Face as u8,
            orientation_sign: 1,
            material_or_stratum: 0,
        },
        CellAnchor {
            entity_hash: reified_vid,
            cell_index: 5,
            anchor_domain: AnchorDomain::Cell2Face as u8,
            orientation_sign: 0,
            material_or_stratum: 0,
        },
    ];
    let csr_row_ptr = [0u32, 1, 1, 2];
    let csr_col_idx = [1u32, 1];
    let edge_anchor = [0u32, 0];
    let csc_col_ptr = [0u32, 0, 2, 2];
    let csc_row_idx = [0u32, 2];
    let weights = [1.5f32, 0.5f32];

    let view = HarbouredCsrView::from_slices(
        &header,
        &anchors,
        &csr_row_ptr,
        &csr_col_idx,
        &edge_anchor,
        &csc_col_ptr,
        &csc_row_idx,
        Some(&weights),
    )
    .unwrap();

    assert_eq!(view.out_degree(0).unwrap(), 1);
    assert_eq!(view.in_degree(1).unwrap(), 2);
    assert_eq!(view.out_neighbors(0).unwrap(), &[1]);
    assert_eq!(view.in_neighbors(1).unwrap(), &[0, 2]);
    assert_eq!(view.out_edge_weights(0).unwrap().unwrap(), &[1.5]);

    let mut roots = [0u32; 4];
    assert_eq!(view.collect_roots(&mut roots), 2);
    assert_eq!(&roots[..2], &[0, 2]);

    let mut sinks = [0u32; 4];
    assert_eq!(view.collect_sinks(&mut sinks), 1);
    assert_eq!(sinks[0], 1);

    let mut cell_verts = [0u32; 4];
    assert_eq!(
        view.find_vertices_by_cell(AnchorDomain::Cell2Face, 5, &mut cell_verts),
        2
    );
    assert_eq!(&cell_verts[..2], &[0, 2]);

    let quins = [q0];
    assert_eq!(view.resolve_reified_edge_vertex(&quins, 0), Some(2));
}

#[test]
fn merged_graph_view_applies_insert_tombstone_and_anchor_bind() {
    let header = HarbouredGraphHeader {
        vertex_count: 3,
        edge_count: 1,
        ..Default::default()
    };
    let anchors = [CellAnchor::default(); 3];
    let row_ptr = [0u32, 1, 1, 1];
    let col_idx = [1u32];
    let edge_anc = [10u32];
    let csc_ptr = [0u32, 0, 1, 1];
    let csc_row = [0u32];
    let base = HarbouredCsrView::from_slices(
        &header, &anchors, &row_ptr, &col_idx, &edge_anc, &csc_ptr, &csc_row, None,
    )
    .unwrap();

    let mut ring: TopologicalDeltaRing<16> = TopologicalDeltaRing::new();
    ring.insert_edge(0, 0, 2, 20).unwrap();
    ring.tombstone_edge(0, 0, 1).unwrap();
    ring.bind_anchor(0, 2, 77, AnchorDomain::Cell3Volume)
        .unwrap();
    assert_eq!(ring.dirty_slot_mask(), 1);

    let merged = MergedGraphView::new(base, &ring, 0);
    let mut targets = [0u32; 4];
    let mut e_anchors = [0u32; 4];
    let n = merged
        .effective_out_neighbors(0, &mut targets, &mut e_anchors)
        .unwrap();
    assert_eq!(n, 1);
    assert_eq!(targets[0], 2);
    assert_eq!(e_anchors[0], 20);

    let bound = merged.effective_cell_anchor(2).unwrap();
    assert_eq!(bound.cell_index, 77);
    assert_eq!(bound.anchor_domain, AnchorDomain::Cell3Volume as u8);
}

#[test]
fn acyclic_modal_and_sensitivity_invariants_enforced() {
    let header = HarbouredGraphHeader {
        vertex_count: 3,
        edge_count: 2,
        invariants: TopologyInvariants::ACYCLIC
            | TopologyInvariants::DEONTIC_GUARDED
            | TopologyInvariants::PARACONSISTENT_ISOLATED
            | TopologyInvariants::SENSITIVITY_MONOTONIC,
        context_and_state_mask: 1, // max sensitivity = 1 (RESTRICTED)
        ..Default::default()
    };
    let anchors = [CellAnchor::default(); 3];
    let row_ptr = [0u32, 1, 2, 2];
    let col_idx = [1u32, 2];
    let edge_anc = [0u32, 1];
    let csc_ptr = [0u32, 0, 1, 2];
    let csc_row = [0u32, 1];
    let csr = HarbouredCsrView::from_slices(
        &header, &anchors, &row_ptr, &col_idx, &edge_anc, &csc_ptr, &csc_row, None,
    )
    .unwrap();

    let ring: TopologicalDeltaRing<8> = TopologicalDeltaRing::new();
    let merged = MergedGraphView::new(csr, &ring, 0);
    let mut bits = [0u64; 1];
    let mut queue = [0u32; 4];

    assert_eq!(
        check_acyclic_insert(&merged, 2, 0, &mut bits, &mut queue),
        Err(TopologyError::CycleDetected {
            source: 2,
            target: 0
        })
    );
    assert!(check_acyclic_insert(&merged, 0, 2, &mut bits, &mut queue).is_ok());

    let clean_quins = [crate::NQuin::default(); 2];
    let summary = verify_graph_invariants(&csr, &clean_quins, None, &mut bits, &mut queue).unwrap();
    assert_eq!(summary.betti_0, 1);
    assert_eq!(summary.betti_1, 0);
    assert_eq!(summary.euler_characteristic, 1);

    // Undefeated OP_FORBID must fail closed
    let mut forbid_quins = clean_quins;
    forbid_quins[1].predicate = OP_FORBID as u64;
    assert_eq!(
        verify_graph_invariants(&csr, &forbid_quins, None, &mut bits, &mut queue),
        Err(TopologyError::DeonticForbidViolation { edge_anchor_idx: 1 })
    );

    // Defeated OP_FORBID (with DEFEATER_BIT) succeeds
    forbid_quins[1].predicate = (OP_FORBID as u64) | DEFEATER_BIT;
    assert!(verify_graph_invariants(&csr, &forbid_quins, None, &mut bits, &mut queue).is_ok());

    // Belnap Both (0x32) paraconsistent contagion must fail closed
    let mut para_quins = clean_quins;
    para_quins[0].predicate = OP_BELNAP_BOTH as u64;
    assert_eq!(
        verify_graph_invariants(&csr, &para_quins, None, &mut bits, &mut queue),
        Err(TopologyError::ParaconsistentContagion { edge_anchor_idx: 0 })
    );
}

#[test]
fn intrinsic_induction_and_multi_graph_geometry_compaction_roundtrip() {
    // Closed tetrahedron surface mesh: V=4, E=6 (12 half-edges), F=4 -> Euler χ = 4 - 6 + 4 = 2
    let triangles = [[0u32, 1, 2], [2, 1, 3], [0, 3, 1], [0, 2, 3]];
    let mut half_edges = [HalfEdge::default(); 12];
    let mut scratch = [EdgeSlot::default(); 32];
    build_triangle_half_edges(4, &triangles, &mut half_edges, &mut scratch).unwrap();

    let geom_id = 0xCAFE_BABE_0042_u64;
    let budget = 256 * 1024;

    let (primal, dual) =
        induce_primal_and_dual_from_half_edges(geom_id, &half_edges, 4, 4, budget).unwrap();
    let incidence =
        induce_simplicial_boundary_incidence(geom_id, &triangles, 4, budget).unwrap();
    let dart_map = induce_combinatorial_dart_graph(geom_id, &half_edges, budget).unwrap();

    // Also induce a 3D mixed-cell dual from two adjacent tetrahedra sharing face (0, 1, 2)
    let mixed_cells = [
        MixedCell::tetra(0, 1, 2, 3, 1),
        MixedCell::tetra(0, 2, 1, 4, 2),
    ];
    let vol_dual = induce_mixed_cell_dual_graph(geom_id, &mixed_cells, budget).unwrap();
    assert_eq!(vol_dual.anchors.len(), 2);
    assert_eq!(vol_dual.edges.len(), 2); // 0 <-> 1

    // Semantic overlay graph anchored to Face #2 of the tetrahedron
    let sem_graph = UnsealedGraphSpec {
        graph_id: 0x9999,
        embed_role: GraphEmbedRole::SemanticOverlay,
        morphology: GraphMorphology::DeterministicDAG,
        invariants: TopologyInvariants::ACYCLIC,
        context_and_state_mask: 0,
        anchors: vec![
            CellAnchor {
                entity_hash: 0xAAAA,
                cell_index: 2,
                anchor_domain: AnchorDomain::Cell2Face as u8,
                orientation_sign: 1,
                material_or_stratum: 0,
            },
            CellAnchor {
                entity_hash: 0xBBBB,
                cell_index: 3,
                anchor_domain: AnchorDomain::Cell2Face as u8,
                orientation_sign: 1,
                material_or_stratum: 0,
            },
        ],
        edges: vec![(0, 1, 0)],
        edge_weights: None,
    };

    let spec = UnsealedGeometrySpec {
        geometry_id: geom_id,
        construct_schema_hash: 0x1234_5678,
        manifold_id: 7,
        complex_kind: GeometryComplexKind::SimplicialComplex,
        metric_kind: GeometryMetricKind::Euclidean,
        ambient_dim: 10,
        intrinsic_dim: 2,
        cell_counts: [4, 6, 4, 0],
        active_graph_slot: 0,
        graphs: vec![primal, dual, incidence, dart_map, sem_graph],
    };

    let mut aligned = AlignedBuf([0u8; 16384]);
    let written = compile_geometry_segment(&spec, budget, &mut aligned.0).unwrap();
    let geom_view = PoetGeometryView::from_segment_bytes(&aligned.0[..written]).unwrap();

    assert_eq!(geom_view.harboured_graph_count(), 5);
    assert_eq!(verify_geometry_complex_homology(&geom_view).unwrap(), 2);

    // Functorial hop: hop from SemanticOverlay (slot 4, vertex 0 anchored at Cell2Face #2)
    // to PoincareDual (slot 1, where vertex 2 is anchored at Cell2Face #2)!
    let mut hopped = [0u32; 4];
    let hop_count = geom_view
        .hop_via_cell_anchor(4, 0, 1, &mut hopped)
        .unwrap();
    assert_eq!(hop_count, 1);
    assert_eq!(hopped[0], 2);

    // Verify pullback and T16 epistemic projection
    let dual_csr = geom_view.graph_at_slot(1).unwrap();
    let mut bits = [0u64; 1];
    let mut queue = [0u32; 8];
    let homology = verify_graph_invariants(&dual_csr, &[], None, &mut bits, &mut queue).unwrap();
    assert_eq!(homology.betti_0, 1);

    let mut ep = ManifoldCoordinate10D::default();
    project_t16_epistemic_10d(geom_view.header(), dual_csr.header(), &homology, &mut ep);
    assert!(ep.topological_spin != 0.0);
    assert!(ep.manifold_curvature != 0.0);

    // Verify 10D metric proximity induction
    let pts = [
        Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        Tensor10D::new(0.0, 0.0, 0.0, 10.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
    ];
    let prox = induce_metric_proximity_10d(
        geom_id,
        &pts,
        &[101, 102, 103],
        2.0,
        TopologyClass::Euclidean,
        budget,
    )
    .unwrap();
    assert_eq!(prox.edges.len(), 2); // 0 <-> 1 only
}
