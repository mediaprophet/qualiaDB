use std::path::Path;

use super::import_samsung::{
    import_samsung_folder, ingest_companion_health_bundle, SamsungImportReport,
};
use wellfare_core::companion_sync::CompanionHealthBundle;
use super::policy::{DecisionResult, PolicyDecisionService};
use super::snapshot::build_host_snapshot;
use super::vault::VaultService;
use super::host_state::WellfairHostSnapshot;
use ed25519_dalek::SigningKey;
use qualia_core_db::key_vault::KeyVault;
use wellfare_core::record::RecordEnvelope;

/// Transport-neutral Host API exported for UI and qApps.
pub struct WebizenHostApi {
    vault: VaultService,
    policy: PolicyDecisionService,
    signing_key: SigningKey,
    owner_did: String,
    author_did: String,
}

impl WebizenHostApi {
    pub fn new(
        vault: VaultService,
        policy: PolicyDecisionService,
        signing_key: SigningKey,
        owner_did: String,
        author_did: String,
    ) -> Self {
        Self {
            vault,
            policy,
            signing_key,
            owner_did,
            author_did,
        }
    }

    pub fn snapshot_from_vault(key_vault: &KeyVault, owner_label: &str, demo_mode: bool) -> WellfairHostSnapshot {
        build_host_snapshot(key_vault, true, owner_label, demo_mode)
    }

    pub fn submit_record(&mut self, qapp_id: &str, envelope: RecordEnvelope) -> Result<usize, String> {
        let decision = self.policy.evaluate_access(
            qapp_id,
            "write_record",
            envelope.sensitivity,
            envelope.epistemic_status,
        );

        match decision {
            DecisionResult::Deny { reasons } => {
                Err(format!("Policy denied: {}", reasons.join("; ")))
            }
            DecisionResult::Prompt { .. } | DecisionResult::Suspend { .. } => {
                Err("Policy requires consent or guardian approval before write".into())
            }
            DecisionResult::Permit { .. } => {
                let principal_did = qualia_core_db::q_hash(&self.owner_did);
                self.vault
                    .commit_envelope(&envelope, &self.signing_key, principal_did)
                    .map_err(|e| e.to_string())
            }
        }
    }

    pub fn import_samsung_health_folder(&mut self, folder: &Path) -> SamsungImportReport {
        let owner = self.owner_did.clone();
        let author = self.author_did.clone();
        import_samsung_folder(self, folder, &owner, &author)
    }

    /// Primary ingest path: companion bundle from the user's phone.
    pub fn ingest_companion_health_bundle(&mut self, bundle: &CompanionHealthBundle) -> SamsungImportReport {
        let owner = self.owner_did.clone();
        let author = self.author_did.clone();
        ingest_companion_health_bundle(self, bundle, &owner, &author)
    }
}
