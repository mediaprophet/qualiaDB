//! Hostile-environment protection profiles (E16.1–E16.7). Named immutable
//! control sets. Negotiate only compatible profiles. Never silently lower
//! protection for cost (Recipe F).
//!
//! # What the endpoint cannot guarantee (E16.7)
//!
//! Remote wipe and attestation cannot guarantee safety after endpoint
//! compromise or seizure. An unlocked, coerced or imaged device can disclose
//! plaintext that was accessible at that moment. Crash dumps are redacted of
//! key material; notification previews are truncated. Hardware-backed key
//! references are unavailable unless a real token is present (this Linux CI
//! path does not fake a TPM). Cover traffic and padding do not constitute
//! anonymity; correlation resistance is unmeasured.

pub mod budget;
pub mod catalog;
pub mod cover;
pub mod gateway;
pub mod negotiate;
pub mod offline;
pub mod requirements;
pub mod scenarios;
pub mod vault;

pub use budget::{
    crypto_work_units, estimate_overhead, handshake_bytes_hybrid, pad_to_next_bucket,
    record_bucket, OverheadBudget, P1_PADDING_BUCKET, P2_RECORD_BYTES, P3_CELL_BYTES,
    P3_COVER_BYTES_PER_SEC, P3_RELAY_HOPS,
};
pub use catalog::{
    catalog_entry, control_set_from_predicates, digest_control_list, ControlId, ControlPredicate,
    ControlSet, ProtectionProfile, MAX_CONTROLS,
};
pub use cover::{
    approved_relay_required, correlation_resistance_measured, cover_class, cover_mandatory,
    cover_mandatory_controls, drop_cover_for_controls, drop_cover_for_cost,
    isolated_bearer_required, CoverClass,
};
pub use gateway::{gateway_transfer, GatewayMedia};
pub use negotiate::{
    bind_version_digest, negotiate, negotiate_outcome, CostPreference, NegotiateOutcome,
    SelectedProfile,
};
pub use offline::{
    activate_capability, extend_capability_by_copy, queue_offline, release_offline, release_queued,
    OfflinePackage, OfflineQueue, ScopedCapability, MAX_OFFLINE,
};
pub use requirements::{
    min_profile_for_confidentiality, required_control_set, AssessedEnvironment, RequirementContext,
    SenderPolicy, TaskClass,
};
pub use vault::{
    crash_dump_redacted, hardware_key_ref_supported, load_key_ref, load_ref, lock,
    notification_preview_max_bytes, require_hardware_backed_p4, store_ref, KeyRef, LocalVault,
    VaultSlot, NOTIFICATION_PREVIEW_MAX_BYTES, VAULT_SLOTS,
};

// Concurrent host tests call `Result<(NativePeer, NativePeer), _>::unwrap_err`,
// which requires Debug. Host is outside this lane; this test-only impl is
// opaque and does not dump keys or session state.
#[cfg(test)]
impl core::fmt::Debug for crate::net::peer::host::NativePeer {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str("NativePeer")
    }
}
