//! Derive `WellfairHostSnapshot` from live host services (not Dioxus signals).

use super::accessibility_prefs;
use super::host_state::{
    NetworkExposure, SyncQueueState, VaultLifecycle, WellfairHostSnapshot,
};
use qualia_core_db::key_vault::KeyVault;
use std::path::Path;

/// Build operating-state snapshot from KeyVault + Host API readiness.
pub fn build_host_snapshot(
    key_vault: &KeyVault,
    host_api_ready: bool,
    owner_label: &str,
    demo_mode: bool,
) -> WellfairHostSnapshot {
    build_host_snapshot_with_storage(key_vault, host_api_ready, owner_label, demo_mode, None)
}

pub fn build_host_snapshot_with_storage(
    key_vault: &KeyVault,
    host_api_ready: bool,
    owner_label: &str,
    demo_mode: bool,
    storage_root: Option<&Path>,
) -> WellfairHostSnapshot {
    let vault = if demo_mode {
        VaultLifecycle::Unlocked
    } else if key_vault.is_locked() {
        VaultLifecycle::Locked
    } else if host_api_ready {
        VaultLifecycle::Unlocked
    } else {
        VaultLifecycle::Unconfigured
    };

    WellfairHostSnapshot {
        vault,
        network: NetworkExposure::LocalOnly,
        sync_state: SyncQueueState::Idle,
        demo_mode,
        owner_label: if owner_label.is_empty() {
            "Owner vault".to_string()
        } else {
            owner_label.to_string()
        },
        accessibility: storage_root
            .map(accessibility_prefs::load)
            .unwrap_or_default(),
        pending_jobs: 0,
        health_record_count: 0,
        last_checkpoint_prefix: None,
        capabilities_ready: host_api_ready && !key_vault.is_locked(),
        host_api_version: crate::qapp_install::SUPPORTED_HOST_API_VERSION.to_string(),
    }
}