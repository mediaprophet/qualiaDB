//! P9.4 — qapp/MCP capability manifests: per-op resource limits and
//! backend descriptors (scalar / SIMD / wgpu / CUDA / exact-fallback).
//!
//! ## Design
//!
//! Every registered computational-geometry op exposes a manifest with:
//! - **Backends**: non-empty list of execution backends.
//! - **Determinism class**: `bit-exact` (identical bits across runs) or
//!   `tolerance` (within a stated tolerance).
//! - **Resource limits**: max input size, max output size, max memory.
//! - **GPU fallback**: any op advertising `wgpu` or `cuda` MUST also
//!   advertise a deterministic `cpu` or `wasm` fallback — never GPU-only
//!   for robust topology.
//!
//! ## Reserve-mode budget query
//!
//! A Reserve-mode budget query returns only the backends runnable on the
//! current device (e.g. if no GPU adapter is available, wgpu/CUDA are
//! filtered out).

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

// ───────────────────────────────────────────────────────────────────────────
//  Types
// ───────────────────────────────────────────────────────────────────────────

/// Execution backend identifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Backend {
    /// Scalar CPU kernel (always available).
    Scalar,
    /// SIMD-optimised CPU kernel.
    Simd,
    /// WebGPU compute shader.
    Wgpu,
    /// CUDA kernel.
    Cuda,
    /// WASM kernel (browser/edge).
    Wasm,
    /// Exact-arithmetic fallback (robust predicates).
    Exact,
}

impl Backend {
    /// Returns `true` if this backend requires a GPU adapter.
    pub fn requires_gpu(&self) -> bool {
        matches!(self, Backend::Wgpu | Backend::Cuda)
    }

    /// Returns `true` if this backend is a deterministic fallback
    /// (CPU or WASM or exact).
    pub fn is_deterministic_fallback(&self) -> bool {
        matches!(
            self,
            Backend::Scalar | Backend::Simd | Backend::Wasm | Backend::Exact
        )
    }
}

/// Determinism class for an op.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeterminismClass {
    /// Bit-identical output for identical input across runs and platforms.
    BitExact,
    /// Output within a stated tolerance (e.g. GPU floating-point).
    Tolerance,
}

// ───────────────────────────────────────────────────────────────────────────
//  P10.1 — Capability truth fields
//
//  The execution plan (P10.1) requires every exported geometry operation to
//  carry a machine-readable maturity, exactness, allocation and dimensionality
//  declaration, so the status surface matches executable reality rather than
//  aspirational prose. These four enums are that declaration. P10.2 splits
//  decision exactness from construction exactness via `ExactnessClass`;
//  P10.3 publishes the hot/cold allocation taxonomy via `AllocationClass`.
// ───────────────────────────────────────────────────────────────────────────

/// Maturity level — mirrors the execution plan's status vocabulary exactly.
///
/// A manifest's `maturity` MUST agree with the corresponding row in
/// `native-computational-geometry-EXECUTION.md`. The audit test
/// (`manifest_maturity_matches_plan_status`) enforces this for the ops that
/// map to a named plan task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Maturity {
    /// Named and scoped in the design doc, but no code exists yet.
    Planned,
    /// A real, compiling, in-tree first slice the rest depends on — the
    /// barriered thing others build on, but NOT the complete capability.
    Foundation,
    /// Fully written in code, compiles green, own `#[cfg(test)]` passing —
    /// but has NOT yet cleared the geometry acceptance gates (golden vectors,
    /// degeneracy, CPU/GPU differential, determinism).
    Implemented,
    /// Implemented AND cleared every applicable acceptance gate: golden
    /// vectors pass (degenerate + near-degenerate included), CPU scalar
    /// oracle matches, CPU/GPU differential is bit-clean, Naga-validated if
    /// it has a shader, determinism/canonical-bytes hold.
    Verified,
    /// Verified AND integrated into every target surface the task promised
    /// (Rust + WASM parity + MCP/qapp route + renderer SDK), with the dated
    /// §9 progress-log entry written and the `NOTICES.md` RELEASE posted.
    Done,
}

/// Exactness class — splits decision exactness from construction exactness (P10.2).
///
/// A boolean op with f64 intersections cannot advertise `ExactConstruction`;
/// it is `ApproximateMetric` (or `TopologyGuaranteed` only if the topology is
/// proven independent of the coordinate error). An orientation predicate is
/// `ExactPredicate` (a sign decision, no constructed point). A segment
/// intersection that returns a re-predicable exact point is
/// `ExactConstruction`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ExactnessClass {
    /// Exact predicate sign (orientation / incircle / insphere) — the
    /// decision is exact; no coordinate is constructed.
    ExactPredicate,
    /// Exact constructed coordinates (intersection points) that survive
    /// re-predication without sign drift.
    ExactConstruction,
    /// Approximate floating-point metric output (distances, areas, volumes)
    /// within a stated tolerance — NOT a topology decision.
    ApproximateMetric,
    /// Topology guarantee (manifold / orientation / watertight) proven
    /// independent of coordinate exactness.
    TopologyGuaranteed,
    /// No exactness claim — structural / authoring / serialization op (mesh
    /// generation, section encoding, hashing).
    Structural,
}

/// Allocation class — hot/cold path taxonomy (P10.3).
///
/// `HotZeroHeap` ops must not allocate on the predicate/evaluation loop (no
/// `Vec` / `String` / `Box` — AGENTS.md §0). `ColdBounded` ops may allocate
/// bounded heap during construction/build but must not be called from
/// evaluator loops. `TestTooling` is unconstrained (test harnesses, JSON
/// tooling).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AllocationClass {
    /// Hot path: zero heap allocations on the predicate/evaluation loop.
    HotZeroHeap,
    /// Cold path: bounded heap during construction/build; not called from
    /// evaluator loops.
    ColdBounded,
    /// Test/tooling path: allocation unconstrained (test harnesses, JSON
    /// tooling, manifest queries).
    TestTooling,
}

/// Dimensionality of the op's geometric domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Dimensionality {
    /// 2-D planar.
    D2,
    /// 3-D spatial.
    D3,
    /// N-D (dimension-generic; works in 2-D, 3-D, and higher).
    DN,
    /// 10-D Tensor10D (the full `[q,v,w,x,y,z,t,α,μ,σ]` axis set).
    D10,
    /// Dimension-independent (serialization, encoding, section I/O).
    DimensionIndependent,
}

/// Resource limits for an op invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Maximum number of input points.
    pub max_input_points: u32,
    /// Maximum output size (bytes).
    pub max_output_bytes: u32,
    /// Maximum memory (bytes) the op may allocate on the hot path.
    pub max_memory_bytes: u32,
    /// Maximum wall-clock time in microseconds (0 = unbounded).
    pub max_time_us: u32,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_input_points: 1_000_000,
            max_output_bytes: 256 * 1024 * 1024,
            max_memory_bytes: 512 * 1024 * 1024,
            max_time_us: 0,
        }
    }
}

