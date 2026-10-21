use crate::effects::CommandEffect;
use crate::id::{AppId, DecimalU64};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationMode {
    Observe,
    Propose,
    ApproveEachAction,
    BoundedDelegation,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DelegationLifecycle {
    Submitted,
    Validating,
    AwaitingApproval,
    Approved,
    Running,
    Paused,
    Cancelling,
    Completed,
    Failed,
    Cancelled,
    Revoked,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ResourceBudgets {
    pub max_steps: u32,
    pub max_cpu_time_ms: u64,
    pub max_memory_bytes: u64,
    pub external_cost_minor_units: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskDelegation {
    pub delegation_id: String,
    pub principal_id: String,
    pub actor_id: String,
    pub app_id: AppId,
    pub goal: String,
    pub allowed_effects: Vec<CommandEffect>,
    pub allowed_command_ids: Vec<String>,
    pub mode: DelegationMode,
    pub state: DelegationLifecycle,
    pub budgets: ResourceBudgets,
    pub expiry_timestamp_ms: DecimalU64,
}

impl TaskDelegation {
    /// `true` when active.
    pub fn is_active(&self) -> bool {
        matches!(self.state, DelegationLifecycle::Approved | DelegationLifecycle::Running)
    }

    /// `true` when effect-allowed.
    pub fn is_effect_allowed(&self, effect: &CommandEffect) -> bool {
        self.allowed_effects.contains(effect)
    }

    /// `true` when command-allowed.
    pub fn is_command_allowed(&self, command_id: &str) -> bool {
        self.allowed_command_ids.is_empty() || self.allowed_command_ids.iter().any(|id| id == command_id)
    }

    /// Revokes.
    pub fn revoke(&mut self) {
        self.state = DelegationLifecycle::Revoked;
    }

    /// Pauses.
    pub fn pause(&mut self) {
        if self.state == DelegationLifecycle::Running {
            self.state = DelegationLifecycle::Paused;
        }
    }
}
