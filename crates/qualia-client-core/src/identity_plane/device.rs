//! Apparatus (device install) identity — one Qualia install on one machine.

use crate::setup::DeviceContext;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// What this apparatus can accept for placement (honest flags, not marketing).
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceCapabilities {
    /// Can run local job-queue work on this process.
    #[serde(default = "default_true")]
    pub local_jobs: bool,
    /// May host local inference when a model is loaded.
    #[serde(default = "default_true")]
    pub inference: bool,
    /// Mesh / peer transport keys are present.
    #[serde(default)]
    pub mesh_transport: bool,
}

fn default_true() -> bool {
    true
}

/// Full device record (no private keys — node secrets stay in `node_identity.json`).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceRecord {
    /// `did:q42:device:{node_identity_pubkey_hex}`.
    pub device_id: String,
    /// Person principal this apparatus is bound to.
    pub person_id: String,
    /// Ed25519 verifying key hex for the node identity (mesh/signing).
    pub identity_pubkey_hex: String,
    /// Human label (optional).
    #[serde(default)]
    pub label: String,
    /// OS hostname if known (informational only — not an identity).
    #[serde(default)]
    pub hostname: String,
    /// Situation of this machine (ownership, fleet, multi-user setting).
    #[serde(default)]
    pub device_context: DeviceContext,
    pub capabilities: DeviceCapabilities,
    /// HTTP control plane base URL for fleet job delivery (e.g. `http://192.168.1.10:8080`).
    /// Empty on pure-local installs; set so other apparatus can POST jobs here.
    #[serde(default)]
    pub control_base_url: String,
    /// True only for the install running in this process.
    pub is_local: bool,
    pub created_at_unix: u64,
    pub last_seen_unix: u64,
}

/// Public view (identical fields today; kept separate for API stability).
pub type DeviceRecordPublic = DeviceRecord;

impl DeviceRecord {
    pub fn device_did_from_pubkey_hex(pubkey_hex: &str) -> String {
        format!("did:q42:device:{}", pubkey_hex.trim().to_ascii_lowercase())
    }

    pub fn new_local(
        person_id: impl Into<String>,
        identity_pubkey_hex: impl Into<String>,
        device_context: DeviceContext,
        label: impl Into<String>,
    ) -> Self {
        let identity_pubkey_hex = identity_pubkey_hex.into().to_ascii_lowercase();
        let device_id = Self::device_did_from_pubkey_hex(&identity_pubkey_hex);
        let now = now_unix();
        let hostname = hostname_best_effort();
        Self {
            device_id,
            person_id: person_id.into(),
            identity_pubkey_hex,
            label: label.into(),
            hostname,
            device_context,
            capabilities: DeviceCapabilities {
                local_jobs: true,
                inference: true,
                mesh_transport: true,
            },
            control_base_url: String::new(),
            is_local: true,
            created_at_unix: now,
            last_seen_unix: now,
        }
    }

    pub fn touch(&mut self) {
        self.last_seen_unix = now_unix();
    }

    pub fn with_control_base_url(mut self, url: impl Into<String>) -> Self {
        self.control_base_url = url.into().trim().trim_end_matches('/').to_string();
        self
    }
}

fn hostname_best_effort() -> String {
    std::env::var("COMPUTERNAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_is_not_person_id_shape() {
        let d = DeviceRecord::new_local(
            "did:q42:person:aabb",
            "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff",
            DeviceContext::default(),
            "desk",
        );
        assert!(d.device_id.starts_with("did:q42:device:"));
        assert!(!d.device_id.starts_with("did:q42:person:"));
        assert_ne!(d.device_id, d.person_id);
    }
}
