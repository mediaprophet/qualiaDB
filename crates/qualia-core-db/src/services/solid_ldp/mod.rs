//! Solid leave/migrate — project Qualia Quins to portable RDF for Solid Pods.
//!
//! # Architecture
//!
//! ```text
//! QualiaDB / Webizen Desktop          Solid Pod / Solid-Databox
//! (48-byte Quins, Q42, lexicon)  →    (triples / quads: Turtle, JSON-LD, N-Quads)
//!                                    ←  (wasm-webcivics parse → Qualia backup)
//! ```
//!
//! Solid does **not** store Quins. The leave path is a **projector**: the user
//! must choose omit-sanctuary vs reclassify-for-solid when sanctuary data exists;
//! classified never egresses. Solid-Databox imports with LDP PUT and may use
//! licensed `wasm-webcivics` for backup import/export bridges.
//!
//! G-SOLID-IDP stays parked: QualiaDB remains the identity/capability source for
//! in-ecosystem work; this module is the off-ramp / on-ramp, not an IdP.

mod bundle;
mod consent;
#[cfg(not(target_arch = "wasm32"))]
mod export_q42;
mod filter;
mod import_rdf;
mod terms;

pub use bundle::{
    build_migration_bundle, SolidBundleFormats, SolidMigrationBundle, SolidMigrationOptions,
};
pub use consent::{
    count_classified_quins, count_sanctuary_quins, export_outcome_summary,
    plan_sanctuary_migration_notice, require_sanctuary_choice, SanctuaryExportChoice,
    SanctuaryMigrationNotice,
};
#[cfg(not(target_arch = "wasm32"))]
pub use export_q42::SolidExporter;
pub use filter::{filter_for_egress, may_egress, EgressStats, SolidEgressPolicy};
pub use import_rdf::{import_rdf_to_quins, solid_rdf_to_qualia_backup_json};

/// Backward-compat stub used by an existing lib test (RDF-star mapping demo).
pub struct SolidLdpFacade;

impl SolidLdpFacade {
    pub fn serialize_to_rdf_star(quin: &crate::NQuin) -> String {
        format!(
            "GRAPH <urn:qualia:context:{}> {{ geo:asWKT qualia:hardwareIntegrity \"VERIFIED_ECC_PASS\" }}",
            quin.context
        )
    }
}
