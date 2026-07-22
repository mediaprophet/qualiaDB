//! Backup/restore + clinical documents


use super::super::journal::JournalEntry;
use super::super::backup::{self, BackupReport};
use super::super::sync_outbox::SyncOutbox;
use super::super::blob_store::BlobStore;
use wellfare_core::clinical::{
    build_clinical_attachment_envelope, build_clinical_report_envelope, clinical_attachment_summary,
    clinical_report_summary, AttachmentMeta, ClinicalReport, ClinicalReportType,
};


use super::*;

impl WebizenHostApi {
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
            sanctuary_configured: super::super::sanctuary_vault::is_configured(&self.storage_root),
            sanctuary_keychain_wrapped: super::super::sanctuary_vault::is_keychain_wrapped(
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
    /// government-letter documents, â€¦), integrity-verified by the blob store.
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

}