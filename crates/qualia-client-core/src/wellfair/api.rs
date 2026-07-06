use std::path::Path;

use std::time::{SystemTime, UNIX_EPOCH};

use super::accessibility_prefs;
use super::consent_store::ConsentGrantRecord;
use super::host_state::{
    AccessibilityPreferences, ConsentGrantDraft, GuardianshipProposalView, PolicyDecisionDto,
    SubmitOutcome,
};
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
use super::backup::{self, BackupReport};
use super::sync_outbox::{SyncOutbox, SyncOutboxEntry, SyncOutboxState};
use super::sync_transport::SyncTransport;
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
use qualia_cooperative_core::work_item::{
    build_work_item_envelope, build_work_item_status_envelope, derive_board,
    parse_work_item_status_summary, parse_work_item_summary, work_item_status_summary,
    work_item_summary, BoardColumn, WorkItem, WorkItemStatusEvent,
};
use qualia_cooperative_core::agency_delegation::{
    agency_delegation_full_json, build_agency_delegation_envelope, delegation_permits,
    parse_agency_delegation, AccessDecision, AccessRequest, AgencyDelegation, ConsentState,
    Precedence,
};
use qualia_cooperative_core::agency_domain::agency_domain_taxonomy;
use qualia_cooperative_core::taxonomy::Sphere;
use qualia_cooperative_core::trigger::TriggerContext;
use wellfare_core::guardianship::{
    build_proposal_envelope, build_vote_envelope, derive_status, parse_proposal_summary,
    parse_vote_summary, proposal_summary, vote_summary, GuardianshipProposal, GuardianshipVote,
    ProposalState,
};
use wellfare_core::authority_attestation::{
    authority_attestation_summary, build_authority_attestation_envelope, AgentInCapacity, Authority,
    AuthorityAttestation, Representation,
};
use wellfare_core::clinical::{
    build_clinical_attachment_envelope, build_clinical_report_envelope, clinical_attachment_summary,
    clinical_report_summary, AttachmentMeta, ClinicalReport, ClinicalReportType,
};
use wellfare_core::welfare_support::{
    build_assistance_need_envelope, build_government_letter_envelope, build_welfare_stream_envelope,
    AssistanceNeed, GovernmentLetter, StreamStatus, Urgency, WelfareStream,
};
use wellfare_core::assessment::{
    assessment_summary, build_assessment_envelope, instrument, instrument_dto, instruments,
    parse_assessment, score, AssessmentResult, InstrumentDto,
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
const QAPP_COOPERATIVE: &str = "qualia-cooperative";
const SOURCE_COOPERATIVE: &str = "qualia:cooperative";
const QAPP_GUARDIANSHIP: &str = "wellfair-guardianship";
const SOURCE_GUARDIANSHIP: &str = "wellfair:guardianship";

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
            false,
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
        match self.submit_record_guarded(qapp_id, envelope, source, summary)? {
            SubmitOutcome::Committed { quins } => Ok(quins),
            SubmitOutcome::Suspended { .. } => {
                Err("Policy requires guardian approval before write".into())
            }
        }
    }

    /// Policy-gated write that surfaces the guardian-escrow outcome instead of collapsing it to an
    /// error. A **proxy** write of a protected (Restricted) record does not commit immediately — it
    /// is held in a [`GuardianshipProposal`] pending M-of-N guardian co-signature (see
    /// [`Self::vote_guardianship_proposal`]). Non-proxy writes commit exactly as before.
    pub fn submit_record_guarded(
        &mut self,
        qapp_id: &str,
        envelope: RecordEnvelope,
        source: &str,
        summary: Option<String>,
    ) -> Result<SubmitOutcome, String> {
        let now = Self::now_unix();
        let grants = self
            .vault
            .list_active_consents(now)
            .map_err(|e| e.to_string())?;
        let is_proxy = envelope
            .proxy_did
            .as_deref()
            .map(|p| p != envelope.owner_did)
            .unwrap_or(false);
        let decision = self.policy.evaluate_access(
            qapp_id,
            "write_record",
            envelope.sensitivity,
            envelope.epistemic_status,
            &grants,
            now,
            is_proxy,
        );

        match &decision {
            DecisionResult::Deny { reasons } => {
                Err(format!("Policy denied: {}", reasons.join("; ")))
            }
            DecisionResult::Prompt { .. } => Err("Policy requires consent before write".into()),
            DecisionResult::Suspend { required_approvals } => {
                let proposal = self.escrow_proxy_write(&envelope, summary, *required_approvals)?;
                let threshold = proposal.threshold;
                Ok(SubmitOutcome::Suspended {
                    proposal_id: proposal.id,
                    threshold,
                })
            }
            DecisionResult::Permit { .. } => {
                let quins =
                    self.commit_permitted(qapp_id, &envelope, source, summary, &decision)?;
                Ok(SubmitOutcome::Committed { quins })
            }
        }
    }

    /// Commit a policy-permitted envelope through the signed vault path and emit its receipt.
    fn commit_permitted(
        &mut self,
        qapp_id: &str,
        envelope: &RecordEnvelope,
        source: &str,
        summary: Option<String>,
        decision: &DecisionResult,
    ) -> Result<usize, String> {
        let principal_did = qualia_core_db::q_hash(&self.owner_did);
        let committed = self
            .vault
            .commit_envelope(envelope, &self.signing_key, principal_did, source, summary)
            .map_err(|e| e.to_string())?;
        let ts = envelope.asserted_time_unix;
        let receipt = receipt_from_decision(
            qapp_id,
            &envelope.id,
            ts,
            decision,
            self.vault.last_checkpoint_hash(),
        );
        self.vault
            .append_receipt(&receipt)
            .map_err(|e| e.to_string())?;
        Ok(committed)
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

    /// 3D Anatomy Qapp — compute the whole-person systemic view for a lens (`"person"` /
    /// `"clinician"`). Reads the person's condition / medication / diet records, maps them onto body
    /// systems via the anatomy knowledge base, and returns the lens narrative + per-system burden +
    /// an honest account of what did not map. Read-only; a computed set of **hypotheses**, never a
    /// diagnosis. `convergence_threshold` is how many distinct adverse factors flag a system.
    pub fn compute_anatomy_view(
        &self,
        lens: &str,
        convergence_threshold: usize,
    ) -> Result<super::anatomy_view::AnatomyViewReport, String> {
        let conditions = self.list_journal_by_kind("condition", 256)?;
        let medications = self.list_journal_by_kind("medication", 256)?;
        let diet = self.list_journal_by_kind("diet", 256)?;
        Ok(super::anatomy_view::build_report_from_journal(
            &conditions,
            &medications,
            &diet,
            super::anatomy_view::parse_lens(lens),
            convergence_threshold,
        ))
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

    // --- Vault v2 (S6): per-session decoy audit, real→decoy curation, real-lane audit review ---

    /// Add a note, attributing a **decoy** (duress) write to `session_ref` — a fresh ref per duress
    /// unlock yields the git-like per-session branch in the audit DAG (ADR §10). Real-lane writes
    /// ignore `session_ref` (real activity is never audited). The host should mint one `session_ref`
    /// per unlock (e.g. a UUID) and reuse it for every write in that session.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn add_sanctuary_vault_note_in_session(
        &self,
        pin: &str,
        body: &str,
        session_ref: &str,
    ) -> Result<super::sanctuary_vault::SanctuaryLane, String> {
        super::sanctuary_vault::add_note_in_session(
            &self.storage_root,
            pin,
            body,
            Self::now_unix() as u32,
            session_ref,
        )
    }

    /// **Curate the decoy from a real session (ADR §3.2).** Write a plausible note into the decoy
    /// lane *without* the decoy PIN, so a coercer's re-unlock shows fresh, believable content.
    /// Requires the **real** PIN; the decoy/wrong PIN is rejected.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn curate_sanctuary_decoy_note(&self, real_pin: &str, body: &str) -> Result<(), String> {
        super::sanctuary_vault::real_curate_decoy_add_note(
            &self.storage_root,
            real_pin,
            body,
            Self::now_unix() as u32,
        )
    }

    /// **Review decoy activity from the real lane (ADR §3.1 / §10).** Decrypts every sealed
    /// decoy-session record, verifies chain integrity + each witnessed-prefix head anchor, advances
    /// the anchors, and returns the decrypted actions with an integrity verdict. Requires the
    /// **real** PIN. `session_count` is a proxy for "number of attackers", never a hard head-count.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn review_sanctuary_decoy_activity(
        &self,
        real_pin: &str,
    ) -> Result<super::sanctuary_vault::DecoyActivityReport, String> {
        super::sanctuary_vault::review_decoy_activity(&self.storage_root, real_pin)
    }

    /// Read the decoy-audit retention policy (ADR §8). **Real-session only** — requires the real PIN;
    /// the setting is invisible/unreachable from a decoy session. Defaults to auto-archive.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn get_sanctuary_decoy_retention_mode(
        &self,
        real_pin: &str,
    ) -> Result<qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode, String> {
        super::sanctuary_vault::get_retention_mode(&self.storage_root, real_pin)
    }

    /// Set the decoy-audit retention policy (ADR §8). **Real-session only** — requires the real PIN.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn set_sanctuary_decoy_retention_mode(
        &self,
        real_pin: &str,
        mode: qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode,
    ) -> Result<(), String> {
        super::sanctuary_vault::set_retention_mode(&self.storage_root, real_pin, mode)
    }

    // --- T1.2: OS-keychain vault wrapping (opt-in, off by default; recovery-gated) ---

    /// Is the on-disk Sanctuary vault keychain-wrapped (bound to an OS-keychain pepper)?
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sanctuary_vault_is_keychain_wrapped(&self) -> bool {
        super::sanctuary_vault::is_keychain_wrapped(&self.storage_root)
    }

    /// Opt-in: create the Sanctuary vault with an OS-keychain-held pepper so disk + PIN alone can't
    /// open it. Returns the hex **recovery code** the user MUST record — losing the keychain entry
    /// otherwise loses the vault. The ordinary [`Self::setup_sanctuary_vault`] path stays unwrapped.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn setup_sanctuary_vault_wrapped(
        &self,
        real_pin: &str,
        decoy_pin: &str,
    ) -> Result<String, String> {
        super::sanctuary_vault::setup_wrapped(&self.storage_root, real_pin, decoy_pin)
    }

    /// Recover a keychain-wrapped vault on a device whose keychain entry is missing, using the
    /// recovery code from [`Self::setup_sanctuary_vault_wrapped`]. Re-seats the pepper on success.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sanctuary_vault_unlock_with_recovery(
        &self,
        pin: &str,
        recovery_code_hex: &str,
    ) -> Result<super::sanctuary_vault::SanctuaryLane, String> {
        super::sanctuary_vault::unlock_with_recovery(&self.storage_root, pin, recovery_code_hex)
    }

    // --- WP2: Package & Publish a qapp as an installable PWA bundle (companion-PWA P0/WP2) ---

    /// Author a qapp from discrete fields and write its installable PWA bundle to `target_dir`.
    /// Returns the written (bundle-relative) file paths. Serving the bundle over a secure origin so
    /// a phone can install it is a later stage (P1); this produces the artifact.
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(clippy::too_many_arguments)]
    pub fn publish_qapp_pwa(
        &self,
        target_dir: &str,
        id: &str,
        name: &str,
        kind: &str,
        description: &str,
        capabilities_csv: &str,
        wasm_filename: &str,
    ) -> Result<Vec<String>, String> {
        let manifest = super::qapp_publish::build_manifest(
            id,
            name,
            kind,
            description,
            capabilities_csv,
            wasm_filename,
        );
        super::qapp_publish::write_pwa_bundle(std::path::Path::new(target_dir), &manifest)
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

    // --- Sync transport orchestration (T3.1: drain outbox → transport → peer inbox) ---

    /// The next Lamport value to stamp on outbound operations: one past the greatest observed among
    /// the locally-validated inbox operations. Keeps outbound clocks causally ahead of what we've seen.
    fn next_sync_lamport(&self) -> Result<u64, String> {
        let max = self
            .validated_sync_operations()?
            .iter()
            .map(|o| o.lamport)
            .max()
            .unwrap_or(0);
        Ok(max + 1)
    }

    /// **Drain the outbox through a transport.** For each `Queued` outbox entry, build a signed
    /// [`SyncOperation`] from its committed journal entry and publish it; on success the entry is
    /// marked `Sent`. Classified/Sanctuary records never enter the ordinary lane — they are marked
    /// `Rejected` so they stop being retried. Returns the number of operations published.
    ///
    /// The transport is a dumb pipe; correctness (dedup, convergence) is enforced by the peer's
    /// fail-closed inbox on the other side.
    pub fn sync_push_via<T: SyncTransport>(
        &self,
        transport: &T,
        limit: usize,
    ) -> Result<usize, String> {
        let outbox = SyncOutbox::open(&self.storage_root).map_err(|e| e.to_string())?;
        let queued: Vec<SyncOutboxEntry> = outbox
            .list_all()
            .map_err(|e| e.to_string())?
            .into_iter()
            .filter(|e| e.state == SyncOutboxState::Queued)
            .take(limit)
            .collect();
        if queued.is_empty() {
            return Ok(0);
        }
        let journal = self.list_health_records(512)?;
        let mut lamport = self.next_sync_lamport()?;
        let mut ops = Vec::new();
        let mut sent_ids = Vec::new();
        for entry in &queued {
            let Some(journal_entry) = journal.iter().find(|j| j.id == entry.record_id) else {
                continue; // the record is outside the recent window; leave it queued for a later drain
            };
            match self.build_outbound_operation(journal_entry, lamport) {
                Some(op) => {
                    lamport += 1;
                    ops.push(op);
                    sent_ids.push(entry.operation_id.clone());
                }
                None => {
                    // Classified/Sanctuary — never syncs; stop retrying it.
                    let _ = outbox.update_state(&entry.operation_id, SyncOutboxState::Rejected);
                }
            }
        }
        if ops.is_empty() {
            return Ok(0);
        }
        transport.publish(&ops)?;
        for id in &sent_ids {
            let _ = outbox.update_state(id, SyncOutboxState::Sent);
        }
        Ok(ops.len())
    }

    /// **Pull from a transport and admit into the quarantined inbox.** Every op is validated
    /// fail-closed on admission (bad signature/hash/version/oversize/Classified → `Rejected`;
    /// replays → `Duplicate`), so a hostile peer can only cause rejections. Returns the admission
    /// tally.
    pub fn sync_pull_via<T: SyncTransport>(
        &self,
        transport: &T,
        since: u64,
    ) -> Result<SyncPullReport, String> {
        let ops = transport.pull(since)?;
        let mut report = SyncPullReport {
            pulled: ops.len(),
            validated: 0,
            duplicate: 0,
            rejected: 0,
        };
        for op in &ops {
            match self.admit_sync_operation(op)? {
                AdmitOutcome::Validated => report.validated += 1,
                AdmitOutcome::Duplicate => report.duplicate += 1,
                AdmitOutcome::Rejected(_) => report.rejected += 1,
            }
        }
        Ok(report)
    }

    /// One-shot sync against an HTTP relay (the production wire): drain the outbox to the relay,
    /// then pull + admit from it. Returns `(pushed, pull_report)`. Native-only (`reqwest`).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sync_with_http_relay(
        &self,
        base_url: &str,
        since: u64,
    ) -> Result<(usize, SyncPullReport), String> {
        let transport = super::sync_transport::HttpRelayTransport::new(base_url);
        let pushed = self.sync_push_via(&transport, 256)?;
        let report = self.sync_pull_via(&transport, since)?;
        Ok((pushed, report))
    }

    /// One-shot sync against a **libp2p** peer/relay (noise-encrypted request-response — the peer-to-peer
    /// wire): drain the outbox to the peer, then pull + admit from it. `peer_id` is the base58 peer id,
    /// `peer_addr` a libp2p multiaddr (e.g. `/ip4/1.2.3.4/tcp/4001`). Returns `(pushed, pull_report)`.
    /// Native-only (libp2p). Same dumb-pipe contract as [`Self::sync_with_http_relay`]: correctness is
    /// enforced by the fail-closed inbox, not the transport.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn sync_with_libp2p_peer(
        &self,
        peer_id: &str,
        peer_addr: &str,
        since: u64,
    ) -> Result<(usize, SyncPullReport), String> {
        let transport = super::sync_transport::Libp2pSyncTransport::connect(peer_id, peer_addr)?;
        let pushed = self.sync_push_via(&transport, 256)?;
        let report = self.sync_pull_via(&transport, since)?;
        Ok((pushed, report))
    }

    // --- Backup / restore of the WellFair data subtree (T3.3) ---

    /// Build a portable backup of this node's WellFair data (the `wellfair/` subtree) as archive
    /// bytes. The Sanctuary vault stays encrypted inside it.
    pub fn export_backup_bytes(&self) -> Result<Vec<u8>, String> {
        backup::create_backup(&self.storage_root, Self::now_unix() as u32)
    }

    /// Restore a backup (archive bytes) into this node's storage. Path-traversal-safe.
    pub fn import_backup_bytes(&self, bytes: &[u8]) -> Result<BackupReport, String> {
        backup::restore_backup(&self.storage_root, bytes)
    }

    /// Write a backup archive to `path`; returns the file count + archive size.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn export_backup_to_path(&self, path: &str) -> Result<BackupReport, String> {
        let archive = backup::build_archive(&self.storage_root, Self::now_unix() as u32)?;
        let files = archive.files.len();
        let bytes = backup::encode_archive(&archive)?;
        let size = bytes.len() as u64;
        std::fs::write(path, &bytes).map_err(|e| e.to_string())?;
        Ok(BackupReport { files, bytes: size })
    }

    /// Restore a backup archive from `path` into this node's storage.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn import_backup_from_path(&self, path: &str) -> Result<BackupReport, String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        self.import_backup_bytes(&bytes)
    }

    /// A node health/status snapshot (record counts, sync queue depths, data footprint, Sanctuary
    /// state, build version). Native-only (reads the on-disk Sanctuary vault state).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn diagnostics_report(&self) -> Result<DiagnosticsReport, String> {
        let journal_records = self.list_health_records(4096)?.len();
        let outbox_queued = SyncOutbox::open(&self.storage_root)
            .map_err(|e| e.to_string())?
            .count_queued()
            .map_err(|e| e.to_string())?;
        let inbox_validated = self.validated_sync_operations()?.len();
        let (data_files, data_bytes) = backup::wellfair_data_stats(&self.storage_root)?;
        Ok(DiagnosticsReport {
            crate_version: env!("CARGO_PKG_VERSION").to_string(),
            sanctuary_configured: super::sanctuary_vault::is_configured(&self.storage_root),
            sanctuary_keychain_wrapped: super::sanctuary_vault::is_keychain_wrapped(
                &self.storage_root,
            ),
            journal_records,
            outbox_queued,
            inbox_validated,
            data_files,
            data_bytes,
        })
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

    /// Read the blob bytes for any record that carries a `blob_hash` (clinical attachments,
    /// government-letter documents, …), integrity-verified by the blob store.
    pub fn attachment_bytes(&self, record_id: &str) -> Result<Option<Vec<u8>>, String> {
        let Some(entry) = self
            .list_health_records(256)?
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

    /// Record a general **authority attestation** — the ontological generalization of a government
    /// letter: an authorizing body (extensible type + jurisdiction + department) attested by an
    /// agent-in-capacity, delivered as a PDF, a credential, or a PDF-with-embedded-credential.
    /// `add_government_letter` remains a preset (`authority:government`, PDF) of this model.
    #[allow(clippy::too_many_arguments)]
    pub fn add_authority_attestation(
        &mut self,
        authority_type: &str,
        authority_label: &str,
        jurisdiction: Option<String>,
        department: Option<String>,
        agent_name: Option<String>,
        agent_capacity: Option<String>,
        representation: &str,
        subject: &str,
        statement: &str,
        action_required: bool,
    ) -> Result<JournalEntry, String> {
        let issued = Self::now_unix() as u32;
        let authority = Authority::new(authority_type, authority_label);
        let representation = match representation.to_ascii_lowercase().as_str() {
            "credential" => Representation::Credential,
            "pdf_with_embedded_credential" | "both" => Representation::PdfWithEmbeddedCredential,
            _ => Representation::Pdf,
        };
        let mut att = AuthorityAttestation::new(authority, subject, statement, issued)
            .with_representation(representation)
            .with_action_required(action_required);
        if let Some(j) = jurisdiction {
            att = att.with_jurisdiction(j);
        }
        if let Some(d) = department {
            att = att.with_department(d);
        }
        if let (Some(n), Some(c)) = (agent_name, agent_capacity) {
            att = att.with_agent(AgentInCapacity::new(n, c));
        }
        let envelope = build_authority_attestation_envelope(
            &att,
            &self.owner_did,
            &self.author_did,
            issued,
        );
        let summary = authority_attestation_summary(&att);
        self.submit_record_with_summary(QAPP_WELFARE, envelope, SOURCE_WELFARE, Some(summary))?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    /// Record a government letter together with its document bytes (stored as a content-addressed
    /// blob; the letter's `attachment_blob_hash` is that blob's hash, retrievable via `attachment_bytes`).
    pub fn add_government_letter_attachment(
        &mut self,
        sender: &str,
        subject: &str,
        action_required: bool,
        bytes: &[u8],
    ) -> Result<JournalEntry, String> {
        let hash = BlobStore::open(&self.storage_root)
            .and_then(|store| store.put(bytes))
            .map_err(|e| e.to_string())?;
        let mut letter = GovernmentLetter::new(sender, subject, Self::now_unix() as u32);
        letter.action_required = action_required;
        letter.attachment_blob_hash = Some(hash);
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

    // --- Cooperative work items (shared cooperative-core domain; plan §8, WP3) ---
    //
    // Work items persist through the same signed journal/policy path as WellFair records; a
    // future dedicated cooperative service may take over persistence, but the domain types and
    // derivations already live in `qualia-cooperative-core` so the Cooperative Qapp and the
    // WellFair panels share one implementation.

    pub fn add_work_item(&mut self, item: &WorkItem) -> Result<JournalEntry, String> {
        let asserted = Self::now_unix() as u32;
        let envelope =
            build_work_item_envelope(item, &self.owner_did, &self.author_did, asserted);
        let summary = work_item_summary(item);
        self.submit_record_with_summary(
            QAPP_COOPERATIVE,
            envelope,
            SOURCE_COOPERATIVE,
            Some(summary),
        )?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    /// Append an immutable status transition. The current status is a derived projection
    /// (latest event), never a mutated field — so replayed transitions can't corrupt the board.
    pub fn add_work_item_status(
        &mut self,
        event: &WorkItemStatusEvent,
    ) -> Result<JournalEntry, String> {
        let asserted = Self::now_unix() as u32;
        let envelope =
            build_work_item_status_envelope(event, &self.owner_did, &self.author_did, asserted);
        let summary = work_item_status_summary(event);
        self.submit_record_with_summary(
            QAPP_COOPERATIVE,
            envelope,
            SOURCE_COOPERATIVE,
            Some(summary),
        )?;
        self.finalize_batch().ok();
        self.latest_journal_entry()
    }

    pub fn list_work_items(&self, limit: usize) -> Result<Vec<JournalEntry>, String> {
        self.list_journal_by_kind("work_item", limit)
    }

    /// Derive the Kanban board for a project from committed work items and their status events.
    /// Pure over the unique-event-id set, so duplicate/replayed transitions never mis-place a card.
    pub fn work_item_board(
        &self,
        project_id: &str,
        limit: usize,
    ) -> Result<Vec<BoardColumn>, String> {
        let rows = self.list_health_records(limit)?;
        let mut items = Vec::new();
        let mut events = Vec::new();
        for row in rows {
            let Some(ref summary) = row.summary else { continue };
            match row.kind.as_str() {
                "work_item" => {
                    if let Some(item) = parse_work_item_summary(summary) {
                        if item.project_id == project_id {
                            items.push(item);
                        }
                    }
                }
                "work_item_status" => {
                    if let Some(ev) = parse_work_item_status_summary(summary) {
                        events.push(ev);
                    }
                }
                _ => {}
            }
        }
        Ok(derive_board(&items, &events))
    }

    // --- Agency layer: supported-agency delegations (ADR §7–§10; cooperative-core agency_*) -------
    //
    // A delegation binds a principal to their agent(s) for a *domain of agency* under an authority
    // profile + values anchor, gated by an optional trigger and fail-closed ABAC. Persisted through
    // the same signed journal path as other Restricted records (self-authored → commits; a proxy
    // write would suspend into guardianship, T1.5). The **lossless** delegation JSON is stored as the
    // record summary so the full object reconstructs on read; updates append a superseding version of
    // the same delegation id (latest-wins projection in `list_agency_delegations`).

    /// Persist a delegation (create or supersede). Returns the committed journal entry.
    pub fn add_agency_delegation(
        &mut self,
        delegation: &AgencyDelegation,
    ) -> Result<JournalEntry, String> {
        let asserted = Self::now_unix() as u32;
        let envelope =
            build_agency_delegation_envelope(delegation, &self.owner_did, &self.author_did, asserted);
        let summary = agency_delegation_full_json(delegation);
        self.submit_record_with_summary(
            QAPP_COOPERATIVE,
            envelope,
            SOURCE_COOPERATIVE,
            Some(summary),
        )?;
        self.finalize_batch().ok();
        self.list_journal_by_kind("agency_delegation", 1)?
            .into_iter()
            .next()
            .ok_or_else(|| "agency delegation committed but journal empty".into())
    }

    /// Build and persist a new delegation from primitive fields (so the Tauri layer needs no
    /// cooperative-core types). Validates the domain against the seeded taxonomy; an empty
    /// `values_anchor` defaults to the UN-HR anchor (`urn:un:hr:udhr`). Returns the created record.
    #[allow(clippy::too_many_arguments)]
    pub fn create_agency_delegation(
        &mut self,
        principal_did: &str,
        domain: &str,
        values_anchor: &str,
        agent_dids: Vec<String>,
        precedence: &str,
        consent: &str,
    ) -> Result<AgencyDelegation, String> {
        if agency_domain_taxonomy().get(domain).is_none() {
            return Err(format!("unknown domain of agency: {domain}"));
        }
        let anchor = if values_anchor.trim().is_empty() {
            "urn:un:hr:udhr"
        } else {
            values_anchor
        };
        let mut d =
            AgencyDelegation::new(principal_did, domain, anchor, Self::now_unix() as u32);
        d.agent_dids = agent_dids
            .into_iter()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        d.precedence = match precedence {
            "secondary" => Precedence::Secondary,
            "local_temporary" => Precedence::LocalTemporary,
            _ => Precedence::Primary,
        };
        d.consent = agency_consent_from_str(consent).unwrap_or(ConsentState::Pending);
        self.add_agency_delegation(&d)?;
        Ok(d)
    }

    /// List the current delegations — latest version per delegation id (updates supersede).
    ///
    /// The journal is append-only and lists **newest-first**, so the first record seen for a given
    /// logical delegation id is its latest version (append order == version order). This is robust
    /// even when several versions share the same `asserted_time_unix` second.
    pub fn list_agency_delegations(&self, limit: usize) -> Result<Vec<AgencyDelegation>, String> {
        use std::collections::HashSet;
        let entries = self.list_journal_by_kind("agency_delegation", limit)?;
        let mut seen: HashSet<String> = HashSet::new();
        let mut out: Vec<AgencyDelegation> = Vec::new();
        for e in entries {
            let Some(summary) = e.summary.as_deref() else {
                continue;
            };
            let Some(d) = parse_agency_delegation(summary) else {
                continue;
            };
            if seen.insert(d.id.clone()) {
                out.push(d); // first-seen (newest-first order) == the latest version
            }
        }
        out.sort_by(|a, b| {
            a.valid_from_unix
                .cmp(&b.valid_from_unix)
                .then_with(|| a.id.cmp(&b.id))
        });
        Ok(out)
    }

    /// Fetch a single current delegation by its logical id.
    pub fn get_agency_delegation(&self, delegation_id: &str) -> Result<AgencyDelegation, String> {
        self.list_agency_delegations(512)?
            .into_iter()
            .find(|d| d.id == delegation_id)
            .ok_or_else(|| format!("agency delegation '{delegation_id}' not found"))
    }

    /// Update the principal's consent state (grant / withdraw) — appends a superseding version.
    pub fn set_agency_delegation_consent(
        &mut self,
        delegation_id: &str,
        consent: ConsentState,
    ) -> Result<JournalEntry, String> {
        let mut d = self.get_agency_delegation(delegation_id)?;
        d.consent = consent;
        self.add_agency_delegation(&d)
    }

    /// Revoke a delegation — appends a superseding, revoked version (revocation is monotonic).
    pub fn revoke_agency_delegation(
        &mut self,
        delegation_id: &str,
    ) -> Result<JournalEntry, String> {
        let mut d = self.get_agency_delegation(delegation_id)?;
        d.revoked = true;
        self.add_agency_delegation(&d)
    }

    /// The seeded domains of agency (id + label + description + consequential/selfhood flags), for a
    /// delegation-creation picker. Category terms are excluded — only the 17 leaf domains.
    pub fn list_agency_domains(&self) -> Vec<AgencyDomainInfo> {
        let tax = agency_domain_taxonomy();
        tax.all()
            .iter()
            .filter(|t| t.category.is_some())
            .map(|t| AgencyDomainInfo {
                id: t.id.clone(),
                label: t.label.clone(),
                category: t.category.clone(),
                description: t.description.clone(),
                consequential: t.attr("consequential") == Some("true"),
                selfhood: t.sphere() == Sphere::Selfhood,
            })
            .collect()
    }

    /// Evaluate the fail-closed ABAC for a delegation against an access request built from the
    /// delegation's own domain. `action` is `"read" | "write" | "decide"`. Uses a bare trigger
    /// context (now only) — trigger-gated delegations therefore read as inactive here; supplying a
    /// richer context (events/attestations) is a follow-up. Demonstrates the safety invariants:
    /// selfhood default-deny, and consequential judgements requiring declared provenance + horizon.
    pub fn evaluate_agency_access(
        &self,
        delegation_id: &str,
        action: &str,
        data_class: &str,
    ) -> Result<AccessDecision, String> {
        let d = self.get_agency_delegation(delegation_id)?;
        let tax = agency_domain_taxonomy();
        let sphere = match tax.get(&d.domain).map(|t| t.sphere()) {
            Some(Sphere::Selfhood) => Sphere::Selfhood,
            _ => Sphere::Personhood,
        };
        let request = AccessRequest {
            domain: d.domain.clone(),
            data_class: data_class.to_string(),
            action: action.to_string(),
            sphere,
            jurisdiction: None,
            provenance: None,
        };
        let ctx = TriggerContext::at(Self::now_unix() as u32);
        Ok(delegation_permits(&d, &tax, &request, &ctx))
    }

    // --- Wellbeing self-assessment instruments (T2.2; PHQ-9 / GAD-7) ---------------------------
    //
    // A self-monitoring aid, not a diagnosis. Scoring is fail-closed in the domain layer; results
    // persist as Restricted records through the signed journal (lossless summary → reconstructs).

    /// The instruments this build ships (definitions: items, options, bands, disclaimer).
    pub fn list_assessment_instruments(&self) -> Vec<InstrumentDto> {
        instruments().into_iter().map(instrument_dto).collect()
    }

    /// Score `responses` against the given instrument and persist the result. Returns the scored
    /// outcome (total, band, interpretation, any safety flags). Errors if the instrument is unknown
    /// or the responses are the wrong count / out of range (fail-closed in `score`).
    pub fn record_assessment(
        &mut self,
        instrument_id: &str,
        responses: Vec<u8>,
    ) -> Result<AssessmentResult, String> {
        let inst = instrument(instrument_id)
            .ok_or_else(|| format!("unknown assessment instrument: {instrument_id}"))?;
        let now = Self::now_unix() as u32;
        let result = score(inst, &responses, now)?;
        let envelope =
            build_assessment_envelope(&result, &self.owner_did, &self.author_did, now);
        let summary = assessment_summary(&result);
        self.submit_record_with_summary(
            QAPP_WELLBEING,
            envelope,
            SOURCE_WELLBEING,
            Some(summary),
        )?;
        self.finalize_batch().ok();
        Ok(result)
    }

    /// Past assessment results, newest-first, reconstructed from the journal.
    pub fn list_assessments(&self, limit: usize) -> Result<Vec<AssessmentResult>, String> {
        let entries = self.list_journal_by_kind("wellbeing_assessment", limit)?;
        Ok(entries
            .iter()
            .filter_map(|e| e.summary.as_deref().and_then(parse_assessment))
            .collect())
    }

    // --- Guardianship approval escrow (M-of-N co-signature for proxy actions; T1.5) -------------
    //
    // Supported agency, not warden control: a proxy writing a protected record on the principal's
    // behalf suspends into a `GuardianshipProposal`; guardians co-sign with immutable votes; the
    // escrowed record commits on ratification. See `wellfare_core::guardianship`.

    /// Submit a record that may be a proxy action, surfacing the guardian-escrow outcome. Callers
    /// that set `envelope.proxy_did` use this instead of `submit_record` so a suspended write is a
    /// first-class result (a pending proposal), not an error.
    pub fn submit_proxy_record(
        &mut self,
        qapp_id: &str,
        envelope: RecordEnvelope,
        source: &str,
        summary: Option<String>,
    ) -> Result<SubmitOutcome, String> {
        let outcome = self.submit_record_guarded(qapp_id, envelope, source, summary)?;
        self.finalize_batch().ok();
        Ok(outcome)
    }

    /// A supporter records a condition **on the principal's behalf** (a proxy action). The write is
    /// escrowed for M-of-N guardian co-signature; the returned outcome carries the pending proposal
    /// id. This is the supported-agency entry point the desktop exposes for the approval tray.
    pub fn propose_proxy_condition(
        &mut self,
        proxy_did: &str,
        report: &wellfare_core::conditions::ConditionReport,
    ) -> Result<SubmitOutcome, String> {
        let asserted = Self::now_unix() as u32;
        let mut envelope =
            build_condition_envelope(report, &self.owner_did, proxy_did, asserted, None);
        envelope.proxy_did = Some(proxy_did.to_string());
        let summary = condition_summary(report);
        self.submit_proxy_record(QAPP_CLINICAL, envelope, SOURCE_CLINICAL, Some(summary))
    }

    /// Escrow a proxy write as a guardianship proposal pending M-of-N co-signature.
    fn escrow_proxy_write(
        &mut self,
        envelope: &RecordEnvelope,
        summary: Option<String>,
        threshold: u8,
    ) -> Result<GuardianshipProposal, String> {
        let proxy = envelope
            .proxy_did
            .clone()
            .unwrap_or_else(|| self.author_did.clone());
        let kind = wellfare_core::conditions::journal_kind_for_record_id(&envelope.id);
        let reason = format!(
            "Proxy write of a protected '{kind}' record on the principal's behalf requires guardian co-signature"
        );
        let proposal = GuardianshipProposal::new(
            &envelope.owner_did,
            proxy,
            threshold,
            envelope,
            summary,
            reason,
            Self::now_unix() as u32,
        );
        let asserted = Self::now_unix() as u32;
        let prop_env =
            build_proposal_envelope(&proposal, &self.owner_did, &self.author_did, asserted);
        // The proposal record is a non-proxy governance write → commits normally (no recursion).
        self.submit_record_with_summary(
            QAPP_GUARDIANSHIP,
            prop_env,
            SOURCE_GUARDIANSHIP,
            Some(proposal_summary(&proposal)),
        )?;
        self.finalize_batch().ok();
        Ok(proposal)
    }

    /// Pending and recently-resolved guardianship proposals for the approval tray.
    pub fn list_guardianship_proposals(
        &self,
        limit: usize,
    ) -> Result<Vec<GuardianshipProposalView>, String> {
        let rows = self.list_health_records(limit)?;
        let mut proposals = Vec::new();
        let mut votes = Vec::new();
        for row in &rows {
            let Some(ref summary) = row.summary else { continue };
            match row.kind.as_str() {
                "guardianship_proposal" => {
                    if let Some(p) = parse_proposal_summary(summary) {
                        proposals.push(p);
                    }
                }
                "guardianship_vote" => {
                    if let Some(v) = parse_vote_summary(summary) {
                        votes.push(v);
                    }
                }
                _ => {}
            }
        }
        let committed_ids: std::collections::HashSet<&str> =
            rows.iter().map(|r| r.id.as_str()).collect();
        let mut views: Vec<GuardianshipProposalView> = proposals
            .iter()
            .map(|p| {
                let status = derive_status(p, &votes);
                let committed = p
                    .escrowed_record_id()
                    .map(|id| committed_ids.contains(id.as_str()))
                    .unwrap_or(false);
                GuardianshipProposalView::from_status(p, &status, committed)
            })
            .collect();
        views.sort_by(|a, b| b.created_unix.cmp(&a.created_unix));
        Ok(views)
    }

    /// Record a guardian's co-signature (or objection). On ratification the escrowed record commits
    /// through the normal signed vault path; the commit is idempotent (a replayed final vote will
    /// not double-write the record).
    pub fn vote_guardianship_proposal(
        &mut self,
        proposal_id: &str,
        guardian_did: &str,
        approve: bool,
        reason: Option<String>,
    ) -> Result<GuardianshipProposalView, String> {
        let proposal = self
            .find_proposal(proposal_id)?
            .ok_or_else(|| format!("Unknown guardianship proposal: {proposal_id}"))?;

        let vote = GuardianshipVote::new(
            proposal_id,
            guardian_did,
            approve,
            reason,
            Self::now_unix() as u32,
        );
        let asserted = Self::now_unix() as u32;
        let vote_env = build_vote_envelope(&vote, &self.owner_did, &self.author_did, asserted);
        self.submit_record_with_summary(
            QAPP_GUARDIANSHIP,
            vote_env,
            SOURCE_GUARDIANSHIP,
            Some(vote_summary(&vote)),
        )?;
        self.finalize_batch().ok();

        let votes = self.list_guardianship_votes(proposal_id)?;
        let status = derive_status(&proposal, &votes);

        let mut committed = self.escrowed_already_committed(&proposal)?;
        if status.state == ProposalState::Ratified && !committed {
            if let Some(escrowed) = proposal.escrowed_envelope() {
                let decision = DecisionResult::Permit {
                    obligations: vec![
                        "guardianship_ratified".into(),
                        "emit_wal_receipt".into(),
                    ],
                };
                // Already M-of-N approved: commit through the signed path, bypassing re-escrow.
                self.commit_permitted(
                    QAPP_GUARDIANSHIP,
                    &escrowed,
                    SOURCE_GUARDIANSHIP,
                    proposal.escrowed_summary.clone(),
                    &decision,
                )?;
                self.finalize_batch().ok();
                committed = true;
            }
        }

        Ok(GuardianshipProposalView::from_status(
            &proposal, &status, committed,
        ))
    }

    fn find_proposal(&self, proposal_id: &str) -> Result<Option<GuardianshipProposal>, String> {
        let rows = self.list_journal_by_kind("guardianship_proposal", super::journal::MAX_LIST)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| r.summary.as_deref().and_then(parse_proposal_summary))
            .find(|p| p.id == proposal_id))
    }

    fn list_guardianship_votes(
        &self,
        proposal_id: &str,
    ) -> Result<Vec<GuardianshipVote>, String> {
        let rows = self.list_journal_by_kind("guardianship_vote", super::journal::MAX_LIST)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| r.summary.as_deref().and_then(parse_vote_summary))
            .filter(|v| v.proposal_id == proposal_id)
            .collect())
    }

    fn escrowed_already_committed(
        &self,
        proposal: &GuardianshipProposal,
    ) -> Result<bool, String> {
        let Some(escrowed_id) = proposal.escrowed_record_id() else {
            return Ok(false);
        };
        let kind = wellfare_core::conditions::journal_kind_for_record_id(&escrowed_id);
        let rows = self.list_journal_by_kind(kind, super::journal::MAX_LIST)?;
        Ok(rows.iter().any(|r| r.id == escrowed_id))
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

/// The admission tally from a [`WebizenHostApi::sync_pull_via`] round.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct SyncPullReport {
    /// Operations received from the transport.
    pub pulled: usize,
    /// Newly admitted as valid.
    pub validated: usize,
    /// Already-seen operation ids (idempotent replays).
    pub duplicate: usize,
    /// Failed fail-closed validation (bad signature/hash/version/oversize/Classified lane).
    pub rejected: usize,
}

