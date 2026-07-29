//! Agency layer + wellbeing assessment

use super::super::journal::JournalEntry;
use qualia_cooperative_core::agency_delegation::{
    agency_delegation_full_json, build_agency_delegation_envelope, delegation_permits,
    parse_agency_delegation, AccessDecision, AccessRequest, AgencyDelegation, ConsentState,
    Precedence,
};
use qualia_cooperative_core::agency_domain::agency_domain_taxonomy;
use qualia_cooperative_core::taxonomy::Sphere;
use qualia_cooperative_core::trigger::TriggerContext;
use wellfare_core::assessment::{
    assessment_summary, build_assessment_envelope, instrument, instrument_dto, instruments,
    parse_assessment, score, AssessmentResult, InstrumentDto,
};

use super::*;

impl WebizenHostApi {
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
        let envelope = build_agency_delegation_envelope(
            delegation,
            &self.owner_did,
            &self.author_did,
            asserted,
        );
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
        let mut d = AgencyDelegation::new(principal_did, domain, anchor, Self::now_unix() as u32);
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
        let envelope = build_assessment_envelope(&result, &self.owner_did, &self.author_did, now);
        let summary = assessment_summary(&result);
        self.submit_record_with_summary(QAPP_WELLBEING, envelope, SOURCE_WELLBEING, Some(summary))?;
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
}
