//! Guardianship + transparency + disclosure

use super::super::host_state::{GuardianshipProposalView, SubmitOutcome};
use super::super::journal::JournalEntry;
use super::super::live_share::{
    append_live_share_journal, live_share_decision_journal_entry, live_share_request_journal_entry,
    sanctuary_allows_classified_projection, validate_live_share_decision, LiveShareStore,
};
use super::super::policy::DecisionResult;
use super::super::sanctuary::load_prefs as load_sanctuary_prefs;
use wellfare_core::conditions::{build_condition_envelope, condition_summary};
use wellfare_core::guardianship::{
    build_proposal_envelope, build_vote_envelope, derive_status, parse_proposal_summary,
    parse_vote_summary, proposal_summary, vote_summary, GuardianshipProposal, GuardianshipVote,
    ProposalState,
};
use wellfare_core::live_share::{LiveSectionRequest, UsageAgreement};
use wellfare_core::record::RecordEnvelope;

use super::*;

impl WebizenHostApi {
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
    pub(crate) fn escrow_proxy_write(
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
            let Some(ref summary) = row.summary else {
                continue;
            };
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
                    obligations: vec!["guardianship_ratified".into(), "emit_wal_receipt".into()],
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

    pub(crate) fn find_proposal(
        &self,
        proposal_id: &str,
    ) -> Result<Option<GuardianshipProposal>, String> {
        let rows =
            self.list_journal_by_kind("guardianship_proposal", super::super::journal::MAX_LIST)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| r.summary.as_deref().and_then(parse_proposal_summary))
            .find(|p| p.id == proposal_id))
    }

    fn list_guardianship_votes(&self, proposal_id: &str) -> Result<Vec<GuardianshipVote>, String> {
        let rows =
            self.list_journal_by_kind("guardianship_vote", super::super::journal::MAX_LIST)?;
        Ok(rows
            .into_iter()
            .filter_map(|r| r.summary.as_deref().and_then(parse_vote_summary))
            .filter(|v| v.proposal_id == proposal_id)
            .collect())
    }

    fn escrowed_already_committed(&self, proposal: &GuardianshipProposal) -> Result<bool, String> {
        let Some(escrowed_id) = proposal.escrowed_record_id() else {
            return Ok(false);
        };
        let kind = wellfare_core::conditions::journal_kind_for_record_id(&escrowed_id);
        let rows = self.list_journal_by_kind(kind, super::super::journal::MAX_LIST)?;
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
        if pending.status != super::super::live_share::LiveShareRequestStatus::Pending {
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
            .decide(request_id, approved, projection_kinds, now, deny.as_deref())
            .map_err(|e| e.to_string())?;
        let committed_unix = now as u32;
        let entry = live_share_decision_journal_entry(&updated, committed_unix);
        append_live_share_journal(&self.storage_root, &entry)?;
        Ok(entry)
    }

    pub fn get_live_share_record(
        &self,
        request_id: &str,
    ) -> Result<Option<super::super::live_share::LiveShareRequestRecord>, String> {
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
