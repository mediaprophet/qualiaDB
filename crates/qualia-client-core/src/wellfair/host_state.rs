//! Host operating-state DTOs consumed by the WellFair shell (Workstream 2).
//!
//! UI renders these snapshots; it does not derive policy or vault authority.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum VaultLifecycle {
    Unconfigured,
    Locked,
    Unlocked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NetworkExposure {
    Offline,
    LocalOnly,
    ExternalCapable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SyncQueueState {
    Idle,
    Queued,
    Sending,
    Acknowledged,
    Conflicted,
    Rejected,
    Revoked,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SensitivityClassDto {
    Public,
    Restricted,
    Classified,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityPreferences {
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub text_scale_percent: u8,
    pub screen_reader_hints: bool,
}

impl Default for AccessibilityPreferences {
    fn default() -> Self {
        Self {
            high_contrast: false,
            reduced_motion: false,
            text_scale_percent: 100,
            screen_reader_hints: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProvenanceHop {
    pub label: String,
    pub evidence_type: String,
    pub hash_prefix: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentGrantDraft {
    pub recipient: String,
    pub purpose: String,
    pub fields: Vec<String>,
    pub expires_at_unix: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyDecisionDto {
    Permit { obligations: Vec<String> },
    Deny { reasons: Vec<String> },
    Prompt { requested_consent: ConsentGrantDraft },
    Suspend { required_approvals: u8 },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WellfairHostSnapshot {
    pub vault: VaultLifecycle,
    pub network: NetworkExposure,
    pub sync_state: SyncQueueState,
    pub demo_mode: bool,
    pub owner_label: String,
    pub accessibility: AccessibilityPreferences,
    pub pending_jobs: u32,
    pub capabilities_ready: bool,
    pub host_api_version: String,
}

impl Default for WellfairHostSnapshot {
    fn default() -> Self {
        Self {
            vault: VaultLifecycle::Unconfigured,
            network: NetworkExposure::Offline,
            sync_state: SyncQueueState::Idle,
            demo_mode: false,
            owner_label: String::new(),
            accessibility: AccessibilityPreferences::default(),
            pending_jobs: 0,
            capabilities_ready: false,
            host_api_version: crate::qapp_install::SUPPORTED_HOST_API_VERSION.to_string(),
        }
    }
}

/// Phase 0 fixture snapshot until VaultService and IdentityService wire live state.
pub fn fixture_host_snapshot() -> WellfairHostSnapshot {
    WellfairHostSnapshot {
        vault: VaultLifecycle::Locked,
        network: NetworkExposure::LocalOnly,
        sync_state: SyncQueueState::Idle,
        demo_mode: false,
        owner_label: "Owner vault (fixture)".to_string(),
        accessibility: AccessibilityPreferences::default(),
        pending_jobs: 0,
        capabilities_ready: true,
        host_api_version: crate::qapp_install::SUPPORTED_HOST_API_VERSION.to_string(),
    }
}

pub fn demo_host_snapshot() -> WellfairHostSnapshot {
    WellfairHostSnapshot {
        vault: VaultLifecycle::Unlocked,
        network: NetworkExposure::Offline,
        sync_state: SyncQueueState::Idle,
        demo_mode: true,
        owner_label: "Demo persona (isolated)".to_string(),
        ..WellfairHostSnapshot::default()
    }
}