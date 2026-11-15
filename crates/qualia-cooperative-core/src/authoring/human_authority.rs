//! Attribution evidence is deliberately separate from a reusable permission.
use super::PrincipalKind;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HumanAuthorityStatus {
    /// Project admission and controller possession establish the acting actor;
    /// the native human-root/adopted-chain adapter has not admitted this path.
    UnverifiedLegacyAdmission,
}

/// A projection of verified owner state, never an authentication input.
/// A Human capacity tag alone does not populate the human principal field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HumanAuthorityEvidence {
    pub acting_subject: String,
    pub acting_kind: PrincipalKind,
    pub status: HumanAuthorityStatus,
    pub human_principal_subject: Option<String>,
    pub delegation_path: Vec<[u8; 32]>,
}

impl HumanAuthorityEvidence {
    pub fn legacy(acting_subject: &str, acting_kind: PrincipalKind) -> Self {
        Self {
            acting_subject: acting_subject.into(),
            acting_kind,
            status: HumanAuthorityStatus::UnverifiedLegacyAdmission,
            human_principal_subject: None,
            delegation_path: Vec::new(),
        }
    }
}
