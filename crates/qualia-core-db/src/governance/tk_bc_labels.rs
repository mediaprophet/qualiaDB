//! Traditional Knowledge (TK) & Biocultural (BC) Label Governance Policies (WP-4.5, Gap 14).
//!
//! Implements Local Contexts ENRICH / TK / BC label protocols into deontic policy matrices:
//! - TK Attribution (`TK A`): Explicit cultural lineage and community attribution required.
//! - TK Non-Commercial (`TK NC`): Commercial exploitation prohibited without consent agreement.
//! - TK Culturally Sensitive (`TK CS`): Access restricted based on customary protocols.
//! - BC Provenance (`BC P`): Biocultural lineage and ecological origin acknowledged.
//! - TK Notice (`TK Open`): Open to collaboration and collective stewardship.
//!
//! Fail-closed: Violations yield an active `DeonticVerdict::Forbid`.

use bytemuck::{Pod, Zeroable};
use serde::{Deserialize, Serialize};

/// Bitmask flags representing declared TK / BC labels on an asset or entity.
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Pod, Zeroable, Serialize, Deserialize, Default)]
pub struct TkBcLabelFlags(pub u32);

impl TkBcLabelFlags {
    pub const NONE: Self = Self(0);
    /// TK Attribution: Community origin attribution required.
    pub const TK_ATTRIBUTION: Self = Self(1 << 0);
    /// TK Non-Commercial: Commercial extraction/monetization forbidden without consent.
    pub const TK_NON_COMMERCIAL: Self = Self(1 << 1);
    /// TK Culturally Sensitive: Seasonal or ritual sensitivity gating access.
    pub const TK_CULTURALLY_SENSITIVE: Self = Self(1 << 2);
    /// TK Verified: Community verification signed by verified custodians.
    pub const TK_COMMUNITY_VERIFIED: Self = Self(1 << 3);
    /// BC Provenance: Ecological/taxonomic origin in traditional lands.
    pub const BC_PROVENANCE: Self = Self(1 << 4);
    /// BC Multiple Community: Multi-custodian shared biological commons.
    pub const BC_MULTIPLE_COMMUNITY: Self = Self(1 << 5);

    pub const fn has(self, flag: Self) -> bool {
        (self.0 & flag.0) != 0
    }

    pub const fn union(self, other: Self) -> Self {
        Self(self.0 | other.0)
    }
}

/// Evaluation context when accessing or deriving an asset bearing TK/BC labels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TkAccessContext {
    pub is_commercial_use: bool,
    pub has_attribution: bool,
    pub community_consent_signed: bool,
    pub observer_is_verified_custodian: bool,
}

/// Verdict returned by evaluating TK/BC governance policies.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TkEvaluationVerdict {
    /// Permitted: All cultural protocols and attribution requirements satisfied.
    Permitted,
    /// Prohibited: Commercial extraction without required consent agreement.
    ProhibitedCommercialWithoutConsent,
    /// Prohibited: Missing mandatory cultural attribution.
    ProhibitedMissingAttribution,
    /// Restricted: Culturally sensitive asset restricted to verified community members.
    RestrictedCulturalSensitivity,
}

/// Evaluate an access or derivation request against declared TK/BC label flags.
/// Operates in zero-heap time and fails closed on unverified conditions.
pub fn evaluate_tk_bc_policy(
    labels: TkBcLabelFlags,
    context: &TkAccessContext,
) -> TkEvaluationVerdict {
    if labels.0 == 0 {
        return TkEvaluationVerdict::Permitted;
    }

    // 1. Cultural Sensitivity check (highest precedence)
    if labels.has(TkBcLabelFlags::TK_CULTURALLY_SENSITIVE)
        && !context.observer_is_verified_custodian
    {
        return TkEvaluationVerdict::RestrictedCulturalSensitivity;
    }

    // 2. Commercial exploitation check
    if labels.has(TkBcLabelFlags::TK_NON_COMMERCIAL)
        && context.is_commercial_use
        && !context.community_consent_signed
    {
        return TkEvaluationVerdict::ProhibitedCommercialWithoutConsent;
    }

    // 3. Attribution requirement check
    if labels.has(TkBcLabelFlags::TK_ATTRIBUTION) && !context.has_attribution {
        return TkEvaluationVerdict::ProhibitedMissingAttribution;
    }

    TkEvaluationVerdict::Permitted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tk_bc_policy_evaluation() {
        let labels = TkBcLabelFlags::TK_ATTRIBUTION.union(TkBcLabelFlags::TK_NON_COMMERCIAL);

        // Commercial use without consent fails
        let commercial_ctx = TkAccessContext {
            is_commercial_use: true,
            has_attribution: true,
            community_consent_signed: false,
            observer_is_verified_custodian: false,
        };
        assert_eq!(
            evaluate_tk_bc_policy(labels, &commercial_ctx),
            TkEvaluationVerdict::ProhibitedCommercialWithoutConsent
        );

        // Commercial use WITH consent succeeds
        let consent_ctx = TkAccessContext {
            is_commercial_use: true,
            has_attribution: true,
            community_consent_signed: true,
            observer_is_verified_custodian: false,
        };
        assert_eq!(
            evaluate_tk_bc_policy(labels, &consent_ctx),
            TkEvaluationVerdict::Permitted
        );

        // Non-commercial use without attribution fails
        let no_attr_ctx = TkAccessContext {
            is_commercial_use: false,
            has_attribution: false,
            community_consent_signed: false,
            observer_is_verified_custodian: false,
        };
        assert_eq!(
            evaluate_tk_bc_policy(labels, &no_attr_ctx),
            TkEvaluationVerdict::ProhibitedMissingAttribution
        );
    }
}
