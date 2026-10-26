//! POET Geometry-Topology Complex (`Geometries Harbour Many Graphs`).
//!
//! Copyright (c) 2026 Timothy Charles Holborn. All rights reserved.
//! Principal / inventor: Timothy Charles Holborn <timothy.holborn@gmail.com>
//! Assignment: COPYRIGHT.md  Licence: LICENSE (CC BY-NC-ND 4.0)
//!
//! Defines [`GeometrySeed`] (the host geometric complex carrying metric,
//! cellular bounds $N_0 \dots N_3$, and `.10d` `v`/`w` register bindings),
//! [`HarbouredGraphSeed`] (each intrinsic or semantic graph harboured inside
//! a host geometry), and [`CellAnchorSpec`] (binding a semantic `NQuin` or
//! container wire to a $k$-cell of the host geometry).

use serde::{Deserialize, Serialize};

/// Host Geometry primitive in POET capable of harbouring multiple graphs.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct GeometrySeed {
    /// Stable geometry identifier (e.g. `"north-spring-catchment"` or `"hypercanvas-default"`).
    #[serde(default)]
    pub id: String,
    /// Human-readable geometry label.
    #[serde(default)]
    pub label: String,
    /// Cellular complex family (`"MixedCellComplex3D"`, `"SurfaceManifold2D"`,
    /// `"SimplicialComplex"`, `"MetricPointCloud10D"`, `"HyperCanvasLayout"`).
    #[serde(default)]
    pub complex_kind: String,
    /// Construct ID whose SHACL shapes / norms this geometry satisfies.
    #[serde(default)]
    pub satisfies_construct_id: String,
    /// Manifold ID hosting this geometry lens.
    #[serde(default)]
    pub manifold_id: String,
    /// `.10d` `v` axis topology/metric selector register.
    #[serde(default)]
    pub v_register: u16,
    /// `.10d` `w` axis manifold domain selector.
    #[serde(default)]
    pub w_domain: u32,
    /// Metric kind (`"Euclidean"`, `"ToroidalCyclic"`, `"HyperbolicPoincare"`, `"FisherRaoSimplex"`).
    #[serde(default)]
    pub metric_kind: String,
    /// Cellular counts $[N_0, N_1, N_2, N_3]$ (0-vertices, 1-edges, 2-faces, 3-volumes).
    #[serde(default)]
    pub cell_counts: [u32; 4],
    /// Currently active harboured graph slot ($0 \le m < M$).
    #[serde(default)]
    pub active_graph_slot: usize,
    /// Fleet of intrinsic and semantic graphs harboured inside this geometry.
    #[serde(default)]
    pub harboured_graphs: Vec<HarbouredGraphSeed>,
}

/// An individual graph harboured inside a [`GeometrySeed`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct HarbouredGraphSeed {
    /// Stable graph identifier within the host geometry.
    #[serde(default)]
    pub id: String,
    /// Human-readable graph lens label.
    #[serde(default)]
    pub label: String,
    /// Embedding role (`"Primal1Skeleton"`, `"PoincareDual"`, `"BoundaryIncidence"`,
    /// `"SemanticOverlay"`, `"DeonticGovernance"`, `"CausalFlux"`).
    #[serde(default)]
    pub embed_role: String,
    /// Graph morphology (`"DirectedMultigraph"`, `"DeterministicDAG"`, `"HyperGraph"`,
    /// `"SymmetricGraph"`, `"BipartiteFactor"`, `"InvolutionMap"`).
    #[serde(default)]
    pub morphology: String,
    /// Primary cellular anchor domain (`"Cell0Vertex"`, `"Cell1Edge"`, `"Cell2Face"`,
    /// `"Cell3Volume"`, `"SemanticQuin"`).
    #[serde(default)]
    pub anchor_domain: String,
    /// Enforced topological invariants (e.g. `["ACYCLIC", "TRANSITIVE", "DEONTIC_GUARDED"]`).
    #[serde(default)]
    pub invariants: Vec<String>,
    /// CSR/CSC adjacency and predicate binding specification.
    #[serde(default)]
    pub adjacency: AdjacencySpec,
    /// Boundary source/sink query and homology target specification.
    #[serde(default)]
    pub boundary: BoundarySpec,
}

/// CSR/CSC adjacency and semantic predicate binding for a [`HarbouredGraphSeed`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AdjacencySpec {
    /// Storage layout (e.g. `"CSR_CSC"`, `"CSR_ONLY"`, `"DART_INV"`).
    #[serde(default)]
    pub layout: String,
    /// Bound predicate IRIs (e.g. `["qualia:drains_to", "qualia:groundsObservation"]`).
    #[serde(default)]
    pub edge_binding: Vec<String>,
    /// Context / world filter (`0` = all worlds).
    #[serde(default)]
    pub context_filter: u32,
    /// Modality state filter (e.g. `"ACTIVE"`, `"VERIFIED"`).
    #[serde(default)]
    pub state_filter: String,
}

