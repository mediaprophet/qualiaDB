use super::super::{AuthoringAction, DocumentRevision};
use crate::agency_delegation::AgencyDelegation;
pub use qualia_app_contract::{
    AppId, CommandEffect, DecimalU64, DelegationLifecycle, DelegationMode, ResourceBudgets,
    TaskDelegation,
};
use serde::{Deserialize, Serialize};

pub const MAX_AUTHORING_DELEGATION_DEPTH: usize = 8;
pub const MAX_AUTHORING_DELEGATION_BYTES: usize = 64 * 1024;
pub const AUTHORING_DATA_CLASS: &str = "native_authoring_source";

/// Signed consent bytes bind all shared agency/task semantics, source scope and
/// controller associations. This asserted structure is not admitted authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthoringDelegation {
    pub version: u32,
    pub root_subject: String,
    pub root_adoption_digest: [u8; 32],
    pub project_id: String,
    pub document_id: String,
    pub source_id: String,
    pub grantor_subject: String,
    pub grantor_controller: [u8; 32],
    pub grantee_subject: String,
    pub grantee_controller: [u8; 32],
    pub parent_digest: Option<[u8; 32]>,
    /// Zero forbids onward delegation; a child must strictly decrease it.
    pub remaining_depth: u8,
    pub actions: Vec<AuthoringAction>,
    pub agency: AgencyDelegation,
    /// Goal is the exact adopted purpose. Empty command lists grant nothing in
    /// this native adapter, regardless of the generic membership helper.
    pub task: TaskDelegation,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SignedAuthoringDelegation {
    pub delegation: AuthoringDelegation,
    pub signature: Vec<u8>,
}

/// Requested context is compared with host-admitted bytes; it mints no rights.
pub struct DelegatedOperation<'a> {
    pub actor_subject: &'a str,
    pub actor_controller: &'a [u8; 32],
    pub purpose: &'a str,
    pub command_id: &'a str,
    pub action: AuthoringAction,
    pub effects: &'a [CommandEffect],
    pub target: &'a DocumentRevision,
    pub now_unix: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DelegationError {
    Bounds,
    Binding,
    Consent,
    ExpiredOrRevoked,
    UnsupportedPolicy,
    Scope,
    Attenuation,
    Budget,
}
