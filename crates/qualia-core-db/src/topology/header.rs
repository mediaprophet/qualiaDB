//! Normative 64-byte cache-line-aligned headers (`PoetGeometryHeader`, `HarbouredGraphHeader`),
//! 16-byte `CellAnchor`, morphological & geometric enums, and `TopologyInvariants` bitflags
//! for the POET Geometry-Topology Complex ("Geometries Harbour Many Graphs").
//!
//! Tier-1 Zero-Heap (`AGENTS.md` §0 & §0-A): all binary descriptors are `#[repr(C)]`,
//! `bytemuck::Pod + bytemuck::Zeroable`, with compile-time size and alignment assertions.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// Magic 4-byte tag (`"GTOP"`) identifying a Geometry-Topology Complex segment in `.q42` / `.10d`.
pub const GTOP_MAGIC: [u8; 4] = *b"GTOP";

/// Classification of the host Geometry Complex that harbours graphs.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeometryComplexKind {
    /// Pure metric point-cloud / 0-cell space in Tensor10D (induces ε-ball, Delaunay, c-kNN graphs).
    #[default]
    MetricPointCloud10D = 0x01,
    /// 2D Planar subdivision / Doubly-Connected Edge List (`dcel_overlay::Dcel`, polygons with holes).
    PlanarDcel2D = 0x02,
    /// 2-Manifold surface mesh / Combinatorial 2-Map (`mesh_section`, `topology_section`, `combinatorial_map::Dart`).
    SurfaceManifold2D = 0x03,
    /// k-Simplicial / Cochain Complex with DEC operators (`ddg_operators::ChainTerm`, `HodgeSummary`, ∂∘∂=0).
    SimplicialComplex = 0x04,
    /// Mixed polyhedral volumetric complex: Triangles, Quads, Tetrahedra, Hexahedra (`mixed_cell_topology::MixedCell`).
    MixedCellComplex3D = 0x05,
    /// Constructive Solid Geometry & 3D surface arrangement (`csg_section::CSG1`, `arrangement_3d`, CAD B-Rep).
    CsgArrangement3D = 0x06,
    /// 2:1 Balanced Quadtree / Octree / Sparse Voxel DAG (`quad_octree_mesh`, `voxel_section::VOX0`).
    HierarchicalTreeGrid = 0x07,
    /// Statistical manifold / probability simplex Δ^{n-1} with Fisher-Rao & Bregman-KL geometry (`statistical_manifold`).
    StatisticalSimplex = 0x08,
    /// Periodic Toroidal / Lattice wave-field domain (`TopologyClass::Cyclic`, PDE/CFD grid).
    ToroidalWaveLattice = 0x09,
    /// POET HyperCanvas Multi-Strata Layout Geometry (2D/3D/10D container boxes, strata planes & portals).
    HyperCanvasLayout = 0x0A,
}

impl GeometryComplexKind {
    #[inline]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::MetricPointCloud10D),
            0x02 => Some(Self::PlanarDcel2D),
            0x03 => Some(Self::SurfaceManifold2D),
            0x04 => Some(Self::SimplicialComplex),
            0x05 => Some(Self::MixedCellComplex3D),
            0x06 => Some(Self::CsgArrangement3D),
            0x07 => Some(Self::HierarchicalTreeGrid),
            0x08 => Some(Self::StatisticalSimplex),
            0x09 => Some(Self::ToroidalWaveLattice),
            0x0A => Some(Self::HyperCanvasLayout),
            _ => None,
        }
    }

    #[inline]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "MetricPointCloud10D" | "PointCloud" => Some(Self::MetricPointCloud10D),
            "PlanarDcel2D" | "DCEL" => Some(Self::PlanarDcel2D),
            "SurfaceManifold2D" | "SurfaceMesh" => Some(Self::SurfaceManifold2D),
            "SimplicialComplex" => Some(Self::SimplicialComplex),
            "MixedCellComplex3D" | "MixedCell" => Some(Self::MixedCellComplex3D),
            "CsgArrangement3D" | "CSG" => Some(Self::CsgArrangement3D),
            "HierarchicalTreeGrid" | "OctreeGrid" => Some(Self::HierarchicalTreeGrid),
            "StatisticalSimplex" => Some(Self::StatisticalSimplex),
            "ToroidalWaveLattice" | "ToroidalLattice" => Some(Self::ToroidalWaveLattice),
            "HyperCanvasLayout" | "HyperCanvas" => Some(Self::HyperCanvasLayout),
            _ => None,
        }
    }
}

