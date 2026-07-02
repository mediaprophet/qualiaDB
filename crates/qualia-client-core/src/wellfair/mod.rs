pub mod accessibility_prefs;
pub mod api;
pub mod companion_tests;
pub mod consent_store;
pub mod export_package;
pub mod graph_query;
pub mod personal_profile;
pub mod checkpoint_store;
pub mod graph_store;
pub mod host_state;
pub mod import_samsung;
pub mod journal;
pub mod live_share;
pub mod med_reminders;
#[cfg(test)]
mod medication_flow;
pub mod policy;
pub mod receipt;
pub mod sanctuary;
pub mod snapshot;
pub mod sync_outbox;
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
