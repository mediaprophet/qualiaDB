//! Immersive SPARQL (QISP) — Phase 2 core: typed values + generation-safe
//! dense-asset registry + typed function descriptors.
//!
//! This module is the Phase 2 slice of the **Qualia Immersive SPARQL Profile
//! (QISP)** described in `docs/plans/immersive-sparql-hypermedia-profile.md`. It is
//! a *profile and extension layer over the existing SPARQL engine* — not a fork of
//! the query language and not a second parser/registry. It contributes three
//! things, each in its own single-purpose submodule (CLAUDE.md §11):
//!
//! - [`value`] — the typed function descriptor ([`ImmersiveFunctionDescriptor`]),
//!   its [`ImmersiveValueKind`]/[`ExecutionClass`]/[`ExactnessClass`]
//!   classification, and the stable [`QispError`] codes (plan §4.2–§4.4).
//! - [`asset_registry`] — the bounded, generation-safe [`DenseAssetRegistry`] and
//!   its fail-closed [`DenseAssetRef`] handle. A handle carries only numeric,
//!   validated fields; **no Rust address is ever stored in an NQuin/RDF term**
//!   (plan §3.4, §10.1 QISP-R03/R04).
//! - [`profile`] — the fixed Tensor10D dimension order and honest classification,
//!   plus inline-value validation (plan §3.5, §3.6).
//!
//! # Standards posture
//!
//! These are **Editor's-Draft, provisional IRIs** (plan §2.4): the vocabulary and
//! semantics may change and there is no compatibility promise until the profile
//! advances. Nothing here claims W3C or OGC standard status. The namespace owner
//! and continuity plan are governance items tracked in the plan (§15 QISP-D02).
//!
//! The `functions` submodule (the concrete typed-function registry that *uses*
//! these descriptors) is added by the integrating session — it is intentionally
//! **not** declared here.

pub mod asset_registry;
pub mod functions;
pub mod profile;
pub mod value;

// ---------------------------------------------------------------------------
// Provisional namespace IRIs (plan §3.1). Separate ontology / function /
// datatype namespaces so predicates are never confused with executable
// functions. These are versioned `0.1` draft IRIs.
// ---------------------------------------------------------------------------

/// Ontology namespace — classes and properties (`qisp:`).
pub const QISP_NS: &str = "https://webizen.org/immersive/0.1#";

/// Function namespace — executable extension functions (`qispf:`).
pub const QISPF_NS: &str = "https://webizen.org/immersive/function/0.1#";

/// Datatype namespace — QISP-specific datatypes (`qispd:`).
pub const QISPD_NS: &str = "https://webizen.org/immersive/datatype/0.1#";

// ---------------------------------------------------------------------------
// Public API re-exports.
// ---------------------------------------------------------------------------

pub use value::{
    ExactnessClass, ExecutionClass, ImmersiveFunctionDescriptor, ImmersiveValueKind, QispError,
};

pub use asset_registry::{
    AssetRecord, DenseAssetRef, DenseAssetRegistry, SectionKind, MAX_ASSETS,
};

pub use profile::{
    validate_inline_tensor10d, DimClass, TENSOR10D_DIMS, TENSOR10D_PROFILE_IRI,
};

pub use functions::{
    admit_inline, entry_for_iri, entry_for_iri_hash, tensor_distance, tensor_knn_into,
    tensor_within, FunctionEntry, TensorNeighbor, FUNCTIONS,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn namespaces_are_distinct_and_versioned() {
        assert_ne!(QISP_NS, QISPF_NS);
        assert_ne!(QISP_NS, QISPD_NS);
        assert_ne!(QISPF_NS, QISPD_NS);
        assert!(QISP_NS.contains("/immersive/0.1#"));
        assert!(QISPF_NS.contains("/immersive/function/0.1#"));
        assert!(QISPD_NS.contains("/immersive/datatype/0.1#"));
    }

    #[test]
    fn re_exports_are_reachable() {
        // Compile-time proof the public surface is wired through `mod`.
        let _k: ImmersiveValueKind = ImmersiveValueKind::AssetRef;
        let _e: ExecutionClass = ExecutionClass::HotZeroHeap;
        let _x: ExactnessClass = ExactnessClass::Exact;
        let _err: QispError = QispError::UnknownAsset;
        let _sec: SectionKind = SectionKind::Mesh;
        let _dim: DimClass = DimClass::Spatial;
        let mut reg = DenseAssetRegistry::new();
        let r: DenseAssetRef = reg.insert(SectionKind::Mesh, 0, 1, 0).unwrap();
        let rec: AssetRecord = *reg.resolve(&r).unwrap();
        assert_eq!(rec.token(), r.token());
        assert_eq!(TENSOR10D_DIMS.len(), 10);
        assert!(!TENSOR10D_PROFILE_IRI.is_empty());
        assert!(MAX_ASSETS >= 1);
        assert!(validate_inline_tensor10d(&[0.0; 10]).is_ok());
    }
}