/// A node health/status snapshot for support + the Sync & Backup panel.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DiagnosticsReport {
    pub crate_version: String,
    pub sanctuary_configured: bool,
    pub sanctuary_keychain_wrapped: bool,
    pub journal_records: usize,
    pub outbox_queued: usize,
    pub inbox_validated: usize,
    pub data_files: usize,
    pub data_bytes: u64,
}

/// A domain of agency, flattened for a delegation-creation picker (host → Tauri → Studio).
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct AgencyDomainInfo {
    pub id: String,
    pub label: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub category: Option<String>,
    pub description: String,
    pub consequential: bool,
    pub selfhood: bool,
}

/// Parse the wire form of a consent state (`"granted" | "withdrawn" | "pending" | "not_required"`)
/// into [`ConsentState`]. Used by the Tauri command layer.
pub fn agency_consent_from_str(s: &str) -> Result<ConsentState, String> {
    match s {
        "granted" => Ok(ConsentState::Granted),
        "withdrawn" => Ok(ConsentState::Withdrawn),
        "pending" => Ok(ConsentState::Pending),
        "not_required" => Ok(ConsentState::NotRequired),
        other => Err(format!("unknown consent state: {other}")),
    }
}

/// Parse the wire form of a decoy-audit retention mode (`"auto_archive"` | `"manual_triage"`) into
/// the [`RetentionMode`](qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode) enum. Used by
/// the Tauri command layer so the desktop crate needs no direct `qualia-core-db` dependency.
#[cfg(not(target_arch = "wasm32"))]
pub fn sanctuary_retention_mode_from_str(
    mode: &str,
) -> Result<qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode, String> {
    use qualia_core_db::crypto::sanctuary_audit_dag::RetentionMode;
    match mode {
        "auto_archive" => Ok(RetentionMode::AutoArchive),
        "manual_triage" => Ok(RetentionMode::ManualTriage),
        other => Err(format!("unknown retention mode: {other}")),
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
    fn agency_delegation_create_list_evaluate_supersede() {
        use qualia_cooperative_core::agency_delegation::AgencyDelegation;
        use qualia_cooperative_core::agency_domain::ids as dom;

        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());

        // 17 seeded domains available to the picker.
        assert_eq!(host.list_agency_domains().len(), 17);

        // Create a consented MEDICAL delegation to a carer.
        let mut d = AgencyDelegation::new("did:wf:alice", dom::MEDICAL, "urn:un:hr:udhr", 100);
        d.agent_dids = vec!["did:wf:carer".into()];
        d.consent = ConsentState::Granted;
        let entry = host.add_agency_delegation(&d).unwrap();
        assert_eq!(entry.kind, "agency_delegation");
        assert!(entry.id.contains(":agency-delegation:"));

        // It lists back losslessly (agent + domain preserved).
        let listed = host.list_agency_delegations(16).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].domain, dom::MEDICAL);
        assert_eq!(listed[0].agent_dids, vec!["did:wf:carer".to_string()]);

        // A read is permitted; a consequential (medical) *decide* is denied without provenance.
        assert!(host
            .evaluate_agency_access(&d.id, "read", "diagnosis")
            .unwrap()
            .is_permit());
        assert!(!host
            .evaluate_agency_access(&d.id, "decide", "diagnosis")
            .unwrap()
            .is_permit());

        // Withdraw consent → supersedes; the projection shows one delegation, now non-permitting.
        host.set_agency_delegation_consent(&d.id, ConsentState::Withdrawn)
            .unwrap();
        let listed = host.list_agency_delegations(16).unwrap();
        assert_eq!(listed.len(), 1, "supersede must not create a second logical delegation");
        assert_eq!(listed[0].consent, ConsentState::Withdrawn);
        assert!(!host
            .evaluate_agency_access(&d.id, "read", "diagnosis")
            .unwrap()
            .is_permit());

        // Revoke → still one logical delegation, now revoked.
        host.revoke_agency_delegation(&d.id).unwrap();
        let listed = host.list_agency_delegations(16).unwrap();
        assert_eq!(listed.len(), 1);
        assert!(listed[0].revoked);
    }

    #[test]
    fn wellbeing_assessment_record_score_and_list() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());

        // Ships PHQ-9 and GAD-7.
        let insts = host.list_assessment_instruments();
        assert_eq!(insts.len(), 2);
        assert!(insts.iter().any(|i| i.id == "phq9"));

        // Record a PHQ-9 that trips the self-harm flag (item 9 endorsed).
        let mut resp = vec![0u8; 9];
        resp[8] = 2;
        let result = host.record_assessment("phq9", resp).unwrap();
        assert_eq!(result.total, 2);
        assert_eq!(result.band_label, "Minimal");
        assert_eq!(result.flags.len(), 1, "self-harm flag must surface");

        // Persisted + reconstructed via the journal.
        let listed = host.list_assessments(16).unwrap();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].instrument_id, "phq9");
        assert_eq!(listed[0].flags.len(), 1);

        // Fail-closed: unknown instrument and bad response count are rejected (no record written).
        assert!(host.record_assessment("bdi2", vec![0; 9]).is_err());
        assert!(host.record_assessment("gad7", vec![0; 3]).is_err());
        assert_eq!(host.list_assessments(16).unwrap().len(), 1);
    }

    #[test]
    fn sync_push_pull_round_trip_over_in_memory_relay() {
        use crate::wellfair::sync_outbox::SyncOutbox;
        use crate::wellfair::sync_transport::InMemoryRelay;

        let relay = InMemoryRelay::new();
        let dir_a = tempdir().unwrap();
        let dir_b = tempdir().unwrap();
        let mut host_a = test_host(dir_a.path());
        let host_b = test_host(dir_b.path());

        // Host A commits a (Restricted) record — the vault auto-enqueues it to A's outbox.
        host_a.add_condition(&ConditionReport::new("Asthma")).unwrap();
        let queued_before = SyncOutbox::open(dir_a.path()).unwrap().count_queued().unwrap();
        assert!(queued_before >= 1);

        // Push drains the outbox onto the relay and marks entries Sent.
        let pushed = host_a.sync_push_via(&relay, 32).unwrap();
        assert_eq!(pushed, queued_before);
        assert_eq!(SyncOutbox::open(dir_a.path()).unwrap().count_queued().unwrap(), 0);
        assert_eq!(relay.len(), pushed);

        // Host B pulls + admits — the op lands in B's validated set.
        let report = host_b.sync_pull_via(&relay, 0).unwrap();
        assert_eq!(report.pulled, pushed);
        assert_eq!(report.validated, pushed);
        assert_eq!(report.rejected, 0);
        let validated = host_b.validated_sync_operations().unwrap();
        assert!(validated.iter().any(|o| o.kind == "condition"));

        // Re-pull is idempotent: duplicates, never double-admitted.
        let again = host_b.sync_pull_via(&relay, 0).unwrap();
        assert_eq!(again.validated, 0);
        assert_eq!(again.duplicate, pushed);
        assert_eq!(host_b.validated_sync_operations().unwrap().len(), validated.len());
    }

    #[test]
    fn backup_export_restore_reconstructs_a_working_node() {
        let src = tempdir().unwrap();
        let mut host = test_host(src.path());
        host.add_condition(&ConditionReport::new("Eczema")).unwrap();
        host.add_allergy(&AllergyReport::new("Penicillin")).unwrap();
        let before = host.list_health_records(16).unwrap();
        assert!(before.len() >= 2);

        // Export, then restore into a brand-new, empty node.
        let archive = host.export_backup_bytes().unwrap();
        let dst = tempdir().unwrap();
        let restored_host = test_host(dst.path());
        assert!(restored_host.list_health_records(16).unwrap().is_empty());

        let report = restored_host.import_backup_bytes(&archive).unwrap();
        assert!(report.files >= 1);

        // A fresh host over the restored storage reads the same records back.
        let reopened = test_host(dst.path());
        let after = reopened.list_health_records(16).unwrap();
        assert_eq!(after.len(), before.len());
        assert!(after.iter().any(|e| e.kind == "condition"));
        assert!(after.iter().any(|e| e.kind == "allergy"));
    }

    #[test]
    fn diagnostics_report_reflects_node_state() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        host.add_condition(&ConditionReport::new("Migraine")).unwrap();

        let report = host.diagnostics_report().unwrap();
        assert!(!report.crate_version.is_empty());
        assert!(report.journal_records >= 1);
        assert!(report.outbox_queued >= 1, "a committed record auto-enqueues to the outbox");
        assert!(report.data_files >= 1);
        assert!(report.data_bytes > 0);
        // No Sanctuary vault set up in this test.
        assert!(!report.sanctuary_configured);
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

    fn proxy_condition_envelope(id: &str) -> wellfare_core::record::RecordEnvelope {
        use wellfare_core::record::{
            EpistemicStatus, EvidenceType, RecordEnvelope, SensitivityClass,
        };
        RecordEnvelope {
            id: id.to_string(),
            owner_did: "did:wf:owner".into(),
            author_did: "did:wf:supporter".into(),
            proxy_did: Some("did:wf:supporter".into()),
            epistemic_status: EpistemicStatus::Asserted,
            evidence_type: EvidenceType::SelfReported,
            sensitivity: SensitivityClass::Restricted,
            asserted_time_unix: 1_700_000_000,
            valid_time_start_unix: Some(1_700_000_000),
            valid_time_end_unix: None,
            predecessor_id: None,
            blob_hash: None,
            tombstone: false,
        }
    }

    #[test]
    fn proxy_write_suspends_then_ratifies_and_commits() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());

        // A supporter (proxy) drafts a protected record on the principal's behalf.
        let env = proxy_condition_envelope("urn:wellfair:condition:proxy1");
        let outcome = host
            .submit_proxy_record(
                QAPP_CLINICAL,
                env,
                SOURCE_CLINICAL,
                Some("{\"label\":\"BP\"}".into()),
            )
            .unwrap();
        let proposal_id = match outcome {
            SubmitOutcome::Suspended { proposal_id, threshold } => {
                assert_eq!(threshold, 2);
                proposal_id
            }
            other => panic!("expected suspension, got {other:?}"),
        };

        // The escrowed record is NOT yet committed.
        assert!(host.list_journal_by_kind("condition", 64).unwrap().is_empty());

        // One approval → still pending.
        let v1 = host
            .vote_guardianship_proposal(&proposal_id, "did:wf:guardianA", true, None)
            .unwrap();
        assert_eq!(v1.state, "pending");
        assert!(!v1.committed);
        assert!(host.list_journal_by_kind("condition", 64).unwrap().is_empty());

        // Second distinct approval → ratified; the escrowed record commits.
        let v2 = host
            .vote_guardianship_proposal(&proposal_id, "did:wf:guardianB", true, None)
            .unwrap();
        assert_eq!(v2.state, "ratified");
        assert!(v2.committed);
        let committed = host.list_journal_by_kind("condition", 64).unwrap();
        assert_eq!(committed.len(), 1);
        assert_eq!(committed[0].id, "urn:wellfair:condition:proxy1");

        // A replayed final vote must not double-commit (idempotent).
        let v3 = host
            .vote_guardianship_proposal(&proposal_id, "did:wf:guardianB", true, None)
            .unwrap();
        assert!(v3.committed);
        assert_eq!(host.list_journal_by_kind("condition", 64).unwrap().len(), 1);

        // The tray shows it ratified + committed.
        let tray = host.list_guardianship_proposals(64).unwrap();
        let view = tray.iter().find(|p| p.proposal_id == proposal_id).unwrap();
        assert_eq!(view.state, "ratified");
        assert!(view.committed);
    }

    #[test]
    fn guardian_objection_denies_and_blocks_commit() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let env = proxy_condition_envelope("urn:wellfair:condition:proxy2");
        let outcome = host
            .submit_proxy_record(QAPP_CLINICAL, env, SOURCE_CLINICAL, None)
            .unwrap();
        let proposal_id = match outcome {
            SubmitOutcome::Suspended { proposal_id, .. } => proposal_id,
            other => panic!("expected suspension, got {other:?}"),
        };
        host.vote_guardianship_proposal(&proposal_id, "did:wf:guardianA", true, None)
            .unwrap();
        let denied = host
            .vote_guardianship_proposal(
                &proposal_id,
                "did:wf:guardianB",
                false,
                Some("not in her interest".into()),
            )
            .unwrap();
        assert_eq!(denied.state, "denied");
        assert!(!denied.committed);
        assert_eq!(denied.denied_by.as_deref(), Some("did:wf:guardianB"));
        assert!(host.list_journal_by_kind("condition", 64).unwrap().is_empty());
    }

    #[test]
    fn non_proxy_write_is_unaffected_by_escrow() {
        // Regression guard: an ordinary (non-proxy) write still commits directly, no proposal.
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let entry = host.add_condition(&ConditionReport::new("Asthma")).unwrap();
        assert_eq!(entry.kind, "condition");
        assert!(host.list_guardianship_proposals(64).unwrap().is_empty());
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
    fn work_items_commit_and_board_derives_replay_safe() {
        use qualia_cooperative_core::work_item::{
            WorkItem, WorkItemStatus, WorkItemStatusEvent, WorkItemType,
        };
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let wi = WorkItem::new("proj-A", WorkItemType::Task, "Write tests", 1_700_000_000);
        host.add_work_item(&wi).unwrap();
        host.add_work_item_status(&WorkItemStatusEvent::new(
            &wi.id,
            WorkItemStatus::InProgress,
            1_700_000_100,
        ))
        .unwrap();

        let board = host.work_item_board("proj-A", 64).unwrap();
        let in_progress = board
            .iter()
            .find(|c| c.status == WorkItemStatus::InProgress)
            .unwrap();
        assert_eq!(in_progress.cards.len(), 1);
        assert_eq!(in_progress.cards[0].title, "Write tests");
        // No card left in the default Todo column.
        let todo = board.iter().find(|c| c.status == WorkItemStatus::Todo).unwrap();
        assert!(todo.cards.is_empty());

        // A different project's board has no cards.
        assert!(host
            .work_item_board("proj-B", 64)
            .unwrap()
            .iter()
            .all(|c| c.cards.is_empty()));
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
    fn government_letter_attachment_stores_and_retrieves_bytes() {
        let dir = tempdir().unwrap();
        let mut host = test_host(dir.path());
        let bytes = b"%PDF-1.4 letter from the agency";
        let entry = host
            .add_government_letter_attachment("Services Australia", "Payment review", true, bytes)
            .unwrap();
        assert_eq!(entry.kind, "government_letter");
        // The generalized attachment_bytes reads any record's blob back.
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