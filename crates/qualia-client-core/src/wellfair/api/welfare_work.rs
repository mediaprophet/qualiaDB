//! Welfare support + cooperative work items


use super::super::journal::JournalEntry;
use super::super::blob_store::BlobStore;
use qualia_cooperative_core::work_item::{
    build_work_item_envelope, build_work_item_status_envelope, derive_board,
    parse_work_item_status_summary, parse_work_item_summary, work_item_status_summary,
    work_item_summary, BoardColumn, WorkItem, WorkItemStatusEvent,
};
use wellfare_core::authority_attestation::{
    authority_attestation_summary, build_authority_attestation_envelope, AgentInCapacity, Authority,
    AuthorityAttestation, Representation,
};
use wellfare_core::welfare_support::{
    build_assistance_need_envelope, build_government_letter_envelope, build_welfare_stream_envelope,
    AssistanceNeed, GovernmentLetter, StreamStatus, Urgency, WelfareStream,
};


use super::*;

impl WebizenHostApi {
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

}