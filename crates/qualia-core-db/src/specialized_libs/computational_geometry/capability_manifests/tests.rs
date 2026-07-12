use super::*;
use serde_json::Value;

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
        // nary_boolean reduces over binary boolean_3 (f64 intersections),
        // so it inherits ApproximateMetric — it must never claim exact
        // construction just because it operates on topology-critical meshes.
        "nary_boolean",
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
        "anisotropic_remesh",
        "mixed_cell_topology",
        "ddg_operators",
        "screened_poisson_reconstruct_3d",
        "fem_mesh_certificate",
        "deterministic_geometry",
        "motion_planning",
        "math_geometry",
        "parametric_cad",
        "geometry_integration",
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
