//! Core methods: accessibility, snapshot, policy, consent, records, conditions, companion import, medications, diet, journal

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use super::super::accessibility_prefs;
use super::super::consent_store::ConsentGrantRecord;
use super::super::export_package::{
    build_export_package, export_policy_receipt, ExportReceipt, HealthExportPackage,
};
use super::super::graph_query::GraphCoverageRow;
use super::super::host_state::WellfairHostSnapshot;
use super::super::host_state::{
    AccessibilityPreferences, ConsentGrantDraft, PolicyDecisionDto, SubmitOutcome,
};
use super::super::import_samsung::{
    import_samsung_folder, ingest_companion_health_bundle, SamsungImportReport,
};
use super::super::journal::JournalEntry;
use super::super::policy::{DecisionResult, PolicyDecisionService};
use super::super::receipt::{receipt_from_decision, ReceiptRecord};
use super::super::sanctuary::{apply_sanctuary_projection, load_prefs as load_sanctuary_prefs};
use super::super::snapshot::build_host_snapshot;
use super::super::sync_outbox::SyncOutboxEntry;
use super::super::vault::VaultService;
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
use wellfare_core::personal_records::{
    build_disputed_diagnosis_envelope, build_housing_safety_envelope, disputed_diagnosis_summary,
    housing_safety_summary, DisputedDiagnosisReport, HousingSafetyReport,
};
use wellfare_core::record::RecordEnvelope;

use super::*;

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

    pub fn snapshot_from_vault(
        key_vault: &KeyVault,
        owner_label: &str,
        demo_mode: bool,
    ) -> WellfairHostSnapshot {
        build_host_snapshot(key_vault, true, owner_label, demo_mode)
    }

    pub fn build_snapshot(
        &mut self,
        key_vault: &KeyVault,
        owner_label: &str,
    ) -> WellfairHostSnapshot {
        let mut snap = super::super::snapshot::build_host_snapshot_with_storage(
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
                super::super::host_state::SyncQueueState::Queued
            } else {
                super::super::host_state::SyncQueueState::Idle
            };
        }
        if let Some(hash) = self.vault.last_checkpoint_hash() {
            snap.last_checkpoint_prefix = Some(hex::encode(&hash[..4]));
        }
        if let Ok(queued) = self.vault.outbox_queued_count() {
            snap.pending_jobs = snap.pending_jobs.saturating_add(queued as u32);
            if queued > 0 {
                snap.sync_state = super::super::host_state::SyncQueueState::Queued;
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

    /// Stable hash used by local graph evaluators without exposing the person's DID.
    pub fn owner_did_hash(&self) -> u64 {
        qualia_core_db::q_hash(&self.owner_did)
    }

    pub(crate) fn chora_signing_key(&self) -> &SigningKey {
        &self.signing_key
    }

    pub(crate) fn chora_owner_did(&self) -> &str {
        &self.owner_did
    }

    pub(crate) fn now_unix() -> u64 {
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
        self.vault
            .append_receipt(&receipt)
            .map_err(|e| e.to_string())?;
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
            self.vault
                .append_receipt(&receipt)
                .map_err(|e| e.to_string())?;
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
    pub(crate) fn commit_permitted(
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
        let ts = envelope.asserted_instant().to_unix_secs() as u32;
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
        let pkg = build_export_package(&entries, exported_at, self.vault.last_checkpoint_hash());
        let receipt = export_policy_receipt(&pkg, exported_at);
        self.vault
            .append_receipt(&receipt)
            .map_err(|e| e.to_string())?;
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
            .filter(|row| !super::super::sanctuary::is_sanctuary_protected_kind(&row.kind))
            .collect())
    }

    pub(crate) fn payload_hash_hex(payload: &str) -> String {
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
            ceased_at_instant: None,
            created_at_unix: now,
            created_at_instant: None,
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
            administered_at_instant: None,
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
            logged_at_instant: None,
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

    pub fn list_journal_by_kind(
        &self,
        kind: &str,
        limit: usize,
    ) -> Result<Vec<JournalEntry>, String> {
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
    ) -> Result<super::super::anatomy_view::AnatomyViewReport, String> {
        let conditions = self.list_journal_by_kind("condition", 256)?;
        let medications = self.list_journal_by_kind("medication", 256)?;
        let diet = self.list_journal_by_kind("diet", 256)?;
        let state = self.get_physiological_state();
        Ok(super::super::anatomy_view::build_report_from_journal(
            &conditions,
            &medications,
            &diet,
            super::super::anatomy_view::parse_lens(lens),
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
        let fit = self.body_fit();
        Ok(super::super::anatomy_render::body_scene_with_fit(
            &report,
            azimuth_deg,
            elevation_deg,
            &fit,
        ))
    }
}
