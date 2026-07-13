//! QISP typed function registry (Phase 4) — the concrete descriptor table that
//! makes the §4.2 "no untyped `u64 -> u64` public function contract" real.
//!
//! Every QISP extension function (`qispf:`) is registered here with its typed
//! [`ImmersiveFunctionDescriptor`]: argument kinds, result kind, execution class,
//! determinism, exactness, and I/O byte budgets. The SPARQL evaluator can look a
//! function up by the `q_hash` of its IRI (the `Function::Custom(iri_hash)` path)
//! and apply **admission control** — reject an inline `FILTER`/`BIND` use of an
//! async/table-producing function, or an over-budget call — *before* dispatch,
//! instead of discovering the problem mid-execution (plan §4.2, §6.1).
//!
//! Where GeoSPARQL already defines an operation for 2D geometry, the QISP function
//! **defers** to the `geof:` name (plan §4.1 "GeoSPARQL names and semantics take
//! precedence"); the QISP descriptor exists for the mesh / volumetric / tensor /
//! higher-dimensional cases GeoSPARQL does not define. The deference is recorded in
//! [`FunctionEntry::defers_to`] so a planner can route a 2D call to the GeoSPARQL
//! engine and a 3D/mesh call to the native kernel.
//!
//! This is the typed *registry*; binding a descriptor to a live geometry/tensor
//! kernel over resolved [`DenseAssetRef`](super::asset_registry::DenseAssetRef)s is
//! the next Phase-4 increment (it needs the asset registry populated with real
//! ingested geometry). The registry + admission contract land first so the boundary
//! is typed and honest.

use super::value::{
    ExactnessClass as E, ExecutionClass as X, ImmersiveFunctionDescriptor as Desc,
    ImmersiveValueKind as K, QispError,
};
use crate::tensor::Tensor10D;

/// One bounded nearest-neighbour result. `index` identifies the input slice slot;
/// `distance` is the canonical [`Tensor10D::full_distance`] value.
#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TensorNeighbor {
    pub index: u32,
    pub distance: f32,
}

/// Exact, allocation-free Tensor10D distance using the resident substrate's
/// canonical topology-aware metric.
pub fn tensor_distance(left: &Tensor10D, right: &Tensor10D) -> Result<f32, QispError> {
    let distance = left.full_distance(right);
    if distance.is_finite() {
        Ok(distance)
    } else {
        Err(QispError::ProfileMismatch)
    }
}

/// Exact radius predicate. Invalid/non-finite radii fail as profile errors rather
/// than silently producing `false`.
pub fn tensor_within(left: &Tensor10D, right: &Tensor10D, radius: f32) -> Result<bool, QispError> {
    if !radius.is_finite() || radius < 0.0 {
        return Err(QispError::ProfileMismatch);
    }
    Ok(tensor_distance(left, right)? <= radius)
}

/// Deterministic, allocation-free bounded k-nearest-neighbour scan.
///
/// Results are ordered by `(distance, input_index)`, making ties stable across
/// runs. The caller controls both the maximum result count (`k`) and storage. An
/// undersized output is an explicit budget error; no partial result is exposed.
pub fn tensor_knn_into(
    query: &Tensor10D,
    candidates: &[Tensor10D],
    k: usize,
    out: &mut [TensorNeighbor],
) -> Result<usize, QispError> {
    if k > out.len() || k > u32::MAX as usize || candidates.len() > u32::MAX as usize {
        return Err(QispError::BudgetExceeded);
    }
    if k == 0 || candidates.is_empty() {
        return Ok(0);
    }
    let wanted = k.min(candidates.len());
    for (index, candidate) in candidates.iter().enumerate() {
        let distance = tensor_distance(query, candidate)?;
        let item = TensorNeighbor {
            index: index as u32,
            distance,
        };
        let populated = index.min(wanted);
        let mut pos = populated;
        while pos > 0 {
            let prev = out[pos - 1];
            if prev.distance < distance || (prev.distance == distance && prev.index < item.index) {
                break;
            }
            pos -= 1;
        }
        if pos < wanted {
            let upper = populated.min(wanted - 1);
            for slot in (pos..upper).rev() {
                out[slot + 1] = out[slot];
            }
            out[pos] = item;
        }
    }
    Ok(wanted)
}

