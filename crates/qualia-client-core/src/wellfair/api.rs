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
use super::export_package::{
    build_export_package, export_policy_receipt, ExportReceipt, HealthExportPackage,
};
use super::graph_query::GraphCoverageRow;
use super::sync_outbox::SyncOutboxEntry;
use super::snapshot::build_host_snapshot;
use super::vault::VaultService;
use super::host_state::WellfairHostSnapshot;
use super::sync_protocol::{AdmitOutcome, InboxRecord, SyncInbox, SyncOperation};
use ed25519_dalek::{Signer, SigningKey};
use qualia_core_db::key_vault::KeyVault;
use sha2::{Digest, Sha256};
use wellfare_core::companion_sync::CompanionHealthBundle;
use wellfare_core::live_share::{LiveSectionRequest, UsageAgreement};
use wellfare_core::conditions::{
    allergy_summary, build_allergy_envelope, build_condition_envelope, condition_summary,
    AllergyReport, ConditionReport,
};
use wellfare_core::personal_records::{
    build_disputed_diagnosis_envelope, build_housing_safety_envelope,
    disputed_diagnosis_summary, housing_safety_summary, DisputedDiagnosisReport,
    HousingSafetyReport,
};
use super::live_share::{
    append_live_share_journal, live_share_decision_journal_entry,
    live_share_request_journal_entry, sanctuary_allows_classified_projection, validate_live_share_decision,
    LiveShareStore,
};
use super::med_reminders::{
    compute_due_reminders, load_prefs, save_prefs, DueMedReminder, MedReminderPrefs,
};
use super::sanctuary::{
    apply_sanctuary_projection, load_prefs as load_sanctuary_prefs, lock_sanctuary, setup_sanctuary,
    unlock_sanctuary, SanctuaryPrefs,
};
use wellfare_core::life_records::{
    build_case_task_envelope, build_life_event_envelope, build_welfare_case_envelope,
    case_task_summary, life_event_summary, welfare_case_summary, CaseTaskReport, LifeEventReport,
    WelfareCaseReport,
};
use wellfare_core::mental_wellbeing::{
    build_therapy_note_envelope, build_wellbeing_observation_envelope, therapy_note_summary,
    wellbeing_observation_summary, TherapyNote, WellbeingObservation,
};
use wellfare_core::medication::{
    self, AdministrationStatus, DietEntry, MedicationAdministration, MedicationCatalogEntry,
};
use wellfare_core::finance::{
    build_ledger_entry_envelope, derived_balance, ledger_entry_summary, parse_ledger_summary,
    BalanceReport, LedgerEntry,
};
use wellfare_core::projects::{
    build_contribution_envelope, build_membership_envelope, build_project_envelope,
    contribution_summary, derive_obligations, membership_summary, project_summary, Contribution,
    Obligation, Project, ProjectMembership,
};
use wellfare_core::credentials::{
    build_credential_envelope, build_presentation, credential_summary, CredentialRecord,
    FieldSelectedPresentation,
};
use super::blob_store::BlobStore;
use wellfare_core::clinical::{
    build_clinical_attachment_envelope, build_clinical_report_envelope, clinical_attachment_summary,
    clinical_report_summary, AttachmentMeta, ClinicalReport, ClinicalReportType,
};
use wellfare_core::welfare_support::{
    build_assistance_need_envelope, build_government_letter_envelope, build_welfare_stream_envelope,
    AssistanceNeed, GovernmentLetter, StreamStatus, Urgency, WelfareStream,
};
use wellfare_core::record::RecordEnvelope;
use wellfare_core::sleep_analytics::{
    self, SleepDebtReport, SleepHeatmapReport, SleepNightSample, DEFAULT_TARGET_SLEEP_MIN,
};

use super::personal_profile::{EmergencyContact, EmergencyContactStore, new_contact_id};

const QAPP_SHELL: &str = "wellfair-shell";
const QAPP_LIFE: &str = "wellfair-life";
const QAPP_WELLBEING: &str = "wellfair-wellbeing";
const QAPP_FINANCE: &str = "wellfair-finance";
const QAPP_PROJECTS: &str = "wellfair-projects";
const QAPP_CREDENTIALS: &str = "wellfair-credentials";
const QAPP_CLINICAL: &str = "wellfair-clinical";
const QAPP_WELFARE: &str = "wellfair-welfare";
const SOURCE_PERSONAL: &str = "wellfair:personal";
const SOURCE_LIFE: &str = "wellfair:life";
const SOURCE_WELLBEING: &str = "wellfair:wellbeing";
const SOURCE_FINANCE: &str = "wellfair:finance";
const SOURCE_PROJECTS: &str = "wellfair:projects";
const SOURCE_CREDENTIALS: &str = "wellfair:credentials";
const SOURCE_CLINICAL: &str = "wellfair:clinical";
const SOURCE_WELFARE: &str = "wellfair:welfare";

