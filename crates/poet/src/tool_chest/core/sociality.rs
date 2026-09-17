//! Sociality of a manifold — one observer, or many people.
//!
//! A construct remains personal (this principal’s mindware on their hardware).
//! A **manifold** may be social: several people participate in the same lens.
//! Projects are the primary case (shared, time-bound delivery).

use serde::{Deserialize, Serialize};

/// Whether this lens is solitary or multi-person.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ManifoldSociality {
    /// One observer. Health, anatomy, settings, sanctuary.
    #[default]
    Personal,
    /// Many people. Projects, social graph, communications.
    Social,
}

impl ManifoldSociality {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Personal => "personal",
            Self::Social => "social",
        }
    }

    pub fn is_social(self) -> bool {
        matches!(self, Self::Social)
    }
}

/// A person or agent on a social manifold. Natural persons are not owl:Thing.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManifoldParticipant {
    /// DID of the participant (principal or agent).
    pub did: String,
    /// Display name. Empty until bound.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub label: String,
    /// Role on this lens — `observer`, `member`, `steward`.
    #[serde(default)]
    pub role: String,
}

impl ManifoldParticipant {
    pub fn new(did: impl Into<String>, role: impl Into<String>) -> Self {
        Self {
            did: did.into(),
            label: String::new(),
            role: role.into(),
        }
    }
}

/// Bundled manifold ids that are social even if an older saved seed omitted the field.
pub fn bundled_social_manifold(id: &str) -> bool {
    matches!(id, "projects" | "social" | "communications")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projects_are_social_health_is_not() {
        assert!(bundled_social_manifold("projects"));
        assert!(bundled_social_manifold("social"));
        assert!(!bundled_social_manifold("health"));
        assert!(!bundled_social_manifold("anatomy"));
        assert_eq!(ManifoldSociality::default(), ManifoldSociality::Personal);
    }
}
