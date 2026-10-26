use super::*;

impl NativeAuthority {
    pub fn record_contribution(
        &self,
        handle: &SessionHandle,
        id: &str,
        description: &str,
        minutes: u32,
        status: WorkStatus,
    ) -> Result<ContributionRecord, String> {
        self.contribution_guarded(handle, None, id, description, minutes, status)
    }
    pub fn record_contribution_at(
        &self,
        handle: &SessionHandle,
        expected: &DocumentRevision,
        id: &str,
        description: &str,
        minutes: u32,
        status: WorkStatus,
    ) -> Result<ContributionRecord, String> {
        self.contribution_guarded(handle, Some(expected), id, description, minutes, status)
    }
    fn contribution_guarded(
        &self,
        handle: &SessionHandle,
        expected: Option<&DocumentRevision>,
        id: &str,
        description: &str,
        minutes: u32,
        status: WorkStatus,
    ) -> Result<ContributionRecord, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let now = now()?;
        if state.durable.contributions.len() >= 256 {
            return Err("contribution evidence budget exhausted".into());
        }
        let record = self.view(&state, handle, now, |view| {
            let permit = authorize(
                view,
                &AuthoringRequest {
                    action: AuthoringAction::ContributionRecord,
                    target: expected.unwrap_or(view.current_document),
                    expected_authority_generation: view.generation,
                },
            )
            .map_err(|e| e.to_string())?;
            let previous = state
                .durable
                .contributions
                .iter()
                .rev()
                .find(|r| r.contributor_subject == permit.principal_subject());
            permit
                .contribution_record(id, description, minutes, status, previous)
                .map_err(|e| e.to_string())
        })?;
        if state
            .durable
            .contributions
            .iter()
            .any(|r| r.contribution.id == id)
        {
            return Err("duplicate contribution identifier".into());
        }
        let mut next = state.durable.clone();
        next.contributions.push(record.clone());
        if let Err(e) = state.journal.append(&next) {
            state.reconciliation = true;
            return Err(e);
        }
        state.durable = next;
        Ok(record)
    }
    pub fn record_review(
        &self,
        handle: &SessionHandle,
        id: &str,
        decision: ReviewDecision,
        rationale: &str,
        evidence: &str,
    ) -> Result<ReviewRecord, String> {
        self.review_guarded(handle, None, None, id, decision, rationale, evidence)
    }
    /// An explicit request and exact source revision must still be current at
    /// the same locked decision commit. This is source review only.
    pub fn record_requested_source_review(
        &self,
        handle: &SessionHandle,
        expected: &DocumentRevision,
        request_id: &str,
        id: &str,
        decision: ReviewDecision,
        rationale: &str,
        evidence: &str,
    ) -> Result<ReviewRecord, String> {
        self.review_guarded(
            handle,
            Some(expected),
            Some(request_id),
            id,
            decision,
            rationale,
            evidence,
        )
    }
    fn review_guarded(
        &self,
        handle: &SessionHandle,
        expected: Option<&DocumentRevision>,
        request_id: Option<&str>,
        id: &str,
        decision: ReviewDecision,
        rationale: &str,
        evidence: &str,
    ) -> Result<ReviewRecord, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let now = now()?;
        if state.durable.reviews.len() >= 256 {
            return Err("review evidence budget exhausted".into());
        }
        if let Some(request_id) = request_id {
            let request = state
                .durable
                .requests
                .iter()
                .find(|request| request.id == request_id)
                .ok_or("source review request is unavailable")?;
            if expected != Some(&request.target) || request.target != state.durable.document {
                return Err("source review request is stale; source and draft preserved".into());
            }
        }
        let record = self.view(&state, handle, now, |view| {
            authorize(
                view,
                &AuthoringRequest {
                    action: AuthoringAction::ReviewDecision,
                    target: expected.unwrap_or(view.current_document),
                    expected_authority_generation: view.generation,
                },
            )
            .map_err(|e| e.to_string())?
            .review_record(id, decision, rationale, evidence)
            .map_err(|e| e.to_string())
        })?;
        if state.durable.reviews.iter().any(|r| r.id == id) {
            return Err("duplicate review identifier".into());
        }
        let mut next = state.durable.clone();
        next.reviews.push(record.clone());
        if let Some(request_id) = request_id {
            next.requests
                .iter_mut()
                .find(|request| request.id == request_id)
                .ok_or("source review request disappeared")?
                .decision_ids
                .push(record.id.clone());
        }
        if let Err(e) = state.journal.append(&next) {
            state.reconciliation = true;
            return Err(e);
        }
        state.durable = next;
        Ok(record)
    }
}
