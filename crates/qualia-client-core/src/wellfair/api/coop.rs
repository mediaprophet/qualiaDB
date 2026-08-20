//! Cooperative projects + credentials

use super::super::blob_store::BlobStore;
use super::super::journal::JournalEntry;
use wellfare_core::credentials::{
    build_credential_envelope, build_presentation, credential_summary, CredentialRecord,
    FieldSelectedPresentation,
};
use wellfare_core::projects::{
    build_contribution_envelope, build_membership_envelope, build_project_envelope,
    contribution_summary, derive_obligations, membership_summary, project_summary, Contribution,
    Obligation, Project, ProjectMembership,
};

use super::*;

impl WebizenHostApi {
    // --- Cooperative projects (Phase 5 / COP-01..) ---

    pub fn add_project(&mut self, project: &Project) -> Result<JournalEntry, String> {
        let asserted = Self::now_unix() as u32;
        let hash =
            Self::payload_hash_hex(&serde_json::to_string(project).map_err(|e| e.to_string())?);
        let envelope = build_project_envelope(
            project,
            &self.owner_did,
            &self.author_did,
            asserted,
            Some(hash),
        );
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

    pub fn add_contribution(
        &mut self,
        contribution: &Contribution,
    ) -> Result<JournalEntry, String> {
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
                if let Some(c) = contribution_from_summary(
                    row.id.clone(),
                    summary,
                    row.asserted_instant.to_unix_secs() as u32,
                ) {
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
                if let Some(c) = contribution_from_summary(
                    op.record_id.clone(),
                    &op.payload_summary,
                    op.committed_unix,
                ) {
                    contributions.push(c);
                }
            }
        }
        Ok(derive_obligations(&contributions))
    }

    // --- Credentials (Phase 3/7 / CRE-01..) ---

    pub fn add_credential(
        &mut self,
        credential: &CredentialRecord,
    ) -> Result<JournalEntry, String> {
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
}
