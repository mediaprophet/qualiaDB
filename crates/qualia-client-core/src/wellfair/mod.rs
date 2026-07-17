pub mod accessibility_prefs;
pub mod anatomy_body;
pub mod anatomy_render;
pub mod anatomy_assets;
/// Producer for a curated `.qualia` anatomy asset pack (ships in the release /
/// web demo). Native-only (blocking network I/O against the HRA endpoints).
pub mod anatomy_pack;
pub mod anatomy_view;
pub mod api;
pub mod anatomy_dyad;
/// Ingest BodyParts3D (FMA-keyed, CC-BY-SA) — the muscles/bones/glands/nerves that complete the body
/// CCF/HRA (viscera-only) cannot. Pure part-of→system mapping + a native fetch/pack producer.
pub mod bodyparts3d_resolver;
/// The BodyParts3D anatomy ONTOLOGY emitter → `.q42` (OBO FMA IRIs + house aliases, is-a + part-of +
/// system + geometry). The addressable semantic backbone the `.10d` mesh library is cited by.
pub mod bodyparts3d_ontology;
pub mod ccf_resolver;
pub mod fetal_stages;
pub mod backup;
pub mod blob_store;
pub mod companion_tests;
pub mod consent_store;
pub mod export_package;
pub mod graph_query;
pub mod personal_profile;
pub mod physiology_prefs;
pub mod checkpoint_store;
pub mod graph_store;
pub mod host_state;
pub mod cml_context;
pub mod hypermedia_store;
pub mod bookmarks;
pub mod legislation_ingest;
pub mod qapp_catalog;
pub mod perception_catalog;
pub mod import_samsung;
pub mod ingest_guardian;
pub mod journal;
pub mod live_share;
pub mod med_reminders;
#[cfg(test)]
mod medication_flow;
pub mod policy;
#[cfg(not(target_arch = "wasm32"))]
pub mod qapp_publish;
pub mod receipt;
pub mod scorecard_prefs;
pub mod sanctuary;
#[cfg(not(target_arch = "wasm32"))]
pub mod sanctuary_vault;
// The v2 vault container is a native-only on-disk format (the desktop owns keys + the vault) and it
// persists the native-only audit DAG; gate it to non-wasm like `sanctuary_vault`.
#[cfg(not(target_arch = "wasm32"))]
pub mod vault_container;
pub mod snapshot;
pub mod sync_outbox;
pub mod sync_protocol;
pub mod sync_transport;
// The HTTP relay server is native-only (tiny_http).
#[cfg(not(target_arch = "wasm32"))]
pub mod sync_relay_server;
pub mod vault;

#[cfg(test)]
mod replay_tests;

#[cfg(test)]
mod journey_tests;

#[cfg(test)]
mod phase3_tests;

#[cfg(test)]
mod phase4_tests;

pub use host_state::{
    demo_host_snapshot, fixture_host_snapshot, WellfairHostSnapshot,
};
pub use import_samsung::{
    ingest_companion_health_bundle, parse_csv_named_content, SamsungFileReport, SamsungImportReport,
};
pub use snapshot::build_host_snapshot;