/// Boundary condition and Betti homology target for a [`HarbouredGraphSeed`].
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BoundarySpec {
    /// Root / source query expression (e.g. `"in_degree == 0"`).
    #[serde(default)]
    pub roots_query: String,
    /// Sink query expression (e.g. `"out_degree == 0"`).
    #[serde(default)]
    pub sinks_query: String,
    /// Maximum traversal depth for bounded zero-heap evaluation.
    #[serde(default)]
    pub max_depth: u32,
    /// Optional target Betti numbers $[\beta_0, \beta_1, \beta_2]$.
    #[serde(default)]
    pub betti_target: Option<[u32; 3]>,
}

/// Cellular anchor binding a container wire or semantic `NQuin` to a $k$-cell of the host geometry.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellAnchorSpec {
    /// Anchor domain (`"Cell0Vertex"`, `"Cell1Edge"`, `"Cell2Face"`, `"Cell3Volume"`, `"SemanticQuin"`).
    #[serde(default)]
    pub anchor_domain: String,
    /// Index of the target cell within `anchor_domain`.
    #[serde(default)]
    pub cell_index: u32,
    /// Orientation sign (`+1`, `-1`, or `0` for unoriented).
    #[serde(default)]
    pub orientation_sign: i8,
    /// Material or stratum layer identifier.
    #[serde(default)]
    pub material_or_stratum: u16,
}

impl GeometrySeed {
    /// Computes the cellular Euler characteristic $\chi = N_0 - N_1 + N_2 - N_3$.
    pub fn euler_characteristic(&self) -> i32 {
        (self.cell_counts[0] as i32) - (self.cell_counts[1] as i32)
            + (self.cell_counts[2] as i32)
            - (self.cell_counts[3] as i32)
    }

    /// Returns a reference to the currently active harboured graph, if any.
    pub fn active_graph(&self) -> Option<&HarbouredGraphSeed> {
        self.harboured_graphs
            .get(self.active_graph_slot)
            .or_else(|| self.harboured_graphs.first())
    }

    /// Switches `active_graph_slot` to the harboured graph matching `graph_id`.
    /// Returns `true` if found and switched.
    pub fn switch_active_graph(&mut self, graph_id: &str) -> bool {
        if let Some(pos) = self.harboured_graphs.iter().position(|g| g.id == graph_id) {
            self.active_graph_slot = pos;
            true
        } else {
            false
        }
    }

    /// Populates canonical `Primal1Skeleton`, `PoincareDual`, and `SemanticOverlay`
    /// harboured graphs if `self.harboured_graphs` is currently empty.
    pub fn add_intrinsic_default_graphs(&mut self) {
        if !self.harboured_graphs.is_empty() {
            return;
        }
        self.harboured_graphs.push(HarbouredGraphSeed {
            id: "primal-1-skeleton".into(),
            label: "Primal 1-Skeleton (0-Vertices ↔ 1-Edges)".into(),
            embed_role: "Primal1Skeleton".into(),
            morphology: "SymmetricGraph".into(),
            anchor_domain: "Cell0Vertex".into(),
            invariants: vec!["SYMMETRIC".into(), "SPATIAL_METRIC".into()],
            adjacency: AdjacencySpec {
                layout: "CSR_CSC".into(),
                edge_binding: vec![],
                context_filter: 0,
                state_filter: "ACTIVE".into(),
            },
            boundary: BoundarySpec {
                roots_query: "boundary_vertex == true".into(),
                sinks_query: String::new(),
                max_depth: 16,
                betti_target: Some([1, 0, 0]),
            },
        });
        self.harboured_graphs.push(HarbouredGraphSeed {
            id: "poincare-dual".into(),
            label: "Poincaré Dual Adjacency".into(),
            embed_role: "PoincareDual".into(),
            morphology: "DirectedMultigraph".into(),
            anchor_domain: "Cell2Face".into(),
            invariants: vec!["MANIFOLD_CLOSED".into(), "CHAIN_BOUNDARY_EXACT".into()],
            adjacency: AdjacencySpec {
                layout: "CSR_CSC".into(),
                edge_binding: vec![],
                context_filter: 0,
                state_filter: "ACTIVE".into(),
            },
            boundary: BoundarySpec {
                roots_query: "in_degree == 0".into(),
                sinks_query: "out_degree == 0".into(),
                max_depth: 16,
                betti_target: Some([1, 0, 1]),
            },
        });
        self.harboured_graphs.push(HarbouredGraphSeed {
            id: "semantic-overlay".into(),
            label: "RDF-Star Semantic Overlay".into(),
            embed_role: "SemanticOverlay".into(),
            morphology: "HyperGraph".into(),
            anchor_domain: "SemanticQuin".into(),
            invariants: vec!["REIFIED_EDGES".into(), "DEONTIC_GUARDED".into()],
            adjacency: AdjacencySpec {
                layout: "CSR_CSC".into(),
                edge_binding: vec!["qualia:groundsObservation".into()],
                context_filter: 0,
                state_filter: "ACTIVE".into(),
            },
            boundary: BoundarySpec {
                roots_query: "in_degree == 0".into(),
                sinks_query: "out_degree == 0".into(),
                max_depth: 8,
                betti_target: None,
            },
        });
    }

