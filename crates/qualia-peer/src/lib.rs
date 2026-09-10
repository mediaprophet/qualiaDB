//! Qualia Peer Runtime facade.
//!
//! This crate is the application replacement for libp2p. It does not import
//! `libp2p`. It enables core-db `qdnf` only (no `libp2p-compat`, `gpu-runtime`,
//! `wgsl-forge`, `privacy-he`, `zk-culling`, or `profile_target_1024`).
//! Default `qualia-core-db` is Native Independent (`libp2p-compat` is LIG-only).
//! Packages remain open.
//!
//! Application API: [`PeerHost::pair`], [`PeerHost::exchange_protected`],
//! and [`PeerHost::clinical_session`]. The 64-byte vertical slice
//! (`two_peer_ipc_exchange`) is an example, not the application API.

mod api;
mod clinical;
mod inventory;
mod native_independent;
mod negotiate;
mod ops;

pub use api::PeerHost;
pub use clinical::ClinicalSession;
pub use inventory::{libp2p_imported, public_entry_points, InventoryEntry};
pub use native_independent::{
    cargo_toml_depends_on_libp2p, implicit_dns_ip_fallback, native_independent_daemon_proven,
    remaining_libp2p_couplings, Libp2pCoupling,
};
pub use negotiate::{
    admit_capability_generation, negotiate_capability, pin_capability_version, verify_update_bytes,
    CapabilityVersion,
};
pub use ops::{
    active_session_count, admit_scoped_credential, denied_op_count,
    patient_identifiers_on_dashboard, realistic_device_study_executed, redacted_dashboard,
    revoked_credential_count, safe_failure, universal_org_superuser, CredentialScope,
    RedactedDashboard,
};
pub use qualia_core_db::net::peer::cells::{extra_identity_multiplies_host_budget, HostAdmission};
pub use qualia_core_db::net::peer::host::exchange::{
    authorised_ipc_stream_exchange_in, pair_ipc_cells, AUTHORISED_IPC_MTU, AUTHORISED_PAYLOAD_CAP,
    AUTHORISED_ROUNDTRIP_CAP,
};
pub use qualia_core_db::net::peer::host::{
    authorised_ipc_stream_exchange, native_ipc_stream_exchange, ControllerIdentity, NativePeer,
    PeerBuilder, ServiceId, CELL_BYTES_DEFAULT,
};
pub use qualia_core_db::net::peer::runtime::{
    LeaseTable, ReservationHandle, ReservationLedger, ResourceBudget,
};
pub use qualia_core_db::net::qdnf::authority::{Plane, PolicyOutcome};
pub use qualia_core_db::net::qdnf::bearer::{ipc_pair, Bearer, IpcEndpoint};
pub use qualia_core_db::net::qdnf::errors::QdnfError;
pub use qualia_core_db::net::qdnf::session::{SessionBinding, SessionState};
pub use qualia_core_db::net::qdnf::types::{LinkId, ObservedLocator, OperationId, ScopeEpoch};
pub use qualia_core_db::net::qdnf::vertical::two_peer_ipc_exchange;
