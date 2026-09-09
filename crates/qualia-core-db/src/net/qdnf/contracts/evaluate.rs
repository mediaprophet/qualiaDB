//! Separate contract evaluator. Calls the public compile_decision API only.
//!
//! Does not pass signed/supported booleans into compile internals.

use super::compile::{compile_decision, ContractBundle};
use super::generations::{recheck_permit, BoundGenerations, LiveGenerations};
use super::identity::{
    consume_authority, refuse_alias_merge, same_as_transfers_authority, ReferentKind,
};
use super::provenance::{admit_claim, contradicting_claims, ClaimRecord};
use crate::net::qdnf::authority::{PolicyOutcome, TemporalGrant};
use crate::net::qdnf::errors::QdnfError;

/// Inputs for one evaluation. Digests and generations, not booleans.
#[derive(Clone, Copy, Debug)]
pub struct EvaluationView<'a> {
    pub bundle: &'a ContractBundle,
    pub expected: &'a ContractBundle,
    pub original_bytes: &'a [u8],
    pub bound: BoundGenerations,
    pub live: LiveGenerations,
    pub referent: ReferentKind,
    pub claims: &'a [ClaimRecord],
    pub delegation: Option<&'a TemporalGrant>,
    pub now_unix: u64,
}

/// Evaluate a pinned contract at a consumption boundary.
///
/// Order: referent authority, alias/person merge guard, claim provenance,
/// generation recheck, delegation freshness, then [`compile_decision`].
pub fn evaluate(view: &EvaluationView<'_>) -> Result<PolicyOutcome, QdnfError> {
    consume_authority(view.referent)?;
    if same_as_transfers_authority() {
        return Err(QdnfError::Unauthorized);
    }
    let mut i = 0usize;
    while i < view.claims.len() {
        admit_claim(&view.claims[i])?;
        if i > 0 {
            contradicting_claims(&view.claims[0], &view.claims[i])?;
        }
        i += 1;
    }
    recheck_permit(view.bound, view.live)?;
    if let Some(grant) = view.delegation {
        grant.current_at(view.now_unix)?;
    }
    compile_decision(view.bundle, view.expected, view.original_bytes)
}

