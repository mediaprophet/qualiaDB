//! PWA package & publish

use super::super::journal::JournalEntry;
use wellfare_core::finance::{
    build_ledger_entry_envelope, derived_balance, ledger_entry_summary, parse_ledger_summary,
    BalanceReport, LedgerEntry,
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

use super::*;

impl WebizenHostApi {
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
        let manifest = super::super::qapp_publish::build_manifest(
            id,
            name,
            kind,
            description,
            capabilities_csv,
            wasm_filename,
        );
        super::super::qapp_publish::write_pwa_bundle(std::path::Path::new(target_dir), &manifest)
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

    pub(crate) fn latest_journal_entry(&self) -> Result<JournalEntry, String> {
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
                        occurred_at_unix: row.asserted_instant.to_unix_secs() as u32,
                        occurred_at_instant: None,
                    });
                }
            }
        }
        Ok(derived_balance(&entries))
    }
}