/// Ambient metric law governing distance & curvature in the host Geometry.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GeometryMetricKind {
    #[default]
    Euclidean = 0x00,
    ToroidalCyclic = 0x01,
    HyperbolicPoincare = 0x02,
    SphericalGeodesic = 0x03,
    FisherRaoSimplex = 0x04,
    LorentzianCausal = 0x05,
}

impl GeometryMetricKind {
    #[inline]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x00 => Some(Self::Euclidean),
            0x01 => Some(Self::ToroidalCyclic),
            0x02 => Some(Self::HyperbolicPoincare),
            0x03 => Some(Self::SphericalGeodesic),
            0x04 => Some(Self::FisherRaoSimplex),
            0x05 => Some(Self::LorentzianCausal),
            _ => None,
        }
    }
}

/// How a HarbouredGraph relates to its host `GeometryComplexKind`.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphEmbedRole {
    /// Primal 1-skeleton: 0-cells (vertices) connected by 1-cells (edges).
    #[default]
    Primal1Skeleton = 0x01,
    /// Poincaré / Voronoi / Circumcentric Dual: k-cells (faces/volumes) connected across shared (k-1)-facets.
    PoincareDual = 0x02,
    /// Levi / Hasse Boundary Incidence Bipartite Graph: (k)-cells ↔ boundary (k-1)-cells (∂_k operator).
    BoundaryIncidence = 0x03,
    /// Combinatorial Dart Involution Graph: darts linked by β₁ (next-in-face), β₂ (twin), β₃ (adjacent-volume).
    CombinatorialDartMap = 0x04,
    /// Multigrid / Spatial Hierarchy Tree: parent-child subdivision + 2:1 balanced face-neighbour links.
    SpatialHierarchy = 0x05,
    /// Metric-Induced Proximity Graph: Geodesic kNN (`cknn_laplacian`), ε-ball, Delaunay, or Gabriel graph.
    MetricProximity = 0x06,
    /// Medial Axis / Curve Skeleton / Reeb Graph (`curve_skeleton`, `chordal_axis_2d`, Morse-Smale).
    TopologicalSkeleton = 0x07,
    /// Embedded Semantic / Ontological Graph: `NQuin` triples/hyperedges anchored to k-cells of the geometry.
    SemanticOverlay = 0x08,
    /// Deontic / Normative Governance Graph: permissions, obligations, prohibitions & consent boundaries over regions.
    DeonticGovernance = 0x09,
    /// Causal / Temporal / Flux Graph: CFD/hydrological flux, neural message-passing, or LTL state trace over cells.
    CausalFlux = 0x0A,
}

impl GraphEmbedRole {
    #[inline]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::Primal1Skeleton),
            0x02 => Some(Self::PoincareDual),
            0x03 => Some(Self::BoundaryIncidence),
            0x04 => Some(Self::CombinatorialDartMap),
            0x05 => Some(Self::SpatialHierarchy),
            0x06 => Some(Self::MetricProximity),
            0x07 => Some(Self::TopologicalSkeleton),
            0x08 => Some(Self::SemanticOverlay),
            0x09 => Some(Self::DeonticGovernance),
            0x0A => Some(Self::CausalFlux),
            _ => None,
        }
    }

    #[inline]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "Primal1Skeleton" | "Primal" => Some(Self::Primal1Skeleton),
            "PoincareDual" | "Dual" => Some(Self::PoincareDual),
            "BoundaryIncidence" | "Incidence" => Some(Self::BoundaryIncidence),
            "CombinatorialDartMap" | "DartMap" => Some(Self::CombinatorialDartMap),
            "SpatialHierarchy" | "Hierarchy" => Some(Self::SpatialHierarchy),
            "MetricProximity" | "Proximity" => Some(Self::MetricProximity),
            "TopologicalSkeleton" | "Skeleton" => Some(Self::TopologicalSkeleton),
            "SemanticOverlay" | "Semantic" => Some(Self::SemanticOverlay),
            "DeonticGovernance" | "Deontic" => Some(Self::DeonticGovernance),
            "CausalFlux" | "Causal" => Some(Self::CausalFlux),
            _ => None,
        }
    }
}

/// Fundamental graph morphology of an individual `HarbouredGraph`.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum GraphMorphology {
    /// Unconstrained directed multigraph (standard semantic triples / general wiring).
    #[default]
    DirectedMultigraph = 0x01,
    /// Strictly acyclic directed graph; enforced on edit/seal for proofs, calc DAGs, watersheds, and toolchains.
    DeterministicDAG = 0x02,
    /// RDF-Star / RDF 1.2 triple-terms (`TAG_EMBEDDED`) where edges are simultaneously vertices.
    HyperGraph = 0x03,
    /// Undirected symmetric graph (e.g. primal mesh 1-skeleton, dual face adjacency, c-kNN Laplacian).
    SymmetricGraph = 0x04,
    /// Bipartite / Factor / Incidence graph (e.g. Levi boundary graph C_k ↔ C_{k-1}, Petri net, REA flow).
    BipartiteFactor = 0x05,
    /// Involution / Permutation dart graph (Combinatorial Map β₁/β₂/β₃).
    InvolutionMap = 0x06,
}