/// Alias collision is never a person merge.
#[inline]
pub fn evaluate_alias(
    left: ReferentKind,
    right: ReferentKind,
    left_id: crate::net::qdnf::types::StrongDigest,
    right_id: crate::net::qdnf::types::StrongDigest,
) -> Result<(), QdnfError> {
    refuse_alias_merge(left, right, left_id, right_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;
    use crate::net::qdnf::types::{Generation, ProfileId, StrongDigest};

    const BYTES: &[u8] = b"qdnf-eval-artifact-v1";

    fn vocab() -> StrongDigest {
        StrongDigest([0x31; 48])
    }

    fn pinned() -> ContractBundle {
        ContractBundle {
            exact_bytes_digest: sha384(BYTES),
            context_digest: StrongDigest([0xB1; 48]),
            ontology_digest: StrongDigest([0xB2; 48]),
            shape_digest: StrongDigest([0xB3; 48]),
            rules_digest: StrongDigest([0xB4; 48]),
            required_vocab_digest: vocab(),
            known_vocab_digest: vocab(),
        }
    }

    fn gens() -> BoundGenerations {
        BoundGenerations::new(Generation(1), Generation(1), Generation(1))
    }

    fn view<'a>(
        bundle: &'a ContractBundle,
        expected: &'a ContractBundle,
        claims: &'a [ClaimRecord],
        delegation: Option<&'a TemporalGrant>,
        now: u64,
    ) -> EvaluationView<'a> {
        EvaluationView {
            bundle,
            expected,
            original_bytes: BYTES,
            bound: gens(),
            live: gens().as_live(),
            referent: ReferentKind::Entity,
            claims,
            delegation,
            now_unix: now,
        }
    }

    fn grant_until(expires: u64) -> TemporalGrant {
        TemporalGrant {
            purpose_digest: StrongDigest([0x51; 48]),
            audience_digest: StrongDigest([0x52; 48]),
            authority_generation: 1,
            not_before_unix: 0,
            expires_unix: expires,
            profile: ProfileId::QDNF_CRYPTO_1,
        }
    }

    #[test]
    fn matching_view_allows() {
        let bundle = pinned();
        let v = view(&bundle, &bundle, &[], None, 1);
        assert_eq!(evaluate(&v).unwrap(), PolicyOutcome::Allow);
    }

    #[test]
    fn wrong_context_digest_is_conflict() {
        let bundle = pinned();
        let mut expected = bundle;
        expected.context_digest = StrongDigest([0x99; 48]);
        let v = view(&bundle, &expected, &[], None, 1);
        assert_eq!(evaluate(&v), Err(QdnfError::Conflict));
    }

    #[test]
    fn ontology_digest_change_invalidates() {
        let bundle = pinned();
        let mut expected = bundle;
        expected.ontology_digest = StrongDigest([0x77; 48]);
        let v = view(&bundle, &expected, &[], None, 1);
        assert_eq!(evaluate(&v), Err(QdnfError::Conflict));
        assert_ne!(evaluate(&v), Ok(PolicyOutcome::Allow));
    }

    #[test]
    fn expired_delegation_is_not_allow() {
        let bundle = pinned();
        let grant = grant_until(10);
        let v = view(&bundle, &bundle, &[], Some(&grant), 10);
        assert_eq!(evaluate(&v), Err(QdnfError::Expired));
    }

    #[test]
    fn current_delegation_still_requires_pinned_bytes() {
        let bundle = pinned();
        let grant = grant_until(10);
        let v = view(&bundle, &bundle, &[], Some(&grant), 9);
        assert_eq!(evaluate(&v).unwrap(), PolicyOutcome::Allow);
    }

    #[test]
    fn alias_collision_does_not_merge_persons() {
        assert_eq!(
            evaluate_alias(
                ReferentKind::Handle,
                ReferentKind::NaturalPerson,
                StrongDigest([1u8; 48]),
                StrongDigest([2u8; 48]),
            ),
            Err(QdnfError::Unauthorized)
        );
        assert!(!same_as_transfers_authority());
    }

    #[test]
    fn contradictory_claims_are_conflict() {
        let bundle = pinned();
        let claims = [
            ClaimRecord {
                issuer: StrongDigest([1u8; 48]),
                scope: 4,
                disagreement: false,
                confidence: 200,
            },
            ClaimRecord {
                issuer: StrongDigest([2u8; 48]),
                scope: 4,
                disagreement: false,
                confidence: 200,
            },
        ];
        let v = view(&bundle, &bundle, &claims, None, 1);
        assert_eq!(evaluate(&v), Err(QdnfError::Conflict));
    }

    #[test]
    fn flagged_disagreement_is_conflict() {
        let bundle = pinned();
        let mut rec = ClaimRecord {
            issuer: StrongDigest([3u8; 48]),
            scope: 8,
            disagreement: true,
            confidence: 1,
        };
        rec.disagreement = true;
        let claims = [rec];
        let v = view(&bundle, &bundle, &claims, None, 1);
        assert_eq!(evaluate(&v), Err(QdnfError::Conflict));
    }

    #[test]
    fn handle_referent_cannot_evaluate_as_authority() {
        let bundle = pinned();
        let mut v = view(&bundle, &bundle, &[], None, 1);
        v.referent = ReferentKind::Handle;
        assert_eq!(evaluate(&v), Err(QdnfError::Unauthorized));
        v.referent = ReferentKind::Instrument;
        assert_eq!(evaluate(&v), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn stale_generation_at_evaluate_is_not_allow() {
        let bundle = pinned();
        let mut v = view(&bundle, &bundle, &[], None, 1);
        v.live.policy = Generation(9);
        assert_eq!(evaluate(&v), Err(QdnfError::StaleGeneration));
    }
}
