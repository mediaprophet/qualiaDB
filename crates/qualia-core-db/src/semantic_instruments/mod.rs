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
pub mod reference;
pub mod execution;
pub mod manufacture;
pub mod manifest;
pub mod package;
pub mod resolve;
pub mod rpl;
pub mod visual;

pub use canonical::{canonical_manifest_bytes, manifest_digest, sha256_prefixed};
pub use category::ContentCategory;
pub use demo::{build_demo, build_demo_catalog, demo_catalog, seed_by_slug, DemoSeed};
pub use reference::{build_reference, reference_by_slug, reference_catalog, ReferenceSeed};
pub use errors::InstrumentError;
pub use manifest::{InstrumentManifest, COLLECTABLE_FORMAT_VERSION};
pub use package::{
    build_collectable, hcf_from_manifest, open_collectable, CollectableParts, OpenedCollectable,
    KEY_GRAPH, KEY_HCF, KEY_MANIFEST, KEY_Q42, KEY_VISUAL,
};
pub use visual::{demo_badge_10d, VISUAL_MEDIA_TYPE};
pub use resolve::{
    plan_resolution, CatalogRecord, DependencyLock, DependencyReq, InstalledRelease, LocalRegistry,
};
pub use execution::{preflight, run_entry, ExecutionReceipt, RunOutcome, RunRequest};
pub use attestation::{
    AttestationKind, AttestationStatus, InstrumentAttestation, StatusRegistry,
    issue_capability_award, issue_native, issue_w3c, verify_native, verify_w3c, badge_export,
};
pub use catalogue::{CatalogueListing, LocalCatalogue};
pub use rpl::{evaluate_rpl, maybe_issue_award, EvidenceBasis, EvidenceItem, GapResult};
pub use manufacture::{assert_can_publish, inspect_for_publish, AuthoringLayer, PublishInspection, WASM_Q42_HELD};
