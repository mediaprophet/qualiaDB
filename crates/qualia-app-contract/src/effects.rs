//! Command effect classification, undo policy, and interaction semantics.

use crate::error::ContractError;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fmt;

/// Individual effect of an executed command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CommandEffect {
    /// Read-only inspection of document, workspace, or session state.
    Read,
    /// Mutation confined to the active document state.
    DocumentMutation,
    /// Mutation to workspace, catalog, or project state outside the document.
    WorkspaceMutation,
    /// Persistent write to local storage, OPFS, or filesystem.
    FilesystemWrite,
    /// Disclosure of data or telemetry over network or remote transport.
    NetworkDisclosure,
    /// Capture from hardware device (camera, microphone, tablet, keyboard).
    DeviceCapture,
    /// Invocation causing external financial charge or credit deduction.
    ExternalCharge,
    /// Lifecycle transition of a package, worker, or application service.
    PackageLifecycle,
}

impl CommandEffect {
    /// `&str` form.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Read => "read",
            Self::DocumentMutation => "document_mutation",
            Self::WorkspaceMutation => "workspace_mutation",
            Self::FilesystemWrite => "filesystem_write",
            Self::NetworkDisclosure => "network_disclosure",
            Self::DeviceCapture => "device_capture",
            Self::ExternalCharge => "external_charge",
            Self::PackageLifecycle => "package_lifecycle",
        }
    }

    /// Constructs from str-strict.
    pub fn from_str_strict(s: &str) -> Result<Self, ContractError> {
        match s {
            "read" => Ok(Self::Read),
            "document_mutation" => Ok(Self::DocumentMutation),
            "workspace_mutation" => Ok(Self::WorkspaceMutation),
            "filesystem_write" => Ok(Self::FilesystemWrite),
            "network_disclosure" => Ok(Self::NetworkDisclosure),
            "device_capture" => Ok(Self::DeviceCapture),
            "external_charge" => Ok(Self::ExternalCharge),
            "package_lifecycle" => Ok(Self::PackageLifecycle),
            other => Err(ContractError::UnknownEffect(other.to_string())),
        }
    }
}

impl fmt::Display for CommandEffect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Set of effects associated with a command descriptor or execution receipt.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(transparent)]
pub struct EffectSet(BTreeSet<CommandEffect>);

impl EffectSet {
    /// Empty instance.
    pub fn empty() -> Self {
        Self(BTreeSet::new())
    }

    /// Single-element set.
    pub fn single(effect: CommandEffect) -> Self {
        let mut set = BTreeSet::new();
        set.insert(effect);
        Self(set)
    }

    /// Inserts.
    pub fn insert(&mut self, effect: CommandEffect) {
        self.0.insert(effect);
    }

    /// `true` when it contains the item.
    pub fn contains(&self, effect: CommandEffect) -> bool {
        self.0.contains(&effect)
    }

    /// `true` when read-only.
    pub fn is_read_only(&self) -> bool {
        self.0.len() == 1 && self.0.contains(&CommandEffect::Read)
    }

    /// Iterator.
    pub fn iter(&self) -> std::collections::btree_set::Iter<'_, CommandEffect> {
        self.0.iter()
    }
}

impl FromIterator<CommandEffect> for EffectSet {
    fn from_iter<T: IntoIterator<Item = CommandEffect>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

/// The undoability contract for a command.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UndoPolicy {
    /// Command cannot be undone (e.g. external disclosure or financial charge).
    None,
    /// Command can be reversed cleanly within the local document transaction stack.
    ReversibleDocument,
    /// Command has external side effects that require a compensating action.
    CompensatingExternal,
}

/// The interaction kind defining how a command is triggered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InteractionKind {
    /// Point-in-time discrete action (e.g. button click or keyboard shortcut).
    Discrete,
    /// Interactive gesture with begin/update/commit/cancel phases (e.g. pen stroke or drag).
    Gesture,
    /// Read-only query or inspection.
    Query,
    /// View/presentation transition without document mutation.
    View,
    /// Long-running asynchronous background job.
    Job,
}
