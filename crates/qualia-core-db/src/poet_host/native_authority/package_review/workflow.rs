use super::*;

impl NativeAuthority {
    pub fn record_package_contribution(
        &self,
        handle: &SessionHandle,
        expected_generation: u64,
        target: &PackageRevision,
        id: &str,
        description: &str,
        minutes: u32,
        status: WorkStatus,
        verify_current: impl FnOnce(&PackageRevision) -> Result<(), String>,
    ) -> Result<PackageContributionRecord, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        if state.durable.package_contributions.len() >= 256
            || state
                .durable
                .package_contributions
                .iter()
                .any(|r| r.contribution.id == id)
        {
            return Err("duplicate package contribution or evidence budget exhausted".into());
        }
        let record = self.with_package(
            &state,
            handle,
            expected_generation,
            target,
            PackageAction::Contribute,
            |permit| {
                let record = permit
                    .contribution_record(id, description, minutes, status)
                    .map_err(|e| e.to_string())?;
                verify_current(target)?;
                Ok(record)
            },
        )?;
        let mut next = state.durable.clone();
        next.package_contributions.push(record.clone());
        self.commit_package_records(&mut state, next)?;
        Ok(record)
    }

    pub fn request_package_review(
        &self,
        handle: &SessionHandle,
        expected_generation: u64,
        target: &PackageRevision,
        contribution_id: &str,
        id: &str,
        evidence: &str,
        verify_current: impl FnOnce(&PackageRevision) -> Result<(), String>,
    ) -> Result<PackageReviewRequest, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        if state.durable.package_requests.len() >= 256
            || state.durable.package_requests.iter().any(|r| r.id == id)
        {
            return Err("duplicate package request or evidence budget exhausted".into());
        }
        let contribution = state
            .durable
            .package_contributions
            .iter()
            .find(|r| r.contribution.id == contribution_id)
            .ok_or("exact saved-package contribution required before review request")?;
        let record = self.with_package(
            &state,
            handle,
            expected_generation,
            target,
            PackageAction::RequestReview,
            |permit| {
                let record = permit
                    .request_record(id, contribution, evidence)
                    .map_err(|e| e.to_string())?;
                verify_current(target)?;
                Ok(record)
            },
        )?;
        let mut next = state.durable.clone();
        next.package_requests.push(record.clone());
        self.commit_package_records(&mut state, next)?;
        Ok(record)
    }

    pub fn record_requested_package_review(
        &self,
        handle: &SessionHandle,
        expected_generation: u64,
        target: &PackageRevision,
        request_id: &str,
        id: &str,
        decision: ReviewDecision,
        rationale: &str,
        evidence: &str,
        verify_current: impl FnOnce(&PackageRevision) -> Result<(), String>,
    ) -> Result<PackageReviewRecord, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        if state.durable.package_reviews.len() >= 256
            || state.durable.package_reviews.iter().any(|r| r.id == id)
        {
            return Err("duplicate package decision or evidence budget exhausted".into());
        }
        let request = state
            .durable
            .package_requests
            .iter()
            .find(|r| r.id == request_id)
            .ok_or("package review request unavailable")?;
        if &request.target != target {
            return Err("package review request target changed".into());
        }
        if request.decision_ids.len() >= 256 {
            return Err("package request decision budget exhausted".into());
        }
        let record = self.with_package(
            &state,
            handle,
            expected_generation,
            target,
            PackageAction::Decide,
            |permit| {
                let record = permit
                    .review_record(id, request_id, decision, rationale, evidence)
                    .map_err(|e| e.to_string())?;
                verify_current(target)?;
                Ok(record)
            },
        )?;
        let mut next = state.durable.clone();
        next.package_reviews.push(record.clone());
        next.package_requests
            .iter_mut()
            .find(|r| r.id == request_id)
            .ok_or("package request disappeared")?
            .decision_ids
            .push(record.id.clone());
        self.commit_package_records(&mut state, next)?;
        Ok(record)
    }
}
