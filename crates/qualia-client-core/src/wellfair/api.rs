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
        capital_cents: v.get("capital_cents").and_then(|x| x.as_u64()).unwrap_or(0),
        roi_multiplier: v
            .get("roi_multiplier")
            .and_then(|x| x.as_f64())
            .map(|f| f as f32)
            .unwrap_or(1.0),
        privacy_level: Default::default(),
        occurred_at_unix,
        predecessor_id: None,
    })
}

/// A per-entry summary of a hypermedia library entry for the UI (drops the raw quins).
fn library_summary(e: &super::hypermedia_store::LibraryEntry) -> serde_json::Value {
    serde_json::json!({
        "asset_uri": e.asset_uri,
        "media_type": e.media_type,
        "topics": e.topics,
        "projects": e.projects,
        "purposes": e.purposes,
        "place": e.place,
        "occurred_at": e.occurred_at,
        "lat": e.lat,
        "lon": e.lon,
        "flags": e.flags,
        "ingested_unix": e.ingested_unix,
        "excerpt": e.excerpt,
        "sensitivity": e.sensitivity,
        "section": e.section,
        "commons_visibility": e.commons_visibility,
        "is_secret": e.is_secret(),
        "cml_signals": e.cml_signals,
        "cml_concept_count": e.cml_concept_count,
        "cml_n3_chars": e.cml_n3.len(),
        "quin_count": e.quins.len(),
        "cof_segment_count": e.cof_segment_count,
        "cof_segment_index": e.cof_segment_index,
        "cof_profile": e.cof_profile,
        "cof_html_chars": e.cof_html.len(),
        "has_cof": !e.cof_html.is_empty(),
    })
}

/// Facets a **person** attaches to an asset at ingest — the "software provides the means, the person
/// authors the meaning" path. These merge *on top of* whatever a processor derived automatically (a photo's
/// EXIF still wins for its own time/place); they let a plain document be placed on the **timeline** (a date)
/// or the **map** (coordinates), or collected under a **project** / **purpose** — none of it imposed.
#[derive(Debug, Clone, Default)]
pub struct ManualFacets {
    pub occurred_at: Option<i64>,
    pub place_label: Option<String>,
    pub lat: Option<f32>,
    pub lon: Option<f32>,
    pub projects: Vec<String>,
    pub purposes: Vec<String>,
    /// `public` | `restricted` | `classified` — high sensitivity forces Secret section.
    pub sensitivity: Option<String>,
    /// Preferred product section: secret | wellfair | personal | work | tools | software | commons.
    pub section: Option<String>,
    /// `none` | `peers` | `commons` — social / micro-commons visibility.
    pub commons_visibility: Option<String>,
}

impl ManualFacets {
    fn is_empty(&self) -> bool {
        self.occurred_at.is_none()
            && self.place_label.is_none()
            && self.lat.is_none()
            && self.projects.is_empty()
            && self.purposes.is_empty()
            && self.sensitivity.is_none()
            && self.section.is_none()
            && self.commons_visibility.is_none()
    }
}

/// Decode a lowercase/uppercase hex string to bytes (the desktop passes binary assets — a JPEG is not utf-8 —
/// as hex across the command boundary). Dependency-free; rejects odd length / non-hex.
fn decode_hex(s: &str) -> Result<Vec<u8>, String> {
    let s = s.trim();
    if s.len() % 2 != 0 {
        return Err("odd-length hex".to_string());
    }
    let val = |c: u8| -> Result<u8, String> {
        match c {
            b'0'..=b'9' => Ok(c - b'0'),
            b'a'..=b'f' => Ok(c - b'a' + 10),
            b'A'..=b'F' => Ok(c - b'A' + 10),
            _ => Err(format!("non-hex byte {:#x}", c)),
        }
    };
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len() / 2);
    let mut i = 0;
    while i < b.len() {
        out.push((val(b[i])? << 4) | val(b[i + 1])?);
        i += 2;
    }
    Ok(out)
}

