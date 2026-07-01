//! Derive `WellfairHostSnapshot` from live host services (not Dioxus signals).

use super::host_state::{
    AccessibilityPreferences, NetworkExposure, SyncQueueState, VaultLifecycle, WellfairHostSnapshot,
};
use qualia_core_db::key_vault::KeyVault;

/// Build operating-state snapshot from KeyVault + Host API readiness.
pub fn build_host_snapshot(
    key_vault: &KeyVault,
    host_api_ready: bool,
    owner_label: &str,
    demo_mode: bool,
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
        accessibility: AccessibilityPreferences::default(),
        pending_jobs: 0,
        capabilities_ready: host_api_ready && !key_vault.is_locked(),
        host_api_version: crate::qapp_install::SUPPORTED_HOST_API_VERSION.to_string(),
    }
}