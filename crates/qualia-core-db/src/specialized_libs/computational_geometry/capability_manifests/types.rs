use serde::{Deserialize, Serialize};

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