/// Parse a model string (`"male"` / `"female"`, case-insensitive) into an [`AnatomyModel`].
pub fn parse_anatomy_model(s: &str) -> Result<wellfare_core::anatomy::AnatomyModel, String> {
    match s.trim().to_ascii_lowercase().as_str() {
        "male" | "m" | "xy" => Ok(wellfare_core::anatomy::AnatomyModel::Male),
        "female" | "f" | "xx" => Ok(wellfare_core::anatomy::AnatomyModel::Female),
        _ => Err(format!("unknown anatomy model '{s}' (expected male/female)")),
    }
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


    /// Fetches a `.10d` asset on-demand instead of loading all assets upfront.
    pub fn fetch_10d_asset_on_demand(&self, _asset_uri: &str) -> Result<Vec<u8>, String> {
        // Returns the raw asset binary. For now, it returns an empty vector.
        Ok(Vec::new())
    }

    pub(crate) fn chora_storage_root(&self) -> &std::path::Path {
        &self.storage_root
    }

    /// The on-disk storage root for this host (where the asset cache + prefs live). Exposed so the
    /// desktop can run blocking acquisition off the async runtime without holding the host lock.
    pub fn storage_root(&self) -> &std::path::Path {
        &self.storage_root
    }

    pub(crate) fn chora_signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub(crate) fn chora_owner_did(&self) -> &str {
        &self.owner_did
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
        let state = self.get_physiological_state();
        Ok(super::anatomy_view::build_report_from_journal(
            &conditions,
            &medications,
            &diet,
            super::anatomy_view::parse_lens(lens),
            convergence_threshold,
            state,
        ))
    }

    /// 3D Anatomy Qapp — build the **whole-body render scene** (S5.7 interim visual) for the current
    /// records + declared physiological state, viewed from `(azimuth, elevation)` in degrees. Returns a
    /// [`webizen_render::scene_contract::RenderScene`] coloured by accumulated burden (σ → RGBA), ready
    /// for the headless `render_scene_png` pipeline. The orbit camera lets the Studio UI spin the body.
    /// Read-only; a computed visual of **hypotheses**, never a diagnosis.
    pub fn compute_body_scene(
        &self,
        azimuth_deg: f64,
        elevation_deg: f64,
    ) -> Result<webizen_render::scene_contract::RenderScene, String> {
        let report = self.compute_anatomy_view("person", 2)?;
        Ok(super::anatomy_render::body_scene(&report, azimuth_deg, elevation_deg))
    }

    // --- 3D Anatomy asset cache (S5.8 — user-triggered real-mesh acquisition) -------------------
    //
    // The person triggers a download of the CCF/HRA reference-organ GLB set from the live SPARQL
    // endpoint; the host fetches + compiles each to a sealed `.10d` and caches both under
    // `{storage_root}/assets/ccf/{model}/`. Subsequent runs load the cached `.10d` directly — no
    // re-download. The cache is the person's own, generated on demand.

    /// Whether the body assets for a model are cached + complete (manifest exists + every referenced
    /// `.10d` is on disk). `model` is `"male"` / `"female"` (case-insensitive).
    pub fn body_assets_status(
        &self,
        model: &str,
    ) -> Result<super::anatomy_assets::BodyAssetsStatus, String> {
        let m = parse_anatomy_model(model)?;
        Ok(super::anatomy_assets::status(&self.storage_root, m))
    }

    /// The cached organ keys for a model (empty if not cached).
    pub fn cached_organ_keys(&self, model: &str) -> Result<Vec<String>, String> {
        let m = parse_anatomy_model(model)?;
        Ok(super::anatomy_assets::cached_organ_keys(&self.storage_root, m))
    }

    /// Load a cached `.10d` for one organ. Returns the raw container bytes (for the browser portal's
    /// `load_10d_colored`).
    pub fn load_cached_organ_10d(
        &self,
        model: &str,
        organ_key: &str,
    ) -> Result<Vec<u8>, String> {
        let m = parse_anatomy_model(model)?;
        super::anatomy_assets::load_cached_10d(&self.storage_root, m, organ_key)
    }

    /// The per-organ dual-modality percepts for the cached organ set — so the browser portal knows what
    /// colour to paint each organ (σ → RGBA via `paint_organs`). Returns `(painted, unmapped)`.
    pub fn cached_body_organ_percepts(
        &self,
        model: &str,
    ) -> Result<(Vec<super::anatomy_view::OrganPercept>, Vec<String>), String> {
        let m = parse_anatomy_model(model)?;
        let organ_keys = super::anatomy_assets::cached_organ_keys(&self.storage_root, m);
        if organ_keys.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }
        let report = self.compute_anatomy_view("person", 2)?;
        let key_refs: Vec<&str> = organ_keys.iter().map(|s| s.as_str()).collect();
        Ok(report.paint_organs(&key_refs))
    }

    /// Clear the cache for a model (idempotent). The person can re-acquire later.
    pub fn clear_body_cache(&self, model: &str) -> Result<(), String> {
        let m = parse_anatomy_model(model)?;
        super::anatomy_assets::clear_cache(&self.storage_root, m)
    }

    /// The accumulative, traceable **score-card** + investigable hypotheses over the person's own records —
    /// the reading they can act on. Forum-internum / `Sanctuary`-class selfhood content; a set of
    /// **hypotheses** and pathway-starts, never a diagnosis, never a rating. The card is computed at the
    /// person's **declared physiological state** (their point on the reproductive continuum), or
    /// [`PhysiologicalState::Baseline`] if they have not declared one.
    pub fn compute_scorecard(
        &self,
        convergence_threshold: usize,
    ) -> Result<super::anatomy_view::WellbeingScorecardReport, String> {
        let conditions = self.list_journal_by_kind("condition", 256)?;
        let medications = self.list_journal_by_kind("medication", 256)?;
        let diet = self.list_journal_by_kind("diet", 256)?;
        // Read the person through **their own** weight model — their authorship of how they're read — falling
        // back to the seed *suggestion* only if they have not authored one.
        let weights = self.get_weight_model();
        // Read the person at **their declared physiological state** — their own statement of where they are
        // on the reproductive continuum — falling back to Baseline if they have not declared one.
        let state = self.get_physiological_state();
        Ok(super::anatomy_view::build_scorecard_report_from_journal_with_weights(
            &conditions,
            &medications,
            &diet,
            convergence_threshold,
            &weights,
            state,
        ))
    }

    /// The person's own score-card **weight model** — the interpretive lens the card uses — or the seed
    /// *suggestion* if they have not authored one. Theirs to see, edit, or reset; the software offers a
    /// starting point, it does not *define* how they are read.
    pub fn get_weight_model(&self) -> wellfare_core::anatomy::WeightModel {
        super::scorecard_prefs::load(&self.storage_root)
            .unwrap_or_else(wellfare_core::anatomy::seed_weight_model)
    }

    /// The seed **suggestion** on its own — so a UI can show "this is the starting point; here's yours" and
    /// let the person compare / adopt / edit.
    pub fn seed_weight_model(&self) -> wellfare_core::anatomy::WeightModel {
        wellfare_core::anatomy::seed_weight_model()
    }

    /// Whether the person has **authored their own** model (vs. still using the seed suggestion).
    pub fn weight_model_is_authored(&self) -> bool {
        super::scorecard_prefs::load(&self.storage_root).is_some()
    }

    /// **Set the person's own** weight model — their authorship of how the score-card reads them.
    pub fn set_weight_model(
        &self,
        model: &wellfare_core::anatomy::WeightModel,
    ) -> Result<(), String> {
        super::scorecard_prefs::save(&self.storage_root, model)
    }

    /// **Reset** to the seed suggestion (clears the person's authored model — a choice, always reversible by
    /// re-authoring).
    pub fn reset_weight_model(&self) -> Result<(), String> {
        super::scorecard_prefs::clear(&self.storage_root)
    }

    // --- Physiological state (P6 — the reproductive-continuum declaration) -----------------------
    //
    // The person's own statement of where they are on the reproductive continuum — their inward knowledge
    // of their own body. Forum-internum / Sanctuary-class. The score-card is computed at this state so it
    // reads them at their current life stage, not a neutral baseline.

    /// The person's **declared** physiological state, or [`PhysiologicalState::Baseline`] if they have not
    /// declared one. Their own statement; the software never assumes.
    pub fn get_physiological_state(&self) -> wellfare_core::anatomy::PhysiologicalState {
        super::physiology_prefs::load(&self.storage_root).unwrap_or(wellfare_core::anatomy::PhysiologicalState::Baseline)
    }

    /// Whether the person has **declared** their physiological state (vs. still at the implicit baseline).
    pub fn physiological_state_is_declared(&self) -> bool {
        super::physiology_prefs::load(&self.storage_root).is_some()
    }

    /// **Set** the person's declared physiological state — their own statement of where they are on the
    /// reproductive continuum. Forum-internum / Sanctuary-class.
    pub fn set_physiological_state(
        &self,
        state: &wellfare_core::anatomy::PhysiologicalState,
    ) -> Result<(), String> {
        super::physiology_prefs::save(&self.storage_root, state)
    }

    /// **Clear** the declared state — revert to the implicit [`PhysiologicalState::Baseline`]. Idempotent.
    pub fn reset_physiological_state(&self) -> Result<(), String> {
        super::physiology_prefs::clear(&self.storage_root)
    }

    // --- Accountability fabric (ADR 0011) — tamper-evident ledger + revocable consent credentials ---
    //
    // Turns the tested domain models (`crate::accountability_ledger`, `crate::consent_credential`) into a
    // usable loop: grant a worker scoped access, record how/why they acted (attributable, court-auditable),
    // let the person revoke (crypto-enforced — the key is destroyed, access ends), and keep the conduct trail
    // un-erasable. All acts are written into a signed, hash-chained ledger the person's own key signs; a
    // betrayer cannot quietly drop the inconvenient act without `verify()` naming it. Anti-deletion durability
    // across parties (commons replication) and real envelope encryption of the wrapped key are the deferred
    // composition steps (coordinate) — the wrapped key is carried as opaque bytes here, as the model intends.

    fn accountability_store(&self) -> Result<crate::accountability_store::AccountabilityStore, String> {
        crate::accountability_store::AccountabilityStore::open(&self.storage_root)
            .map_err(|e| e.to_string())
    }

    /// Append a raw record to the person's tamper-evident accountability ledger, signed by the owner key.
    pub fn ledger_append(
        &self,
        kind: &str,
        payload_json: &str,
    ) -> Result<crate::accountability_ledger::LedgerEntry, String> {
        self.accountability_store()?
            .append_ledger(kind, payload_json, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// Verify the whole ledger chain. `Ok(None)` = intact; `Ok(Some(tamper))` = a detected, named tamper.
    pub fn ledger_verify(
        &self,
    ) -> Result<Option<crate::accountability_ledger::LedgerTamper>, String> {
        let verdict = self.accountability_store()?.verify_ledger().map_err(|e| e.to_string())?;
        Ok(verdict.err())
    }

    /// The most-recent ledger entries (newest first), capped to `limit`.
    pub fn ledger_entries(
        &self,
        limit: usize,
    ) -> Result<Vec<crate::accountability_ledger::LedgerEntry>, String> {
        self.accountability_store()?.ledger_entries(limit).map_err(|e| e.to_string())
    }

    /// **Grant a consent credential** to an agent (e.g. a social worker) over a committed payload. The
    /// subject is the vault owner. `commitment_hex` is the 32-byte payload commitment; `wrapped_key_hex` is
    /// the (opaque) wrapped data key that revocation destroys; `expiry_unix` optionally auto-expires access.
    pub fn grant_consent_credential(
        &self,
        agent_did: &str,
        scope: &str,
        purpose: &str,
        commitment_hex: &str,
        wrapped_key_hex: &str,
        expiry_unix: Option<u64>,
    ) -> Result<crate::consent_credential::ConsentCredential, String> {
        let commitment = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let wrapped_key = hex::decode(wrapped_key_hex.trim())
            .map_err(|e| format!("wrapped key not hex: {e}"))?;
        let now = Self::now_unix();
        let id = {
            let digest = Sha256::digest(format!("{agent_did}:{scope}:{now}").as_bytes());
            format!("cc-{}", hex::encode(&digest[..6]))
        };
        let cred = crate::consent_credential::ConsentCredential::grant(
            id,
            &self.owner_did,
            agent_did,
            scope,
            purpose,
            commitment,
            wrapped_key,
            now,
            expiry_unix,
        );
        self.accountability_store()?
            .grant_credential(cred, &self.signing_key, now)
            .map_err(|e| e.to_string())
    }

    /// **Revoke a consent credential** — crypto-enforced (the wrapped key is destroyed). Returns whether a
    /// live credential was revoked. The conduct trail under it persists.
    pub fn revoke_consent_credential(&self, credential_id: &str) -> Result<bool, String> {
        self.accountability_store()?
            .revoke_credential(credential_id, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// All stored consent credentials (active and revoked — revoked rows remain as the audit anchor).
    pub fn list_consent_credentials(
        &self,
    ) -> Result<Vec<crate::consent_credential::ConsentCredential>, String> {
        self.accountability_store()?.list_credentials().map_err(|e| e.to_string())
    }

    /// **Record an agent's conduct** under a credential — signed (attributable + court-auditable) — into the
    /// durable trail and the tamper-evident ledger. Binds to the payload commitment, not the payload.
    pub fn record_conduct(
        &self,
        agent_did: &str,
        credential_id: &str,
        action: &str,
        reason: &str,
        commitment_hex: &str,
    ) -> Result<crate::consent_credential::ConductRecord, String> {
        let commitment = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .record_conduct(
                agent_did,
                credential_id,
                action,
                reason,
                commitment,
                &self.signing_key,
                Self::now_unix(),
            )
            .map_err(|e| e.to_string())
    }

    /// The **audit view** — every conduct record taken under one credential (survives its revocation).
    pub fn conduct_audit_trail(
        &self,
        credential_id: &str,
    ) -> Result<Vec<crate::consent_credential::ConductRecord>, String> {
        self.accountability_store()?.audit_trail(credential_id).map_err(|e| e.to_string())
    }

    /// **Record guardian notifications** from a flagged ingest into the tamper-evident ledger — so a flagged
    /// ingest under a guardianship relation is both a notification to the guardian AND an auditable,
    /// un-erasable event (who was notified, about what, when). Composes the hypermedia flags → guardian layer
    /// (`super::ingest_guardian`) with the accountability ledger.
    pub fn record_guardian_notifications(
        &self,
        notifications: &[super::ingest_guardian::GuardianNotification],
    ) -> Result<(), String> {
        let store = self.accountability_store()?;
        for n in notifications {
            let payload = serde_json::to_string(n).map_err(|e| e.to_string())?;
            store
                .append_ledger("guardian_notified", &payload, &self.signing_key, Self::now_unix())
                .map_err(|e| e.to_string())?;
        }
        Ok(())
    }

    // --- Hypermedia asset library: ingest a document → make it searchable by meaning ---

    fn library(&self) -> Result<super::hypermedia_store::HypermediaStore, String> {
        super::hypermedia_store::HypermediaStore::open(&self.storage_root).map_err(|e| e.to_string())
    }

    /// **Ingest a text document** into the library: derive its topics + searchable text, bind them into a
    /// hypermedia container, persist it (findable by meaning), and — if `guardian_did` is set (the principal
    /// is under a guardianship relation) and a flag is raised — notify the guardian **and record it in the
    /// tamper-evident ledger**. Returns a summary (topics, flags, any guardian notifications).
    pub fn ingest_document(
        &self,
        uri: &str,
        media_type: &str,
        text: &str,
        guardian_did: Option<String>,
    ) -> Result<serde_json::Value, String> {
        self.ingest_bytes(uri, media_type, text.as_bytes(), text, &ManualFacets::default(), guardian_did)
    }

    /// Ingest a text document **with person-authored facets** — an optional date (→ timeline), place
    /// (→ map), project and purpose the person chooses to attach. The document's derived topics still come
    /// from its content; these facets are added on top (the person authoring meaning, not being defined).
    pub fn ingest_document_annotated(
        &self,
        uri: &str,
        media_type: &str,
        text: &str,
        manual: &ManualFacets,
        guardian_did: Option<String>,
    ) -> Result<serde_json::Value, String> {
        self.ingest_bytes(uri, media_type, text.as_bytes(), text, manual, guardian_did)
    }

    /// **Ingest any asset bytes** (a document, a **photo**, an audio clip) into the library. The processor
    /// registered for `media_type` derives searchability — a text doc → topics; a **JPEG/PNG → its EXIF
    /// capture time (timeline) + GPS place (map)**; a WAV → duration + dominant frequency — and it all folds
    /// into the container so the original is findable by meaning. `excerpt_source` is a short human string for
    /// the results list (the text for a doc; a caption/filename for binary). Guardianship + ledger hook as
    /// [`Self::ingest_document`].
    pub fn ingest_bytes(
        &self,
        uri: &str,
        media_type: &str,
        bytes: &[u8],
        excerpt_source: &str,
        manual: &ManualFacets,
        guardian_did: Option<String>,
    ) -> Result<serde_json::Value, String> {
        use qualia_core_db::hypermedia::processors::processor_for;
        use qualia_core_db::hypermedia::{
            content_digest, descriptors_to_nquins, ingest_with, Descriptors, FlagSeverity, Place,
        };

        let proc = processor_for(media_type)
            .ok_or_else(|| format!("no ingest processor for media type '{media_type}'"))?;
        let digest = content_digest(bytes);
        let out = proc.process(uri, bytes, media_type);
        let mut r = ingest_with(proc.as_ref(), uri, media_type, digest, bytes);
        let now = Self::now_unix();
        let primary_subject = r.container.primary.subject();

        // Merge the person-authored facets as additional descriptor edges on the primary asset. A processor's
        // own derivation (a photo's EXIF) takes precedence for its fields; manual facets fill / extend.
        let manual_place = match (manual.lat, manual.lon) {
            (Some(lat), Some(lon)) => Some(Place {
                label: manual.place_label.clone().unwrap_or_else(|| format!("{lat:.5},{lon:.5}")),
                lat,
                lon,
            }),
            _ => None,
        };
        if !manual.is_empty() {
            let extra = Descriptors {
                occurred_at: manual.occurred_at.filter(|_| out.descriptors.occurred_at.is_none()),
                place: if out.descriptors.place.is_none() { manual_place.clone() } else { None },
                projects: manual.projects.clone(),
                purposes: manual.purposes.clone(),
                ..Default::default()
            };
            let (eq, _lex) = descriptors_to_nquins(primary_subject, &extra);
            r.quins.extend(eq);
        }

        let flags: Vec<super::hypermedia_store::LibraryFlag> = out
            .flags
            .iter()
            .map(|f| super::hypermedia_store::LibraryFlag {
                kind: f.kind.clone(),
                severity_level: f.severity.level(),
                detail: f.detail.clone(),
            })
            .collect();

        // Effective facets for the entry's display fields: processor-derived first, else the person's.
        let eff_occurred_at = out.descriptors.occurred_at.or(manual.occurred_at);
        let eff_place = out.descriptors.place.clone().or(manual_place);
        let (lat, lon) = eff_place
            .as_ref()
            .map(|p| (Some(p.lat), Some(p.lon)))
            .unwrap_or((None, None));
        let mut projects = out.descriptors.projects.clone();
        projects.extend(manual.projects.iter().cloned());

        let purposes = manual.purposes.clone();
        let sensitivity = super::hypermedia_store::normalize_sensitivity(
            manual
                .sensitivity
                .as_deref()
                .unwrap_or("public"),
        );
        let commons = super::hypermedia_store::CommonsVisibility::parse(
            manual
                .commons_visibility
                .as_deref()
                .unwrap_or("none"),
        );
        // Rust-native CML context graph for text-like assets (TEXT→CONCEPT→LOGIC, cml:Proposed).
        let mut cml_topics = Vec::new();
        let mut cml_purposes = purposes.clone();
        let mut cml_signals = Vec::new();
        let mut cml_concept_count = 0u32;
        let mut cml_n3 = String::new();
        let mut cml_quins = Vec::new();
        if media_type.starts_with("text/") || media_type.contains("json") || media_type.contains("markdown")
        {
            let text = String::from_utf8_lossy(bytes);
            let units = super::cml_context::units_from_headings(&text);
            let g = super::cml_context::build_document_context(uri, excerpt_source, &units);
            cml_topics = g.topics.clone();
            for p in &g.purposes {
                if !cml_purposes.iter().any(|x| x == p) {
                    cml_purposes.push(p.clone());
                }
            }
            cml_signals = g.signal_tags.clone();
            cml_concept_count = g.concepts.len() as u32;
            cml_n3 = if g.n3.len() > 48_000 {
                format!("{}…\n# [cml_n3 truncated]", &g.n3[..48_000])
            } else {
                g.n3
            };
            cml_quins = g.quins;
        }

        let mut topics = out.descriptors.topics.clone();
        for t in cml_topics {
            if !topics.iter().any(|x| x == &t) {
                topics.push(t);
            }
        }

        let mut all_quins = r.quins;
        all_quins.extend(cml_quins);

        let mut entry = super::hypermedia_store::LibraryEntry {
            asset_uri: uri.to_string(),
            primary_subject,
            media_type: media_type.to_string(),
            quins: all_quins,
            topics,
            projects,
            purposes: cml_purposes,
            place: eff_place.as_ref().map(|p| p.label.clone()),
            occurred_at: eff_occurred_at,
            lat,
            lon,
            flags: flags.clone(),
            ingested_unix: now,
            excerpt: excerpt_source.chars().take(160).collect(),
            sensitivity: sensitivity.clone(),
            section: manual
                .section
                .clone()
                .unwrap_or_else(|| "personal".into()),
            commons_visibility: commons,
            cml_signals,
            cml_concept_count,
            cml_n3,
            cof_html: String::new(),
            cof_segment_count: 0,
            cof_segment_index: 0,
            cof_profile: String::new(),
        };

        // COF HTML+RDFa package (token-bounded segments) for text assets.
        let mut cof_segment_count = 0u32;
        let mut cof_profile = String::new();
        let mut cof_body_segments: Vec<super::cml_context::CofSegment> = Vec::new();
        if media_type.starts_with("text/") {
            let text = String::from_utf8_lossy(bytes);
            let units = super::cml_context::units_from_headings(&text);
            let pkg = super::cml_context::build_cof_package(
                uri,
                excerpt_source,
                &units,
                super::cml_context::DEFAULT_SEGMENT_MAX_CHARS,
                super::cml_context::CofStyle::AgentLean,
            );
            cof_segment_count = pkg.segments.len() as u32;
            cof_profile = pkg.profile.clone();
            entry.cof_segment_count = cof_segment_count;
            entry.cof_profile = cof_profile.clone();
            if let Some(index_seg) = pkg.segments.iter().find(|s| s.is_index) {
                entry.cof_html = index_seg.html.clone();
                entry.cof_segment_index = 0;
            } else if let Some(first) = pkg.segments.first() {
                entry.cof_html = first.html.clone();
                entry.cof_segment_index = first.index;
            }
            cof_body_segments = pkg
                .segments
                .into_iter()
                .filter(|s| !s.is_index)
                .collect();
        }

        entry.recompute_section();
        // High sensitivity can never be commons.
        if entry.is_secret() {
            entry.commons_visibility = super::hypermedia_store::CommonsVisibility::None;
        }
        let section = entry.section.clone();
        let commons_visibility = entry.commons_visibility;
        let entry_topics = entry.topics.clone();
        let entry_purposes = entry.purposes.clone();
        let store = self.library()?;
        store.add(entry).map_err(|e| e.to_string())?;

        // Sibling COF body segments — load only the budget needed for a turn.
        for seg in &cof_body_segments {
            let seg_uri = format!("{uri}#cof-seg-{}", seg.index);
            let mut se = super::hypermedia_store::LibraryEntry {
                asset_uri: seg_uri.clone(),
                primary_subject: qualia_core_db::hypermedia::fnv60(seg_uri.as_bytes()),
                media_type: super::cml_context::MEDIA_TYPE_COF.into(),
                quins: Vec::new(),
                topics: entry_topics.clone(),
                projects: Vec::new(),
                purposes: entry_purposes.clone(),
                place: None,
                occurred_at: None,
                lat: None,
                lon: None,
                flags: Vec::new(),
                ingested_unix: now,
                excerpt: format!(
                    "COF segment {}/{} · ~{} tokens · units: {}",
                    seg.index + 1,
                    seg.total,
                    seg.approx_tokens,
                    seg.unit_frags.join(", ")
                ),
                sensitivity: sensitivity.clone(),
                section: section.clone(),
                commons_visibility,
                cml_signals: Vec::new(),
                cml_concept_count: seg.unit_frags.len() as u32,
                cml_n3: String::new(),
                cof_html: seg.html.clone(),
                cof_segment_count,
                cof_segment_index: seg.index,
                cof_profile: cof_profile.clone(),
            };
            se.recompute_section();
            store.add(se).map_err(|e| e.to_string())?;
        }

        // Guardianship hook: a flagged ingest under a guardianship relation notifies + records.
        let mut notified = Vec::new();
        if let Some(g) = &guardian_did {
            if !out.flags.is_empty() {
                let ns = super::ingest_guardian::guardian_notifications(
                    &out.flags,
                    uri,
                    g,
                    &self.owner_did,
                    FlagSeverity::Notice,
                    now,
                );
                self.record_guardian_notifications(&ns)?;
                notified = ns;
            }
        }
        Ok(serde_json::json!({
            "asset_uri": uri,
            "topics": entry_topics,
            "occurred_at": eff_occurred_at,
            "place": eff_place.as_ref().map(|p| &p.label),
            "lat": lat,
            "lon": lon,
            "flags": flags,
            "guardian_notifications": notified,
            "section": section,
            "sensitivity": sensitivity,
            "commons_visibility": commons_visibility,
            "purposes": entry_purposes,
            "cml_concept_count": cml_concept_count,
            "cof_segment_count": cof_segment_count,
            "cof_profile": cof_profile,
        }))
    }

    /// **Ingest a photo/audio file from hex-encoded bytes** — the boundary form for the desktop, which reads a
    /// picked file and passes its bytes as hex (a JPEG is not valid utf-8, so it cannot come through the text
    /// path). A photo's EXIF capture-time + GPS auto-populate the timeline + map. `caption` is the short
    /// display string. Same derive + persist + guardian hook as [`Self::ingest_bytes`].
    pub fn ingest_file_hex(
        &self,
        uri: &str,
        media_type: &str,
        bytes_hex: &str,
        caption: &str,
        guardian_did: Option<String>,
    ) -> Result<serde_json::Value, String> {
        let bytes = decode_hex(bytes_hex).map_err(|e| format!("bad hex: {e}"))?;
        self.ingest_bytes(uri, media_type, &bytes, caption, &ManualFacets::default(), guardian_did)
    }

    /// Search the library by facet (`topic` | `depicts` | `place` | `project` | `purpose`). Returns per-entry
    /// summaries (not the raw quins).
    pub fn search_library(&self, facet: &str, value: &str) -> Result<Vec<serde_json::Value>, String> {
        let entries = self.library()?.search(facet, value).map_err(|e| e.to_string())?;
        Ok(entries.iter().map(library_summary).collect())
    }

    /// The **timeline** query — entries whose event instant falls within `[start, end]` (unix seconds).
    pub fn search_library_time(&self, start: i64, end: i64) -> Result<Vec<serde_json::Value>, String> {
        let entries = self.library()?.search_time_range(start, end).map_err(|e| e.to_string())?;
        Ok(entries.iter().map(library_summary).collect())
    }

    /// Everything in the library (newest first), as summaries.
    /// Optional `section` filters to secret | wellfair | personal | work | commons.
    pub fn list_library(&self) -> Result<Vec<serde_json::Value>, String> {
        self.list_library_section(None)
    }

    pub fn list_library_section(
        &self,
        section: Option<&str>,
    ) -> Result<Vec<serde_json::Value>, String> {
        let store = self.library()?;
        let entries = match section {
            Some(s) if !s.is_empty() && s != "all" => store
                .by_section(super::hypermedia_store::LibrarySection::parse(s))
                .map_err(|e| e.to_string())?,
            _ => store.all().map_err(|e| e.to_string())?,
        };
        Ok(entries.iter().map(library_summary).collect())
    }

    /// Free-text search over uri / excerpt / topics / projects / place.
    pub fn search_library_text(&self, query: &str) -> Result<Vec<serde_json::Value>, String> {
        let entries = self.library()?.search_text(query).map_err(|e| e.to_string())?;
        Ok(entries.iter().map(library_summary).collect())
    }

    /// Multi-facet library query with sort. `filter_json` is a [`FacetFilter`] object;
    /// `sort` is newest|oldest|title_asc|title_desc|media_type|category.
    pub fn query_library_faceted(
        &self,
        filter_json: &str,
        sort: &str,
    ) -> Result<serde_json::Value, String> {
        let filter: super::hypermedia_store::FacetFilter = if filter_json.trim().is_empty() {
            Default::default()
        } else {
            serde_json::from_str(filter_json).map_err(|e| format!("facet filter json: {e}"))?
        };
        let sort = super::hypermedia_store::LibrarySort::parse(sort);
        let store = self.library()?;
        let entries = store
            .query_faceted(&filter, sort)
            .map_err(|e| e.to_string())?;
        let counts = store.facet_counts(&filter).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "entries": entries.iter().map(library_summary).collect::<Vec<_>>(),
            "total": entries.len(),
            "sort": sort.as_str(),
            "filter": filter,
            "facets": counts,
        }))
    }

    /// Facet value counts for chip UI (optionally narrowed by the same filter JSON).
    pub fn library_facet_counts(&self, filter_json: &str) -> Result<serde_json::Value, String> {
        let filter: super::hypermedia_store::FacetFilter = if filter_json.trim().is_empty() {
            Default::default()
        } else {
            serde_json::from_str(filter_json).map_err(|e| format!("facet filter json: {e}"))?
        };
        let counts = self
            .library()?
            .facet_counts(&filter)
            .map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(counts).map_err(|e| e.to_string())?)
    }

    /// Seed the early studio academic QApp inventory into Library → Software.
    /// Idempotent; returns add/update counts.
    pub fn seed_studio_qapps_library(&self) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let report = super::qapp_catalog::seed_studio_qapps_into_library(&store)
            .map_err(|e| e.to_string())?;
        Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
    }

    /// Seed perception models + ontology catalogue rows into Library → Software.
    /// Also ensures seed weight files under `{storage}/models/`.
    pub fn seed_perception_library(&self) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let root = self.storage_root();
        let report =
            super::perception_catalog::seed_perception_into_library(&store, root)?;
        Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
    }

    /// Native legislation ingest (structure parse, no Ollama): PDF bytes → Work shelf
    /// entries for the instrument and every Part/Section/Subsection with full body text.
    pub fn ingest_legislation_pdf_hex(
        &self,
        hex_bytes: &str,
        register_id: Option<&str>,
        jurisdiction: Option<&str>,
        title_hint: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let bytes = decode_hex(hex_bytes)?;
        let store = self.library()?;
        let report = super::legislation_ingest::ingest_legislation_pdf_bytes(
            &store,
            &bytes,
            register_id,
            jurisdiction.unwrap_or("AU"),
            title_hint,
        )?;
        Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
    }

    /// Native legislation ingest from plain text (already extracted PDF text or HTML).
    pub fn ingest_legislation_text(
        &self,
        text: &str,
        register_id: Option<&str>,
        jurisdiction: Option<&str>,
        title_hint: Option<&str>,
    ) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let report = super::legislation_ingest::ingest_legislation_text(
            &store,
            text,
            register_id,
            jurisdiction.unwrap_or("AU"),
            title_hint,
        )?;
        Ok(serde_json::to_value(report).map_err(|e| e.to_string())?)
    }

    /// Build a Rust-native CML context graph for arbitrary text (no Python).
    /// Returns concepts, signal tags, N3, and deontic/privacy counts — does not persist.
    pub fn build_cml_context_graph(
        &self,
        uri: &str,
        title: &str,
        text: &str,
    ) -> Result<serde_json::Value, String> {
        let units = super::cml_context::units_from_headings(text);
        let g = super::cml_context::build_document_context(uri, title, &units);
        Ok(serde_json::json!({
            "document_uri": g.document_uri,
            "title": g.title,
            "concepts": g.concepts,
            "signal_tags": g.signal_tags,
            "topics": g.topics,
            "purposes": g.purposes,
            "deontic_norms": g.deontic_norms,
            "privacy_hits": g.privacy_hits,
            "rights_hits": g.rights_hits,
            "quin_count": g.quins.len(),
            "n3": g.n3,
            "curation": "cml:Proposed",
            "engine": "qualia-client-core::wellfair::cml_context",
        }))
    }

    /// Build a **COF HTML+RDFa** package (token-bounded segments) without persisting.
    /// `max_chars` defaults to 24000 when zero/None.
    pub fn build_cof_html_package(
        &self,
        uri: &str,
        title: &str,
        text: &str,
        max_chars: Option<usize>,
        dual_surface: bool,
    ) -> Result<serde_json::Value, String> {
        let units = super::cml_context::units_from_headings(text);
        let style = if dual_surface {
            super::cml_context::CofStyle::DualSurface
        } else {
            super::cml_context::CofStyle::AgentLean
        };
        let max = max_chars
            .filter(|n| *n >= 2000)
            .unwrap_or(super::cml_context::DEFAULT_SEGMENT_MAX_CHARS);
        let pkg = super::cml_context::build_cof_package(uri, title, &units, max, style);
        Ok(serde_json::json!({
            "document_uri": pkg.document_uri,
            "title": pkg.title,
            "profile": pkg.profile,
            "segment_max_chars": pkg.segment_max_chars,
            "total_chars": pkg.total_chars,
            "total_approx_tokens": pkg.total_approx_tokens,
            "segments": pkg.segments.iter().map(|s| serde_json::json!({
                "index": s.index,
                "total": s.total,
                "id": s.id,
                "title": s.title,
                "char_count": s.char_count,
                "approx_tokens": s.approx_tokens,
                "unit_frags": s.unit_frags,
                "is_index": s.is_index,
                "html": s.html,
            })).collect::<Vec<_>>(),
            "how": [
                "Load segment 0 (index) for a token-cheap map of the instrument.",
                "Load only the body segment(s) whose unit_frags match the query.",
                "RDFa attributes carry CML edges; do not strip typeof/property/resource.",
            ],
        }))
    }

    /// Re-run CML context enrichment on an existing library entry's excerpt/text fields.
    pub fn enrich_library_entry_cml(&self, asset_uri: &str) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let mut entries = store.load().map_err(|e| e.to_string())?;
        let e = entries
            .iter_mut()
            .find(|x| x.asset_uri == asset_uri)
            .ok_or_else(|| format!("unknown asset '{asset_uri}'"))?;
        let text = if e.excerpt.len() > 40 {
            e.excerpt.clone()
        } else {
            return Err("entry has no usable text in excerpt to enrich".into());
        };
        let units = super::cml_context::units_from_headings(&text);
        let g = super::cml_context::build_document_context(&e.asset_uri, &e.asset_uri, &units);
        for t in &g.topics {
            if !e.topics.iter().any(|x| x == t) {
                e.topics.push(t.clone());
            }
        }
        for p in &g.purposes {
            if !e.purposes.iter().any(|x| x == p) {
                e.purposes.push(p.clone());
            }
        }
        e.cml_signals = g.signal_tags.clone();
        e.cml_concept_count = g.concepts.len() as u32;
        e.cml_n3 = if g.n3.len() > 48_000 {
            format!("{}…", &g.n3[..48_000])
        } else {
            g.n3.clone()
        };
        e.quins.extend(g.quins);
        e.recompute_section();
        let out = library_summary(e);
        store.replace_all(&entries).map_err(|e| e.to_string())?;
        Ok(out)
    }

    /// List catalogue categories (for Software shelf UI without seeding first).
    pub fn list_qapp_catalog_categories(&self) -> Result<serde_json::Value, String> {
        let cats: Vec<serde_json::Value> = super::qapp_catalog::catalogue_categories()
            .into_iter()
            .map(|slug| {
                serde_json::json!({
                    "slug": slug,
                    "label": super::qapp_catalog::category_label(slug),
                    "count": super::qapp_catalog::STUDIO_QAPP_CATALOG
                        .iter()
                        .filter(|e| e.category == slug)
                        .count(),
                })
            })
            .collect();
        Ok(serde_json::json!({
            "total": super::qapp_catalog::STUDIO_QAPP_CATALOG.len(),
            "categories": cats,
        }))
    }

    /// Aggregate library stats for the UI header (includes section counts).
    pub fn library_stats(&self) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let s = store.stats().map_err(|e| e.to_string())?;
        let sections = store.section_counts().map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "total": s.total,
            "with_date": s.with_date,
            "with_place": s.with_place,
            "flags": s.flags,
            "quins": s.quins,
            "topics": s.topics,
            "projects": s.projects,
            "sections": sections,
        }))
    }

    /// Set commons / peer visibility (refuses Secret).
    pub fn set_library_commons_visibility(
        &self,
        asset_uri: &str,
        visibility: &str,
    ) -> Result<serde_json::Value, String> {
        let vis = super::hypermedia_store::CommonsVisibility::parse(visibility);
        let e = self
            .library()?
            .set_commons_visibility(asset_uri, vis)
            .map_err(|e| e.to_string())?;
        Ok(library_summary(&e))
    }

    /// Build a **permissive commons share card** for social networking (no secret payloads).
    /// Returns metadata peers can list; raw content stays on-device until a fuller mesh transfer.
    pub fn library_commons_share_card(&self, asset_uri: &str) -> Result<serde_json::Value, String> {
        let entries = self.library()?.all().map_err(|e| e.to_string())?;
        let e = entries
            .iter()
            .find(|x| x.asset_uri == asset_uri)
            .ok_or_else(|| format!("unknown asset '{asset_uri}'"))?;
        if e.is_secret() {
            return Err("secret items cannot be offered to the commons".into());
        }
        if e.commons_visibility == super::hypermedia_store::CommonsVisibility::None {
            return Err(
                "set commons visibility to peers or commons before sharing".into(),
            );
        }
        Ok(serde_json::json!({
            "qualia_library_commons": "1",
            "asset_uri": e.asset_uri,
            "media_type": e.media_type,
            "topics": e.topics,
            "projects": e.projects,
            "purposes": e.purposes,
            "excerpt": e.excerpt,
            "section": e.section,
            "commons_visibility": e.commons_visibility,
            "how": [
                "Host: Keep → Library → Commons → Share to peers.",
                "Peer: accept via Talk social connection; request content over mesh when available.",
            ],
            "note": "Card is metadata only — not the secret body. High-sensitivity items never appear here.",
        }))
    }

    /// Remove one library entry by asset URI.
    pub fn remove_library_entry(&self, asset_uri: &str) -> Result<serde_json::Value, String> {
        let ok = self.library()?.remove(asset_uri).map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "removed": ok, "asset_uri": asset_uri }))
    }

    /// Export the full hypermedia graph mass (quin count + optional dump for inject).
    /// Returns `{ quin_count, entries }` — the live graph inject seam for daemon/MCP.
    pub fn export_library_graph(&self) -> Result<serde_json::Value, String> {
        let store = self.library()?;
        let entries = store.all().map_err(|e| e.to_string())?;
        let quins = store.all_quins().map_err(|e| e.to_string())?;
        Ok(serde_json::json!({
            "quin_count": quins.len(),
            "entry_count": entries.len(),
            "message": "Hypermedia edge-graph ready for daemon /query inject. Quins are the searchable semantic form.",
            "sample_subjects": entries.iter().take(8).map(|e| e.primary_subject).collect::<Vec<_>>(),
        }))
    }

    // --- Real envelope encryption over the consent credential (ADR 0011 D1/D2) ---
    //
    // Makes "revoke destroys the wrapped key ⇒ no key, no payload" a *fact*: the payload is AEAD-encrypted
    // under a random DEK; the DEK is sealed (X25519 sealed box) to the recipient's public key — that sealed
    // DEK is the credential's real `wrapped_key`; revoke destroys it. The owner's envelope keypair is
    // **derived** from the owner signing-key seed (nothing secret stored at rest). Native-only (the sealed-box
    // primitives are `not(wasm32)`; the desktop owns keys).

    /// The owner's envelope **public** key (hex) — publishable so others can seal payloads *to* the owner.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn owner_envelope_public_hex(&self) -> String {
        use crate::envelope_encryption::{EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN).public_hex()
    }

    /// **Seal a plaintext payload and grant a consent credential over it** — real envelope encryption. If
    /// `agent_public_hex` is empty, the payload is sealed to the OWNER's derived envelope key (self-custody,
    /// so the owner can [`open_owner_payload`]); supply an agent's X25519 public key to grant *that* agent
    /// access (they open it on their own device with their secret — the owner cannot).
    ///
    /// [`open_owner_payload`]: WebizenHostApi::open_owner_payload
    #[cfg(not(target_arch = "wasm32"))]
    pub fn seal_and_grant_consent_credential(
        &self,
        agent_did: &str,
        agent_public_hex: &str,
        scope: &str,
        purpose: &str,
        plaintext: &str,
        expiry_unix: Option<u64>,
    ) -> Result<crate::consent_credential::ConsentCredential, String> {
        use crate::envelope_encryption::{EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        let owner = EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN);
        let recipient_public: [u8; 32] = if agent_public_hex.trim().is_empty() {
            owner.public
        } else {
            let bytes = hex::decode(agent_public_hex.trim())
                .map_err(|e| format!("agent public key not hex: {e}"))?;
            bytes
                .as_slice()
                .try_into()
                .map_err(|_| "agent public key must be 32 bytes".to_string())?
        };
        let now = Self::now_unix();
        let id = {
            let d = Sha256::digest(format!("{agent_did}:{scope}:{now}").as_bytes());
            format!("cc-{}", hex::encode(&d[..6]))
        };
        self.accountability_store()?
            .seal_and_grant_credential(
                id,
                &self.owner_did,
                agent_did,
                scope,
                purpose,
                plaintext.as_bytes(),
                &recipient_public,
                vec![self.owner_did.clone()],
                expiry_unix,
                &self.signing_key,
                now,
            )
            .map_err(|e| e.to_string())
    }

    /// **Open an owner-sealed payload** through a credential — proves the crypto-revoke property end-to-end:
    /// works while the credential is live, fails once revoked (the wrapped key is gone), though the commons
    /// ciphertext survives. Only opens payloads sealed to the owner (an agent-sealed payload opens on the
    /// agent's device).
    #[cfg(not(target_arch = "wasm32"))]
    pub fn open_owner_payload(&self, credential_id: &str) -> Result<String, String> {
        use crate::envelope_encryption::{EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        let owner = EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN);
        let bytes = self
            .accountability_store()?
            .open_payload_via_credential(credential_id, &owner.secret, Self::now_unix())
            .map_err(|e| e.to_string())?;
        String::from_utf8(bytes).map_err(|e| format!("payload not valid utf-8: {e}"))
    }

    // --- Safeguard switches (ADR 0011 D6/D7): dead-man + incapacity, owner-signed into the ledger ---

    /// Arm a **dead-man switch** over a payload (post-death disposition; gamified + reversible).
    pub fn arm_dead_mans_switch(
        &self,
        switch: crate::dead_mans_switch::DeadMansSwitch,
    ) -> Result<(), String> {
        self.accountability_store()?
            .arm_dead_mans_switch(switch, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// **I'm alive** — touch the heartbeat + un-fire a not-yet-enacted switch (reversibility). The routine
    /// owner-side action that keeps a dead-man switch from firing.
    pub fn dead_mans_alive(&self, commitment_hex: &str) -> Result<bool, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .dead_mans_alive(&c, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// Record a **party attestation** toward a dead-man switch's gamified trigger.
    pub fn attest_dead_mans(
        &self,
        commitment_hex: &str,
        attestation: crate::dead_mans_switch::PartyAttestation,
    ) -> Result<bool, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .attest_dead_mans(&c, attestation, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// **Enact** a dead-man switch if the gamified rule holds — returns the [`Disposition`] to carry out.
    ///
    /// [`Disposition`]: crate::dead_mans_switch::Disposition
    pub fn enact_dead_mans(
        &self,
        commitment_hex: &str,
    ) -> Result<Option<crate::dead_mans_switch::Disposition>, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?
            .enact_dead_mans(&c, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// List armed dead-man switches (with accumulated attestations).
    pub fn list_dead_mans_switches(
        &self,
    ) -> Result<Vec<crate::accountability_store::DeadMansSwitchRecord>, String> {
        self.accountability_store()?.list_dead_mans_switches().map_err(|e| e.to_string())
    }

    /// **Enact a dead-man switch AND release the keys** (ADR 0011 D6, key-release-on-enact). Recovers the
    /// payload DEK by unwrapping the owner's own credential, then — for a `ReleaseTo` disposition — re-seals
    /// the DEK to each supplied party X25519 pubkey and grants them a credential, so the disposition actually
    /// hands over access. `party_keys` = `(did, pubkey_hex)` pairs. (The owner key is derivable here; the true
    /// post-death friend-side release without the owner needs Shamir pre-positioning — separate.)
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enact_dead_mans_release(
        &self,
        commitment_hex: &str,
        party_keys_hex: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        use crate::envelope_encryption::{unwrap_dek, EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let now = Self::now_unix();
        let store = self.accountability_store()?;
        let owner = EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN);
        // Recover the DEK by unwrapping the owner's own credential for this payload.
        let wrapped = store
            .wrapped_key_for(&c, &self.owner_did, now)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| {
                "no owner credential holds the DEK for this payload (seal it to yourself first)".to_string()
            })?;
        let dek = unwrap_dek(&owner.secret, &wrapped)?;
        let mut party_keys: Vec<(String, [u8; 32])> = Vec::new();
        for (did, pk_hex) in party_keys_hex {
            let bytes = hex::decode(pk_hex.trim()).map_err(|e| format!("party key not hex: {e}"))?;
            let pk: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| "party key must be 32 bytes".to_string())?;
            party_keys.push((did, pk));
        }
        let disposition = store
            .enact_dead_mans_release(&c, &dek, &party_keys, &self.owner_did, &self.signing_key, now)
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "enacted": disposition.is_some(), "disposition": disposition }))
    }

    /// **Split a payload's DEK into Shamir social-recovery shares** (`threshold`-of-`parties.len()`), so a
    /// quorum of friends can later reconstruct the key **without the owner**. Recovers the DEK from the owner's
    /// own credential, splits it, and returns the shares paired with the parties they should be handed to
    /// (the caller distributes them off-device — they are NOT stored here). Owner-side, done while alive.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn split_dek_recovery(
        &self,
        commitment_hex: &str,
        threshold: usize,
        parties: Vec<String>,
    ) -> Result<serde_json::Value, String> {
        use crate::envelope_encryption::{unwrap_dek, EnvelopeKeypair, OWNER_ENVELOPE_DOMAIN};
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let now = Self::now_unix();
        let store = self.accountability_store()?;
        let owner = EnvelopeKeypair::derive(&self.signing_key.to_bytes(), OWNER_ENVELOPE_DOMAIN);
        let wrapped = store
            .wrapped_key_for(&c, &self.owner_did, now)
            .map_err(|e| e.to_string())?
            .ok_or_else(|| "no owner credential holds the DEK for this payload".to_string())?;
        let dek = unwrap_dek(&owner.secret, &wrapped)?;
        let shares = crate::shamir_recovery::split(&dek, threshold, parties.len())?;
        let tagged: Vec<serde_json::Value> = parties
            .iter()
            .zip(shares.iter())
            .map(|(party, share)| serde_json::json!({ "party": party, "share": share }))
            .collect();
        Ok(serde_json::json!({ "threshold": threshold, "shares": tagged }))
    }

    /// **Social-recovery enactment (no owner key):** given a quorum of friends' Shamir shares, reconstruct the
    /// DEK, enact the dead-man switch, and release to the disposition parties. `party_keys` = `(did, pubkey_hex)`.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn reconstruct_and_release(
        &self,
        commitment_hex: &str,
        shares: Vec<crate::shamir_recovery::Share>,
        party_keys_hex: Vec<(String, String)>,
    ) -> Result<serde_json::Value, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let now = Self::now_unix();
        let mut party_keys: Vec<(String, [u8; 32])> = Vec::new();
        for (did, pk_hex) in party_keys_hex {
            let bytes = hex::decode(pk_hex.trim()).map_err(|e| format!("party key not hex: {e}"))?;
            let pk: [u8; 32] = bytes
                .as_slice()
                .try_into()
                .map_err(|_| "party key must be 32 bytes".to_string())?;
            party_keys.push((did, pk));
        }
        let disposition = self
            .accountability_store()?
            .reconstruct_and_release(&c, &shares, &party_keys, &self.owner_did, &self.signing_key, now)
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "enacted": disposition.is_some(), "disposition": disposition }))
    }

    /// Publish a **peer's envelope (X25519) public key** into their peer record, so releases to that party
    /// can resolve the key automatically (remote-key distribution). The owner's own publishable key is
    /// [`owner_envelope_public_hex`](Self::owner_envelope_public_hex).
    pub fn set_peer_envelope_key(&self, did: &str, pubkey_hex: &str) -> Result<(), String> {
        crate::social_peers::set_peer_envelope_key(did, pubkey_hex)
    }

    /// **Enact + release resolving the disposition parties' keys from the peer store** (remote-key
    /// distribution). Reads the switch's `ReleaseTo` parties, looks up each one's published envelope key from
    /// `social_peers`, and releases to those with a known key — reporting any parties whose key is still
    /// missing (so the owner knows to obtain it). No keys pasted by hand.
    #[cfg(not(target_arch = "wasm32"))]
    pub fn enact_dead_mans_release_via_peers(
        &self,
        commitment_hex: &str,
    ) -> Result<serde_json::Value, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let switches = self
            .accountability_store()?
            .list_dead_mans_switches()
            .map_err(|e| e.to_string())?;
        let rec = switches
            .iter()
            .find(|r| r.switch.payload_commitment == c)
            .ok_or_else(|| "no dead-man switch for that commitment".to_string())?;
        let parties = match &rec.switch.disposition {
            crate::dead_mans_switch::Disposition::ReleaseTo { parties } => parties.clone(),
            _ => Vec::new(),
        };
        let peers = crate::social_peers::list_peers();
        let resolved = crate::social_peers::resolve_envelope_keys(&peers, &parties);
        let have: std::collections::BTreeSet<&str> = resolved.iter().map(|(d, _)| d.as_str()).collect();
        let missing: Vec<String> =
            parties.iter().filter(|d| !have.contains(d.as_str())).cloned().collect();
        let result = self.enact_dead_mans_release(commitment_hex, resolved)?;
        Ok(serde_json::json!({ "result": result, "missing_keys_for": missing }))
    }

    /// Arm an **incapacity switch** (advocate activation on validated, reversible incapacity).
    pub fn arm_incapacity_switch(
        &self,
        switch: crate::incapacity_switch::IncapacitySwitch,
    ) -> Result<(), String> {
        self.accountability_store()?
            .arm_incapacity_switch(switch, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// **Activate** advocacy if the corroborated trigger holds (quorum + optional official instrument).
    pub fn activate_incapacity(
        &self,
        principal_did: &str,
        attesting_parties: Vec<String>,
        official_instrument: Option<String>,
    ) -> Result<bool, String> {
        self.accountability_store()?
            .activate_incapacity(
                principal_did,
                &attesting_parties,
                official_instrument.as_deref(),
                &self.signing_key,
                Self::now_unix(),
            )
            .map_err(|e| e.to_string())
    }

    /// **Regain capacity** — the advocate stands down (reversibility).
    pub fn regain_capacity(&self, principal_did: &str) -> Result<bool, String> {
        self.accountability_store()?
            .regain_capacity(principal_did, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// List armed incapacity switches.
    pub fn list_incapacity_switches(
        &self,
    ) -> Result<Vec<crate::incapacity_switch::IncapacitySwitch>, String> {
        self.accountability_store()?.list_incapacity_switches().map_err(|e| e.to_string())
    }

    // --- Disclosure traceability (ADR 0011 D5) + duty of inquiry (D8) ---

    /// Record a **transparency cc** — the protective "I informed authority X for purpose Y" note.
    pub fn record_transparency_cc(
        &self,
        credential_id: &str,
        informed_authority_did: &str,
        purpose: &str,
    ) -> Result<(), String> {
        let cc = crate::disclosure_trace::TransparencyCc {
            credential_id: credential_id.to_string(),
            informed_authority_did: informed_authority_did.to_string(),
            purpose: purpose.to_string(),
            informed_unix: Self::now_unix(),
        };
        self.accountability_store()?
            .record_transparency_cc(cc, &self.signing_key, Self::now_unix())
            .map_err(|e| e.to_string())
    }

    /// Record a **disclosure event** (an access, or an onward-share if `onward_to` is set). A per-recipient
    /// fingerprint + id are generated. Returns the recorded event (its `fingerprint` is the tracing anchor).
    pub fn record_disclosure(
        &self,
        commitment_hex: &str,
        credential_id: &str,
        recipient_did: &str,
        acting_delegate_did: Option<String>,
        onward_to: Option<String>,
    ) -> Result<crate::disclosure_trace::DisclosureEvent, String> {
        let commitment = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        let now = Self::now_unix();
        let actor = acting_delegate_did.as_deref().unwrap_or(recipient_did);
        // Deterministic per-recipient/per-disclosure fingerprint (the traitor-tracing anchor).
        let digest = Sha256::digest(
            format!("{}:{recipient_did}:{actor}:{now}", hex::encode(commitment)).as_bytes(),
        );
        let mut fingerprint = [0u8; 16];
        fingerprint.copy_from_slice(&digest[..16]);
        let id = format!("d-{}", hex::encode(&digest[16..22]));
        let kind = match onward_to {
            Some(to_did) => crate::disclosure_trace::DisclosureKind::OnwardShare { to_did },
            None => crate::disclosure_trace::DisclosureKind::DirectAccess,
        };
        let event = crate::disclosure_trace::DisclosureEvent {
            id,
            payload_commitment: commitment,
            credential_id: credential_id.to_string(),
            recipient_did: recipient_did.to_string(),
            acting_delegate_did,
            time_unix: now,
            fingerprint,
            kind,
        };
        self.accountability_store()?
            .record_disclosure_event(event.clone(), &self.signing_key, now)
            .map_err(|e| e.to_string())?;
        Ok(event)
    }

    /// The disclosure chain for a payload (who saw it, via which route).
    pub fn disclosure_chain(
        &self,
        commitment_hex: &str,
    ) -> Result<Vec<crate::disclosure_trace::DisclosureEvent>, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?.disclosure_chain(&c).map_err(|e| e.to_string())
    }

    /// The distinct actors who had access to a payload — the set a leak must be within.
    pub fn actors_with_access(&self, commitment_hex: &str) -> Result<Vec<String>, String> {
        let c = crate::accountability_store::parse_commitment_hex(commitment_hex)?;
        self.accountability_store()?.actors_with_access(&c).map_err(|e| e.to_string())
    }

    /// **Trace a leak** by its fingerprint (hex, 16 bytes) → the disclosure + accountable actor.
    pub fn trace_leak(
        &self,
        fingerprint_hex: &str,
    ) -> Result<Option<crate::disclosure_trace::DisclosureEvent>, String> {
        let bytes = hex::decode(fingerprint_hex.trim()).map_err(|e| format!("fingerprint not hex: {e}"))?;
        let fp: crate::disclosure_trace::DisclosureFingerprint = bytes
            .as_slice()
            .try_into()
            .map_err(|_| "fingerprint must be 16 bytes".to_string())?;
        self.accountability_store()?.trace_leak(&fp).map_err(|e| e.to_string())
    }

    /// List transparency cc records.
    pub fn list_transparency_ccs(
        &self,
    ) -> Result<Vec<crate::disclosure_trace::TransparencyCc>, String> {
        self.accountability_store()?.list_transparency_ccs().map_err(|e| e.to_string())
    }

    /// **Assess a duty of inquiry** — classify conduct against the duty (the fair negligence classifier: was
    /// an accessible means left unchecked, and did a harmful act follow?). Pure; no persistence.
    pub fn assess_duty_of_inquiry(
        &self,
        duty: crate::duty_of_inquiry::DutyOfInquiry,
        conduct: crate::duty_of_inquiry::ConductAgainstDuty,
    ) -> crate::duty_of_inquiry::InquiryVerdict {
        crate::duty_of_inquiry::assess(&duty, &conduct)
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
        let p = Project::new("Community Garden", "shared beds", vec![], 1_700_000_000);
        host.add_project(&p).unwrap();
        host.add_contribution(&Contribution::new(&p.id, "did:wf:owner", "dig", 60, 0, 1.0, wellfare_core::projects::ContributionPrivacy::Public, 1_700_000_050))
            .unwrap();
        host.add_contribution(&Contribution::new(&p.id, "did:wf:owner", "plant", 30, 0, 1.0, wellfare_core::projects::ContributionPrivacy::Public, 1_700_000_100))
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
        let pa = Project::new("Shared Garden", "beds", vec![], 1_700_000_000);
        peer_a.add_project(&pa).unwrap();
        let a_entry = peer_a
            .add_contribution(&Contribution::new(&pa.id, "did:wf:alice", "dig", 60, 0, 1.0, wellfare_core::projects::ContributionPrivacy::Public, 1_700_000_050))
            .unwrap();
        let remote_op = peer_a.build_outbound_operation(&a_entry, 5).unwrap();

        // Peer B has its own local contribution to the same project id.
        let dir_b = tempdir().unwrap();
        let mut peer_b = test_host(dir_b.path());
        peer_b
            .add_contribution(&Contribution::new(&pa.id, "did:wf:bob", "plant", 30, 0, 1.0, wellfare_core::projects::ContributionPrivacy::Public, 1_700_000_100))
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