/// Capability manifest for a single op.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OpManifest {
    /// Op name (matches the `op` field in `execute_geometry_tool_json`).
    pub op: &'static str,
    /// Human-readable description.
    pub description: &'static str,
    /// Available backends, in preference order.
    pub backends: &'static [Backend],
    /// Determinism class.
    pub determinism: DeterminismClass,
    /// Resource limits.
    pub limits: ResourceLimits,
    /// Whether this op is topology-critical (requires exact fallback).
    pub topology_critical: bool,
    /// P10.1 — maturity level. MUST agree with the execution-plan row for
    /// this op. The audit test enforces this for ops mapping to a named task.
    pub maturity: Maturity,
    /// P10.2 — exactness class. Splits decision exactness (predicate sign)
    /// from construction exactness (re-predicable coordinates) from
    /// approximate metric output.
    pub exactness: ExactnessClass,
    /// P10.3 — allocation class. `HotZeroHeap` ops must not allocate on the
    /// evaluation loop; `ColdBounded` ops may allocate during build.
    pub allocation: AllocationClass,
    /// P10.1 — dimensionality of the op's geometric domain.
    pub dimensionality: Dimensionality,
}

// ───────────────────────────────────────────────────────────────────────────
//  Manifest registry
// ───────────────────────────────────────────────────────────────────────────

