use std::path::Path;

use std::time::{SystemTime, UNIX_EPOCH};

use super::accessibility_prefs;
use super::consent_store::ConsentGrantRecord;
use super::host_state::{AccessibilityPreferences, ConsentGrantDraft, PolicyDecisionDto};
use super::import_samsung::{
    import_samsung_folder, ingest_companion_health_bundle, SamsungImportReport,
};
use super::journal::JournalEntry;
use super::policy::{DecisionResult, PolicyDecisionService};
use super::receipt::{receipt_from_decision, ReceiptRecord};
use super::sync_outbox::SyncOutboxEntry;
use super::snapshot::build_host_snapshot;
use super::vault::VaultService;
use super::host_state::WellfairHostSnapshot;
use ed25519_dalek::SigningKey;
use qualia_core_db::key_vault::KeyVault;
use sha2::{Digest, Sha256};
use wellfare_core::companion_sync::CompanionHealthBundle;
use wellfare_core::conditions::{
    allergy_summary, build_allergy_envelope, build_condition_envelope, condition_summary,
    AllergyReport, ConditionReport,
};
use wellfare_core::medication::{
    self, AdministrationStatus, DietEntry, MedicationAdministration, MedicationCatalogEntry,
};
use wellfare_core::record::RecordEnvelope;
use wellfare_core::sleep_analytics::{
    self, SleepDebtReport, SleepHeatmapReport, SleepNightSample, DEFAULT_TARGET_SLEEP_MIN,
};

use super::personal_profile::{EmergencyContact, EmergencyContactStore, new_contact_id};

