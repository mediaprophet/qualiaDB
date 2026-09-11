//! Source / policy / identity generations bound into compiled decisions.
//!
//! A live generation past the bound is StaleGeneration, never Allow.

use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

/// Generations captured when the decision was compiled.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BoundGenerations {
    pub source: Generation,
    pub policy: Generation,
    pub identity: Generation,
}

/// Live generations at commit / release.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LiveGenerations {
    pub source: Generation,
    pub policy: Generation,
    pub identity: Generation,
}

impl BoundGenerations {
    pub const fn new(source: Generation, policy: Generation, identity: Generation) -> Self {
        Self {
            source,
            policy,
            identity,
        }
    }

    #[inline]
    pub const fn as_live(self) -> LiveGenerations {
        LiveGenerations {
            source: self.source,
            policy: self.policy,
            identity: self.identity,
        }
    }
}

/// Recheck a queued compiled permit at commit/release.
///
/// Any live generation greater than the bound is [`QdnfError::StaleGeneration`].
/// Zero bound generations are not a compiled permit.
pub fn recheck_permit(bound: BoundGenerations, live: LiveGenerations) -> Result<(), QdnfError> {
    if bound.source == Generation::ZERO
        || bound.policy == Generation::ZERO
        || bound.identity == Generation::ZERO
    {
        return Err(QdnfError::Unauthorized);
    }
    if live.source == Generation::ZERO
        || live.policy == Generation::ZERO
        || live.identity == Generation::ZERO
    {
        return Err(QdnfError::Unauthorized);
    }
    if live.source.0 > bound.source.0
        || live.policy.0 > bound.policy.0
        || live.identity.0 > bound.identity.0
    {
        return Err(QdnfError::StaleGeneration);
    }
    Ok(())
}

/// Bind an already-compiled outcome at the commit boundary. Stale is not Allow.
pub fn commit_compiled(
    outcome: PolicyOutcome,
    bound: BoundGenerations,
    live: LiveGenerations,
) -> Result<PolicyOutcome, QdnfError> {
    recheck_permit(bound, live)?;
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bound() -> BoundGenerations {
        BoundGenerations::new(Generation(3), Generation(5), Generation(2))
    }

    #[test]
    fn matching_live_generations_pass() {
        let b = bound();
        assert!(recheck_permit(b, b.as_live()).is_ok());
        assert_eq!(
            commit_compiled(PolicyOutcome::Allow, b, b.as_live()).unwrap(),
            PolicyOutcome::Allow
        );
    }

    #[test]
    fn source_generation_advance_is_stale_not_allow() {
        let b = bound();
        let mut live = b.as_live();
        live.source = Generation(4);
        assert_eq!(recheck_permit(b, live), Err(QdnfError::StaleGeneration));
        assert_eq!(
            commit_compiled(PolicyOutcome::Allow, b, live),
            Err(QdnfError::StaleGeneration)
        );
    }

    #[test]
    fn policy_generation_advance_is_stale_not_allow() {
        let b = bound();
        let mut live = b.as_live();
        live.policy = Generation(9);
        assert_eq!(recheck_permit(b, live), Err(QdnfError::StaleGeneration));
    }

    #[test]
    fn identity_generation_advance_is_stale_not_allow() {
        let b = bound();
        let mut live = b.as_live();
        live.identity = Generation(3);
        assert_eq!(recheck_permit(b, live), Err(QdnfError::StaleGeneration));
    }

    #[test]
    fn zero_bound_is_unauthorized() {
        let live = LiveGenerations {
            source: Generation(1),
            policy: Generation(1),
            identity: Generation(1),
        };
        let bound = BoundGenerations::new(Generation::ZERO, Generation(1), Generation(1));
        assert_eq!(recheck_permit(bound, live), Err(QdnfError::Unauthorized));
    }
}
