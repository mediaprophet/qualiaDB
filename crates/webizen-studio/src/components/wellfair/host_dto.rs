//! WellFair host DTOs — mirror `qualia-client-core::wellfair::host_state` for UI rendering.

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

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityPreferences {
    pub high_contrast: bool,
    pub reduced_motion: bool,
    pub text_scale_percent: u8,
    pub screen_reader_hints: bool,
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
    pub health_record_count: u32,
    pub graph_quin_count: u32,
    pub last_checkpoint_prefix: Option<String>,
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
            accessibility: AccessibilityPreferences {
                high_contrast: false,
                reduced_motion: false,
                text_scale_percent: 100,
                screen_reader_hints: true,
            },
            pending_jobs: 0,
            health_record_count: 0,
            graph_quin_count: 0,
            last_checkpoint_prefix: None,
            capabilities_ready: false,
            host_api_version: "1".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthRecordDto {
    pub id: String,
    pub kind: String,
    pub asserted_time_unix: u32,
    pub evidence_type: String,
    pub sensitivity: String,
    pub blob_hash: Option<String>,
    pub source: String,
    pub committed_unix: u32,
    #[serde(default)]
    pub summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorDto {
    pub id: String,
    pub actor_type: String,
    pub name: String,
    pub organization: Option<String>,
    pub roles: Vec<String>,
    pub verification_status: String,
    pub pairwise_did: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DelegationRuleDto {
    pub id: String,
    pub actor_id: String,
    pub granted_roles: Vec<String>,
    pub legal_basis: String,
    pub is_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConsentGrantDto {
    pub id: String,
    pub recipient: String,
    pub purpose: String,
    pub fields: Vec<String>,
    pub scope: String,
    pub granted_at_unix: u32,
    pub expires_at_unix: Option<u64>,
    pub revoked: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GraphCoverageDto {
    pub record_id: String,
    pub kind: String,
    pub quin_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReceiptDto {
    pub id: String,
    pub timestamp_unix: u32,
    pub qapp_id: String,
    pub record_id: String,
    pub decision: String,
    pub obligations: Vec<String>,
    pub checkpoint_hash: Option<String>,
}

pub fn fixture_snapshot() -> WellfairHostSnapshot {
    WellfairHostSnapshot {
        vault: VaultLifecycle::Locked,
        network: NetworkExposure::LocalOnly,
        sync_state: SyncQueueState::Idle,
        capabilities_ready: true,
        owner_label: "Owner vault (fixture)".to_string(),
        ..WellfairHostSnapshot::default()
    }
}