/// The canonical capability manifest table for computational-geometry ops.
///
/// **P10.1 claim-to-code audit.** Every entry's `maturity` MUST agree with
/// the corresponding row in `native-computational-geometry-EXECUTION.md`. As
/// of 2026-07-05 the CG library is at `Implemented` across P0–P9 (own
/// `#[cfg(test)]` green; 752-test suite green) but NOT `Verified` (CC0
/// golden-vector corpus not fetched; GPU-on-adapter differentials not run;
/// `mcp_server --lib` gate cleared but full `done` integration pending). So
/// every current entry honestly carries `Maturity::Implemented`. When a task
/// clears its acceptance gates, raise its row here AND in the execution plan
/// in the same change.
pub const GEOMETRY_OP_MANIFESTS: &[OpManifest] = &[
    // ── P1 — Predicates (exact sign decisions; hot zero-heap) ──
    OpManifest {
        op: "orientation_2",
        description: "Robust 2-D orientation predicate (filtered → exact fallback).",
        backends: &[Backend::Scalar, Backend::Exact, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 3,
            max_output_bytes: 64,
            max_memory_bytes: 1024,
            max_time_us: 100,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::HotZeroHeap,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "orient_3d",
        description: "Robust 3-D orientation predicate (filtered → exact fallback).",
        backends: &[Backend::Scalar, Backend::Exact, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 4,
            max_output_bytes: 64,
            max_memory_bytes: 1024,
            max_time_us: 100,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::HotZeroHeap,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "incircle",
        description: "2-D in-circle predicate (filtered → exact fallback).",
        backends: &[Backend::Scalar, Backend::Exact, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 4,
            max_output_bytes: 64,
            max_memory_bytes: 1024,
            max_time_us: 100,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::HotZeroHeap,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "insphere",
        description: "3-D in-sphere predicate (filtered → exact fallback).",
        backends: &[Backend::Scalar, Backend::Exact, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 5,
            max_output_bytes: 64,
            max_memory_bytes: 1024,
            max_time_us: 100,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::HotZeroHeap,
        dimensionality: Dimensionality::D3,
    },
    // ── P1.7 — Exact construction (re-predicable coordinates) ──
    OpManifest {
        op: "construct_segment_intersection_2",
        description: "Exact 2-D segment-segment intersection point (re-predicable).",
        backends: &[Backend::Scalar, Backend::Exact, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 4,
            max_output_bytes: 128,
            max_memory_bytes: 4096,
            max_time_us: 100,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactConstruction,
        allocation: AllocationClass::HotZeroHeap,
        dimensionality: Dimensionality::D2,
    },
    // ── P2 — Hulls (predicate-driven; cold build) ──
    OpManifest {
        op: "convex_hull_2",
        description: "2-D convex hull via monotone chain with exact predicate fallback.",
        backends: &[Backend::Scalar, Backend::Simd, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 8 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "convex_hull_3",
        description: "3-D convex hull (incremental; rejects all-coplanar input — see P10 audit).",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P3 — Delaunay / Voronoi (predicate-driven; cold build) ──
    OpManifest {
        op: "delaunay_2",
        description: "2-D Delaunay triangulation with incircle exact predicate.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "voronoi_2",
        description: "Voronoi diagram as Delaunay dual (connectivity exact from Delaunay; vertex coords are f64 circumcenters — approximate).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "delaunay_3",
        description: "3-D Delaunay tetrahedralization (Bowyer-Watson; empty-circumsphere).",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "conforming_delaunay_2",
        description: "Constrained 2-D Delaunay with edge constraints (approximate collinearity — P13 supplies real refinement).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P3 — Spatial index (BVH / kd-tree; cold build, hot query) ──
    OpManifest {
        op: "bvh_3d",
        description: "3-D BVH construction + closest/overlap query (cold build, hot query).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "kd_tree_3d",
        description: "3-D kd-tree construction + nearest/radius query.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "box_join",
        description: "BVH-driven box join (candidate pair enumeration).",
        backends: &[Backend::Scalar, Backend::Wgpu, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "spatial_order",
        description: "Morton/Hilbert curve encoding for spatial sorting (2-D/3-D).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::HotZeroHeap,
        dimensionality: Dimensionality::DN,
    },
    OpManifest {
        op: "nearest_site",
        description: "Brute-force nearest-site query.",
        backends: &[Backend::Scalar, Backend::Simd, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P2.8 / P3.7 — Topology + spatial-index .10d sections ──
    OpManifest {
        op: "triangle_topology",
        description: "Build half-edge topology from triangle list.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::DN,
    },
    OpManifest {
        op: "mesh_topology",
        description: "Encode mesh topology to .10d section with Euler characteristic.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::DimensionIndependent,
    },
    // ── P4 — 2-D booleans / Minkowski (approximate metric; f64 intersections) ──
    OpManifest {
        op: "boolean_2",
        description: "2-D polygon boolean ops (union/intersection/difference) — area-based, f64.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "minkowski_2",
        description: "2-D Minkowski sum/difference: O(n+m) edge-merge for convex polygons, non-convex decomposition + pairwise convex sum for general polygons, brute-force hull fallback.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P5 — 3-D mesh processing (approximate; f64 intersections) ──
    OpManifest {
        op: "tri_tri_intersect_3",
        description: "3-D triangle-triangle intersection classification.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 6,
            max_output_bytes: 256,
            max_memory_bytes: 4096,
            max_time_us: 100,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::HotZeroHeap,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "boolean_3",
        description: "3-D mesh boolean (union/intersection/difference) — approximate, f64 intersections (P12 supplies exact CSG).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "decimate_qem",
        description: "3-D mesh decimation via quadric error metrics (LOD generation).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 500_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "isotropic_remesh",
        description: "Isotropic remeshing (edge-length targeting; manifold + orientation preserved).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 500_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "exact_construct_3",
        description: "Exact 3-D construction (segment-plane/triangle intersection; re-predicable).",
        backends: &[Backend::Scalar, Backend::Exact, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 8,
            max_output_bytes: 256,
            max_memory_bytes: 8192,
            max_time_us: 100,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactConstruction,
        allocation: AllocationClass::HotZeroHeap,
        dimensionality: Dimensionality::D3,
    },
    // ── P6 — Reconstruction / meshing / TDA ──
    OpManifest {
        op: "alpha_shape_2d",
        description: "2-D alpha shapes (simplex classification over a range of α).",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "alpha_shape_3d",
        description: "3-D alpha shapes + alpha-wrap surface extraction (watertight).",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "marching_cubes",
        description: "Marching-cubes isosurface extraction (canonical 256-case Lorensen–Cline table; 2-manifold).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::TopologyGuaranteed,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "poisson_reconstruct_3d",
        description: "Oriented-point SDF reconstruction (nearest-normal signed distance + marching cubes; NOT real Poisson — P13 supplies real screened-Poisson).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "point_set_3d",
        description: "3-D point-set processing: kNN/CkNN neighbourhood, average spacing, outlier removal, local density.",
        backends: &[Backend::Scalar, Backend::Simd, Backend::Wgpu, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 500_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P8 — Spectral / TDA / statistical manifold (10-D Tensor10D) ──
    OpManifest {
        op: "vr_filtration",
        description: "Vietoris-Rips filtration over Tensor10D point cloud.",
        backends: &[Backend::Scalar, Backend::Wgpu, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 10_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D10,
    },
    OpManifest {
        op: "persistence",
        description: "Persistent homology barcode (H0/H1) from filtered complex.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::TopologyGuaranteed,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D10,
    },
    OpManifest {
        op: "alpha_filtration_2d",
        description: "2-D alpha filtration for persistent homology (TDA).",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "cknn_laplacian",
        description: "CkNN graph Laplacian and local density estimation.",
        backends: &[Backend::Scalar, Backend::Simd, Backend::Wgpu, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D10,
    },
    OpManifest {
        op: "cknn_laplacian_3d",
        description: "3-D CkNN graph Laplacian (Laplace-Beltrami discrete approximation).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "nn_query",
        description: "Nearest-neighbour inference query (radius + kNN).",
        backends: &[Backend::Scalar, Backend::Simd, Backend::Wgpu, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 8 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D10,
    },
    OpManifest {
        op: "gpu_oracle",
        description: "GPU acceleration + CPU oracle for P8 batch distances/density/circumradius.",
        backends: &[Backend::Scalar, Backend::Wgpu, Backend::Wasm],
        determinism: DeterminismClass::Tolerance,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D10,
    },
    OpManifest {
        op: "natural_neighbour",
        description: "Natural-neighbour interpolation (Laplace coordinates; linear precision verified).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 4 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P9.5 — Authoring ops (primitive generation; structural) ──
    OpManifest {
        op: "create_box",
        description: "Generate a box mesh with custom dimensions.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 0,
            max_output_bytes: 1024,
            max_memory_bytes: 4096,
            max_time_us: 100,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "create_sphere",
        description: "Generate a UV sphere mesh.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 0,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 32 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "create_cylinder",
        description: "Generate a cylinder mesh.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 0,
            max_output_bytes: 4 * 1024 * 1024,
            max_memory_bytes: 8 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "create_plane",
        description: "Generate a plane mesh in the XZ plane.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 0,
            max_output_bytes: 256,
            max_memory_bytes: 1024,
            max_time_us: 100,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "create_torus",
        description: "Generate a torus mesh.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 0,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 32 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "create_grid",
        description: "Generate a subdivided grid mesh.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 0,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 32 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P9.5 — Authoring: mesh booleans + edits (tool-dispatched; 3-D) ──
    OpManifest {
        op: "boolean_union",
        description: "Authoring 3-D mesh union (approximate; f64 intersections — P12 supplies exact CSG).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "boolean_intersect",
        description: "Authoring 3-D mesh intersection (approximate; f64 intersections — P12 supplies exact CSG).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "boolean_difference",
        description: "Authoring 3-D mesh difference (approximate; f64 intersections — P12 supplies exact CSG).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    OpManifest {
        op: "drag_vertex",
        description: "Authoring edit: drag a mesh vertex with prior-τ consent gate.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::Structural,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P11.9 — Half-plane intersection + 2-D randomized LP ──
    OpManifest {
        op: "half_plane_intersection",
        description: "2-D half-plane intersection (sort-and-intersect + deque); bounded/empty/unbounded.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "linear_program_2d",
        description: "2-D LP (Seidel randomized incremental); seeded determinism + feasible/infeasible/unbounded certificates.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.13 — Rotating calipers + smallest enclosing disk ──
    OpManifest {
        op: "rotating_calipers",
        description: "Rotating calipers: diameter (farthest pair), width (min-width), antipodal pairs over a CCW convex polygon.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    OpManifest {
        op: "smallest_enclosing_disk",
        description: "Smallest enclosing disk (Welzl randomized incremental); seeded determinism + support set certificate.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.5 — Monotone partition + triangulation ──
    OpManifest {
        op: "triangulate_polygon",
        description: "Simple-polygon triangulation via monotone partition + linear monotone triangulation, with ear-clipping fallback.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.6 — Point location in planar subdivisions ──
    OpManifest {
        op: "point_location",
        description: "Point location in planar subdivisions: walking location in triangulations (O(√n) expected) and slab decomposition (O(log n) query, O(n log n) preprocessing).",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.6 gap — Trapezoidal map with randomized incremental point location ──
    OpManifest {
        op: "trapezoidal_map",
        description: "Trapezoidal map: randomized incremental construction with search DAG for O(log n) expected point location. Seeded-deterministic insertion order, non-crossing segment input, bounding-box-clipped trapezoids.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.7 — Kirkpatrick hierarchy for O(log n) point location ──
    OpManifest {
        op: "kirkpatrick_hierarchy",
        description: "Kirkpatrick hierarchy: guaranteed O(log n) point location in triangulated planar subdivisions via independent-set vertex removal and retriangulation. Bounding triangle, multi-level refinement, deterministic.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.12 — Simplex/halfspace range reporting ──
    OpManifest {
        op: "range_reporting",
        description: "Simplex and halfspace range reporting: kd-tree (O(√n+k) expected), simplicial partition tree (O(n^{1+ε}+k)), and cutting tree (grid-based, O(log n+k) for halfspace). Cross-validated against brute-force oracle.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.14 — Ham-sandwich cuts, centrepoints, width coresets ──
    OpManifest {
        op: "ham_sandwich_centrepoint",
        description: "Ham-sandwich cuts (simultaneous bisection of two point sets), centrepoints (Tukey depth ≥ n/3 via arrangement search), and directional-width coresets (Dudley construction, O(1/ε) points).",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 10_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P12.1 — N-ary CSG on 2D polygons + 2D mesh co-refinement ──
    OpManifest {
        op: "nary_csg_2d",
        description: "N-ary CSG operations (union/intersection/difference/symmetric-difference) on N 2D polygons via chained DCEL overlays. 2D mesh co-refinement: split two triangle meshes at edge-edge intersections for compatible boundaries.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P12.2 — Exact 2D arrangement with exact construction ──
    OpManifest {
        op: "exact_arrangement_2d",
        description: "Exact 2D line arrangement using exact-construction intersection points (ExactPoint2 from exact_kernel). Vertices carry exact rational coordinates; orientation predicates cross-multiply to eliminate division. Zone traversal with exact predicates. Euler V-E+F=2 verified.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 10_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactConstruction,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P12.3 — Exact 3D mesh co-refinement ──
    OpManifest {
        op: "corefine_3d",
        description: "3D mesh co-refinement: split two triangle meshes along their intersection curves for compatible boundaries. BVH-accelerated broad phase (P12.5) reduces candidate pairs from O(nm) to O(n log m + k) with no false negatives vs brute-force oracle. Uses tri_tri_intersect_3_exact for exact-rational intersection points, exact expansion comparison for point dedup, and exact orient_3d for coplanarity tests. Workspace bounded by 42-MiB Sentinel ceiling.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactConstruction,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P12.4 — Exact 3D boolean operations ──
    OpManifest {
        op: "boolean_3_exact",
        description: "3-D mesh boolean (union/intersection/difference) with exact-rational intersection points. Uses tri_tri_intersect_3_exact for exact construction of split geometry and orient_3d-based ray-triangle intersection for classification. Upgrades boolean_3 from ApproximateMetric to ExactConstruction.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactConstruction,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P12.2 — Simulation of Simplicity for orient_3d ──
    OpManifest {
        op: "orient_3d_sos",
        description: "3-D orientation predicate with Simulation of Simplicity (Edelsbrunner-Mücke 1990). When orient_3d returns Zero (coplanar), evaluates 12 first-order cofactors (2D orientations in coordinate planes) in order of increasing symbolic-perturbation power. Never returns Zero. Deterministic, zero-heap.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 4,
            max_output_bytes: 16,
            max_memory_bytes: 1,
            max_time_us: 100,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::HotZeroHeap,
        dimensionality: Dimensionality::D3,
    },
    // ── P12.4 — Per-facet exact constrained Delaunay re-triangulation ──
    OpManifest {
        op: "cdt_retriangulate_facet",
        description: "Per-facet constrained Delaunay re-triangulation: given a 3-D triangle and intersection constraint segments on its surface, projects to 2-D (dropping the dominant normal axis), runs conforming Delaunay with exact incircle predicate, and maps back to 3-D. Every constraint edge is present in the output; no output triangle crosses a constraint.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100,
            max_output_bytes: 1024 * 1024,
            max_memory_bytes: 4 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P12.6 — Radial sort and Weiler 3-D arrangement model ──
    OpManifest {
        op: "build_arrangement_3d",
        description: "Weiler 3-D arrangement model: radial-sorts facets around non-manifold edges by outward-normal angle, identifies shells (connected closed surface components) and volumetric regions. Validates incidence/involution invariants: every edge has ≥1 facet, every triangle's edges are present, radial ordering matches input facets.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P12.7 — Arbitrary n-ary boolean-expression evaluator ──
    OpManifest {
        op: "nary_boolean",
        description: "N-ary boolean-expression evaluator: supports union, intersection, difference, xor, and complement over expression trees with 2+ mesh operands. Uses pairwise reduction of binary boolean_3 operations. Region classification via bitmask (u64, up to 64 operands). Deterministic bottom-up evaluation.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactConstruction,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P12.8 — Coplanar-region simplification and snap rounding ──
    OpManifest {
        op: "simplify_coplanar_regions",
        description: "Coplanar-region simplification: merges adjacent coplanar triangles with matching region labels into larger polygonal regions, then re-triangulates each region via fan triangulation. Uses exact orient_3d for coplanarity testing. Region labels preserved. Deterministic flood-fill with sorted traversal.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P12.8b — Snap rounding ──
    OpManifest {
        op: "snap_round_3d",
        description: "Topology-preserving snap rounding: rounds 3D coordinates to a grid with spacing epsilon. Verifies no triangle becomes degenerate after rounding. Fails closed if topology would change.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P12.9 — CSG/arrangement .10d sections and repair ──
    OpManifest {
        op: "encode_csg_section",
        description: "Serialize CSG expression tree, exact-point pool, region labels, and output mesh into canonical .10d section with per-section CRC-32C. Round-trip verified: encode → decode is bit-identical. Expression tree uses tagged byte encoding for Operand/Union/Intersection/Difference/Xor/Complement nodes.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactConstruction,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P12.9b — Mesh repair ──
    OpManifest {
        op: "repair_mesh",
        description: "Mesh repair: removes degenerate triangles, detects and removes duplicate sheets (identical triangles with opposite winding), reports non-manifold edges, extracts shells (connected components). Returns cleaned mesh and repair report.",
        backends: &[Backend::Scalar, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── Convex decomposition (Hertel-Mehlhorn + triangulation-only) ──
    OpManifest {
        op: "convex_decomposition",
        description: "Convex decomposition of simple polygons: Hertel-Mehlhorn (triangulate + merge convex pairs, ≤4× optimal) and triangulation-only fallback.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 16 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.3 — DCEL subdivision, overlay, and polygon-set booleans ──
    OpManifest {
        op: "dcel_overlay",
        description: "DCEL planar overlay of two simple polygons: split edges at intersections, label faces by (in_a, in_b), extract union/intersection/difference/xor boundary cycles with holes; Euler and area identities.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.8 — Arrangements, point-line duality, topological sweep ──
    OpManifest {
        op: "line_arrangement",
        description: "Line arrangement construction (all pairwise intersections → V/E/F subdivision with bbox clipping), zone traversal (face sequence crossed by a query line), and point-line duality (round-trip + incidence preservation).",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 10_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P11.10 — Interval, segment, priority-search and range trees ──
    OpManifest {
        op: "range_trees",
        description: "Interval tree (stabbing), segment tree (stabbing), priority search tree (x-range + y-bounded), 1-D and 2-D range trees (orthogonal range reporting/counting). All results match linear-scan oracles; caller buffers report exact capacity needs.",
        backends: &[Backend::Scalar, Backend::Wasm, Backend::Exact],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ExactPredicate,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P13.1 — Mesh quality metrics and size/anisotropy fields ──
    OpManifest {
        op: "mesh_quality",
        description: "Per-element triangle/tet quality (min/max angle, radius-edge, aspect, dihedral, scaled Jacobian, signed volume) and isotropic size / anisotropic metric-tensor fields with field-conformance checking. Inverted/degenerate cells fail closed.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 64 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P13.2 — Delaunay refinement for PSLGs with Steiner points ──
    OpManifest {
        op: "delaunay_refine_2",
        description: "Ruppert Delaunay refinement of a 2-D PSLG: segment encroachment recovery + bad-triangle circumcenter insertion. Terminates for min-angle <= 20.7 deg; meets declared angle/size targets.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 128 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::TopologyGuaranteed,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P13.3 — Optimal fixed-vertex triangulation objectives ──
    OpManifest {
        op: "triangulation_optimise",
        description: "Edge-flip hill-climbing to optimise a fixed-vertex triangulation for a declared objective (max-min angle, min-max angle, min-max edge ratio, min-max radius-edge, max-min area, min-max aspect). Convex-quad flip validity; deterministic tie-breaking.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::ApproximateMetric,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P13.4 — Quadtree balanced meshing (2-D) ──
    OpManifest {
        op: "quadtree_balanced_mesh_2d",
        description: "Size-field-driven quadtree decomposition with 2:1 balance and conforming triangulation. Hanging edge-midpoints resolved by per-cell centre-fan templates; shared vertices deduplicated on the finest-level grid. Identical input → bit-identical mesh.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 128 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::TopologyGuaranteed,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P13.4 — Octree balanced meshing (3-D) ──
    OpManifest {
        op: "octree_balanced_mesh_3d",
        description: "Size-field-driven octree decomposition with 2:1 balance, leaf hexahedron extraction (deduplicated vertices), and 6-tet-per-hex Freudenthal split. Hex mesh preserves hanging nodes (conforming product); tet split is conforming for uniform meshes and documented as non-conforming across T-junction faces for graded meshes.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 256 * 1024 * 1024,
            max_memory_bytes: 512 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::TopologyGuaranteed,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P13.5 — Advancing-front surface meshing (2-D) ──
    OpManifest {
        op: "advancing_front_surface_2d",
        description: "Advancing-front 2-D triangulation from a closed CCW boundary polyline and a target-size function. Shortest-edge-first deterministic ordering; equilateral-vertex placement with snap-to-existing; self-crossing guard rejects any candidate whose new edges properly intersect a non-adjacent front edge; front collapses on exhaustion guaranteeing monotone shrinkage. Identical input → bit-identical mesh.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 128 * 1024 * 1024,
            max_memory_bytes: 256 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::TopologyGuaranteed,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D2,
    },
    // ── P13.5 — Advancing-front volume meshing (3-D) ──
    OpManifest {
        op: "advancing_front_volume_3d",
        description: "Advancing-front 3-D tetrahedralisation from a closed orientable surface mesh and a target-size function. Smallest-area-face-first deterministic ordering; inradius-offset apex placement with snap-to-existing; self-crossing guard rejects any candidate whose new edges properly intersect a non-adjacent front face (Möller–Trumbore); front collapses on exhaustion guaranteeing monotone shrinkage. Identical input → bit-identical mesh.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 256 * 1024 * 1024,
            max_memory_bytes: 512 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::TopologyGuaranteed,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
    // ── P13.7 — Tetrahedral quality improvement & sliver handling ──
    OpManifest {
        op: "tet_quality_improve",
        description: "Tetrahedral mesh quality improvement via four monotonic passes (2-3/3-2 flip, optimisation-based smooth, Delaunay cavity Steiner insertion, sliver exudation by local perturbation). Boundary faces and fixed vertices are never moved; every accepted operation strictly increases the local worst-case score under the selected objective (min dihedral / radius-edge ratio / scaled Jacobian) and preserves positive tet orientation. Identical input -> bit-identical mesh.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 1_000_000,
            max_output_bytes: 256 * 1024 * 1024,
            max_memory_bytes: 512 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: true,
        maturity: Maturity::Implemented,
        exactness: ExactnessClass::TopologyGuaranteed,
        allocation: AllocationClass::ColdBounded,
        dimensionality: Dimensionality::D3,
    },
];

// ───────────────────────────────────────────────────────────────────────────
//  Validation
// ───────────────────────────────────────────────────────────────────────────

/// Validate that every manifest has non-empty backends and that any op
/// advertising a GPU backend also has a deterministic fallback.
///
/// **P10.1** also checks the new capability-truth fields: `maturity` must not
/// be `Planned` for an op that is in the manifest table (a planned op has no
/// code; it should not be advertised), and `topology_critical` ops must not
/// claim `ApproximateMetric` exactness (a topology-critical op whose only
/// exactness is an approximate metric is a misrepresentation — it needs an
/// exact predicate or a topology guarantee).
pub fn validate_manifests(manifests: &[OpManifest]) -> Result<(), String> {
    for m in manifests {
        if m.backends.is_empty() {
            return Err(format!("{}: backends must be non-empty", m.op));
        }

        let has_gpu = m.backends.iter().any(|b| b.requires_gpu());
        let has_fallback = m.backends.iter().any(|b| b.is_deterministic_fallback());

        if has_gpu && !has_fallback {
            return Err(format!(
                "{}: advertises GPU backend without deterministic CPU/WASM fallback",
                m.op
            ));
        }

        if m.limits.max_output_bytes == 0 {
            return Err(format!("{}: max_output_bytes must be finite", m.op));
        }

        if m.limits.max_memory_bytes == 0 {
            return Err(format!("{}: max_memory_bytes must be finite", m.op));
        }

        // P10.1 — a manifest entry is a claim that the op exists in code.
        if matches!(m.maturity, Maturity::Planned) {
            return Err(format!(
                "{}: maturity is Planned but it appears in GEOMETRY_OP_MANIFESTS — planned ops have no code and must not be advertised",
                m.op
            ));
        }

        // P10.1 — topology-critical ops must not claim only approximate metric
        // exactness (that would be claiming a topology decision from an
        // approximate quantity — the kind of overstatement P10 exists to catch).
        if m.topology_critical && matches!(m.exactness, ExactnessClass::ApproximateMetric) {
            return Err(format!(
                "{}: topology_critical but exactness is ApproximateMetric — topology decisions require an exact predicate or topology guarantee",
                m.op
            ));
        }
    }
    Ok(())
}

// ───────────────────────────────────────────────────────────────────────────
//  Reserve-mode budget query
// ───────────────────────────────────────────────────────────────────────────

/// Device availability for Reserve-mode filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAvailability {
    pub cpu: bool,
    pub simd: bool,
    pub wgpu: bool,
    pub cuda: bool,
    pub wasm: bool,
    pub exact: bool,
}

impl Default for DeviceAvailability {
    fn default() -> Self {
        Self {
            cpu: true,
            simd: true,
            wgpu: false,
            cuda: false,
            wasm: false,
            exact: true,
        }
    }
}

impl DeviceAvailability {
    fn supports(&self, backend: Backend) -> bool {
        match backend {
            Backend::Scalar => self.cpu,
            Backend::Simd => self.simd,
            Backend::Wgpu => self.wgpu,
            Backend::Cuda => self.cuda,
            Backend::Wasm => self.wasm,
            Backend::Exact => self.exact,
        }
    }
}

/// Given a device availability mask, return the backends runnable on
/// this device for the given op. Never returns an empty list — if all
/// GPU backends are unavailable, the CPU/WASM fallback is returned.
pub fn reserve_budget_query(op: &str, device: &DeviceAvailability) -> Vec<Backend> {
    GEOMETRY_OP_MANIFESTS
        .iter()
        .find(|m| m.op == op)
        .map(|m| {
            let runnable: Vec<Backend> = m
                .backends
                .iter()
                .copied()
                .filter(|b| device.supports(*b))
                .collect();

            if runnable.is_empty() {
                // Fallback: return deterministic backends regardless of mask.
                m.backends
                    .iter()
                    .copied()
                    .filter(|b| b.is_deterministic_fallback())
                    .collect()
            } else {
                runnable
            }
        })
        .unwrap_or_default()
}

// ───────────────────────────────────────────────────────────────────────────
//  JSON serialisation (for MCP tool listing)
// ───────────────────────────────────────────────────────────────────────────

/// Serialise all manifests as a JSON value (for MCP tool listing).
pub fn manifests_to_json() -> Value {
    let manifests: Vec<Value> = GEOMETRY_OP_MANIFESTS
        .iter()
        .map(|m| {
            json!({
                "op": m.op,
                "description": m.description,
                "backends": m.backends.iter().map(|b| match b {
                    Backend::Scalar => "scalar",
                    Backend::Simd => "simd",
                    Backend::Wgpu => "wgpu",
                    Backend::Cuda => "cuda",
                    Backend::Wasm => "wasm",
                    Backend::Exact => "exact",
                }).collect::<Vec<_>>(),
                "determinism": match m.determinism {
                    DeterminismClass::BitExact => "bit-exact",
                    DeterminismClass::Tolerance => "tolerance",
                },
                "limits": {
                    "max_input_points": m.limits.max_input_points,
                    "max_output_bytes": m.limits.max_output_bytes,
                    "max_memory_bytes": m.limits.max_memory_bytes,
                    "max_time_us": m.limits.max_time_us,
                },
                "topology_critical": m.topology_critical,
                "maturity": match m.maturity {
                    Maturity::Planned => "planned",
                    Maturity::Foundation => "foundation",
                    Maturity::Implemented => "implemented",
                    Maturity::Verified => "verified",
                    Maturity::Done => "done",
                },
                "exactness": match m.exactness {
                    ExactnessClass::ExactPredicate => "exact-predicate",
                    ExactnessClass::ExactConstruction => "exact-construction",
                    ExactnessClass::ApproximateMetric => "approximate-metric",
                    ExactnessClass::TopologyGuaranteed => "topology-guaranteed",
                    ExactnessClass::Structural => "structural",
                },
                "allocation": match m.allocation {
                    AllocationClass::HotZeroHeap => "hot-zero-heap",
                    AllocationClass::ColdBounded => "cold-bounded",
                    AllocationClass::TestTooling => "test-tooling",
                },
                "dimensionality": match m.dimensionality {
                    Dimensionality::D2 => "d2",
                    Dimensionality::D3 => "d3",
                    Dimensionality::DN => "dn",
                    Dimensionality::D10 => "d10",
                    Dimensionality::DimensionIndependent => "dimension-independent",
                },
            })
        })
        .collect();

    json!({
        "op_count": GEOMETRY_OP_MANIFESTS.len(),
        "ops": manifests,
    })
}

/// Serialise a Reserve-mode budget query result as JSON.
pub fn budget_query_to_json(op: &str, device: &DeviceAvailability) -> Value {
    let backends = reserve_budget_query(op, device);
    json!({
        "op": op,
        "runnable_backends": backends.iter().map(|b| match b {
            Backend::Scalar => "scalar",
            Backend::Simd => "simd",
            Backend::Wgpu => "wgpu",
            Backend::Cuda => "cuda",
            Backend::Wasm => "wasm",
            Backend::Exact => "exact",
        }).collect::<Vec<_>>(),
        "backend_count": backends.len(),
    })
}

// ───────────────────────────────────────────────────────────────────────────
//  Tests
// ───────────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_manifest_has_non_empty_backends() {
        for m in GEOMETRY_OP_MANIFESTS {
            assert!(!m.backends.is_empty(), "{} has no backends", m.op);
        }
    }

    #[test]
    fn gpu_ops_have_deterministic_fallback() {
        for m in GEOMETRY_OP_MANIFESTS {
            let has_gpu = m.backends.iter().any(|b| b.requires_gpu());
            let has_fallback = m.backends.iter().any(|b| b.is_deterministic_fallback());
            if has_gpu {
                assert!(
                    has_fallback,
                    "{} advertises GPU without deterministic fallback",
                    m.op
                );
            }
        }
    }

    #[test]
    fn resource_bounds_are_finite() {
        for m in GEOMETRY_OP_MANIFESTS {
            assert!(
                m.limits.max_output_bytes > 0,
                "{}: max_output_bytes is 0",
                m.op
            );
            assert!(
                m.limits.max_memory_bytes > 0,
                "{}: max_memory_bytes is 0",
                m.op
            );
        }
    }

    #[test]
    fn validate_manifests_passes() {
        assert!(validate_manifests(GEOMETRY_OP_MANIFESTS).is_ok());
    }

    #[test]
    fn manifests_to_json_round_trips() {
        let json = manifests_to_json();
        let s = serde_json::to_string(&json).unwrap();
        let parsed: Value = serde_json::from_str(&s).unwrap();
        assert_eq!(
            parsed["op_count"].as_u64().unwrap() as usize,
            GEOMETRY_OP_MANIFESTS.len()
        );
        // Every op has a backends array.
        for op in parsed["ops"].as_array().unwrap() {
            assert!(op["backends"].as_array().unwrap().len() > 0);
        }
    }

    #[test]
    fn reserve_budget_no_gpu_returns_fallback() {
        let device = DeviceAvailability {
            cpu: true,
            simd: true,
            wgpu: false,
            cuda: false,
            wasm: true,
            exact: true,
        };
        // vr_filtration advertises [Scalar, Wgpu, Wasm] — without GPU,
        // should return [Scalar, Wasm].
        let backends = reserve_budget_query("vr_filtration", &device);
        assert!(backends.contains(&Backend::Scalar));
        assert!(backends.contains(&Backend::Wasm));
        assert!(!backends.contains(&Backend::Wgpu));
    }

    #[test]
    fn reserve_budget_with_gpu_returns_all() {
        let device = DeviceAvailability {
            cpu: true,
            simd: true,
            wgpu: true,
            cuda: false,
            wasm: true,
            exact: true,
        };
        let backends = reserve_budget_query("vr_filtration", &device);
        assert!(backends.contains(&Backend::Wgpu));
        assert!(backends.contains(&Backend::Scalar));
    }

    #[test]
    fn reserve_budget_unknown_op_returns_empty() {
        let device = DeviceAvailability::default();
        let backends = reserve_budget_query("nonexistent_op", &device);
        assert!(backends.is_empty());
    }

    #[test]
    fn reserve_budget_never_empty_for_known_op() {
        // Even with no device support at all, a known op should return
        // its deterministic fallback.
        let device = DeviceAvailability {
            cpu: false,
            simd: false,
            wgpu: false,
            cuda: false,
            wasm: false,
            exact: false,
        };
        let backends = reserve_budget_query("convex_hull_2", &device);
        assert!(!backends.is_empty(), "must always return a fallback");
        assert!(backends.iter().all(|b| b.is_deterministic_fallback()));
    }

    #[test]
    fn topology_critical_ops_have_exact_backend() {
        for m in GEOMETRY_OP_MANIFESTS {
            if m.topology_critical {
                let has_exact = m.backends.contains(&Backend::Exact);
                let has_scalar = m.backends.contains(&Backend::Scalar);
                assert!(
                    has_exact || has_scalar,
                    "{} is topology_critical but has no exact/scalar backend",
                    m.op
                );
            }
        }
    }

    #[test]
    fn budget_query_to_json_valid() {
        let device = DeviceAvailability::default();
        let json = budget_query_to_json("convex_hull_2", &device);
        assert_eq!(json["op"], "convex_hull_2");
        assert!(json["backend_count"].as_u64().unwrap() > 0);
    }

    #[test]
    fn no_duplicate_ops() {
        let mut seen = std::collections::BTreeSet::new();
        for m in GEOMETRY_OP_MANIFESTS {
            assert!(seen.insert(m.op), "duplicate op: {}", m.op);
        }
    }

    #[test]
    fn determinism_class_serialises_correctly() {
        let json = manifests_to_json();
        let ops = json["ops"].as_array().unwrap();
        let orientation = ops.iter().find(|op| op["op"] == "orientation_2").unwrap();
        assert_eq!(orientation["determinism"], "bit-exact");

        let oracle = ops.iter().find(|op| op["op"] == "gpu_oracle").unwrap();
        assert_eq!(oracle["determinism"], "tolerance");
    }

    // ── P10.1 — claim-to-code audit tests ──────────────────────────────────

    /// The set of op names dispatched by `execute_geometry_tool_json`
    /// (`tool.rs`). Every tool-dispatched op MUST have a manifest entry —
    /// this is the claim-to-code audit gate: the public JSON API surface must
    /// not advertise an op the capability registry does not describe.
    const TOOL_DISPATCHED_OPS: &[&str] = &[
        "orientation_2",
        "convex_hull_2",
        "triangle_topology",
        "mesh_topology",
        "delaunay_2",
        "voronoi_2",
        "nearest_site",
        "create_box",
        "create_sphere",
        "create_cylinder",
        "create_plane",
        "create_torus",
        "create_grid",
        "boolean_union",
        "boolean_intersect",
        "boolean_difference",
        "drag_vertex",
    ];

    #[test]
    fn every_tool_dispatched_op_has_a_manifest() {
        for op in TOOL_DISPATCHED_OPS {
            assert!(
                GEOMETRY_OP_MANIFESTS.iter().any(|m| m.op == *op),
                "tool dispatches `{}` but no capability manifest exists for it (P10.1 claim-to-code gap)",
                op
            );
        }
    }

    #[test]
    fn no_manifest_claims_planned_maturity() {
        // A manifest entry is itself a claim that the op exists in code.
        // `Planned` maturity contradicts that — planned ops have no code.
        for m in GEOMETRY_OP_MANIFESTS {
            assert!(
                !matches!(m.maturity, Maturity::Planned),
                "{}: manifest carries Maturity::Planned — planned ops must not be advertised",
                m.op
            );
        }
    }

    #[test]
    fn topology_critical_ops_do_not_claim_approximate_metric_only() {
        // A topology-critical op whose sole exactness is an approximate metric
        // is the overstatement P10 exists to catch: a topology decision
        // (manifold/orientation/watertight) cannot rest on an approximate
        // quantity alone.
        for m in GEOMETRY_OP_MANIFESTS {
            if m.topology_critical {
                assert!(
                    !matches!(m.exactness, ExactnessClass::ApproximateMetric),
                    "{}: topology_critical but exactness is ApproximateMetric",
                    m.op
                );
            }
        }
    }

    #[test]
    fn p10_capability_fields_serialise_in_json() {
        let json = manifests_to_json();
        let ops = json["ops"].as_array().unwrap();
        let orientation = ops.iter().find(|op| op["op"] == "orientation_2").unwrap();
        assert_eq!(orientation["maturity"], "implemented");
        assert_eq!(orientation["exactness"], "exact-predicate");
        assert_eq!(orientation["allocation"], "hot-zero-heap");
        assert_eq!(orientation["dimensionality"], "d2");

        let marching = ops.iter().find(|op| op["op"] == "marching_cubes").unwrap();
        assert_eq!(marching["exactness"], "topology-guaranteed");
        assert_eq!(marching["dimensionality"], "d3");
    }

    #[test]
    fn exact_construction_ops_carry_exact_backend() {
        // An op advertising ExactConstruction must have an Exact backend in
        // its list — otherwise the "exact construction" claim is unsupported.
        for m in GEOMETRY_OP_MANIFESTS {
            if matches!(m.exactness, ExactnessClass::ExactConstruction) {
                assert!(
                    m.backends.contains(&Backend::Exact),
                    "{}: exactness is ExactConstruction but no Exact backend advertised",
                    m.op
                );
            }
        }
    }

    #[test]
    fn validate_manifests_rejects_planned_in_registry() {
        let bad = OpManifest {
            op: "bad_planned",
            description: "test fixture",
            backends: &[Backend::Scalar],
            determinism: DeterminismClass::BitExact,
            limits: ResourceLimits::default(),
            topology_critical: false,
            maturity: Maturity::Planned,
            exactness: ExactnessClass::Structural,
            allocation: AllocationClass::ColdBounded,
            dimensionality: Dimensionality::D2,
        };
        let err = validate_manifests(&[bad]).unwrap_err();
        assert!(
            err.contains("Planned"),
            "expected Planned rejection, got: {err}"
        );
    }

    #[test]
    fn validate_manifests_rejects_topology_critical_approximate_metric() {
        let bad = OpManifest {
            op: "bad_topo_approx",
            description: "test fixture",
            backends: &[Backend::Scalar],
            determinism: DeterminismClass::BitExact,
            limits: ResourceLimits::default(),
            topology_critical: true,
            maturity: Maturity::Implemented,
            exactness: ExactnessClass::ApproximateMetric,
            allocation: AllocationClass::ColdBounded,
            dimensionality: Dimensionality::D2,
        };
        let err = validate_manifests(&[bad]).unwrap_err();
        assert!(
            err.contains("ApproximateMetric"),
            "expected ApproximateMetric rejection, got: {err}"
        );
    }

    // ── P10.2 — decision exactness vs construction exactness split ────────

    /// P10.2's core gate: a boolean op with f64 intersections CANNOT
    /// advertise `ExactConstruction`. The boolean ops in the registry
    /// (`boolean_2`, `boolean_3`, `boolean_union/intersect/difference`,
    /// `minkowski_2`) all use f64 intersections today, so they must carry
    /// `ApproximateMetric` — not `ExactConstruction`. This test enforces that
    /// contract by name, so a future edit that upgrades a boolean to exact
    /// construction (P12 exact CSG) must also flip this exactness field and
    /// remove the op from this list.
    #[test]
    fn f64_boolean_ops_do_not_claim_exact_construction() {
        let f64_intersection_ops: &[&str] = &[
            "boolean_2",
            "minkowski_2",
            "boolean_3",
            "boolean_union",
            "boolean_intersect",
            "boolean_difference",
        ];
        for op in f64_intersection_ops {
            let m = GEOMETRY_OP_MANIFESTS
                .iter()
                .find(|m| m.op == *op)
                .unwrap_or_else(|| panic!("`{}` missing from registry", op));
            assert!(
                !matches!(m.exactness, ExactnessClass::ExactConstruction),
                "{} uses f64 intersections today but advertises ExactConstruction — P10.2 overstatement",
                op
            );
        }
    }

    /// P10.2 — the exactness split is exhaustive: every manifest carries one
    /// of the four exactness classes (or `Structural` for non-geometric ops).
    /// This is a type-system guarantee (the enum is closed), but the test
    /// pins the intent so a future variant addition is a conscious choice.
    #[test]
    fn exactness_split_is_exhaustive_over_registry() {
        for m in GEOMETRY_OP_MANIFESTS {
            // Every variant is a valid, intentional classification.
            match m.exactness {
                ExactnessClass::ExactPredicate
                | ExactnessClass::ExactConstruction
                | ExactnessClass::ApproximateMetric
                | ExactnessClass::TopologyGuaranteed
                | ExactnessClass::Structural => {}
            }
        }
    }

    /// P10.2 — exact-construction ops must re-predicate without sign drift,
    /// which requires the `Exact` backend. This is the constructive side of
    /// the split: `ExactPredicate` ops need an exact path for the sign
    /// decision; `ExactConstruction` ops need an exact path for the
    /// constructed coordinates. Both must carry `Backend::Exact`.
    #[test]
    fn exact_predicate_and_exact_construction_ops_carry_exact_backend() {
        for m in GEOMETRY_OP_MANIFESTS {
            if matches!(
                m.exactness,
                ExactnessClass::ExactPredicate | ExactnessClass::ExactConstruction
            ) {
                assert!(
                    m.backends.contains(&Backend::Exact),
                    "{}: exactness is {:?} but no Exact backend advertised",
                    m.op,
                    m.exactness
                );
            }
        }
    }

    // ── P10.3 — hot/cold path taxonomy + zero-heap enforcement ───────────

    /// P10.3 — every `HotZeroHeap` op must be a predicate or structural op
    /// (a sign decision, an exact construction, or a Morton/Hilbert encode).
    /// Cold-build algorithms (hull, Delaunay, voronoi, boolean, remesh,
    /// decimate, alpha shape, isosurface, reconstruction, point-set
    /// processing, TDA, Laplacian, NN query, authoring) MUST be
    /// `ColdBounded` — they allocate during construction. This test catches
    /// a misclassification where a builder is marked hot.
    #[test]
    fn hot_zero_heap_ops_are_not_cold_builders() {
        // Ops that build a structure (allocate during construction).
        const COLD_BUILDERS: &[&str] = &[
            "convex_hull_2",
            "convex_hull_3",
            "delaunay_2",
            "delaunay_3",
            "voronoi_2",
            "conforming_delaunay_2",
            "bvh_3d",
            "kd_tree_3d",
            "box_join",
            "boolean_2",
            "minkowski_2",
            "boolean_3",
            "boolean_3_exact",
            "boolean_union",
            "boolean_intersect",
            "boolean_difference",
            "decimate_qem",
            "isotropic_remesh",
            "alpha_shape_2d",
            "alpha_shape_3d",
            "alpha_filtration_2d",
            "marching_cubes",
            "poisson_reconstruct_3d",
            "point_set_3d",
            "vr_filtration",
            "persistence",
            "cknn_laplacian",
            "cknn_laplacian_3d",
            "nn_query",
            "gpu_oracle",
            "natural_neighbour",
            "create_box",
            "create_sphere",
            "create_cylinder",
            "create_plane",
            "create_torus",
            "create_grid",
            "drag_vertex",
            "triangle_topology",
            "mesh_topology",
        ];
        for op in COLD_BUILDERS {
            let m = GEOMETRY_OP_MANIFESTS
                .iter()
                .find(|m| m.op == *op)
                .unwrap_or_else(|| panic!("`{}` missing from registry", op));
            assert!(
                !matches!(m.allocation, AllocationClass::HotZeroHeap),
                "{} is a cold builder but is marked HotZeroHeap — P10.3 misclassification",
                op
            );
        }
    }

    /// P10.3 — the predicate ops (the actual hot paths) MUST be `HotZeroHeap`.
    /// These are the ops called inside hull/Delaunay/boolean loops; if they
    /// allocate, the whole algorithm's hot path allocates.
    #[test]
    fn predicate_ops_are_hot_zero_heap() {
        const HOT_PREDICATES: &[&str] = &[
            "orientation_2",
            "orient_3d",
            "incircle",
            "insphere",
            "construct_segment_intersection_2",
            "exact_construct_3",
            "spatial_order",
        ];
        for op in HOT_PREDICATES {
            let m = GEOMETRY_OP_MANIFESTS
                .iter()
                .find(|m| m.op == *op)
                .unwrap_or_else(|| panic!("`{}` missing from registry", op));
            assert!(
                matches!(m.allocation, AllocationClass::HotZeroHeap),
                "{} is a hot predicate but is NOT marked HotZeroHeap — P10.3 misclassification",
                op
            );
        }
    }
}

// ── P10.3 — real allocation-counter tests for the hot predicates ──────────
//
// These use the `allocation_counter` module's `assert_zero_alloc` to verify
// that the Tier-1 hot-path predicate ops (orientation_2, orient_3d, incircle,
// insphere) do not allocate on the hot path. This is the enforcement side of
// the `AllocationClass::HotZeroHeap` claim — not just a metadata assertion,
// but a real measurement.
//
// **Parallel-safe (thread-local counter):** the allocation counter uses
// thread-local counters gated by a thread-local `MEASURING` flag. Each test
// thread only counts its own allocations while its guard is active;
// allocations from other test threads running in parallel are invisible.
// These tests are reliable under the default parallel test execution — no
// `--test-threads=1` requirement. (See `allocation_counter.rs` for the
// thread-local design.)

#[cfg(test)]
mod zero_heap_tests {
    use super::super::allocation_counter::assert_zero_alloc;
    use super::super::incircle::incircle;
    use super::super::insphere::insphere;
    use super::super::orient3d::orient_3d;
    use super::super::primitives::{orientation_2, Point2, Point3};

    #[test]
    fn orientation_2_hot_path_is_zero_heap() {
        // Warm up (in case of lazy init).
        let _ = orientation_2(
            Point2::new(0.0, 0.0),
            Point2::new(1.0, 0.0),
            Point2::new(0.5, 0.5),
        );
        assert_zero_alloc("orientation_2", || {
            let _ = orientation_2(
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(0.5, 0.5),
            );
            let _ = orientation_2(
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(2.0, 0.0),
            );
            let _ = orientation_2(
                Point2::new(0.0, 0.0),
                Point2::new(1.0, 0.0),
                Point2::new(1.0, -1.0),
            );
        });
    }

    #[test]
    fn orient_3d_hot_path_is_zero_heap() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(0.0, 0.0, 1.0);
        let _ = orient_3d(a, b, c, d); // warm up
        assert_zero_alloc("orient_3d", || {
            let _ = orient_3d(a, b, c, d);
            let _ = orient_3d(a, b, c, Point3::new(0.0, 0.0, -1.0));
        });
    }

    #[test]
    fn incircle_hot_path_is_zero_heap() {
        let a = Point2::new(0.0, 0.0);
        let b = Point2::new(1.0, 0.0);
        let c = Point2::new(0.0, 1.0);
        let d = Point2::new(0.25, 0.25);
        let _ = incircle(a, b, c, d); // warm up
        assert_zero_alloc("incircle", || {
            let _ = incircle(a, b, c, d);
            let _ = incircle(a, b, c, Point2::new(2.0, 2.0));
        });
    }

    #[test]
    fn insphere_hot_path_is_zero_heap() {
        let a = Point3::new(0.0, 0.0, 0.0);
        let b = Point3::new(1.0, 0.0, 0.0);
        let c = Point3::new(0.0, 1.0, 0.0);
        let d = Point3::new(0.0, 0.0, 1.0);
        let e = Point3::new(0.25, 0.25, 0.25);
        let _ = insphere(a, b, c, d, e); // warm up
        assert_zero_alloc("insphere", || {
            let _ = insphere(a, b, c, d, e);
            let _ = insphere(a, b, c, d, Point3::new(2.0, 2.0, 2.0));
        });
    }
}
