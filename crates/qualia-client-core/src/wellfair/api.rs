use std::path::Path;

use super::accessibility_prefs;
use super::host_state::AccessibilityPreferences;
use super::import_samsung::{
    import_samsung_folder, ingest_companion_health_bundle, SamsungImportReport,
};
use super::journal::JournalEntry;
use super::policy::{DecisionResult, PolicyDecisionService};
use super::receipt::{receipt_from_decision, ReceiptRecord};
use super::snapshot::build_host_snapshot;
use super::vault::VaultService;
use super::host_state::WellfairHostSnapshot;
use ed25519_dalek::SigningKey;
use qualia_core_db::key_vault::KeyVault;
use wellfare_core::companion_sync::CompanionHealthBundle;
use wellfare_core::record::RecordEnvelope;

/// Transport-neutral Host API exported for UI and qApps.
pub struct WebizenHostApi {
    vault: VaultService,
    policy: PolicyDecisionService,
    signing_key: SigningKey,
    owner_did: String,
    author_did: String,
    storage_root: std::path::PathBuf,
}

impl WebizenHostApi {
    pub fn new(
        vault: VaultService,
        policy: PolicyDecisionService,
        signing_key: SigningKey,
        owner_did: String,
        author_did: String,
        storage_root: std::path::PathBuf,
    ) -> Self {
        Self {
            vault,
            policy,
            signing_key,
            owner_did,
            author_did,
            storage_root,
        }
    }

    pub fn save_accessibility(&self, prefs: &AccessibilityPreferences) -> Result<(), String> {
        accessibility_prefs::save(&self.storage_root, prefs).map_err(|e| e.to_string())
    }

    pub fn load_accessibility(&self) -> AccessibilityPreferences {
        accessibility_prefs::load(&self.storage_root)
    }

    pub fn snapshot_from_vault(key_vault: &KeyVault, owner_label: &str, demo_mode: bool) -> WellfairHostSnapshot {
        build_host_snapshot(key_vault, true, owner_label, demo_mode)
    }

    pub fn build_snapshot(&mut self, key_vault: &KeyVault, owner_label: &str) -> WellfairHostSnapshot {
        let mut snap = super::snapshot::build_host_snapshot_with_storage(
            key_vault,
            true,
            owner_label,
            false,
            Some(&self.storage_root),
        );
        if let Ok(count) = self.vault.journal_count() {
            snap.health_record_count = count as u32;
        }
        if let Ok(pending) = self.vault.wal_buffered_quins() {
            snap.pending_jobs = pending as u32;
            snap.sync_state = if pending > 0 {
                super::host_state::SyncQueueState::Queued
            } else {
                super::host_state::SyncQueueState::Idle
            };
        }
        if let Some(hash) = self.vault.last_checkpoint_hash() {
            snap.last_checkpoint_prefix = Some(hex::encode(&hash[..4]));
        }
        snap
    }

    pub fn submit_record(
        &mut self,
        qapp_id: &str,
        envelope: RecordEnvelope,
        source: &str,
    ) -> Result<usize, String> {
        let decision = self.policy.evaluate_access(
            qapp_id,
            "write_record",
            envelope.sensitivity,
            envelope.epistemic_status,
        );

        match &decision {
            DecisionResult::Deny { reasons } => {
                Err(format!("Policy denied: {}", reasons.join("; ")))
            }
            DecisionResult::Prompt { .. } | DecisionResult::Suspend { .. } => {
                Err("Policy requires consent or guardian approval before write".into())
            }
            DecisionResult::Permit { .. } => {
                let principal_did = qualia_core_db::q_hash(&self.owner_did);
                let committed = self
                    .vault
                    .commit_envelope(&envelope, &self.signing_key, principal_did, source)
                    .map_err(|e| e.to_string())?;

                let ts = envelope.asserted_time_unix;
                let receipt = receipt_from_decision(
                    qapp_id,
                    &envelope.id,
                    ts,
                    &decision,
                    self.vault.last_checkpoint_hash(),
                );
                self.vault.append_receipt(&receipt).map_err(|e| e.to_string())?;
                Ok(committed)
            }
        }
    }

    pub fn finalize_batch(&mut self) -> Result<String, String> {
        let hash = self.vault.checkpoint().map_err(|e| e.to_string())?;
        Ok(hex::encode(hash))
    }

    pub fn list_health_records(&self, limit: usize) -> Result<Vec<JournalEntry>, String> {
        self.vault
            .list_health_records(limit)
            .map_err(|e| e.to_string())
    }

    pub fn list_receipts(&self, limit: usize) -> Result<Vec<ReceiptRecord>, String> {
        self.vault.list_receipts(limit).map_err(|e| e.to_string())
    }

    pub fn import_samsung_health_folder(&mut self, folder: &Path) -> SamsungImportReport {
        let owner = self.owner_did.clone();
        let author = self.author_did.clone();
        let mut report = import_samsung_folder(self, folder, &owner, &author);
        if report.records_committed > 0 {
            if let Ok(hash) = self.finalize_batch() {
                report.checkpoint_hash = Some(hash);
            }
        }
        report
    }

    /// Primary ingest path: companion bundle from the user's phone.
    pub fn ingest_companion_health_bundle(
        &mut self,
        bundle: &CompanionHealthBundle,
    ) -> SamsungImportReport {
        let owner = self.owner_did.clone();
        let author = self.author_did.clone();
        let mut report = ingest_companion_health_bundle(self, bundle, &owner, &author);
        if report.records_committed > 0 {
            if let Ok(hash) = self.finalize_batch() {
                report.checkpoint_hash = Some(hash);
            }
        }
        report
    }
}