//! F7 Relation-Scoped Locators (`NET-21`, `identifier-fabric-integration.md` §3).
//!
//! Pairwise directional addresses (such as `jane@bob.tld` <-> `bob@jane.tld`) and
//! service-bond locators bind direction, relationship context, purpose, audience,
//! and temporal lifecycle. Two directed mailboxes between the same counterparts
//! are two distinct instruments and never merge the counterparts' `NaturalAgent`
//! records. Proxy hops inherit purpose restrictions (`no-proxy-launder`, spine §3n).

use crate::net::qdnf::errors::QdnfError;

/// Publication and disclosure scope for a relation-scoped locator.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AudienceScope {
    PrivatePairwise = 1,
    ClosedGroup = 2,
    Community = 3,
    PublicService = 4,
}

impl AudienceScope {
    /// Private alias-to-person bindings and full counterpart graphs are never
    /// public `QSR` discovery records (`identifier-fabric-integration.md` §3).
    #[inline]
    pub const fn can_publish_to_public_qsr(self) -> bool {
        matches!(self, Self::PublicService)
    }
}

/// Zero-heap relation-scoped locator instrument (F2 `idf:RelationScopedLocatorShape`).
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationScopedLocator {
    pub locator_hash: u64,
    /// Directional hash (`A -> B` != `B -> A`).
    pub direction_hash: u64,
    pub relation_context_hash: u64,
    pub purpose_bind_hash: u64,
    pub audience_scope: AudienceScope,
    pub not_before: u64,
    pub expires_at: u64,
    pub withdrawn: bool,
}

impl RelationScopedLocator {
    /// Validate that this relation-scoped locator may be used for `requested_purpose`
    /// at `current_epoch`.
    ///
    /// - Withdrawn or out-of-window locators fail closed with [`QdnfError::Expired`].
    /// - Purpose mismatch fails closed with [`QdnfError::Denied`], even when
    ///   `is_proxy_hop == true` (enforcing spine §3n **no-proxy-launder**).
    pub fn validate_locator_use(
        &self,
        requested_purpose: u64,
        current_epoch: u64,
        is_proxy_hop: bool,
    ) -> Result<(), QdnfError> {
        if self.locator_hash == 0
            || self.direction_hash == 0
            || self.relation_context_hash == 0
            || self.purpose_bind_hash == 0
        {
            return Err(QdnfError::Malformed);
        }
        if self.withdrawn
            || self.not_before >= self.expires_at
            || current_epoch < self.not_before
            || current_epoch >= self.expires_at
        {
            return Err(QdnfError::Expired);
        }
        // Spine §3n: a proxy hop cannot launder or strip the upstream purpose binding.
        let _ = is_proxy_hop;
        if requested_purpose == 0 || requested_purpose != self.purpose_bind_hash {
            return Err(QdnfError::Denied);
        }
        Ok(())
    }

    /// Two directed locators (`A -> B` and `B -> A`) between counterparts are two
    /// distinct instruments. Attempting to merge counterparts or infer a single
    /// `NaturalAgent` from pairwise locators always fails closed.
    #[inline]
    pub const fn merge_counterparts(&self, _reverse: &Self) -> Result<(), QdnfError> {
        Err(QdnfError::Unauthorized)
    }

    /// Admit publication to a public QSR directory. Private pairwise and closed-group
    /// locators are refused (`QdnfError::Denied`).
    #[inline]
    pub const fn admit_public_qsr_publication(&self) -> Result<(), QdnfError> {
        if self.withdrawn {
            return Err(QdnfError::Expired);
        }
        if self.audience_scope.can_publish_to_public_qsr() {
            Ok(())
        } else {
            Err(QdnfError::Denied)
        }
    }
}

/// Convenience free-function wrapper for [`RelationScopedLocator::validate_locator_use`].
#[inline]
pub fn validate_locator_use(
    locator: &RelationScopedLocator,
    requested_purpose: u64,
    current_epoch: u64,
    is_proxy_hop: bool,
) -> Result<(), QdnfError> {
    locator.validate_locator_use(requested_purpose, current_epoch, is_proxy_hop)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::q_hash;

    fn sample_pair() -> (RelationScopedLocator, RelationScopedLocator) {
        let ctx = q_hash("urn:qualia:rel:jane-bob-2026");
        let purpose = q_hash("urn:qualia:purpose:clinical-consult");
        let jane_to_bob = RelationScopedLocator {
            locator_hash: q_hash("mailto:jane@bob.example"),
            direction_hash: q_hash("dir:jane->bob"),
            relation_context_hash: ctx,
            purpose_bind_hash: purpose,
            audience_scope: AudienceScope::PrivatePairwise,
            not_before: 1_000,
            expires_at: 2_000,
            withdrawn: false,
        };
        let bob_to_jane = RelationScopedLocator {
            locator_hash: q_hash("mailto:bob@jane.example"),
            direction_hash: q_hash("dir:bob->jane"),
            relation_context_hash: ctx,
            purpose_bind_hash: purpose,
            audience_scope: AudienceScope::PrivatePairwise,
            not_before: 1_000,
            expires_at: 2_000,
            withdrawn: false,
        };
        (jane_to_bob, bob_to_jane)
    }

    #[test]
    fn directed_mailboxes_are_distinct_and_never_merge_counterparts() {
        let (a_to_b, b_to_a) = sample_pair();
        assert_ne!(a_to_b.locator_hash, b_to_a.locator_hash);
        assert_ne!(a_to_b.direction_hash, b_to_a.direction_hash);
        assert_eq!(
            a_to_b.merge_counterparts(&b_to_a),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn expired_or_withdrawn_locator_fails_closed() {
        let (mut loc, _) = sample_pair();
        let purpose = loc.purpose_bind_hash;
        assert_eq!(validate_locator_use(&loc, purpose, 1_500, false), Ok(()));
        assert_eq!(
            validate_locator_use(&loc, purpose, 999, false),
            Err(QdnfError::Expired)
        );
        assert_eq!(
            validate_locator_use(&loc, purpose, 2_000, false),
            Err(QdnfError::Expired)
        );
        loc.withdrawn = true;
        assert_eq!(
            validate_locator_use(&loc, purpose, 1_500, false),
            Err(QdnfError::Expired)
        );
    }

    #[test]
    fn proxy_hop_cannot_launder_purpose_mismatch() {
        let (loc, _) = sample_pair();
        let wrong_purpose = q_hash("urn:qualia:purpose:ad-targeting");
        assert_eq!(
            validate_locator_use(&loc, wrong_purpose, 1_500, false),
            Err(QdnfError::Denied)
        );
        assert_eq!(
            validate_locator_use(&loc, wrong_purpose, 1_500, true),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn private_pairwise_locator_refuses_public_qsr_publication() {
        let (loc, _) = sample_pair();
        assert_eq!(loc.admit_public_qsr_publication(), Err(QdnfError::Denied));
    }
}
