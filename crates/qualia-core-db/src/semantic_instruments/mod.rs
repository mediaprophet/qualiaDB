//! Semantic-instrument collectables (SI-03).
//!
//! A collectable is credential-like: HCF document + definition graph + `.10d`
//! visual, shipped as an HMC archive (`bundle`). Large Q42 volumes remain
//! dependencies. Demo seeds populate a development environment and stay labelled.

pub mod attestation;
pub mod canonical;
pub mod catalogue;
pub mod category;
pub mod demo;
pub mod errors;
pub mod evaluators;
pub mod evaluators_rights;
pub mod execution;
pub mod manifest;
pub mod manufacture;
pub mod package;
pub mod reference;
pub mod resolve;
pub mod rpl;
pub mod visual;

pub use attestation::{
    badge_export, issue_capability_award, issue_native, issue_w3c, verify_native, verify_w3c,
    AttestationKind, AttestationStatus, InstrumentAttestation, StatusRegistry,
};
pub use canonical::{canonical_manifest_bytes, manifest_digest, sha256_prefixed};
pub use catalogue::{CatalogueListing, LocalCatalogue};
pub use category::ContentCategory;
pub use demo::{build_demo, build_demo_catalog, demo_catalog, seed_by_slug, DemoSeed};
pub use errors::InstrumentError;
pub use evaluators::{
    compile_receipt_quin, evaluate_contextual_assertions, evaluate_contribution_lifecycle,
    evaluate_core_semantic_instrument, evaluate_identifier_grant_scope,
    evaluate_licence_transition_readiness, evaluate_source_terminology,
    evaluate_wellfair_fairness_rights_review, execute_reference_instrument, AssertionKind,
    ContextualAssertionsReview, ContributionLifecycleCompleteness, CoreInstrumentKind,
    CoreInstrumentPayload, DiagnosticFinding, EvaluationOutcome, FairnessReviewDimension,
    IdentifierStewardshipGrantScope, InstrumentEvaluationReport, LicenceTransitionReadiness,
    SourceTerminologyReconciliation, WellfairFairnessRightsReview,
};
pub use execution::{preflight, run_entry, ExecutionReceipt, RunOutcome, RunRequest};
pub use manifest::{InstrumentManifest, COLLECTABLE_FORMAT_VERSION};
pub use manufacture::{
    assert_can_publish, inspect_for_publish, AuthoringLayer, PublishInspection, WASM_Q42_HELD,
};
pub use package::{
    build_collectable, hcf_from_manifest, open_collectable, CollectableParts, OpenedCollectable,
    KEY_GRAPH, KEY_HCF, KEY_MANIFEST, KEY_Q42, KEY_VISUAL,
};
pub use reference::{
    build_reference, core_instrument_by_slug, core_instrument_catalog, reference_by_slug,
    reference_catalog, ReferenceSeed,
};
pub use resolve::{
    plan_resolution, CatalogRecord, DependencyLock, DependencyReq, InstalledRelease, LocalRegistry,
};
pub use rpl::{evaluate_rpl, maybe_issue_award, EvidenceBasis, EvidenceItem, GapResult};
pub use visual::{demo_badge_10d, VISUAL_MEDIA_TYPE};