impl GraphMorphology {
    #[inline]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0x01 => Some(Self::DirectedMultigraph),
            0x02 => Some(Self::DeterministicDAG),
            0x03 => Some(Self::HyperGraph),
            0x04 => Some(Self::SymmetricGraph),
            0x05 => Some(Self::BipartiteFactor),
            0x06 => Some(Self::InvolutionMap),
            _ => None,
        }
    }

    #[inline]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "DirectedMultigraph" | "Multigraph" => Some(Self::DirectedMultigraph),
            "DeterministicDAG" | "DAG" => Some(Self::DeterministicDAG),
            "HyperGraph" | "Hypergraph" => Some(Self::HyperGraph),
            "SymmetricGraph" | "Symmetric" | "SimplicialComplex" => Some(Self::SymmetricGraph),
            "BipartiteFactor" | "Bipartite" => Some(Self::BipartiteFactor),
            "InvolutionMap" | "ToroidalLattice" => Some(Self::InvolutionMap),
            _ => None,
        }
    }
}

/// Dimension / domain of the geometric element a graph vertex is anchored to.
#[repr(u8)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AnchorDomain {
    /// Pure semantic entity (`entity_hash` = 64-bit `q_hash`, `did:q42`, or `TAG_EMBEDDED` Virtual ID).
    #[default]
    SemanticQuin = 0xFF,
    /// 0-Cell: geometric vertex / point index in the host Geometry.
    Cell0Vertex = 0,
    /// 1-Cell: geometric edge / half-edge / dart / curve index in the host Geometry.
    Cell1Edge = 1,
    /// 2-Cell: geometric face / triangle / quad / DCEL polygon index in the host Geometry.
    Cell2Face = 2,
    /// 3-Cell: geometric volume / tetrahedron / hexahedron / CSG solid / octree leaf index.
    Cell3Volume = 3,
    /// Higher k-Simplex or sub-manifold chart index (k >= 4).
    CellKSimplex = 4,
}

impl AnchorDomain {
    #[inline]
    pub const fn from_u8(v: u8) -> Option<Self> {
        match v {
            0 => Some(Self::Cell0Vertex),
            1 => Some(Self::Cell1Edge),
            2 => Some(Self::Cell2Face),
            3 => Some(Self::Cell3Volume),
            4 => Some(Self::CellKSimplex),
            0xFF => Some(Self::SemanticQuin),
            _ => None,
        }
    }

    #[inline]
    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "Cell0Vertex" | "Vertex" | "0" => Some(Self::Cell0Vertex),
            "Cell1Edge" | "Edge" | "1" => Some(Self::Cell1Edge),
            "Cell2Face" | "Face" | "2" => Some(Self::Cell2Face),
            "Cell3Volume" | "Volume" | "3" => Some(Self::Cell3Volume),
            "CellKSimplex" | "Simplex" | "4" => Some(Self::CellKSimplex),
            "SemanticQuin" | "NQuin" | "Quin" => Some(Self::SemanticQuin),
            _ => None,
        }
    }
}

/// Structural, modal, and homological invariants enforced on a `HarbouredGraph` and its host Geometry.
#[repr(transparent)]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Hash, Pod, Zeroable, Serialize, Deserialize)]
pub struct TopologyInvariants(pub u16);

