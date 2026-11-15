use crate::projects::{Project, ProjectMembership};
use serde::{Deserialize, Serialize};

/// Exact source state. This is not an artifact store stamp or an authority grant.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DocumentRevision {
    pub project_id: String,
    pub document_id: String,
    pub source_id: String,
    pub revision: u64,
    pub source_digest: [u8; 32],
    /// Host-admitted last author referent, not a guessed alias or display name.
    pub author_subject: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AuthoringAction {
    ReadSource,
    ExecuteSource,
    MaterialEdit,
    Undo,
    Redo,
    Rebuild,
    SaveCheckpoint,
    ReopenCheckpoint,
    ContributionRecord,
    ReviewDecision,
}

/// Actor capacity is separate from project role, authorship and paid status.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrincipalKind {
    Human,
    SoftwareAgent,
    Organization,
}

/// Current host-verified identifier association. All scope/issuer/source fields
/// qualify the association; an equal string alone does not authenticate anyone.
#[derive(Debug)]
pub struct QualifiedIdentifier<'a> {
    pub value: &'a str,
    pub scheme: &'a str,
    pub issuer: &'a str,
    /// Exact cooperative project key to which this association is admitted.
    pub project_id: &'a str,
    pub evidence_ref: &'a str,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
    pub revoked: bool,
}

/// Rust-only result of a native authentication provider. No Deserialize or
/// source-facing constructor exists. Fields are filled only by trusted host
/// code after native proof/key/session and identifier association checks.
#[derive(Debug)]
pub struct AuthenticatedPrincipal<'a> {
    pub subject: &'a str,
    pub kind: PrincipalKind,
    pub authentication_ref: &'a str,
    pub authenticated_until_unix: u64,
    pub identifiers: &'a [QualifiedIdentifier<'a>],
}

/// A canonical membership plus the native owner's admission/revocation state.
/// Constructing ProjectMembership itself is not admission.
#[derive(Debug)]
pub struct AdmittedMembership<'a> {
    pub membership: &'a ProjectMembership,
    pub member_subject: &'a str,
    pub admission_ref: &'a str,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
    pub revoked: bool,
}

/// Explicit review authority. Steward/contributor roles never imply review.
#[derive(Debug)]
pub struct ReviewGrant<'a> {
    pub id: &'a str,
    pub project_id: &'a str,
    pub document_id: &'a str,
    pub reviewer_subject: &'a str,
    pub admission_ref: &'a str,
    pub valid_from_unix: u64,
    pub valid_until_unix: u64,
    pub revoked: bool,
}

/// Caller-selected bounds checked before scanning or allocating evidence.
#[derive(Debug, Clone, Copy)]
pub struct AuthoringLimits {
    pub max_identifiers: usize,
    pub max_memberships: usize,
    pub max_review_grants: usize,
    pub max_text_bytes: usize,
    pub max_record_bytes: usize,
}

impl Default for AuthoringLimits {
    fn default() -> Self {
        Self {
            max_identifiers: 32,
            max_memberships: 256,
            max_review_grants: 256,
            max_text_bytes: 4096,
            max_record_bytes: 64 * 1024,
        }
    }
}

/// Borrow the current native state, never a saved or request-supplied snapshot.
/// The owner must hold its lock/transaction until mutation and evidence commit.
#[derive(Debug)]
pub struct HostAuthorityView<'a> {
    pub principal: Option<&'a AuthenticatedPrincipal<'a>>,
    pub project: &'a Project,
    pub memberships: &'a [AdmittedMembership<'a>],
    pub review_grants: &'a [ReviewGrant<'a>],
    pub current_document: &'a DocumentRevision,
    /// Bump for any change of membership, identifier binding, review or session.
    pub generation: u64,
    pub now_unix: u64,
    pub limits: AuthoringLimits,
}

#[derive(Debug)]
pub struct AuthoringRequest<'a> {
    pub action: AuthoringAction,
    pub target: &'a DocumentRevision,
    pub expected_authority_generation: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthoringError {
    Unauthenticated,
    InvalidBinding,
    StaleAuthority,
    StaleDocument,
    NotMember,
    RoleDenied,
    ReviewDenied,
    SelfReview,
    WrongAction,
    InvalidSuccessor,
    BudgetExceeded,
    UnsupportedTime,
}

impl std::fmt::Display for AuthoringError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "project authoring denied: {self:?}")
    }
}

impl std::error::Error for AuthoringError {}