/// One registered QISP function: its provisional IRI, typed descriptor, and the
/// GeoSPARQL name it defers to for the 2D case (if any).
#[derive(Debug, Clone, Copy)]
pub struct FunctionEntry {
    /// Absolute provisional IRI (`qispf:` namespace).
    pub iri: &'static str,
    /// Typed descriptor (carries `iri_hash = q_hash(iri)`).
    pub descriptor: Desc,
    /// The `geof:` function this defers to for 2D geometry, or `None` when QISP
    /// owns the operation (mesh / tensor / higher-dimensional).
    pub defers_to: Option<&'static str>,
}

/// Const constructor so the table below can be a `const`.
const fn entry(
    iri: &'static str,
    args: [K; 4],
    arg_count: u8,
    result: K,
    execution: X,
    deterministic: bool,
    exactness: E,
    max_input_bytes: u32,
    max_output_bytes: u32,
    defers_to: Option<&'static str>,
) -> FunctionEntry {
    FunctionEntry {
        iri,
        descriptor: Desc::new(
            crate::q_hash(iri),
            args,
            arg_count,
            result,
            execution,
            deterministic,
            exactness,
            max_input_bytes,
            max_output_bytes,
        ),
        defers_to,
    }
}

// Byte-budget shorthands (admission control; reference the geometry capability
// manifests' scale, not new limits).
const MB: u32 = 1 << 20;