impl TopologyInvariants {
    pub const EMPTY: Self = Self(0);
    /// Reject cycle-forming edits in POET (`DeterministicDAG`).
    pub const ACYCLIC: Self = Self(1 << 0);
    /// Implicit reachability closure.
    pub const TRANSITIVE: Self = Self(1 << 1);
    /// Undirected / bidirectional edges (`u -> v` implies `v -> u`).
    pub const SYMMETRIC: Self = Self(1 << 2);
    /// Implicit self-loops (`u -> u`).
    pub const REFLEXIVE: Self = Self(1 << 3);
    /// Edges resolve to `.q42` `NQuin` IDs / RDF-Star `TAG_EMBEDDED` virtual IDs.
    pub const REIFIED_EDGES: Self = Self(1 << 4);
    /// Edge weights derived from `.10d` Euclidean / geodesic / Fisher-Rao distance.
    pub const SPATIAL_METRIC: Self = Self(1 << 5);
    /// Edges must advance monotonically along `Tensor10D.t` (`d_6`) and Lamport clock (`metadata[32..60]`).
    pub const TEMPORAL_MONOTONIC: Self = Self(1 << 6);
    /// Reject or gate edges carrying active `OP_FORBID` (`0x12`) without a valid `DEFEATER_BIT`.
    pub const DEONTIC_GUARDED: Self = Self(1 << 7);
    /// Contradictory `BelnapStatus::Both` (`0x32`) vertices/edges are isolated to quarantine sub-context.
    pub const PARACONSISTENT_ISOLATED: Self = Self(1 << 8);
    /// Closed manifold homology invariant (no unsealed boundary 1-cells: ∂² = 0).
    pub const MANIFOLD_CLOSED: Self = Self(1 << 9);
    /// Enforce non-increasing sensitivity class (`context[56..63]`) across directed edges (no classified leak).
    pub const SENSITIVITY_MONOTONIC: Self = Self(1 << 10);
    /// Enforce chain-complex boundary exactness (`∂_{k-1} ∘ ∂_k = 0`).
    pub const CHAIN_BOUNDARY_EXACT: Self = Self(1 << 11);

    #[inline]
    pub const fn contains(self, other: Self) -> bool { (self.0 & other.0) == other.0 }
    #[inline]
    pub const fn union(self, other: Self) -> Self { Self(self.0 | other.0) }
    #[inline]
    pub const fn bits(self) -> u16 { self.0 }
    #[inline]
    pub const fn from_bits_truncate(bits: u16) -> Self { Self(bits) }

    pub fn from_name(name: &str) -> Option<Self> {
        match name.trim() {
            "ACYCLIC" => Some(Self::ACYCLIC),
            "TRANSITIVE" => Some(Self::TRANSITIVE),
            "SYMMETRIC" => Some(Self::SYMMETRIC),
            "REFLEXIVE" => Some(Self::REFLEXIVE),
            "REIFIED_EDGES" => Some(Self::REIFIED_EDGES),
            "SPATIAL_METRIC" => Some(Self::SPATIAL_METRIC),
            "TEMPORAL_MONOTONIC" => Some(Self::TEMPORAL_MONOTONIC),
            "DEONTIC_GUARDED" => Some(Self::DEONTIC_GUARDED),
            "PARACONSISTENT_ISOLATED" => Some(Self::PARACONSISTENT_ISOLATED),
            "MANIFOLD_CLOSED" => Some(Self::MANIFOLD_CLOSED),
            "SENSITIVITY_MONOTONIC" => Some(Self::SENSITIVITY_MONOTONIC),
            "CHAIN_BOUNDARY_EXACT" | "BOUNDARY_EXACT" => Some(Self::CHAIN_BOUNDARY_EXACT),
            _ => None,
        }
    }
}

