//! Circumstance tuple (role, audience, quorum, environment, evaluatory).
//!
//! Partial host surface: serializable descriptor for design + session hooks.
//! Full path-steering (job vs private, cafe vs Sanctuary) is host composition.

use serde::{Deserialize, Serialize};

/// Coarse place / environment type for presentation steering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum EnvironmentKind {
    #[default]
    Unspecified = 0,
    PrivateSanctuary = 1,
    Workplace = 2,
    PublicCafe = 3,
    ClinicalCare = 4,
    Education = 5,
    FieldMobile = 6,
}

/// Evaluatory priority axis (what the principal is optimising for now).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
#[repr(u8)]
pub enum EvaluatoryFocus {
    #[default]
    Open = 0,
    CareSafety = 1,
    WorkDelivery = 2,
    Learning = 3,
    SocialCommons = 4,
    LegalProcess = 5,
}

/// Spatio-social-temporal circumstance for one attention session.
///
/// Pure data - does not enforce rights (see `rights_filter` + deontic).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct Circumstance {
    /// Social / organisational role label (employee, parent, patient) - free text cold path.
    pub role: String,
    /// Audience class (self, peer_dyad, team, public, court).
    pub audience: String,
    /// Required quorum of parties (1 = solo; 2+ multi-party consent).
    pub quorum: u8,
    pub environment: EnvironmentKind,
    pub evaluatory: EvaluatoryFocus,
    /// Optional place/label hash carrier (0 = unset); hosts may set from geo/library place.
    pub place_hint: u64,
}

impl Circumstance {
    pub fn private_sanctuary() -> Self {
        Self {
            role: "principal".into(),
            audience: "self".into(),
            quorum: 1,
            environment: EnvironmentKind::PrivateSanctuary,
            evaluatory: EvaluatoryFocus::Open,
            place_hint: 0,
        }
    }

    pub fn workplace_employee() -> Self {
        Self {
            role: "employee".into(),
            audience: "employer_org".into(),
            quorum: 1,
            environment: EnvironmentKind::Workplace,
            evaluatory: EvaluatoryFocus::WorkDelivery,
            place_hint: 0,
        }
    }

    /// Whether this circumstance should bias representation away from private wing UI chrome.
    pub fn prefers_reduced_private_chrome(&self) -> bool {
        matches!(
            self.environment,
            EnvironmentKind::Workplace | EnvironmentKind::PublicCafe
        ) || self.audience == "employer_org"
            || self.audience == "public"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workplace_prefers_reduced_private() {
        let c = Circumstance::workplace_employee();
        assert!(c.prefers_reduced_private_chrome());
        assert!(!Circumstance::private_sanctuary().prefers_reduced_private_chrome());
    }
}
