pub mod accessibility_prefs;
pub mod anatomy_body;
pub mod anatomy_render;
pub mod anatomy_assets;
pub mod anatomy_view;
pub mod api;
pub mod anatomy_dyad;
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
pub mod hypermedia_store;
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
