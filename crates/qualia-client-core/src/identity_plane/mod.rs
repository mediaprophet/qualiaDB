//! Person ≠ machine ≠ OS-account identity plane.
//!
//! # Model
//!
//! | Principal | Meaning | Portable? |
//! |-----------|---------|-----------|
//! | **Person** | Natural-person principal who controls Webizen | Yes — import/export across machines |
//! | **Device (apparatus)** | One install of Qualia on one machine | No — unique per install |
//! | **OS account** | Windows/macOS/Linux login | Not used as identity |
//!
//! A person may run Qualia on several devices. Jobs may name a target
//! `device_id`; work runs locally only when the target is this apparatus
//! (or unspecified). Remote dispatch is fail-closed until a live transport
//! is wired — fleet registration still records peer devices for placement.
//!
//! Secrets stay in app meta (`person_identity.json`, `node_identity.json`,
//! `device_fleet.json`). Public snapshots never include private keys.

mod device;
mod fleet;
mod person;

pub use device::{DeviceCapabilities, DeviceRecord, DeviceRecordPublic};
pub mod fleet_jobs;

pub use fleet::{
    ensure_local_apparatus, export_person_public, export_person_transfer_bundle,
    get_identity_plane, import_person_transfer_bundle, list_devices, register_remote_device,
    resolve_job_placement, set_local_control_base_url, sync_local_device_context,
    IdentityPlaneSnapshot, JobPlacement,
};
pub use fleet_jobs::{
    accept_fleet_job_envelope, deliver_or_queue_remote_job, list_remote_outbox, FleetJobEnvelope,
    RemoteOutboxEntry,
};
pub use person::{PersonPrincipal, PersonPublic, PersonTransferBundle};
