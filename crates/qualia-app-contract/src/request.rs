//! Command requests, receipts, session descriptors, surfaces, and job DTOs.

use crate::effects::EffectSet;
use crate::id::{AppId, CommandId, DecimalU64};
use crate::target::CommandTarget;
use serde::{Deserialize, Serialize};

/// Execution status reported in a CommandReceipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandStatus {
    Success,
    Failure,
    Pending,
    Cancelled,
}

/// Normalized command execution request.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandRequest {
    pub request_id: String,
    pub command_id: CommandId,
    pub command_version: String,
    pub target: CommandTarget,
    pub params: serde_json::Value,
    pub expected_revision: DecimalU64,
    pub actor_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delegation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub approval_id: Option<String>,
    pub idempotency_key: String,
}

/// Cryptographically verifiable or auditable command execution receipt.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandReceipt {
    pub request_id: String,
    pub actor_id: String,
    pub app_id: AppId,
    pub command_id: CommandId,
    pub target_id: String,
    pub before_revision: DecimalU64,
    pub after_revision: DecimalU64,
    pub status: CommandStatus,
    pub effects: EffectSet,
    pub result: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub job_handle: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub authority_decision_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_cents: Option<DecimalU64>,
}

/// Metadata descriptor for an active application session.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppSessionDescriptor {
    pub app_version: String,
    pub package_digest: String,
    pub engine_version: String,
    pub schema_version: u32,
    pub session_id: String,
    pub document_id: String,
    pub open_mode: String,
    pub revision: DecimalU64,
    pub is_dirty: bool,
    pub is_durable: bool,
    pub execution_source: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payload_refs: Vec<String>,
}

/// Descriptor for a presentation or interactive surface.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SurfaceDescriptor {
    pub surface_id: String,
    pub surface_version: String,
    pub kind: String,
    pub document_binding: String,
    pub view_type: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required_capabilities: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub input_kinds: Vec<String>,
    pub accessibility_projection: String,
    pub state_schema: String,
    pub resource_budget_class: String,
}

/// Lifecycle state of an asynchronous domain job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum JobState {
    Queued,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

/// Descriptor for tracking background or long-running tasks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct JobDescriptor {
    pub job_id: String,
    pub request_id: String,
    pub actor_id: String,
    pub target_id: String,
    pub state: JobState,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completed_units: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_units: Option<u64>,
    pub cancellable: bool,
    pub external_effect_state: String,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result_ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provenance_ref: Option<String>,
}