/// The registered QISP function set (plan §4.1). Ordered by family. A predicate over
/// two geometries takes `(GeometryRef, GeometryRef)`; the trailing exactness selector
/// in the surface syntax (e.g. `qisp:Exact`) is a per-call modifier, not a descriptor
/// argument — the descriptor's `exactness` is the registered default.
pub const FUNCTIONS: &[FunctionEntry] = &[
    // ── Topological (hot predicate after coarse filter) ──────────────────────
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#intersects",
        [K::GeometryRef, K::GeometryRef, K::Boolean, K::Boolean],
        2,
        K::Boolean,
        X::HotZeroHeap,
        true,
        E::Exact,
        MB,
        0,
        Some("http://www.opengis.net/def/function/geosparql/sfIntersects"),
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#contains",
        [K::GeometryRef, K::GeometryRef, K::Boolean, K::Boolean],
        2,
        K::Boolean,
        X::HotZeroHeap,
        true,
        E::Exact,
        MB,
        0,
        Some("http://www.opengis.net/def/function/geosparql/sfContains"),
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#touches",
        [K::GeometryRef, K::GeometryRef, K::Boolean, K::Boolean],
        2,
        K::Boolean,
        X::HotZeroHeap,
        true,
        E::Exact,
        MB,
        0,
        Some("http://www.opengis.net/def/function/geosparql/sfTouches"),
    ),
    // ── Proximity (hot query, caller-buffered) ───────────────────────────────
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#distance",
        [K::GeometryRef, K::GeometryRef, K::Boolean, K::Boolean],
        2,
        K::Quantity,
        X::HotZeroHeap,
        true,
        E::Exact,
        MB,
        0,
        Some("http://www.opengis.net/def/function/geosparql/distance"),
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#withinDistance",
        [K::GeometryRef, K::GeometryRef, K::Scalar, K::Boolean],
        3,
        K::Boolean,
        X::HotZeroHeap,
        true,
        E::Exact,
        MB,
        0,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#nearest",
        [K::GeometryRef, K::Scalar, K::Boolean, K::Boolean],
        2,
        K::AssetRef,
        X::ColdBoundedSync,
        true,
        E::DeterministicApproximate,
        4 * MB,
        MB,
        None,
    ),
    // ── Visibility (ray/BVH; exact or labelled approximation) ─────────────────
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#lineOfSight",
        [K::GeometryRef, K::GeometryRef, K::Boolean, K::Boolean],
        2,
        K::Boolean,
        X::ColdBoundedSync,
        true,
        E::Exact,
        4 * MB,
        0,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#occludes",
        [K::GeometryRef, K::GeometryRef, K::Boolean, K::Boolean],
        2,
        K::Boolean,
        X::ColdBoundedSync,
        true,
        E::DeterministicApproximate,
        4 * MB,
        0,
        None,
    ),
    // ── Measurement (hot if precomputed/simple; otherwise bounded) ────────────
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#volume",
        [K::GeometryRef, K::Boolean, K::Boolean, K::Boolean],
        1,
        K::Quantity,
        X::ColdBoundedSync,
        true,
        E::Exact,
        4 * MB,
        0,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#surfaceArea",
        [K::GeometryRef, K::Boolean, K::Boolean, K::Boolean],
        1,
        K::Quantity,
        X::ColdBoundedSync,
        true,
        E::Exact,
        4 * MB,
        0,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#centroid",
        [K::GeometryRef, K::Boolean, K::Boolean, K::Boolean],
        1,
        K::GeometryRef,
        X::HotZeroHeap,
        true,
        E::Exact,
        MB,
        256,
        None,
    ),
    // ── Temporal (bounded trace/window evaluation) ────────────────────────────
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#intersectsAt",
        [K::GeometryRef, K::GeometryRef, K::Instant, K::Boolean],
        3,
        K::Boolean,
        X::ColdBoundedSync,
        true,
        E::Exact,
        4 * MB,
        0,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#trajectoryIntersects",
        [K::GeometryRef, K::GeometryRef, K::Interval, K::Boolean],
        3,
        K::Boolean,
        X::ColdBoundedSync,
        true,
        E::DeterministicApproximate,
        4 * MB,
        0,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#sliceAtTime",
        [K::TensorRef, K::Instant, K::Boolean, K::Boolean],
        2,
        K::TensorRef,
        X::ColdBoundedSync,
        true,
        E::Exact,
        4 * MB,
        MB,
        None,
    ),
    // ── Tensor (resident substrate / GPU batch) ───────────────────────────────
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#tensorDistance",
        [K::TensorRef, K::TensorRef, K::Boolean, K::Boolean],
        2,
        K::Quantity,
        X::HotZeroHeap,
        true,
        E::Exact,
        256,
        0,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#tensorWithin",
        [K::TensorRef, K::TensorRef, K::Scalar, K::Boolean],
        3,
        K::Boolean,
        X::HotZeroHeap,
        true,
        E::Exact,
        256,
        0,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#tensorSlice",
        [K::TensorRef, K::Scalar, K::Boolean, K::Boolean],
        2,
        K::TensorRef,
        X::ColdBoundedSync,
        true,
        E::Exact,
        4 * MB,
        MB,
        None,
    ),
    // knn is a table-producing *graph operator*, not a scalar expression function;
    // it is registered for discovery/admission but is NOT legal inline (see
    // `is_expression_legal`). ColdBoundedSync + non-deterministic ordering-of-ties
    // marks it ineligible for FILTER/BIND (plan §6.1 physical operators).
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#knn",
        [K::TensorRef, K::Scalar, K::Boolean, K::Boolean],
        2,
        K::AssetRef,
        X::ColdBoundedSync,
        false,
        E::DeterministicApproximate,
        4 * MB,
        4 * MB,
        None,
    ),
    // ── Constructive (cold bounded arena) ─────────────────────────────────────
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#intersectionGeometry",
        [K::GeometryRef, K::GeometryRef, K::Boolean, K::Boolean],
        2,
        K::GeometryRef,
        X::ColdBoundedSync,
        true,
        E::Exact,
        8 * MB,
        8 * MB,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#unionGeometry",
        [K::GeometryRef, K::GeometryRef, K::Boolean, K::Boolean],
        2,
        K::GeometryRef,
        X::ColdBoundedSync,
        true,
        E::Exact,
        8 * MB,
        8 * MB,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#differenceGeometry",
        [K::GeometryRef, K::GeometryRef, K::Boolean, K::Boolean],
        2,
        K::GeometryRef,
        X::ColdBoundedSync,
        true,
        E::Exact,
        8 * MB,
        8 * MB,
        None,
    ),
    // ── Transform (cold bounded unless a fixed scalar transform) ──────────────
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#transform",
        [K::GeometryRef, K::Scalar, K::Boolean, K::Boolean],
        2,
        K::GeometryRef,
        X::ColdBoundedSync,
        true,
        E::Exact,
        8 * MB,
        8 * MB,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#reproject",
        [K::GeometryRef, K::Scalar, K::Boolean, K::Boolean],
        2,
        K::GeometryRef,
        X::ColdBoundedSync,
        true,
        E::Exact,
        8 * MB,
        8 * MB,
        None,
    ),
    entry(
        "https://standards.qualiadb.org/immersive/function/0.1#buffer",
        [K::GeometryRef, K::Scalar, K::Boolean, K::Boolean],
        2,
        K::GeometryRef,
        X::ColdBoundedSync,
        true,
        E::DeterministicApproximate,
        8 * MB,
        8 * MB,
        None,
    ),
];

