use super::*;

pub(crate) fn validate_admission_packages(snapshot: &AdmissionSnapshot) -> Result<(), String> {
    let limits = AuthoringLimits::default();
    if snapshot.package_reviews.len() > limits.max_review_grants {
        return Err("package grant count exceeds bounds".into());
    }
    for (i, grant) in snapshot.package_reviews.iter().enumerate() {
        validate_package(&grant.target, limits).map_err(|e| e.to_string())?;
        let reviewer = snapshot
            .subjects
            .iter()
            .find(|s| s.subject == grant.reviewer_subject)
            .ok_or("package review subject lacks explicit admission")?;
        if grant.target.source.project_id != snapshot.project.id
            || grant.id.is_empty()
            || grant.id.len() > limits.max_text_bytes
            || grant.admission_ref.is_empty()
            || grant.admission_ref.len() > limits.max_text_bytes
            || grant.valid_from_unix >= grant.valid_until_unix
            || grant.valid_from_unix < reviewer.valid_from_unix
            || grant.valid_until_unix > reviewer.valid_until_unix
            || grant.valid_from_unix < snapshot.valid_from_unix
            || grant.valid_until_unix > snapshot.valid_until_unix
            || grant.target.producer_subject == grant.reviewer_subject
            || grant.target.source.author_subject.as_deref()
                == Some(grant.reviewer_subject.as_str())
            || !snapshot
                .subjects
                .iter()
                .any(|s| s.subject == grant.target.producer_subject)
            || snapshot.package_reviews[..i].iter().any(|g| {
                g.id == grant.id
                    || (g.target == grant.target && g.reviewer_subject == grant.reviewer_subject)
            })
        {
            return Err("invalid/duplicate exact package-review admission scope".into());
        }
    }
    Ok(())
}

/// A grant identifier cannot be rebound, removed or silently resurrected by a
/// replacement snapshot. Explicit revocation is monotonic for retained grants.
pub(crate) fn validate_grant_successor(
    old: &AdmissionSnapshot,
    new: &AdmissionSnapshot,
) -> Result<(), String> {
    for previous in &old.package_reviews {
        let next = new
            .package_reviews
            .iter()
            .find(|g| g.id == previous.id)
            .ok_or("package grant removal requires explicit migration")?;
        let mut unchanged = next.clone();
        unchanged.revoked = previous.revoked;
        if unchanged != *previous || (previous.revoked && !next.revoked) {
            return Err("unsafe package grant rebinding/resurrection denied".into());
        }
    }
    Ok(())
}

pub(crate) fn validate_ledger(state: &DurableState) -> Result<(), String> {
    if state.contributions.len() > 256
        || state.reviews.len() > 256
        || state.packages.len() > 256
        || state.requests.len() > 256
        || state.enrollment.len() > 256
        || state.package_contributions.len() > 256
        || state.package_requests.len() > 256
        || state.package_reviews.len() > 256
        || state.package_grant_decisions.len() > 256
    {
        return Err("native evidence count budget exceeded before journal write".into());
    }
    let target_ok = |target: &PackageRevision| -> Result<(), String> {
        validate_package(target, AuthoringLimits::default()).map_err(|e| e.to_string())?;
        if target.source.project_id != state.admission.snapshot.project.id
            || !state
                .packages
                .iter()
                .any(|saved| saved_matches(saved, target))
        {
            return Err("package evidence lacks exact native save/source/lineage binding".into());
        }
        Ok(())
    };
    for (i, c) in state.package_contributions.iter().enumerate() {
        target_ok(&c.target)?;
        if c.actor.subject != c.target.producer_subject
            || c.contribution.project_id != c.target.source.project_id
            || c.contribution.contributor_did != c.actor.identifier_association.value
            || state.package_contributions[..i]
                .iter()
                .any(|r| r.contribution.id == c.contribution.id)
        {
            return Err("invalid package contribution attribution/duplicate".into());
        }
    }
    for (i, r) in state.package_requests.iter().enumerate() {
        target_ok(&r.target)?;
        if r.actor.subject != r.target.producer_subject
            || r.decision_ids.len() > 256
            || state.package_requests[..i]
                .iter()
                .any(|other| other.id == r.id)
            || !state.package_contributions.iter().any(|c| {
                c.contribution.id == r.contribution_id
                    && c.target == r.target
                    && c.actor.subject == r.actor.subject
            })
            || r.decision_ids.iter().enumerate().any(|(j, id)| {
                r.decision_ids[..j].contains(id)
                    || !state
                        .package_reviews
                        .iter()
                        .any(|d| &d.id == id && d.request_id == r.id && d.target == r.target)
            })
        {
            return Err("invalid package review request/decision lineage".into());
        }
    }
    for (i, r) in state.package_reviews.iter().enumerate() {
        target_ok(&r.target)?;
        if r.actor.subject == r.target.producer_subject
            || r.target.source.author_subject.as_deref() == Some(r.actor.subject.as_str())
            || state
                .package_contributions
                .iter()
                .any(|c| c.target == r.target && c.actor.subject == r.actor.subject)
            || state.package_reviews[..i]
                .iter()
                .any(|other| other.id == r.id)
            || !state.package_requests.iter().any(|q| {
                q.id == r.request_id && q.target == r.target && q.decision_ids.contains(&r.id)
            })
        {
            return Err("invalid package decision attribution/request binding".into());
        }
    }
    Ok(())
}