const QAPP_SHELL: &str = "wellfair-shell";
const SOURCE_PERSONAL: &str = "wellfair:personal";

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
        snap.graph_quin_count = self.vault.graph_quin_count() as u32;
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
        if let Ok(queued) = self.vault.outbox_queued_count() {
            snap.pending_jobs = snap.pending_jobs.saturating_add(queued as u32);
            if queued > 0 {
                snap.sync_state = super::host_state::SyncQueueState::Queued;
            }
        }
        snap
    }

    fn now_unix() -> u64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0)
    }

    pub fn evaluate_policy(
        &self,
        qapp_id: &str,
        requested_scope: &str,
        sensitivity: wellfare_core::record::SensitivityClass,
        epistemic: wellfare_core::record::EpistemicStatus,
    ) -> Result<PolicyDecisionDto, String> {
        let now = Self::now_unix();
        let grants = self
            .vault
            .list_active_consents(now)
            .map_err(|e| e.to_string())?;
        let decision = self.policy.evaluate_access(
            qapp_id,
            requested_scope,
            sensitivity,
            epistemic,
            &grants,
            now,
        );
        Ok(decision.to_dto())
    }

    pub fn grant_consent(
        &mut self,
        draft: &ConsentGrantDraft,
        scope: &str,
    ) -> Result<ConsentGrantRecord, String> {
        let grant = ConsentGrantRecord::from_draft(draft, scope);
        self.vault
            .append_consent(&grant)
            .map_err(|e| e.to_string())?;
        let ts = grant.granted_at_unix;
        let decision = DecisionResult::Permit {
            obligations: vec!["consent_granted".into(), "emit_wal_receipt".into()],
        };
        let receipt = receipt_from_decision(
            "wellfair-shell",
            &grant.id,
            ts,
            &decision,
            self.vault.last_checkpoint_hash(),
        );
        self.vault.append_receipt(&receipt).map_err(|e| e.to_string())?;
        Ok(grant)
    }

    pub fn revoke_consent(&mut self, grant_id: &str) -> Result<bool, String> {
        let revoked = self
            .vault
            .revoke_consent(grant_id)
            .map_err(|e| e.to_string())?;
        if revoked {
            let ts = Self::now_unix() as u32;
            let decision = DecisionResult::Deny {
                reasons: vec!["consent_revoked".into()],
            };
            let receipt = receipt_from_decision(
                "wellfair-shell",
                grant_id,
                ts,
                &decision,
                self.vault.last_checkpoint_hash(),
            );
            self.vault.append_receipt(&receipt).map_err(|e| e.to_string())?;
        }
        Ok(revoked)
    }

    pub fn list_consents(&self) -> Result<Vec<ConsentGrantRecord>, String> {
        self.vault
            .list_active_consents(Self::now_unix())
            .map_err(|e| e.to_string())
    }

    pub fn submit_record(
        &mut self,
        qapp_id: &str,
        envelope: RecordEnvelope,
        source: &str,
    ) -> Result<usize, String> {
        self.submit_record_with_summary(qapp_id, envelope, source, None)
    }

    pub fn submit_record_with_summary(
        &mut self,
        qapp_id: &str,
        envelope: RecordEnvelope,
        source: &str,
        summary: Option<String>,
    ) -> Result<usize, String> {
        let now = Self::now_unix();
        let grants = self
            .vault
            .list_active_consents(now)
            .map_err(|e| e.to_string())?;
        let decision = self.policy.evaluate_access(
            qapp_id,
            "write_record",
            envelope.sensitivity,
            envelope.epistemic_status,
            &grants,
            now,
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
                    .commit_envelope(
                        &envelope,
                        &self.signing_key,
                        principal_did,
                        source,
                        summary,
                    )
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

    pub fn list_outbox(&self, limit: usize) -> Result<Vec<SyncOutboxEntry>, String> {
        self.vault.list_outbox(limit).map_err(|e| e.to_string())
    }

    fn payload_hash_hex(payload: &str) -> String {
        hex::encode(Sha256::digest(payload.as_bytes()).as_slice())
    }

    pub fn add_condition(&mut self, report: &ConditionReport) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_condition_envelope(
            report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = condition_summary(report);
        self.submit_record_with_summary(QAPP_SHELL, envelope, SOURCE_PERSONAL, Some(summary))?;
        let entries = self.list_health_records(1)?;
        entries
            .into_iter()
            .next()
            .ok_or_else(|| "condition committed but not found in journal".to_string())
    }

    pub fn add_allergy(&mut self, report: &AllergyReport) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_allergy_envelope(
            report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = allergy_summary(report);
        self.submit_record_with_summary(QAPP_SHELL, envelope, SOURCE_PERSONAL, Some(summary))?;
        let entries = self.list_health_records(1)?;
        entries
            .into_iter()
            .next()
            .ok_or_else(|| "allergy committed but not found in journal".to_string())
    }

    pub fn graph_quin_count(&self) -> usize {
        self.vault.graph_quin_count()
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

    const QAPP_MEDICATION: &'static str = "wellfair-medication";

    pub fn add_medication(
        &mut self,
        name: &str,
        dose: &str,
        route: &str,
        schedule_times: Vec<String>,
    ) -> Result<JournalEntry, String> {
        let now = Self::now_unix() as u32;
        let entry = MedicationCatalogEntry {
            id: medication::new_medication_id(name, now),
            name: name.to_string(),
            dose: dose.to_string(),
            route: route.to_string(),
            schedule_times,
            prescriber: None,
            ceased_at_unix: None,
            created_at_unix: now,
        };
        let packed = medication::medication_envelope(&entry, &self.owner_did, &self.author_did);
        self.submit_record_with_summary(
            Self::QAPP_MEDICATION,
            packed.envelope,
            "wellfair-medication:ui",
            Some(packed.summary),
        )?;
        self.finalize_batch().ok();
        self.list_health_records(1)?
            .into_iter()
            .next()
            .ok_or_else(|| "medication committed but journal empty".into())
    }

    pub fn record_administration(
        &mut self,
        medication_id: &str,
        medication_name: &str,
        status: AdministrationStatus,
        notes: Option<String>,
    ) -> Result<JournalEntry, String> {
        let now = Self::now_unix() as u32;
        let admin = MedicationAdministration {
            id: medication::new_administration_id(medication_id, now),
            medication_id: medication_id.to_string(),
            medication_name: medication_name.to_string(),
            status,
            administered_at_unix: now,
            notes,
        };
        let packed = medication::administration_envelope(&admin, &self.owner_did, &self.author_did);
        self.submit_record_with_summary(
            Self::QAPP_MEDICATION,
            packed.envelope,
            "wellfair-medication:ui",
            Some(packed.summary),
        )?;
        self.finalize_batch().ok();
        self.list_health_records(1)?
            .into_iter()
            .next()
            .ok_or_else(|| "administration committed but journal empty".into())
    }

    pub fn add_diet_entry(
        &mut self,
        description: &str,
        meal_type: &str,
        calories_kcal: Option<u32>,
    ) -> Result<JournalEntry, String> {
        let now = Self::now_unix() as u32;
        let diet = DietEntry {
            id: medication::new_diet_id(description, now),
            description: description.to_string(),
            meal_type: meal_type.to_string(),
            calories_kcal,
            logged_at_unix: now,
        };
        let packed = medication::diet_envelope(&diet, &self.owner_did, &self.author_did);
        self.submit_record_with_summary(
            Self::QAPP_MEDICATION,
            packed.envelope,
            "wellfair-medication:ui",
            Some(packed.summary),
        )?;
        self.finalize_batch().ok();
        self.list_health_records(1)?
            .into_iter()
            .next()
            .ok_or_else(|| "diet entry committed but journal empty".into())
    }

    pub fn list_journal_by_kind(&self, kind: &str, limit: usize) -> Result<Vec<JournalEntry>, String> {
        Ok(self
            .list_health_records(limit)?
            .into_iter()
            .filter(|e| e.kind == kind)
            .collect())
    }

    pub fn sleep_analytics(&self, target_min: f64) -> Result<(SleepDebtReport, SleepHeatmapReport), String> {
        let sleep_rows = self.list_journal_by_kind("sleep", 128)?;
        let mut samples = Vec::new();
        for row in sleep_rows {
            if let Some(ref summary) = row.summary {
                if let Some((dur, eff)) = sleep_analytics::parse_sleep_summary_json(summary) {
                    samples.push(SleepNightSample {
                        night_unix: row.asserted_time_unix,
                        duration_min: dur,
                        efficiency: eff,
                    });
                }
            }
        }
        samples.sort_by_key(|s| s.night_unix);
        let debt = sleep_analytics::compute_sleep_debt(&samples, target_min);
        let heatmap = sleep_analytics::compute_weekly_heatmap(&samples, target_min);
        Ok((debt, heatmap))
    }

    pub fn default_sleep_analytics(&self) -> Result<(SleepDebtReport, SleepHeatmapReport), String> {
        self.sleep_analytics(DEFAULT_TARGET_SLEEP_MIN)
    }

    pub fn add_emergency_contact(
        &self,
        display_name: &str,
        relationship: &str,
        phone: Option<String>,
        email: Option<String>,
        notes: Option<String>,
    ) -> Result<EmergencyContact, String> {
        let now = Self::now_unix() as u32;
        let contact = EmergencyContact {
            id: new_contact_id(display_name, now),
            display_name: display_name.to_string(),
            relationship: relationship.to_string(),
            phone,
            email,
            notes,
            created_at_unix: now,
        };
        let store = EmergencyContactStore::open(&self.storage_root).map_err(|e| e.to_string())?;
        store.append(&contact).map_err(|e| e.to_string())?;
        Ok(contact)
    }

    pub fn list_emergency_contacts(&self) -> Result<Vec<EmergencyContact>, String> {
        let store = EmergencyContactStore::open(&self.storage_root).map_err(|e| e.to_string())?;
        store.list().map_err(|e| e.to_string())
    }
}

#[cfg(test)]
mod api_tests {
    use super::*;
    use crate::wellfair::policy::PolicyDecisionService;
    use crate::wellfair::vault::VaultService;
    use ed25519_dalek::SigningKey;
    use tempfile::tempdir;

    fn test_host(dir: &std::path::Path) -> WebizenHostApi {
        let wal = dir.join("test.wal");
        let vault = VaultService::open(&wal, dir, 0xBEEF).unwrap();
        let policy = PolicyDecisionService::new();
        let signing_key = SigningKey::from_bytes(&[9u8; 32]);
        WebizenHostApi::new(
            vault,
            policy,
            signing_key,
            "did:wf:owner".into(),
            "did:wf:owner".into(),
            dir.to_path_buf(),
        )
    }

    #[test]
    fn add_condition_writes_restricted_journal_entry() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let report = ConditionReport::new("Hypertension");
        let entry = host.add_condition(&report).unwrap();
        assert_eq!(entry.kind, "condition");
        assert_eq!(entry.source, SOURCE_PERSONAL);
        assert!(entry.id.contains(":condition:"));
        let listed = host.list_health_records(8).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].kind, "condition");
    }

    #[test]
    fn add_allergy_writes_allergy_journal_entry() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let report = AllergyReport::new("Shellfish");
        let entry = host.add_allergy(&report).unwrap();
        assert_eq!(entry.kind, "allergy");
        assert!(entry.id.contains(":allergy:"));
    }
}