/// Reconstruct a `Contribution` from a stored/transmitted summary JSON. The record id (which
/// is the dedup anchor for obligation derivation) is supplied by the caller — the journal row
/// id locally, or the sync operation's `record_id` for an inbound op.
fn contribution_from_summary(id: String, summary: &str, occurred_at_unix: u32) -> Option<Contribution> {
    let v: serde_json::Value = serde_json::from_str(summary).ok()?;
    Some(Contribution {
        id,
        project_id: v
            .get("project_id")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        contributor_did: v
            .get("contributor_did")
            .and_then(|x| x.as_str())
            .unwrap_or_default()
            .to_string(),
        description: String::new(),
        effort_minutes: v.get("effort_minutes").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
        occurred_at_unix,
        predecessor_id: None,
    })
}

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
        let entries = self
            .vault
            .list_health_records(limit)
            .map_err(|e| e.to_string())?;
        let prefs = load_sanctuary_prefs(&self.storage_root);
        Ok(apply_sanctuary_projection(&prefs, entries))
    }

    pub fn list_receipts(&self, limit: usize) -> Result<Vec<ReceiptRecord>, String> {
        self.vault.list_receipts(limit).map_err(|e| e.to_string())
    }

    pub fn list_outbox(&self, limit: usize) -> Result<Vec<SyncOutboxEntry>, String> {
        self.vault.list_outbox(limit).map_err(|e| e.to_string())
    }

    /// Standards-readable Turtle export bound to the latest checkpoint (§8.1 step 9).
    pub fn export_health_package(
        &mut self,
        limit: usize,
    ) -> Result<(HealthExportPackage, ExportReceipt), String> {
        self.finalize_batch().ok();
        let entries = self.list_health_records(limit)?;
        let exported_at = Self::now_unix() as u32;
        let pkg = build_export_package(
            &entries,
            exported_at,
            self.vault.last_checkpoint_hash(),
        );
        let receipt = export_policy_receipt(&pkg, exported_at);
        self.vault.append_receipt(&receipt).map_err(|e| e.to_string())?;
        let export_receipt = ExportReceipt::from_package(&pkg);
        Ok((pkg, export_receipt))
    }

    /// Journal row → materialized quin coverage (bounded semantic query).
    ///
    /// Applies the Sanctuary projection: while Sanctuary is locked (including a
    /// decoy session) rows for protected kinds are withheld, so the coverage/Tools
    /// view is never an alternate read path around the boundary (master plan §5.2, §17).
    pub fn query_graph_coverage(&self, limit: usize) -> Result<Vec<GraphCoverageRow>, String> {
        let rows = self
            .vault
            .graph_coverage(limit)
            .map_err(|e| e.to_string())?;
        let prefs = load_sanctuary_prefs(&self.storage_root);
        if !prefs.enabled || !prefs.locked {
            return Ok(rows);
        }
        Ok(rows
            .into_iter()
            .filter(|row| !super::sanctuary::is_sanctuary_protected_kind(&row.kind))
            .collect())
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

    pub fn add_disputed_diagnosis(
        &mut self,
        report: &DisputedDiagnosisReport,
    ) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_disputed_diagnosis_envelope(
            report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = disputed_diagnosis_summary(report);
        self.submit_record_with_summary(QAPP_SHELL, envelope, SOURCE_PERSONAL, Some(summary))?;
        self.list_health_records(1)?
            .into_iter()
            .next()
            .ok_or_else(|| "disputed diagnosis committed but not found in journal".to_string())
    }

    pub fn add_housing_safety(
        &mut self,
        report: &HousingSafetyReport,
    ) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_housing_safety_envelope(
            report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = housing_safety_summary(report);
        self.submit_record_with_summary(QAPP_SHELL, envelope, SOURCE_PERSONAL, Some(summary))?;
        self.list_health_records(1)?
            .into_iter()
            .next()
            .ok_or_else(|| "housing/safety committed but not found in journal".to_string())
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

    pub fn med_reminder_prefs(&self) -> MedReminderPrefs {
        load_prefs(&self.storage_root)
    }

    pub fn set_med_reminders_enabled(&self, enabled: bool) -> Result<MedReminderPrefs, String> {
        let mut prefs = load_prefs(&self.storage_root);
        if enabled && !prefs.permission_granted {
            return Err("Grant reminder permission before enabling notifications".into());
        }
        prefs.enabled = enabled;
        save_prefs(&self.storage_root, &prefs).map_err(|e| e.to_string())?;
        Ok(prefs)
    }

    pub fn grant_med_reminder_permission(&self) -> Result<MedReminderPrefs, String> {
        let mut prefs = load_prefs(&self.storage_root);
        prefs.permission_granted = true;
        prefs.permission_granted_at_unix = Some(Self::now_unix() as u32);
        save_prefs(&self.storage_root, &prefs).map_err(|e| e.to_string())?;
        Ok(prefs)
    }

    pub fn list_due_med_reminders(&self, window_minutes: i32) -> Result<Vec<DueMedReminder>, String> {
        let prefs = load_prefs(&self.storage_root);
        if !prefs.enabled || !prefs.permission_granted {
            return Ok(Vec::new());
        }
        let journal = self
            .vault
            .list_health_records(128)
            .map_err(|e| e.to_string())?;
        let now = chrono::Local::now().time();
        Ok(compute_due_reminders(&journal, now, window_minutes))
    }

    pub fn sanctuary_prefs(&self) -> SanctuaryPrefs {
        load_sanctuary_prefs(&self.storage_root)
    }

    pub fn setup_sanctuary(&self, real_pin: &str, decoy_pin: &str) -> Result<SanctuaryPrefs, String> {
        setup_sanctuary(&self.storage_root, real_pin, decoy_pin, Self::now_unix() as u32)
    }

    pub fn lock_sanctuary(&self) -> Result<SanctuaryPrefs, String> {
        lock_sanctuary(&self.storage_root)
    }

    pub fn unlock_sanctuary(&self, pin: &str) -> Result<SanctuaryPrefs, String> {
        unlock_sanctuary(&self.storage_root, pin)
    }

    // --- Encrypted Sanctuary vault (real boundary; native-only, plan §6) ---
    //
    // Sensitive free-text notes are stored ONLY inside AEAD-encrypted lane files keyed by a
    // PBKDF2-derived key — there is no plaintext journal path for them. Nothing is readable
    // without the PIN, and the decoy PIN opens a separate lane that never aliases real data.

    #[cfg(not(target_arch = "wasm32"))]
    pub fn sanctuary_vault_configured(&self) -> bool {
        super::sanctuary_vault::is_configured(&self.storage_root)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_sanctuary_vault(&self, real_pin: &str, decoy_pin: &str) -> Result<(), String> {
        super::sanctuary_vault::setup(&self.storage_root, real_pin, decoy_pin)
    }

    /// Verify a PIN and report which lane it opens (real vs duress decoy).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sanctuary_vault_resolve_lane(
        &self,
        pin: &str,
    ) -> Result<super::sanctuary_vault::SanctuaryLane, String> {
        super::sanctuary_vault::resolve_lane(&self.storage_root, pin)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_sanctuary_vault_note(
        &self,
        pin: &str,
        body: &str,
    ) -> Result<super::sanctuary_vault::SanctuaryLane, String> {
        super::sanctuary_vault::add_note(&self.storage_root, pin, body, Self::now_unix() as u32)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub fn list_sanctuary_vault_notes(
        &self,
        pin: &str,
    ) -> Result<(super::sanctuary_vault::SanctuaryLane, Vec<super::sanctuary_vault::SanctuaryVaultNote>), String> {
        super::sanctuary_vault::list_notes(&self.storage_root, pin)
    }

    pub fn add_life_event(&mut self, report: &LifeEventReport) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_life_event_envelope(
            report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = life_event_summary(report);
        self.submit_record_with_summary(QAPP_LIFE, envelope, SOURCE_LIFE, Some(summary))?;
        self.latest_journal_entry()
    }

    pub fn add_welfare_case(&mut self, report: &WelfareCaseReport) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_welfare_case_envelope(
            report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = welfare_case_summary(report);
        self.submit_record_with_summary(QAPP_LIFE, envelope, SOURCE_LIFE, Some(summary))?;
        self.latest_journal_entry()
    }

    pub fn add_case_task(&mut self, report: &CaseTaskReport) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_case_task_envelope(
            report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = case_task_summary(report);
        self.submit_record_with_summary(QAPP_LIFE, envelope, SOURCE_LIFE, Some(summary))?;
        self.latest_journal_entry()
    }

    pub fn add_wellbeing_observation(
        &mut self,
        report: &WellbeingObservation,
    ) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_wellbeing_observation_envelope(
            report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = wellbeing_observation_summary(report);
        self.submit_record_with_summary(QAPP_WELLBEING, envelope, SOURCE_WELLBEING, Some(summary))?;
        self.latest_journal_entry()
    }

    pub fn add_therapy_note(&mut self, report: &TherapyNote) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(report).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_therapy_note_envelope(
            report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = therapy_note_summary(report);
        self.submit_record_with_summary(QAPP_WELLBEING, envelope, SOURCE_WELLBEING, Some(summary))?;
        self.latest_journal_entry()
    }

    fn latest_journal_entry(&self) -> Result<JournalEntry, String> {
        self.vault
            .list_health_records(1)
            .map_err(|e| e.to_string())?
            .into_iter()
            .next()
            .ok_or_else(|| "record committed but not found in journal".to_string())
    }

    /// Record a signed personal-finance ledger entry (Phase 5 / FIN-01..).
    pub fn add_ledger_entry(&mut self, entry: &LedgerEntry) -> Result<JournalEntry, String> {
        let payload = serde_json::to_string(entry).map_err(|e| e.to_string())?;
        let hash = Self::payload_hash_hex(&payload);
        let asserted = Self::now_unix() as u32;
        let envelope = build_ledger_entry_envelope(
            entry,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = ledger_entry_summary(entry);
        self.submit_record_with_summary(QAPP_FINANCE, envelope, SOURCE_FINANCE, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    /// List ledger journal rows (most recent first).
    pub fn list_ledger_entries(&self, limit: usize) -> Result<Vec<JournalEntry>, String> {
        self.list_journal_by_kind("ledger_entry", limit)
    }

    /// Derived balance across the ledger. Balances are a pure derivation over the
    /// unique-entry-id set, so a duplicate or replayed commit can never move money (§17).
    pub fn ledger_balance(&self, limit: usize) -> Result<BalanceReport, String> {
        let rows = self.list_ledger_entries(limit)?;
        let mut entries = Vec::with_capacity(rows.len());
        for row in rows {
            if let Some(ref summary) = row.summary {
                if let Some((amount_cents, currency)) = parse_ledger_summary(summary) {
                    entries.push(LedgerEntry {
                        id: row.id.clone(),
                        description: String::new(),
                        amount_cents,
                        currency,
                        category: None,
                        counterparty: None,
                        project_id: None,
                        occurred_at_unix: row.asserted_time_unix,
                    });
                }
            }
        }
        Ok(derived_balance(&entries))
    }

    // --- Cooperative projects (Phase 5 / COP-01..) ---

    pub fn add_project(&mut self, project: &Project) -> Result<JournalEntry, String> {
        let asserted = Self::now_unix() as u32;
        let hash =
            Self::payload_hash_hex(&serde_json::to_string(project).map_err(|e| e.to_string())?);
        let envelope =
            build_project_envelope(project, &self.owner_did, &self.author_did, asserted, Some(hash));
        let summary = project_summary(project);
        self.submit_record_with_summary(QAPP_PROJECTS, envelope, SOURCE_PROJECTS, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    pub fn add_project_membership(
        &mut self,
        membership: &ProjectMembership,
    ) -> Result<JournalEntry, String> {
        let asserted = Self::now_unix() as u32;
        let hash =
            Self::payload_hash_hex(&serde_json::to_string(membership).map_err(|e| e.to_string())?);
        let envelope = build_membership_envelope(
            membership,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = membership_summary(membership);
        self.submit_record_with_summary(QAPP_PROJECTS, envelope, SOURCE_PROJECTS, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    pub fn add_contribution(&mut self, contribution: &Contribution) -> Result<JournalEntry, String> {
        let asserted = Self::now_unix() as u32;
        let hash = Self::payload_hash_hex(
            &serde_json::to_string(contribution).map_err(|e| e.to_string())?,
        );
        let envelope = build_contribution_envelope(
            contribution,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = contribution_summary(contribution);
        self.submit_record_with_summary(QAPP_PROJECTS, envelope, SOURCE_PROJECTS, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    pub fn list_contributions(&self, limit: usize) -> Result<Vec<JournalEntry>, String> {
        self.list_journal_by_kind("contribution", limit)
    }

    /// Locally-committed contributions reconstructed from the journal.
    fn local_contributions(&self, limit: usize) -> Result<Vec<Contribution>, String> {
        let mut out = Vec::new();
        for row in self.list_contributions(limit)? {
            if let Some(ref summary) = row.summary {
                if let Some(c) = contribution_from_summary(row.id.clone(), summary, row.asserted_time_unix) {
                    out.push(c);
                }
            }
        }
        Ok(out)
    }

    /// Derive per-(project, contributor) effort obligations from the committed contribution
    /// journal. Pure over the unique-id set, so a duplicate or replayed commit can never
    /// double-count effort (§17 money/obligation safety).
    pub fn project_obligations(&self, limit: usize) -> Result<Vec<Obligation>, String> {
        Ok(derive_obligations(&self.local_contributions(limit)?))
    }

    /// Obligations derived from **both** locally-committed contributions and validated inbound
    /// sync operations (kind `contribution`) — the cross-node convergence view. Because
    /// `derive_obligations` collapses to the unique record-id set first, a remote contribution
    /// that has already been seen locally, or a replayed inbound op, never double-counts effort
    /// (§17). This is the "apply validated inbound ops" step of the sync loop for obligations.
    pub fn synced_project_obligations(&self, limit: usize) -> Result<Vec<Obligation>, String> {
        let mut contributions = self.local_contributions(limit)?;
        for op in self.validated_sync_operations()? {
            if op.kind == "contribution" {
                if let Some(c) =
                    contribution_from_summary(op.record_id.clone(), &op.payload_summary, op.committed_unix)
                {
                    contributions.push(c);
                }
            }
        }
        Ok(derive_obligations(&contributions))
    }

    // --- Credentials (Phase 3/7 / CRE-01..) ---

    pub fn add_credential(&mut self, credential: &CredentialRecord) -> Result<JournalEntry, String> {
        let asserted = Self::now_unix() as u32;
        let json = serde_json::to_string(credential).map_err(|e| e.to_string())?;
        // Persist the full credential (incl. claims) as a content-addressed blob so a
        // presentation can be built later; the envelope blob_hash is that content hash.
        let hash = BlobStore::open(&self.storage_root)
            .and_then(|store| store.put(json.as_bytes()))
            .map_err(|e| e.to_string())?;
        let envelope = build_credential_envelope(
            credential,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = credential_summary(credential);
        self.submit_record_with_summary(
            QAPP_CREDENTIALS,
            envelope,
            SOURCE_CREDENTIALS,
            Some(summary),
        )?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    pub fn list_credentials(&self, limit: usize) -> Result<Vec<JournalEntry>, String> {
        self.list_journal_by_kind("credential", limit)
    }

    /// Load the full credential (including its claims) from its content-addressed blob.
    /// Returns `None` if the record id is unknown or its blob is missing.
    pub fn get_credential(&self, record_id: &str) -> Result<Option<CredentialRecord>, String> {
        let Some(entry) = self
            .list_credentials(256)?
            .into_iter()
            .find(|e| e.id == record_id)
        else {
            return Ok(None);
        };
        let Some(hash) = entry.blob_hash else {
            return Ok(None);
        };
        let store = BlobStore::open(&self.storage_root).map_err(|e| e.to_string())?;
        let Some(bytes) = store.get(&hash).map_err(|e| e.to_string())? else {
            return Ok(None);
        };
        let cred: CredentialRecord = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Ok(Some(cred))
    }

    /// Build a field-selected presentation of a stored credential — plain field selection, NOT
    /// cryptographic selective disclosure (the type name and the domain module say so).
    pub fn present_credential(
        &self,
        record_id: &str,
        selected_claim_keys: &[String],
    ) -> Result<FieldSelectedPresentation, String> {
        let cred = self
            .get_credential(record_id)?
            .ok_or_else(|| format!("credential '{record_id}' not found or blob missing"))?;
        Ok(build_presentation(&cred, selected_claim_keys))
    }

    // --- Phase 5 sync-operation protocol (SyncService, §4.2 / §9.5 / §17) ---

    /// Build a signed outbound sync operation from a committed journal entry.
    /// Returns `None` for Classified/Sanctuary records — they never enter the ordinary sync
    /// lane (§5.2). The signature is a real ed25519 signature over the operation's bound payload.
    pub fn build_outbound_operation(
        &self,
        entry: &JournalEntry,
        lamport: u64,
    ) -> Option<SyncOperation> {
        if entry.sensitivity == "Classified" {
            return None;
        }
        let op = SyncOperation::new(
            uuid::Uuid::new_v4().to_string(),
            entry.id.clone(),
            entry.kind.clone(),
            self.author_did.clone(),
            entry.sensitivity.clone(),
            entry.summary.clone().unwrap_or_default(),
            lamport,
            entry.committed_unix,
        );
        let signature = self.signing_key.sign(&op.signing_payload());
        Some(op.with_signature(hex::encode(signature.to_bytes())))
    }

    /// Admit an inbound sync operation into the durable quarantined inbox. Idempotent: a
    /// replayed operation id is recorded as `Duplicate` and never applied twice.
    pub fn admit_sync_operation(&self, op: &SyncOperation) -> Result<AdmitOutcome, String> {
        let inbox = SyncInbox::open(&self.storage_root).map_err(|e| e.to_string())?;
        inbox
            .admit(op, Self::now_unix() as u32)
            .map_err(|e| e.to_string())
    }

    /// Validated operations currently held in the inbox, in Lamport order.
    pub fn validated_sync_operations(&self) -> Result<Vec<SyncOperation>, String> {
        SyncInbox::open(&self.storage_root)
            .map_err(|e| e.to_string())?
            .validated_operations()
            .map_err(|e| e.to_string())
    }

    pub fn list_sync_inbox(&self, limit: usize) -> Result<Vec<InboxRecord>, String> {
        SyncInbox::open(&self.storage_root)
            .map_err(|e| e.to_string())?
            .list_recent(limit)
            .map_err(|e| e.to_string())
    }

    // --- Clinical documents (Phase 3 / CLI-01..) ---

    pub fn add_clinical_report(
        &mut self,
        title: &str,
        report_type: ClinicalReportType,
        observed_at_unix: u32,
        body: &str,
        author_label: Option<String>,
    ) -> Result<JournalEntry, String> {
        let mut report = ClinicalReport::new(title, report_type, observed_at_unix, body);
        report.author_label = author_label.filter(|s| !s.is_empty());
        let hash = Self::payload_hash_hex(&serde_json::to_string(&report).map_err(|e| e.to_string())?);
        let asserted = Self::now_unix() as u32;
        let envelope = build_clinical_report_envelope(
            &report,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = clinical_report_summary(&report);
        self.submit_record_with_summary(QAPP_CLINICAL, envelope, SOURCE_CLINICAL, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    pub fn list_clinical_reports(&self, limit: usize) -> Result<Vec<JournalEntry>, String> {
        self.list_journal_by_kind("clinical_report", limit)
    }

    /// Store an attachment's bytes as a content-addressed blob and commit its metadata record.
    /// The bytes live only in the blob store; the journal row holds filename/size/hash metadata.
    pub fn add_clinical_attachment(
        &mut self,
        filename: &str,
        media_type: &str,
        bytes: &[u8],
    ) -> Result<JournalEntry, String> {
        let store = BlobStore::open(&self.storage_root).map_err(|e| e.to_string())?;
        let content_hash = store.put(bytes).map_err(|e| e.to_string())?;
        let meta = AttachmentMeta::new(filename, media_type, bytes.len() as u64, content_hash);
        let asserted = Self::now_unix() as u32;
        let envelope =
            build_clinical_attachment_envelope(&meta, &self.owner_did, &self.author_did, asserted);
        let summary = clinical_attachment_summary(&meta);
        self.submit_record_with_summary(QAPP_CLINICAL, envelope, SOURCE_CLINICAL, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    pub fn list_clinical_attachments(&self, limit: usize) -> Result<Vec<JournalEntry>, String> {
        self.list_journal_by_kind("clinical_attachment", limit)
    }

    /// Read an attachment's bytes back from the blob store (integrity-verified).
    pub fn attachment_bytes(&self, record_id: &str) -> Result<Option<Vec<u8>>, String> {
        let Some(entry) = self
            .list_clinical_attachments(256)?
            .into_iter()
            .find(|e| e.id == record_id)
        else {
            return Ok(None);
        };
        let Some(hash) = entry.blob_hash else {
            return Ok(None);
        };
        BlobStore::open(&self.storage_root)
            .map_err(|e| e.to_string())?
            .get(&hash)
            .map_err(|e| e.to_string())
    }

    // --- Welfare support (Phase 3 / LIF-08..) ---

    pub fn add_assistance_need(
        &mut self,
        category: &str,
        description: &str,
        urgency: Urgency,
    ) -> Result<JournalEntry, String> {
        let mut need = AssistanceNeed::new(category, description, Self::now_unix() as u32);
        need.urgency = urgency;
        let hash = Self::payload_hash_hex(&serde_json::to_string(&need).map_err(|e| e.to_string())?);
        let asserted = Self::now_unix() as u32;
        let envelope = build_assistance_need_envelope(
            &need,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = wellfare_core::welfare_support::assistance_need_summary(&need);
        self.submit_record_with_summary(QAPP_WELFARE, envelope, SOURCE_WELFARE, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    pub fn add_welfare_stream(
        &mut self,
        program_name: &str,
        reference: Option<String>,
        status: StreamStatus,
    ) -> Result<JournalEntry, String> {
        let mut stream = WelfareStream::new(program_name, Self::now_unix() as u32);
        stream.reference = reference.filter(|s| !s.is_empty());
        stream.status = status;
        let hash = Self::payload_hash_hex(&serde_json::to_string(&stream).map_err(|e| e.to_string())?);
        let asserted = Self::now_unix() as u32;
        let envelope = build_welfare_stream_envelope(
            &stream,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
        let summary = wellfare_core::welfare_support::welfare_stream_summary(&stream);
        self.submit_record_with_summary(QAPP_WELFARE, envelope, SOURCE_WELFARE, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    pub fn add_government_letter(
        &mut self,
        sender: &str,
        subject: &str,
        action_required: bool,
    ) -> Result<JournalEntry, String> {
        let mut letter = GovernmentLetter::new(sender, subject, Self::now_unix() as u32);
        letter.action_required = action_required;
        let asserted = Self::now_unix() as u32;
        let envelope = build_government_letter_envelope(
            &letter,
            &self.owner_did,
            &self.author_did,
            asserted,
        );
        let summary = wellfare_core::welfare_support::government_letter_summary(&letter);
        self.submit_record_with_summary(QAPP_WELFARE, envelope, SOURCE_WELFARE, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    /// All welfare-support journal rows (assistance needs, streams, government letters).
    pub fn list_welfare_records(&self, limit: usize) -> Result<Vec<JournalEntry>, String> {
        Ok(self
            .list_health_records(limit)?
            .into_iter()
            .filter(|e| {
                matches!(
                    e.kind.as_str(),
                    "assistance_need" | "welfare_stream" | "government_letter"
                )
            })
            .collect())
    }

    /// Companion requests a live section projection; owner must approve minimum kinds before data flows.
    pub fn submit_live_share_request(
        &self,
        request: &LiveSectionRequest,
    ) -> Result<JournalEntry, String> {
        let now = Self::now_unix();
        let store = LiveShareStore::open(&self.storage_root).map_err(|e| e.to_string())?;
        let record = store
            .enqueue_request(request.clone(), now)
            .map_err(|e| e.to_string())?;
        let committed_unix = now as u32;
        let entry = live_share_request_journal_entry(&record, committed_unix);
        append_live_share_journal(&self.storage_root, &entry)?;
        Ok(entry)
    }

    /// Owner approves or denies a pending live share; sanctuary-classified kinds fail closed unless unlocked.
    pub fn decide_live_share_request(
        &self,
        request_id: &str,
        approved: bool,
        projection_kinds: &[String],
        deny_reason: Option<&str>,
    ) -> Result<JournalEntry, String> {
        let now = Self::now_unix();
        let store = LiveShareStore::open(&self.storage_root).map_err(|e| e.to_string())?;
        let pending = store
            .get_request(request_id)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| format!("live share request '{request_id}' not found"))?;
        if pending.status != super::live_share::LiveShareRequestStatus::Pending {
            return Err(format!("live share request '{request_id}' already decided"));
        }
        let sanctuary_prefs = load_sanctuary_prefs(&self.storage_root);
        let sanctuary_unlocked = sanctuary_allows_classified_projection(&sanctuary_prefs);
        validate_live_share_decision(&pending, approved, projection_kinds, sanctuary_unlocked)?;
        let deny = if approved {
            None
        } else {
            Some(
                deny_reason
                    .filter(|s| !s.is_empty())
                    .unwrap_or("owner denied live share request"),
            )
        };
        let updated = store
            .decide(
                request_id,
                approved,
                projection_kinds,
                now,
                deny.as_deref(),
            )
            .map_err(|e| e.to_string())?;
        let committed_unix = now as u32;
        let entry = live_share_decision_journal_entry(&updated, committed_unix);
        append_live_share_journal(&self.storage_root, &entry)?;
        Ok(entry)
    }

    pub fn get_live_share_record(
        &self,
        request_id: &str,
    ) -> Result<Option<super::live_share::LiveShareRequestRecord>, String> {
        LiveShareStore::open(&self.storage_root)
            .map_err(|e| e.to_string())?
            .get_request(request_id)
            .map_err(|e| e.to_string())
    }

    pub fn list_pending_live_shares(
        &self,
        limit: usize,
    ) -> Result<Vec<LiveSectionRequest>, String> {
        LiveShareStore::open(&self.storage_root)
            .map_err(|e| e.to_string())?
            .list_pending(limit)
            .map_err(|e| e.to_string())
    }

    pub fn register_usage_agreement(&self, agreement: &UsageAgreement) -> Result<(), String> {
        LiveShareStore::open(&self.storage_root)
            .map_err(|e| e.to_string())?
            .save_usage_agreement(agreement)
            .map_err(|e| e.to_string())
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

    #[test]
    fn add_disputed_diagnosis_journal_kind() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let report = DisputedDiagnosisReport::new("Bipolar disorder");
        let entry = host.add_disputed_diagnosis(&report).unwrap();
        assert_eq!(entry.kind, "disputed_diagnosis");
        assert!(entry.id.contains(":disputed_diagnosis:"));
    }

    #[test]
    fn add_housing_safety_journal_kind() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let mut report = HousingSafetyReport::new();
        report.dwelling_type = wellfare_core::personal_records::DwellingType::MobileShelter;
        report.homelessness = true;
        let entry = host.add_housing_safety(&report).unwrap();
        assert_eq!(entry.kind, "housing_safety");
    }

    #[test]
    fn ledger_entries_commit_and_balance_by_currency() {
        use wellfare_core::finance::LedgerEntry;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        host.add_ledger_entry(&LedgerEntry::new("Wage", 250_000, "AUD", 1_700_000_000))
            .unwrap();
        host.add_ledger_entry(&LedgerEntry::new("Groceries", -42_000, "AUD", 1_700_000_100))
            .unwrap();
        host.add_ledger_entry(&LedgerEntry::new("Grant (USD)", 100_000, "usd", 1_700_000_200))
            .unwrap();

        let rows = host.list_ledger_entries(16).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().all(|r| r.kind == "ledger_entry"));

        let balance = host.ledger_balance(64).unwrap();
        assert_eq!(balance.total_entries, 3);
        let aud = balance.by_currency.iter().find(|c| c.currency == "AUD").unwrap();
        assert_eq!(aud.net_cents, 208_000);
        assert_eq!(aud.entry_count, 2);
        let usd = balance.by_currency.iter().find(|c| c.currency == "USD").unwrap();
        assert_eq!(usd.net_cents, 100_000);
    }

    #[test]
    fn contributions_commit_and_obligations_derive_through_journal() {
        use wellfare_core::projects::{Contribution, Project};
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let p = Project::new("Community Garden", "shared beds", 1_700_000_000);
        host.add_project(&p).unwrap();
        host.add_contribution(&Contribution::new(&p.id, "did:wf:owner", "dig", 60, 1_700_000_050))
            .unwrap();
        host.add_contribution(&Contribution::new(&p.id, "did:wf:owner", "plant", 30, 1_700_000_100))
            .unwrap();

        let obligations = host.project_obligations(64).unwrap();
        let owner = obligations
            .iter()
            .find(|o| o.project_id == p.id && o.contributor_did == "did:wf:owner")
            .unwrap();
        assert_eq!(owner.total_effort_minutes, 90);
        assert_eq!(owner.contribution_count, 2);
    }

    #[test]
    fn credential_commits_with_credential_kind() {
        use wellfare_core::credentials::CredentialRecord;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let cred = CredentialRecord::new(
            "did:wf:issuer",
            "did:wf:owner",
            "ProofOfAddress",
            1_700_000_000,
        )
        .with_claim("postcode", "3000");
        let entry = host.add_credential(&cred).unwrap();
        assert_eq!(entry.kind, "credential");
        assert_eq!(host.list_credentials(8).unwrap().len(), 1);
    }

    #[test]
    fn credential_claims_persist_and_presentation_selects_fields() {
        use wellfare_core::credentials::CredentialRecord;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let cred = CredentialRecord::new(
            "did:wf:issuer",
            "did:wf:owner",
            "ProofOfAddress",
            1_700_000_000,
        )
        .with_claim("full_name", "Jane Roe")
        .with_claim("street", "1 Camper Lane")
        .with_claim("postcode", "3000");
        let entry = host.add_credential(&cred).unwrap();

        // Full claims survive via the content-addressed blob (not just claim_count).
        let loaded = host.get_credential(&entry.id).unwrap().unwrap();
        assert_eq!(loaded.claims.len(), 3);
        assert_eq!(loaded.issuer_did, "did:wf:issuer");

        // Presentation discloses only the selected claim keys; the rest never appear.
        let pres = host
            .present_credential(&entry.id, &["full_name".into(), "postcode".into()])
            .unwrap();
        assert_eq!(pres.disclosed_claims.len(), 2);
        assert!(pres.disclosed_claims.iter().any(|(k, _)| k == "full_name"));
        assert!(pres.disclosed_claims.iter().all(|(k, _)| k != "street"));
    }

    #[test]
    fn outbound_sync_operation_signed_and_inbox_dedupes_replay() {
        use wellfare_core::finance::LedgerEntry;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let entry = host
            .add_ledger_entry(&LedgerEntry::new("Wage", 100_000, "AUD", 1_700_000_000))
            .unwrap();
        let op = host
            .build_outbound_operation(&entry, 1)
            .expect("a Restricted entry yields an outbound operation");
        assert!(op.signature.as_deref().is_some_and(|s| !s.is_empty()));
        assert_eq!(op.kind, "ledger_entry");

        // A separate peer receives it: first admit validates, replays are idempotent.
        let dir2 = tempdir().unwrap();
        let peer = test_host(dir2.path());
        assert!(peer.admit_sync_operation(&op).unwrap().is_validated());
        assert_eq!(
            peer.admit_sync_operation(&op).unwrap(),
            crate::wellfair::sync_protocol::AdmitOutcome::Duplicate
        );
        assert_eq!(peer.validated_sync_operations().unwrap().len(), 1);
    }

    #[test]
    fn clinical_report_commits_with_clinical_kind() {
        use wellfare_core::clinical::ClinicalReportType;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let entry = host
            .add_clinical_report(
                "Full blood count",
                ClinicalReportType::Pathology,
                1_700_000_000,
                "Hb 140 g/L",
                Some("Dr Smith".into()),
            )
            .unwrap();
        assert_eq!(entry.kind, "clinical_report");
        assert_eq!(host.list_clinical_reports(8).unwrap().len(), 1);
    }

    #[test]
    fn clinical_attachment_stores_and_retrieves_bytes() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let bytes = b"%PDF-1.4 pretend pathology report bytes";
        let entry = host
            .add_clinical_attachment("path_report.pdf", "application/pdf", bytes)
            .unwrap();
        assert_eq!(entry.kind, "clinical_attachment");
        assert_eq!(host.list_clinical_attachments(8).unwrap().len(), 1);
        // The bytes round-trip out of the blob store, integrity-verified.
        let got = host.attachment_bytes(&entry.id).unwrap().unwrap();
        assert_eq!(got, bytes);
    }

    #[test]
    fn welfare_records_commit_and_list() {
        use wellfare_core::welfare_support::{StreamStatus, Urgency};
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        host.add_assistance_need("housing", "emergency accommodation", Urgency::Critical)
            .unwrap();
        host.add_welfare_stream("JobSeeker", Some("ref-42".into()), StreamStatus::Active)
            .unwrap();
        host.add_government_letter("Services Australia", "Payment review", true)
            .unwrap();
        let rows = host.list_welfare_records(16).unwrap();
        assert_eq!(rows.len(), 3);
        assert!(rows.iter().any(|r| r.kind == "assistance_need"));
        assert!(rows.iter().any(|r| r.kind == "welfare_stream"));
        assert!(rows.iter().any(|r| r.kind == "government_letter"));
    }

    #[test]
    fn synced_obligations_fold_in_validated_remote_contributions_replay_safe() {
        use wellfare_core::projects::{Contribution, Project};
        // Peer A commits a contribution and emits a signed sync operation for it.
        let dir_a = tempdir().unwrap();
        let mut peer_a = test_host(dir_a.path());
        let pa = Project::new("Shared Garden", "beds", 1_700_000_000);
        peer_a.add_project(&pa).unwrap();
        let a_entry = peer_a
            .add_contribution(&Contribution::new(&pa.id, "did:wf:alice", "dig", 60, 1_700_000_050))
            .unwrap();
        let remote_op = peer_a.build_outbound_operation(&a_entry, 5).unwrap();

        // Peer B has its own local contribution to the same project id.
        let dir_b = tempdir().unwrap();
        let mut peer_b = test_host(dir_b.path());
        peer_b
            .add_contribution(&Contribution::new(&pa.id, "did:wf:bob", "plant", 30, 1_700_000_100))
            .unwrap();

        // Local-only view: just Bob's 30 min.
        let local = peer_b.project_obligations(64).unwrap();
        assert_eq!(local.iter().map(|o| o.total_effort_minutes).sum::<u64>(), 30);

        // Admit the remote op, then the synced view includes Alice's 60 min too.
        assert!(peer_b.admit_sync_operation(&remote_op).unwrap().is_validated());
        let synced = peer_b.synced_project_obligations(64).unwrap();
        let alice = synced.iter().find(|o| o.contributor_did == "did:wf:alice").unwrap();
        assert_eq!(alice.total_effort_minutes, 60);
        let bob = synced.iter().find(|o| o.contributor_did == "did:wf:bob").unwrap();
        assert_eq!(bob.total_effort_minutes, 30);

        // Replaying the remote op does not double-count: the synced view is unchanged.
        assert_eq!(
            peer_b.admit_sync_operation(&remote_op).unwrap(),
            crate::wellfair::sync_protocol::AdmitOutcome::Duplicate
        );
        let after_replay = peer_b.synced_project_obligations(64).unwrap();
        assert_eq!(after_replay, synced);
    }

    #[test]
    fn classified_record_produces_no_outbound_operation() {
        use wellfare_core::mental_wellbeing::TherapyNote;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        // therapy_note is a Classified, sanctuary-protected kind (the encrypted vault holds
        // free-text sanctuary notes; the journal still carries other Classified kinds).
        let entry = host.add_therapy_note(&TherapyNote::new("private contingency")).unwrap();
        assert_eq!(entry.sensitivity, "Classified");
        assert!(host.build_outbound_operation(&entry, 1).is_none());
    }

    #[test]
    fn locked_sanctuary_hides_protected_kinds_from_graph_coverage() {
        use wellfare_core::mental_wellbeing::TherapyNote;
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        host.add_condition(&ConditionReport::new("Hypertension")).unwrap();
        host.add_therapy_note(&TherapyNote::new("private contingency")).unwrap();
        host.finalize_batch().unwrap();

        // Before Sanctuary is set up, coverage lists every kind.
        let unlocked = host.query_graph_coverage(32).unwrap();
        assert!(unlocked.iter().any(|r| r.kind == "therapy_note"));
        assert!(unlocked.iter().any(|r| r.kind == "condition"));

        // Once set up and locked, the protected kind is withheld from the coverage view.
        host.setup_sanctuary("real-pin-cov", "decoy-pin-cov").unwrap();
        host.lock_sanctuary().unwrap();
        let locked = host.query_graph_coverage(32).unwrap();
        assert!(locked.iter().all(|r| r.kind != "therapy_note"));
        assert!(locked.iter().any(|r| r.kind == "condition"));
    }
}