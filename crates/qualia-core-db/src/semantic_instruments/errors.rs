//! Cold-path errors for semantic-instrument collectables.

use crate::bundle::BundleError;

/// Fail-closed errors for manifest, canonical encoding and HMC packaging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InstrumentError {
    MissingField(&'static str),
    InvalidCategory,
    DemoUnlabelled,
    PathTraversal,
    EmptyMember(&'static str),
    EntryTooLarge { key: String, bytes: usize },
    BundleTooLarge { bytes: usize },
    TooManyEntries { count: usize },
    DigestMismatch,
    Bundle(String),
    Canonical(String),
    Graph(String),
    InvalidAttestationKind,
    SignatureAsTruth,
    AwardIsInstrument,
    RoleCollision,
    Revoked,
    Expired,
    Tampered,
    MissingProof,
    RequiredMissing,
    DigestMismatchDep,
    DependencyCycle,
    RevokedDependency,
    OfflineHeld,
    NotClosed,
    UnknownEntryPoint,
    MissingInput,
    ResourceLimit,
    Cancelled,
    AlreadyPublished,
    CatalogueIsNotEndorsement,
    IncompleteDownload,
    UnauthorisedEquivalence,
    EvaluatorPassIsNotIssuance,
    PrivateEvidence,
}

impl std::fmt::Display for InstrumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::MissingField(name) => write!(f, "instrument: missing {name}"),
            Self::InvalidCategory => write!(f, "instrument: content category is not Demo/Reference/Operational"),
            Self::DemoUnlabelled => {
                write!(f, "instrument: Demo category requires an explicit demonstration/development notice")
            }
            Self::PathTraversal => write!(f, "instrument: member key is not a relative safe path"),
            Self::EmptyMember(key) => write!(f, "instrument: empty member {key}"),
            Self::EntryTooLarge { key, bytes } => {
                write!(f, "instrument: entry {key} is {bytes} bytes (over budget)")
            }
            Self::BundleTooLarge { bytes } => {
                write!(f, "instrument: collectable is {bytes} bytes (over budget)")
            }
            Self::TooManyEntries { count } => {
                write!(f, "instrument: {count} members exceeds the collectable cap")
            }
            Self::DigestMismatch => write!(f, "instrument: content digest does not match canonical bytes"),
            Self::Bundle(msg) => write!(f, "instrument bundle: {msg}"),
            Self::Canonical(msg) => write!(f, "instrument canonical: {msg}"),
            Self::Graph(msg) => write!(f, "instrument graph: {msg}"),
            Self::InvalidAttestationKind => write!(f, "instrument: unknown attestation kind"),
            Self::SignatureAsTruth => {
                write!(f, "instrument: a signature is origin, not substantive truth")
            }
            Self::AwardIsInstrument => {
                write!(f, "instrument: capability award subject must not be the instrument")
            }
            Self::RoleCollision => {
                write!(f, "instrument: authorship/review/publication/issuance must stay distinct")
            }
            Self::Revoked => write!(f, "instrument: attestation is revoked for future reliance"),
            Self::Expired => write!(f, "instrument: attestation expired"),
            Self::Tampered => write!(f, "instrument: attestation tampered"),
            Self::MissingProof => write!(f, "instrument: attestation missing proof"),
            Self::RequiredMissing => write!(f, "instrument: required dependency missing"),
            Self::DigestMismatchDep => write!(f, "instrument: dependency digest mismatch"),
            Self::DependencyCycle => write!(f, "instrument: dependency cycle"),
            Self::RevokedDependency => write!(f, "instrument: required dependency revoked"),
            Self::OfflineHeld => write!(f, "instrument: offline; required dependency unresolved"),
            Self::NotClosed => write!(f, "instrument: cannot activate before verified closure"),
            Self::UnknownEntryPoint => write!(f, "instrument: unknown entry point (not a Host ID)"),
            Self::MissingInput => write!(f, "instrument: required input missing (held)"),
            Self::ResourceLimit => write!(f, "instrument: resource limit reached"),
            Self::Cancelled => write!(f, "instrument: execution cancelled"),
            Self::AlreadyPublished => {
                write!(f, "instrument: same release cannot accept different bytes")
            }
            Self::CatalogueIsNotEndorsement => {
                write!(f, "instrument: catalogue listing is not endorsement")
            }
            Self::IncompleteDownload => {
                write!(f, "instrument: interrupted download cannot register as resolved")
            }
            Self::UnauthorisedEquivalence => {
                write!(f, "instrument: unauthorised capability equivalence")
            }
            Self::EvaluatorPassIsNotIssuance => {
                write!(f, "instrument: a runner pass is not an award issuance")
            }
            Self::PrivateEvidence => {
                write!(f, "instrument: private evidence must not enter a public award")
            }
        }
    }
}

impl std::error::Error for InstrumentError {}

impl From<BundleError> for InstrumentError {
    fn from(value: BundleError) -> Self {
        Self::Bundle(value.to_string())
    }
}
