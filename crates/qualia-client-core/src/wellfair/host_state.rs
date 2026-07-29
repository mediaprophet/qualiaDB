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
    Permit {
        obligations: Vec<String>,
    },
    Deny {
        reasons: Vec<String>,
    },
    Prompt {
        requested_consent: ConsentGrantDraft,
    },
    Suspend {
        required_approvals: u8,
    },
}

/// Outcome of a policy-gated write that may enter the guardianship escrow.
///
/// `Committed` — the record was written (quins materialized). `Suspended` — a proxy write of a
/// protected record is held pending M-of-N guardian co-signature; the returned `proposal_id`
/// identifies the pending [`crate::wellfair`] proposal in the approval tray.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", tag = "outcome")]
pub enum SubmitOutcome {
    Committed { quins: usize },
    Suspended { proposal_id: String, threshold: u8 },
}

/// UI view of a guardianship proposal + its derived approval status.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GuardianshipProposalView {
    pub proposal_id: String,
    pub principal_did: String,
    pub proxy_did: String,
    pub escrowed_kind: String,
    pub reason: String,
    pub created_unix: u32,
    /// "pending" | "ratified" | "denied".
    pub state: String,
    pub approvals: u8,
    pub threshold: u8,
    pub denied_by: Option<String>,
    pub denial_reason: Option<String>,
    /// Whether the escrowed record has been committed (true once ratified + written).
    pub committed: bool,
}

impl GuardianshipProposalView {
    pub fn from_status(
        proposal: &wellfare_core::guardianship::GuardianshipProposal,
        status: &wellfare_core::guardianship::ProposalStatus,
        committed: bool,
    ) -> Self {
        use wellfare_core::guardianship::ProposalState;
        let state = match status.state {
            ProposalState::Pending => "pending",
            ProposalState::Ratified => "ratified",
            ProposalState::Denied => "denied",
        }
        .to_string();
        Self {
            proposal_id: proposal.id.clone(),
            principal_did: proposal.principal_did.clone(),
            proxy_did: proposal.proxy_did.clone(),
            escrowed_kind: proposal.escrowed_kind.clone(),
            reason: proposal.reason.clone(),
            created_unix: proposal.created_unix,
            state,
            approvals: status.approvals,
            threshold: status.threshold,
            denied_by: status.denied_by.clone(),
            denial_reason: status.denial_reason.clone(),
            committed,
        }
    }
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
            accessibility: AccessibilityPreferences::default(),
            pending_jobs: 0,
            health_record_count: 0,
            graph_quin_count: 0,
            last_checkpoint_prefix: None,
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
        health_record_count: 0,
        graph_quin_count: 0,
        last_checkpoint_prefix: None,
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
