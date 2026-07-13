use super::*;
use serde_json::{json, Value};

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
