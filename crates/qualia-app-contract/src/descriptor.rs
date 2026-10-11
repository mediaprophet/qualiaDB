//! Command descriptor definitions and availability status.

use crate::effects::{EffectSet, InteractionKind, UndoPolicy};
use crate::error::ContractError;
use crate::id::{AppId, CommandId};
use serde::{Deserialize, Serialize};

/// Target kind on which a command operates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    TextSelection,
    CellRange,
    Layer,
    VectorPath,
    TimelineClip,
    CadEntity,
    Document,
    Scene,
    Generic,
}

/// Dynamic availability status of a command given current target and session state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum Availability {
    Available,
    Disabled {
        reason_code: String,
        localized_reason: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        missing_prerequisite: Option<String>,
    },
    Hidden {
        reason_code: String,
    },
}

impl Availability {
    pub fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    pub fn disabled(code: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Disabled {
            reason_code: code.into(),
            localized_reason: reason.into(),
            missing_prerequisite: None,
        }
    }
}

/// Canonical metadata descriptor for an application command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandDescriptor {
    pub schema_version: u32,
    pub command_id: CommandId,
    pub command_version: String,
    pub app_id: AppId,
    pub upstream_id: String,
    pub label_key: String,
    pub description_key: String,
    pub input_schema: serde_json::Value,
    pub output_schema: serde_json::Value,
    pub target_kinds: Vec<TargetKind>,
    pub effects: EffectSet,
    pub required_capabilities: Vec<String>,
    pub undo_policy: UndoPolicy,
    pub interaction: InteractionKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub menu: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toolbox: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_chain: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub orb_group: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shortcuts: Vec<String>,
}

impl CommandDescriptor {
    /// Validate descriptor fields against the canonical contract.
    pub fn validate(&self) -> Result<(), ContractError> {
        if self.schema_version != 1 {
            return Err(ContractError::UnknownVersion(self.schema_version));
        }

        if self.label_key.trim().is_empty() {
            return Err(ContractError::MalformedParams("label_key cannot be empty".into()));
        }

        // Schema must be valid JSON Schema object or boolean
        if !self.input_schema.is_object() && !self.input_schema.is_boolean() && !self.input_schema.is_null() {
            return Err(ContractError::MalformedParams("input_schema must be an object or boolean".into()));
        }

        if !self.output_schema.is_object() && !self.output_schema.is_boolean() && !self.output_schema.is_null() {
            return Err(ContractError::MalformedParams("output_schema must be an object or boolean".into()));
        }

        // Validate command ID matches app ID
        if self.command_id.app_id() != self.app_id.as_str() {
            return Err(ContractError::InvalidCommandIdFormat(format!(
                "command ID app prefix '{}' does not match app ID '{}'",
                self.command_id.app_id(),
                self.app_id.as_str()
            )));
        }

        Ok(())
    }
}
