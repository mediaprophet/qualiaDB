use super::{
    super::{
        policy::{text, valid_document, valid_window},
        *,
    },
    *,
};

#[derive(Debug)]
pub struct PackagePermit<'a> {
    pub(super) source: AuthoringPermit<'a>,
    pub(super) target: &'a PackageRevision,
    pub(super) grant: Option<&'a PackageReviewGrant>,
    pub(super) action: PackageAction,
    pub(super) known_destination_lineage_complete: bool,
}

pub fn validate_package(
    target: &PackageRevision,
    limits: AuthoringLimits,
) -> Result<(), AuthoringError> {
    valid_document(&target.source, limits)?;
    for value in [
        &target.package_id,
        &target.filename,
        &target.producer_subject,
    ] {
        text(value, limits)?;
        if value.trim().is_empty() {
            return Err(AuthoringError::InvalidBinding);
        }
    }
    if target.package_digest == [0; 32]
        || target.package_bytes == 0
        || target.predecessor_digest == Some([0; 32])
    {
        return Err(AuthoringError::InvalidBinding);
    }
    Ok(())
}

/// The native owner holds its current source/package/admission lock throughout.
/// Source review_grants are deliberately never consulted for package authority.
pub fn authorize_package<'a>(
    view: &'a PackageAuthorityView<'a>,
    target: &PackageRevision,
    expected_generation: u64,
    action: PackageAction,
) -> Result<PackagePermit<'a>, AuthoringError> {
    let source = authorize(
        view.source,
        &AuthoringRequest {
            action: if action == PackageAction::Decide {
                AuthoringAction::ReadSource
            } else {
                AuthoringAction::ContributionRecord
            },
            target: &target.source,
            expected_authority_generation: expected_generation,
        },
    )?;
    let limits = view.source.limits;
    validate_package(view.current_package, limits)?;
    validate_package(target, limits)?;
    if target != view.current_package {
        return Err(AuthoringError::StaleDocument);
    }
    if view.grants.len() > limits.max_review_grants || view.contributor_subjects.len() > 1024 {
        return Err(AuthoringError::BudgetExceeded);
    }
    for subject in view.contributor_subjects {
        text(subject, limits)?;
    }
    let subject = source.principal_subject();
    let mut selected = None;
    if action == PackageAction::Decide {
        if target.producer_subject == subject
            || target.source.author_subject.as_deref() == Some(subject)
            || view.contributor_subjects.contains(&subject)
        {
            return Err(AuthoringError::SelfReview);
        }
        for grant in view.grants {
            validate_package(&grant.target, limits)?;
            for value in [&grant.id, &grant.reviewer_subject, &grant.admission_ref] {
                text(value, limits)?;
            }
            if &grant.target != target || grant.reviewer_subject != subject {
                continue;
            }
            if grant.revoked
                || !valid_window(
                    view.source.now_unix,
                    grant.valid_from_unix,
                    grant.valid_until_unix,
                )
                || selected.is_some()
            {
                return Err(AuthoringError::ReviewDenied);
            }
            selected = Some(grant);
        }
        if selected.is_none() {
            return Err(AuthoringError::ReviewDenied);
        }
    } else if target.producer_subject != subject {
        return Err(AuthoringError::InvalidBinding);
    }
    Ok(PackagePermit {
        source,
        target: view.current_package,
        grant: selected,
        action,
        known_destination_lineage_complete: view.known_destination_lineage_complete,
    })
}