impl core::ops::BitOr for TopologyInvariants {
    type Output = Self;
    #[inline]
    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl core::ops::BitOrAssign for TopologyInvariants {
    #[inline]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

/// 16-byte cache-aligned anchor mapping a dense graph vertex `u in 0..V_g`
/// to both its host geometric k-cell AND its 64-bit `NQuin` semantic identity.
#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct CellAnchor {
    /// 64-bit semantic identity (`q_hash`, `did:q42`, or RDF-Star `TAG_EMBEDDED` virtual ID; 0 if pure geometric).
    pub entity_hash: u64,
    /// Index into the host Geometry's k-cell table (`vertices`, `half_edges`, `faces`, `mixed_cells`, `oct_nodes`).
    pub cell_index: u32,
    /// `AnchorDomain` discriminant (0=0-cell, 1=1-cell, 2=2-cell, 3=3-cell, 4=k-simplex, 0xFF=pure NQuin).
    pub anchor_domain: u8,
    /// Orientation / incidence sign (+1, -1, or 0) for DEC cochain / boundary coupling.
    pub orientation_sign: i8,
    /// Sub-layer / stratum / material group tag.
    pub material_or_stratum: u16,
}

const _: () = assert!(core::mem::size_of::<CellAnchor>() == 16);
const _: () = assert!(core::mem::align_of::<CellAnchor>() == 16);

impl CellAnchor {
    #[inline]
    pub const fn domain(&self) -> Option<AnchorDomain> {
        AnchorDomain::from_u8(self.anchor_domain)
    }
}

/// 64-byte cache-line aligned Host Geometry Header.
/// A single `PoetGeometryHeader` owns the cellular/metric space and harbours
/// `harboured_graph_count` (`M`) `HarbouredGraphHeader` entries at `graph_dir_offset`.
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct PoetGeometryHeader {
    pub geometry_id: u64,
    pub construct_schema_hash: u64,
    pub manifold_id: u32,
    pub cell_0_count: u32,
    pub cell_1_count: u32,
    pub cell_2_count: u32,
    pub cell_3_count: u32,
    pub geometry_payload_offset: u32,
    pub ddg_operators_offset: u32,
    pub graph_dir_offset: u32,
    pub harboured_graph_count: u16,
    pub active_graph_slot: u16,
    pub euler_characteristic: i32,
    pub complex_kind: u8,
    pub metric_kind: u8,
    pub ambient_dim: u8,
    pub intrinsic_dim: u8,
    pub crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<PoetGeometryHeader>() == 64);
const _: () = assert!(core::mem::align_of::<PoetGeometryHeader>() == 64);

impl Default for PoetGeometryHeader {
    fn default() -> Self { Self::zeroed() }
}

impl PoetGeometryHeader {
    #[inline]
    pub const fn complex_kind_enum(&self) -> Option<GeometryComplexKind> { GeometryComplexKind::from_u8(self.complex_kind) }
    #[inline]
    pub const fn metric_kind_enum(&self) -> Option<GeometryMetricKind> { GeometryMetricKind::from_u8(self.metric_kind) }
    #[inline]
    pub const fn computed_euler_characteristic(&self) -> i32 {
        (self.cell_0_count as i32).wrapping_sub(self.cell_1_count as i32)
            .wrapping_add(self.cell_2_count as i32).wrapping_sub(self.cell_3_count as i32)
    }
}

/// 64-byte cache-line aligned descriptor for one Graph harboured inside a `PoetGeometryHeader`.
#[repr(C, align(64))]
#[derive(Copy, Clone, Debug, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize)]
pub struct HarbouredGraphHeader {
    pub graph_id: u64,
    pub parent_geometry_id: u64,
    pub anchor_table_offset: u32,
    pub csr_row_ptr_offset: u32,
    pub csr_col_idx_offset: u32,
    pub edge_anchor_offset: u32,
    pub csc_col_ptr_offset: u32,
    pub csc_row_idx_offset: u32,
    pub edge_weight_offset: u32,
    pub vertex_count: u32,
    pub edge_count: u32,
    pub context_and_state_mask: u32,
    pub embed_role: u8,
    pub morphology: u8,
    pub invariants: TopologyInvariants,
    pub crc32c: u32,
}

const _: () = assert!(core::mem::size_of::<HarbouredGraphHeader>() == 64);
const _: () = assert!(core::mem::align_of::<HarbouredGraphHeader>() == 64);

impl Default for HarbouredGraphHeader {
    fn default() -> Self {
        Self::zeroed()
    }
}

impl HarbouredGraphHeader {
    #[inline]
    pub const fn embed_role_enum(&self) -> Option<GraphEmbedRole> {
        GraphEmbedRole::from_u8(self.embed_role)
    }

    #[inline]
    pub const fn morphology_enum(&self) -> Option<GraphMorphology> {
        GraphMorphology::from_u8(self.morphology)
    }

    #[inline]
    pub const fn max_sensitivity_class(&self) -> u8 {
        (self.context_and_state_mask & 0xFF) as u8
    }

    #[inline]
    pub const fn max_depth(&self) -> u8 {
        ((self.context_and_state_mask >> 24) & 0xFF) as u8
    }
}

/// Deterministic error enumeration for Tier-1 and Tier-2 Topology operations.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TopologyError {
    BufferTooSmall,
    UnalignedBuffer,
    InvalidHeaderSize,
    InvalidComplexKind(u8),
    InvalidMorphology(u8),
    InvalidEmbedRole(u8),
    GraphSlotOutOfRange { slot: u16, count: u16 },
    VertexOutOfRange { vertex: u32, count: u32 },
    CellOutOfRange { domain: u8, index: u32, count: u32 },
    NonMonotonicRowPtr { vertex: u32 },
    ChecksumMismatch { expected: u32, actual: u32 },
    DeltaRingFull { capacity: usize },
    CycleDetected { source: u32, target: u32 },
    TemporalMonotonicityViolation { source: u32, target: u32 },
    DeonticForbidViolation { edge_anchor_idx: u32 },
    ParaconsistentContagion { edge_anchor_idx: u32 },
    SensitivityEscalation { source_class: u8, target_class: u8 },
    ChainBoundaryNotZero { non_zero_terms: usize },
    BudgetExhausted { requested: usize, budget: usize },
}

impl core::fmt::Display for TopologyError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl std::error::Error for TopologyError {}
