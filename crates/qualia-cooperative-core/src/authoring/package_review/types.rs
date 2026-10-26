use super::super::*;
use crate::projects::Contribution;
use serde::{Deserialize, Serialize};

/// Project-scoped declared release identifier, not universal referent authority.
/// The filename is a locator; its digest is a byte binding, not an identifier mapping.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageRevision {
    pub package_id: String,
    pub filename: String,
    pub package_digest: [u8; 32],
    /// Prior destination bytes from the native overwrite guard, not full derivation.
    pub predecessor_digest: Option<[u8; 32]>,
    pub package_bytes: u64,
    pub source: DocumentRevision,
    /// Actual writer from native audit, separate from source authorship.
    pub producer_subject: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageReviewGrant {
    pub id: String,
    pub target: PackageRevision,
    pub reviewer_subject: String,
    pub admission_ref: String,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
    pub revoked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PackageAction {
    Contribute,
    RequestReview,
    Decide,
}

/// Borrowed host-current view, never deserialized from a client request.
pub struct PackageAuthorityView<'a> {
    pub source: &'a HostAuthorityView<'a>,
    pub current_package: &'a PackageRevision,
    pub grants: &'a [PackageReviewGrant],
    /// All known canonical target contributors, not only the last writer.
    pub contributor_subjects: &'a [&'a str],
    pub known_destination_lineage_complete: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageActorEvidence {
    pub subject: String,
    pub principal_kind: PrincipalKind,
    pub authentication_ref: String,
    pub identifier_association: IdentifierAssociationEvidence,
    pub membership_id: String,
    pub admission_ref: String,
    pub authority_generation: u64,
    pub occurred_at_unix: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageContributionRecord {
    pub contribution: Contribution,
    pub target: PackageRevision,
    pub actor: PackageActorEvidence,
    pub work_status: WorkStatus,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageReviewRequest {
    pub id: String,
    pub target: PackageRevision,
    pub contribution_id: String,
    pub actor: PackageActorEvidence,
    pub evidence_ref: String,
    pub decision_ids: Vec<String>,
    /// This workflow cannot establish complete resource authorship or rights.
    pub contribution_rights_scope: String,
    pub known_destination_lineage_complete: bool,
}

/// A scoped decision, never automatic model completion or human acceptance.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PackageReviewRecord {
    pub id: String,
    pub target: PackageRevision,
    pub request_id: String,
    pub actor: PackageActorEvidence,
    pub review_grant_id: String,
    pub review_admission_ref: String,
    pub decision: ReviewDecision,
    pub rationale: String,
    pub decision_evidence_ref: String,
    pub known_destination_lineage_complete: bool,
    pub contribution_rights_scope: String,
}
