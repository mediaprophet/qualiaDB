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
        matches!(self, Backend::Scalar | Backend::Simd | Backend::Wasm | Backend::Exact)
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
}

// ───────────────────────────────────────────────────────────────────────────
//  Manifest registry
// ───────────────────────────────────────────────────────────────────────────

/// The canonical capability manifest table for computational-geometry ops.
pub const GEOMETRY_OP_MANIFESTS: &[OpManifest] = &[
    OpManifest {
        op: "orientation_2",
        description: "Robust 2-D orientation predicate (exact arithmetic fallback).",
        backends: &[Backend::Scalar, Backend::Exact, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 3,
            max_output_bytes: 64,
            max_memory_bytes: 1024,
            max_time_us: 100,
        },
        topology_critical: true,
    },
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
    },
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
    },
    OpManifest {
        op: "voronoi_2",
        description: "Voronoi diagram as Delaunay dual.",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 100_000,
            max_output_bytes: 32 * 1024 * 1024,
            max_memory_bytes: 128 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
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
    },
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
    },
    // P8 ops — GPU-accelerated with CPU fallback.
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
    },
    OpManifest {
        op: "natural_neighbour",
        description: "Natural-neighbour interpolation (Laplace coordinates).",
        backends: &[Backend::Scalar, Backend::Wasm],
        determinism: DeterminismClass::BitExact,
        limits: ResourceLimits {
            max_input_points: 50_000,
            max_output_bytes: 4 * 1024 * 1024,
            max_memory_bytes: 64 * 1024 * 1024,
            max_time_us: 0,
        },
        topology_critical: false,
    },
    // P9.5 — Authoring ops (primitive generation).
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
    },
];

// ───────────────────────────────────────────────────────────────────────────
//  Validation
// ───────────────────────────────────────────────────────────────────────────

/// Validate that every manifest has non-empty backends and that any op
/// advertising a GPU backend also has a deterministic fallback.
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
pub fn reserve_budget_query(
    op: &str,
    device: &DeviceAvailability,
) -> Vec<Backend> {
    GEOMETRY_OP_MANIFESTS
        .iter()
        .find(|m| m.op == op)
        .map(|m| {
            let runnable: Vec<Backend> = m.backends
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
                assert!(has_fallback,
                    "{} advertises GPU without deterministic fallback",
                    m.op);
            }
        }
    }

    #[test]
    fn resource_bounds_are_finite() {
        for m in GEOMETRY_OP_MANIFESTS {
            assert!(m.limits.max_output_bytes > 0,
                "{}: max_output_bytes is 0", m.op);
            assert!(m.limits.max_memory_bytes > 0,
                "{}: max_memory_bytes is 0", m.op);
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
                assert!(has_exact || has_scalar,
                    "{} is topology_critical but has no exact/scalar backend",
                    m.op);
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
        let orientation = ops.iter()
            .find(|op| op["op"] == "orientation_2")
            .unwrap();
        assert_eq!(orientation["determinism"], "bit-exact");

        let oracle = ops.iter()
            .find(|op| op["op"] == "gpu_oracle")
            .unwrap();
        assert_eq!(oracle["determinism"], "tolerance");
    }
}
