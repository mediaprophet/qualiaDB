use super::*;
use crate::projects::ProjectRole;

/// A borrowed decision, deliberately neither Clone nor Serialize. It is usable
/// only while the host holds current state. Reauthorize every action; do not
/// cache permits or release the owner lock before the operation commits.
#[derive(Debug)]
pub struct AuthoringPermit<'a> {
    pub(super) view: &'a HostAuthorityView<'a>,
    pub(super) principal: &'a AuthenticatedPrincipal<'a>,
    pub(super) membership: &'a AdmittedMembership<'a>,
    pub(super) identifier: &'a QualifiedIdentifier<'a>,
    pub(super) review_grant: Option<&'a ReviewGrant<'a>>,
    pub(super) action: AuthoringAction,
}

pub(super) fn text(value: &str, limits: AuthoringLimits) -> Result<(), AuthoringError> {
    if value.is_empty() || value.len() > limits.max_text_bytes {
        return Err(AuthoringError::InvalidBinding);
    }
    Ok(())
}

pub(super) fn valid_window(now: u64, start: u64, end: u64) -> bool {
    start < end && start <= now && now < end
}

pub(super) fn valid_document(
    document: &DocumentRevision,
    limits: AuthoringLimits,
) -> Result<(), AuthoringError> {
    text(&document.project_id, limits)?;
    text(&document.document_id, limits)?;
    text(&document.source_id, limits)?;
    if let Some(author) = &document.author_subject {
        text(author, limits)?;
    }
    if document.source_digest == [0; 32] {
        return Err(AuthoringError::InvalidBinding);
    }
    Ok(())
}

/// Fail closed against the current cooperative owner. No heap allocation.
/// Authentication and admission evidence are verified by the native host before
/// constructing the view, not by this pure policy or by source-supplied strings.
pub fn authorize<'a>(
    view: &'a HostAuthorityView<'a>,
    request: &AuthoringRequest<'_>,
) -> Result<AuthoringPermit<'a>, AuthoringError> {
    let principal = view.principal.ok_or(AuthoringError::Unauthenticated)?;
    let limits = view.limits;
    if principal.identifiers.len() > limits.max_identifiers
        || view.memberships.len() > limits.max_memberships
        || view.review_grants.len() > limits.max_review_grants
    {
        return Err(AuthoringError::BudgetExceeded);
    }
    text(principal.subject, limits)?;
    text(principal.authentication_ref, limits)?;
    if view.now_unix >= principal.authenticated_until_unix {
        return Err(AuthoringError::Unauthenticated);
    }
    if view.generation == 0 || view.generation != request.expected_authority_generation {
        return Err(AuthoringError::StaleAuthority);
    }
    valid_document(view.current_document, limits)?;
    valid_document(request.target, limits)?;
    text(&view.project.id, limits)?;
    if view.current_document.project_id != view.project.id {
        return Err(AuthoringError::InvalidBinding);
    }
    if request.target != view.current_document {
        return Err(AuthoringError::StaleDocument);
    }
    // Check all bounded host records rather than allowing an invalid identifier
    // association or revoked membership to be hidden by ordering duplicates.
    for identifier in principal.identifiers {
        for value in [
            identifier.value,
            identifier.scheme,
            identifier.issuer,
            identifier.project_id,
            identifier.evidence_ref,
        ] {
            text(value, limits)?;
        }
    }
    let mut admitted = None;
    let mut matched_identifier = None;
    for member in view.memberships {
        let membership = member.membership;
        for value in [
            &membership.id,
            &membership.project_id,
            &membership.member_did,
        ] {
            text(value, limits)?;
        }
        text(member.member_subject, limits)?;
        text(member.admission_ref, limits)?;
        if member.member_subject != principal.subject || membership.project_id != view.project.id {
            continue;
        }
        if member.revoked
            || !valid_window(
                view.now_unix,
                member.valid_from_unix,
                member.valid_until_unix,
            )
            || u64::try_from(membership.agreed_at().to_unix_secs())
                .map_or(true, |agreed_at| agreed_at > view.now_unix)
        {
            return Err(AuthoringError::NotMember);
        }
        for identifier in principal.identifiers {
            if identifier.value != membership.member_did || identifier.project_id != view.project.id
            {
                continue;
            }
            if identifier.revoked
                || !valid_window(
                    view.now_unix,
                    identifier.valid_from_unix,
                    identifier.valid_until_unix,
                )
                || matched_identifier.is_some()
            {
                return Err(AuthoringError::NotMember);
            }
            matched_identifier = Some(identifier);
        }
        if matched_identifier.is_none() || admitted.is_some() {
            return Err(AuthoringError::NotMember);
        }
        admitted = Some(member);
    }
    let membership = admitted.ok_or(AuthoringError::NotMember)?;
    let mut review_grant = None;
    if request.action == AuthoringAction::ReviewDecision {
        if view.current_document.author_subject.as_deref() == Some(principal.subject) {
            return Err(AuthoringError::SelfReview);
        }
        if view.current_document.author_subject.is_none() {
            return Err(AuthoringError::ReviewDenied);
        }
        for grant in view.review_grants {
            for value in [
                grant.id,
                grant.project_id,
                grant.document_id,
                grant.reviewer_subject,
                grant.admission_ref,
            ] {
                text(value, limits)?;
            }
            if grant.project_id != view.project.id
                || grant.document_id != view.current_document.document_id
                || grant.reviewer_subject != principal.subject
            {
                continue;
            }
            if grant.revoked
                || !valid_window(view.now_unix, grant.valid_from_unix, grant.valid_until_unix)
                || review_grant.is_some()
            {
                return Err(AuthoringError::ReviewDenied);
            }
            review_grant = Some(grant);
        }
        if review_grant.is_none() {
            return Err(AuthoringError::ReviewDenied);
        }
    } else if request.action != AuthoringAction::ReadSource
        && membership.membership.role == ProjectRole::Observer
    {
        return Err(AuthoringError::RoleDenied);
    }
    Ok(AuthoringPermit {
        view,
        principal,
        membership,
        identifier: matched_identifier.ok_or(AuthoringError::NotMember)?,
        review_grant,
        action: request.action,
    })
}

impl AuthoringPermit<'_> {
    pub fn action(&self) -> AuthoringAction {
        self.action
    }
    pub fn principal_subject(&self) -> &str {
        self.principal.subject
    }
    pub fn target(&self) -> &DocumentRevision {
        self.view.current_document
    }
    pub fn authority_generation(&self) -> u64 {
        self.view.generation
    }
}