/// Look a registered function up by the `q_hash` of its IRI (the value carried in
/// `Function::Custom(iri_hash)`). Returns `None` for an unregistered function — the
/// caller then returns a normal SPARQL "unknown function" expression error (never a
/// fabricated result, plan §7.4).
pub fn entry_for_iri_hash(iri_hash: u64) -> Option<&'static FunctionEntry> {
    FUNCTIONS.iter().find(|e| e.descriptor.iri_hash == iri_hash)
}

/// Look a registered function up by its absolute IRI.
pub fn entry_for_iri(iri: &str) -> Option<&'static FunctionEntry> {
    FUNCTIONS.iter().find(|e| e.iri == iri)
}

/// Admission check for an **inline expression** (`FILTER`/`BIND`) use of a QISP
/// function identified by `iri_hash`:
///
/// - `Ok(entry)` if it is registered and legal inline (deterministic, synchronous,
///   scalar-producing);
/// - `Err(NonDeterministicDisallowed)` if registered but async/table-producing
///   (e.g. `knn`) — it must be invoked as a job / graph operator instead;
/// - `Ok`-with-`None`-semantics is not used: an unregistered IRI returns `None` so
///   the evaluator can emit a standard unknown-function error.
///
/// This lets the evaluator reject an ill-typed inline call *before* dispatch.
pub fn admit_inline(iri_hash: u64) -> Option<Result<&'static FunctionEntry, QispError>> {
    let e = entry_for_iri_hash(iri_hash)?;
    Some(if e.descriptor.legal_in_filter() {
        Ok(e)
    } else {
        Err(QispError::NonDeterministicDisallowed)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tensor::Tensor10D;

    #[test]
    fn every_entry_hash_matches_its_iri() {
        // The const `entry` builder computes iri_hash from the same literal it stores,
        // so this proves the table is internally consistent (no typo drift).
        for e in FUNCTIONS {
            assert_eq!(
                e.descriptor.iri_hash,
                crate::q_hash(e.iri),
                "hash mismatch for {}",
                e.iri
            );
            assert!(
                e.iri.starts_with(super::super::QISPF_NS),
                "{} not in qispf: ns",
                e.iri
            );
        }
    }

    #[test]
    fn iri_hashes_are_unique() {
        for (i, a) in FUNCTIONS.iter().enumerate() {
            for b in &FUNCTIONS[i + 1..] {
                assert_ne!(
                    a.descriptor.iri_hash, b.descriptor.iri_hash,
                    "collision: {} vs {}",
                    a.iri, b.iri
                );
            }
        }
    }

    #[test]
    fn lookup_by_hash_and_iri_agree() {
        let iri = "https://standards.qualiadb.org/immersive/function/0.1#intersects";
        let by_iri = entry_for_iri(iri).unwrap();
        let by_hash = entry_for_iri_hash(crate::q_hash(iri)).unwrap();
        assert_eq!(by_iri.descriptor.iri_hash, by_hash.descriptor.iri_hash);
        assert_eq!(by_iri.descriptor.result, K::Boolean);
        assert_eq!(by_iri.descriptor.arg_count, 2);
    }

    #[test]
    fn topological_and_proximity_defer_to_geosparql() {
        for name in ["intersects", "contains", "touches", "distance"] {
            let iri = format!("https://standards.qualiadb.org/immersive/function/0.1#{name}");
            let e = entry_for_iri(&iri).unwrap();
            assert!(e.defers_to.is_some(), "{name} should defer to geof: for 2D");
        }
        // withinDistance / volume are QISP-owned (no GeoSPARQL 2D equivalent used here).
        let wd =
            entry_for_iri("https://standards.qualiadb.org/immersive/function/0.1#withinDistance")
                .unwrap();
        assert!(wd.defers_to.is_none());
    }

    #[test]
    fn admission_rejects_knn_inline_but_admits_predicates() {
        let intersects =
            crate::q_hash("https://standards.qualiadb.org/immersive/function/0.1#intersects");
        assert!(
            matches!(admit_inline(intersects), Some(Ok(_))),
            "intersects is legal inline"
        );

        let knn = crate::q_hash("https://standards.qualiadb.org/immersive/function/0.1#knn");
        assert!(
            matches!(
                admit_inline(knn),
                Some(Err(QispError::NonDeterministicDisallowed))
            ),
            "knn is a graph operator, not an inline expression function"
        );

        // Unregistered IRI → None (evaluator emits a standard unknown-function error).
        assert!(admit_inline(0xDEAD_BEEF_0000_0001).is_none());
    }

    #[test]
    fn hot_predicates_are_filter_legal_and_zero_heap() {
        for name in [
            "intersects",
            "contains",
            "touches",
            "distance",
            "withinDistance",
            "tensorDistance",
            "tensorWithin",
        ] {
            let iri = format!("https://standards.qualiadb.org/immersive/function/0.1#{name}");
            let e = entry_for_iri(&iri).unwrap();
            assert_eq!(e.descriptor.execution, X::HotZeroHeap, "{name} must be hot");
            assert!(
                e.descriptor.legal_in_filter(),
                "{name} must be FILTER-legal"
            );
        }
    }

    #[test]
    fn constructive_ops_are_cold_bounded_with_output_budget() {
        for name in [
            "intersectionGeometry",
            "unionGeometry",
            "differenceGeometry",
        ] {
            let iri = format!("https://standards.qualiadb.org/immersive/function/0.1#{name}");
            let e = entry_for_iri(&iri).unwrap();
            assert_eq!(e.descriptor.execution, X::ColdBoundedSync);
            assert!(
                e.descriptor.max_output_bytes > 0,
                "{name} must declare an output budget"
            );
            assert_eq!(e.descriptor.result, K::GeometryRef);
        }
    }

    #[test]
    fn tensor_predicates_use_canonical_metric_and_reject_bad_radius() {
        let a = Tensor10D::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        let b = Tensor10D::new(0.0, 0.0, 0.0, 3.0, 4.0, 0.0, 0.0, 0.0, 0.0, 0.0);
        assert_eq!(tensor_distance(&a, &b).unwrap(), a.full_distance(&b));
        assert_eq!(tensor_within(&a, &b, 5.0), Ok(true));
        assert_eq!(tensor_within(&a, &b, 4.99), Ok(false));
        assert_eq!(
            tensor_within(&a, &b, f32::NAN),
            Err(QispError::ProfileMismatch)
        );
        assert_eq!(tensor_within(&a, &b, -1.0), Err(QispError::ProfileMismatch));
    }

    #[test]
    fn tensor_knn_is_bounded_sorted_and_tie_deterministic() {
        let query = Tensor10D::default();
        let candidates = [
            Tensor10D::new(0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
            Tensor10D::new(0.0, 0.0, 0.0, -1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0),
        ];
        let mut out = [TensorNeighbor {
            index: 99,
            distance: f32::INFINITY,
        }; 2];
        assert_eq!(tensor_knn_into(&query, &candidates, 2, &mut out), Ok(2));
        assert_eq!(out[0].index, 1);
        assert_eq!(out[1].index, 2);
        assert_eq!(out[0].distance, out[1].distance);
        assert_eq!(
            tensor_knn_into(&query, &candidates, 3, &mut out),
            Err(QispError::BudgetExceeded)
        );
    }
}