    /// Returns canonical preset geometries (`SurfaceManifold2D`, `MixedCellComplex3D`,
    /// and `HyperCanvasLayout`), each harbouring multiple graphs.
    pub fn default_presets() -> Vec<GeometrySeed> {
        let mut surface = GeometrySeed {
            id: "surface-manifold-2d".into(),
            label: "Geodesic Surface Manifold 2D".into(),
            complex_kind: "SurfaceManifold2D".into(),
            satisfies_construct_id: "geometry-cad".into(),
            manifold_id: "spatial".into(),
            v_register: 0,
            w_domain: 1,
            metric_kind: "Euclidean".into(),
            cell_counts: [128, 378, 252, 0], // chi = 128 - 378 + 252 = 2 (S^2)
            active_graph_slot: 0,
            harboured_graphs: Vec::new(),
        };
        surface.add_intrinsic_default_graphs();

        let catchment = GeometrySeed {
            id: "north-spring-catchment-3d".into(),
            label: "North Spring Mixed-Cell Complex 3D".into(),
            complex_kind: "MixedCellComplex3D".into(),
            satisfies_construct_id: "research-lab".into(),
            manifold_id: "research".into(),
            v_register: 1,
            w_domain: 3,
            metric_kind: "Euclidean".into(),
            cell_counts: [64, 144, 98, 17], // chi = 64 - 144 + 98 - 17 = 1
            active_graph_slot: 0,
            harboured_graphs: vec![
                HarbouredGraphSeed {
                    id: "primal-mesh".into(),
                    label: "Primal 1-Skeleton Mesh".into(),
                    embed_role: "Primal1Skeleton".into(),
                    morphology: "SymmetricGraph".into(),
                    anchor_domain: "Cell0Vertex".into(),
                    invariants: vec!["SYMMETRIC".into(), "SPATIAL_METRIC".into()],
                    adjacency: AdjacencySpec {
                        layout: "CSR_CSC".into(),
                        edge_binding: vec![],
                        context_filter: 0,
                        state_filter: "ACTIVE".into(),
                    },
                    boundary: BoundarySpec {
                        roots_query: "in_degree == 0".into(),
                        sinks_query: "out_degree == 0".into(),
                        max_depth: 16,
                        betti_target: Some([1, 0, 0]),
                    },
                },
                HarbouredGraphSeed {
                    id: "subsurface-flux-dual".into(),
                    label: "Subsurface Poincaré Dual Flux".into(),
                    embed_role: "PoincareDual".into(),
                    morphology: "DirectedMultigraph".into(),
                    anchor_domain: "Cell3Volume".into(),
                    invariants: vec!["MANIFOLD_CLOSED".into(), "CHAIN_BOUNDARY_EXACT".into()],
                    adjacency: AdjacencySpec {
                        layout: "CSR_CSC".into(),
                        edge_binding: vec!["qualia:drains_to".into()],
                        context_filter: 0,
                        state_filter: "ACTIVE".into(),
                    },
                    boundary: BoundarySpec {
                        roots_query: "in_degree == 0".into(),
                        sinks_query: "out_degree == 0".into(),
                        max_depth: 24,
                        betti_target: Some([1, 0, 0]),
                    },
                },
                HarbouredGraphSeed {
                    id: "water-custody-norms".into(),
                    label: "Deontic Water Custody Governance".into(),
                    embed_role: "DeonticGovernance".into(),
                    morphology: "DeterministicDAG".into(),
                    anchor_domain: "Cell2Face".into(),
                    invariants: vec![
                        "ACYCLIC".into(),
                        "DEONTIC_GUARDED".into(),
                        "SENSITIVITY_MONOTONIC".into(),
                    ],
                    adjacency: AdjacencySpec {
                        layout: "CSR_CSC".into(),
                        edge_binding: vec!["qualia:obligates".into(), "qualia:permits".into()],
                        context_filter: 0,
                        state_filter: "ACTIVE".into(),
                    },
                    boundary: BoundarySpec {
                        roots_query: "in_degree == 0".into(),
                        sinks_query: "out_degree == 0".into(),
                        max_depth: 12,
                        betti_target: None,
                    },
                },
            ],
        };

        let hypercanvas = GeometrySeed {
            id: "hypercanvas-layout".into(),
            label: "POET HyperCanvas Multi-Graph Carrier".into(),
            complex_kind: "HyperCanvasLayout".into(),
            satisfies_construct_id: "poet".into(),
            manifold_id: "workspace".into(),
            v_register: 2,
            w_domain: 0,
            metric_kind: "Euclidean".into(),
            cell_counts: [24, 36, 14, 0], // chi = 24 - 36 + 14 = 2
            active_graph_slot: 0,
            harboured_graphs: vec![
                HarbouredGraphSeed {
                    id: "proof-derivation-dag".into(),
                    label: "Formal Proof & Derivation DAG".into(),
                    embed_role: "CausalFlux".into(),
                    morphology: "DeterministicDAG".into(),
                    anchor_domain: "Cell0Vertex".into(),
                    invariants: vec!["ACYCLIC".into(), "TRANSITIVE".into()],
                    adjacency: AdjacencySpec {
                        layout: "CSR_CSC".into(),
                        edge_binding: vec!["qualia:derivesFrom".into()],
                        context_filter: 0,
                        state_filter: "ACTIVE".into(),
                    },
                    boundary: BoundarySpec {
                        roots_query: "in_degree == 0".into(),
                        sinks_query: "out_degree == 0".into(),
                        max_depth: 16,
                        betti_target: Some([1, 0, 0]),
                    },
                },
                HarbouredGraphSeed {
                    id: "deontic-governance-dag".into(),
                    label: "Deontic Bilateral Governance DAG".into(),
                    embed_role: "DeonticGovernance".into(),
                    morphology: "DeterministicDAG".into(),
                    anchor_domain: "Cell2Face".into(),
                    invariants: vec!["ACYCLIC".into(), "DEONTIC_GUARDED".into()],
                    adjacency: AdjacencySpec {
                        layout: "CSR_CSC".into(),
                        edge_binding: vec!["qualia:obligates".into(), "qualia:forbids".into()],
                        context_filter: 0,
                        state_filter: "ACTIVE".into(),
                    },
                    boundary: BoundarySpec {
                        roots_query: "in_degree == 0".into(),
                        sinks_query: "out_degree == 0".into(),
                        max_depth: 8,
                        betti_target: None,
                    },
                },
                HarbouredGraphSeed {
                    id: "rdf-star-hypergraph".into(),
                    label: "RDF-Star Epistemic HyperGraph".into(),
                    embed_role: "SemanticOverlay".into(),
                    morphology: "HyperGraph".into(),
                    anchor_domain: "SemanticQuin".into(),
                    invariants: vec!["REIFIED_EDGES".into(), "DEONTIC_GUARDED".into()],
                    adjacency: AdjacencySpec {
                        layout: "CSR_CSC".into(),
                        edge_binding: vec!["qualia:groundsObservation".into(), "qualia:knows".into()],
                        context_filter: 0,
                        state_filter: "ACTIVE".into(),
                    },
                    boundary: BoundarySpec {
                        roots_query: "in_degree == 0".into(),
                        sinks_query: "out_degree == 0".into(),
                        max_depth: 12,
                        betti_target: None,
                    },
                },
            ],
        };

        vec![surface, catchment, hypercanvas]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn euler_characteristic_and_graph_switching() {
        let presets = GeometrySeed::default_presets();
        assert_eq!(presets.len(), 3);

        let mut surface = presets[0].clone();
        assert_eq!(surface.euler_characteristic(), 2);
        assert_eq!(surface.harboured_graphs.len(), 3);
        assert_eq!(surface.active_graph().unwrap().id, "primal-1-skeleton");

        assert!(surface.switch_active_graph("poincare-dual"));
        assert_eq!(surface.active_graph_slot, 1);
        assert_eq!(surface.active_graph().unwrap().embed_role, "PoincareDual");
        assert!(!surface.switch_active_graph("nonexistent-graph"));
    }

    #[test]
    fn geometry_seed_json_roundtrip() {
        let presets = GeometrySeed::default_presets();
        let json = serde_json::to_string(&presets[1]).unwrap();
        let decoded: GeometrySeed = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded, presets[1]);
        assert_eq!(decoded.euler_characteristic(), 1);
    }
}
