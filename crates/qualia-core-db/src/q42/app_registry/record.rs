//! Installed-app record types (read-only inspection surface).

use crate::q42::app_manifest::{PermissionIntent, PermissionKind};

/// Lifecycle / admission state for a registry slot.
///
/// The registry never launches or executes an app in any of these states;
/// inspection is always read-only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppRecordState {
    /// Decoded, integrity ok (when checked), engine compatible.
    Active,
    /// Malformed, integrity mismatch, or other fail-closed admission issue.
    Quarantined { reason: String },
    /// Manifest requires a newer (or older) engine than this registry instance.
    Incompatible,
}

impl AppRecordState {
    pub fn is_active(&self) -> bool {
        matches!(self, Self::Active)
    }

    pub fn is_quarantined(&self) -> bool {
        matches!(self, Self::Quarantined { .. })
    }
}

/// Compact permission-intent summary for list/get views.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PermissionIntentSummary {
    /// Distinct permission kind tags (`PermissionKind::as_str`).
    pub kinds: Vec<&'static str>,
    pub total: usize,
    pub required: usize,
}

impl PermissionIntentSummary {
    pub fn from_intents(intents: &[PermissionIntent]) -> Self {
        let mut kinds = Vec::new();
        let mut required = 0usize;
        for intent in intents {
            let tag = intent.kind.as_str();
            if !kinds.contains(&tag) {
                kinds.push(tag);
            }
            if !intent.optional {
                required += 1;
            }
        }
        // Stable order by wire privilege rank then tag.
        kinds.sort_by_key(|k| {
            PermissionKind::parse(k)
                .map(|pk| (pk.privilege_rank(), pk.as_str()))
                .unwrap_or((255, *k))
        });
        Self {
            kinds,
            total: intents.len(),
            required,
        }
    }
}

/// One installed (or quarantined / incompatible) app slot.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledAppRecord {
    pub app_id: String,
    pub version: String,
    /// Package SHA-256 from the manifest integrity block (may be zero when unknown).
    pub digest: [u8; 32],
    pub permission_intents: PermissionIntentSummary,
    /// `true` when a caller-supplied package digest matched (or no digest was supplied).
    pub integrity_ok: bool,
    pub state: AppRecordState,
}

/// Read-only inspection view — never mutates registry state and never launches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInspectView {
    pub app_id: String,
    pub version: String,
    pub author_name: String,
    pub author_did: String,
    pub digest: [u8; 32],
    pub integrity_ok: bool,
    pub state: AppRecordState,
    pub permission_intents: Vec<PermissionIntent>,
    pub permission_summary: PermissionIntentSummary,
    pub entry_ids: Vec<String>,
    pub min_engine_version: String,
    pub max_engine_version: String,
    pub required_features: Vec<String>,
}
