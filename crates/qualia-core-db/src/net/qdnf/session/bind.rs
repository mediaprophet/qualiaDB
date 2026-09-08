//! Complete QSession bind (NET-05.01 partial).
//!
//! Application admission requires a fully bound context: target, route, service,
//! purpose, profile, policy, and a non-zero operation. Incomplete context cannot
//! admit application. A connection or path id is not a grant. Payment is not
//! evaluated here.

use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{OperationId, StrongDigest};

/// Sole session profile admitted by this bind: `qpr-pq-1`.
pub const PROFILE_QPR_PQ_1: u16 = 1;

/// Bound session application context. Policy is recorded; admission is separate.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionContext {
    pub operation: OperationId,
    pub target: StrongDigest,
    pub route: StrongDigest,
    pub service: StrongDigest,
    pub purpose: StrongDigest,
    pub profile: u16,
    pub policy: PolicyOutcome,
}

/// Bind a complete application context. Zero identifiers fail closed as Incomplete.
/// Unknown profile fails as Unauthorized. Non-Allow policy is stored, not rejected.
pub fn bind_complete(
    operation: OperationId,
    target: StrongDigest,
    route: StrongDigest,
    service: StrongDigest,
    purpose: StrongDigest,
    profile: u16,
    policy: PolicyOutcome,
) -> Result<SessionContext, QdnfError> {
    if operation == OperationId::ZERO {
        return Err(QdnfError::Incomplete);
    }
    if target == StrongDigest::ZERO
        || route == StrongDigest::ZERO
        || service == StrongDigest::ZERO
        || purpose == StrongDigest::ZERO
    {
        return Err(QdnfError::Incomplete);
    }
    if profile != PROFILE_QPR_PQ_1 {
        return Err(QdnfError::Unauthorized);
    }
    Ok(SessionContext {
        operation,
        target,
        route,
        service,
        purpose,
        profile,
        policy,
    })
}

/// Admit application on a bound context. Only Allow with a current grant succeeds.
pub fn admit_bound(ctx: &SessionContext, grant_current: bool) -> Result<(), QdnfError> {
    match ctx.policy {
        PolicyOutcome::Allow if grant_current => Ok(()),
        PolicyOutcome::Allow => Err(QdnfError::Unauthorized),
        PolicyOutcome::Deny => Err(QdnfError::Denied),
        PolicyOutcome::Challenge => Err(QdnfError::Challenge),
        PolicyOutcome::NeedsHuman => Err(QdnfError::NeedsHuman),
        PolicyOutcome::Incomplete => Err(QdnfError::Incomplete),
        PolicyOutcome::Error => Err(QdnfError::Unauthorized),
    }
}

/// A connection or path identifier is not a grant handle.
#[inline]
pub fn connection_id_is_grant() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d
    }

    fn operation(tag: u8) -> OperationId {
        let mut id = OperationId::ZERO;
        id.0[0] = tag;
        id
    }

    fn bind(
        policy: PolicyOutcome,
        profile: u16,
        target: StrongDigest,
    ) -> Result<SessionContext, QdnfError> {
        bind_complete(
            operation(1),
            target,
            digest(2),
            digest(3),
            digest(4),
            profile,
            policy,
        )
    }

    #[test]
    fn zero_target_is_incomplete() {
        assert_eq!(
            bind(PolicyOutcome::Allow, PROFILE_QPR_PQ_1, StrongDigest::ZERO),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn profile_zero_is_unauthorized() {
        assert_eq!(
            bind(PolicyOutcome::Allow, 0, digest(1)),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn allow_with_current_grant_admits() {
        let ctx = bind(PolicyOutcome::Allow, PROFILE_QPR_PQ_1, digest(1)).unwrap();
        assert!(admit_bound(&ctx, true).is_ok());
    }

    #[test]
    fn allow_without_current_grant_is_unauthorized() {
        let ctx = bind(PolicyOutcome::Allow, PROFILE_QPR_PQ_1, digest(1)).unwrap();
        assert_eq!(admit_bound(&ctx, false), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn deny_with_current_grant_is_denied() {
        let ctx = bind(PolicyOutcome::Deny, PROFILE_QPR_PQ_1, digest(1)).unwrap();
        assert_eq!(ctx.policy, PolicyOutcome::Deny);
        assert_eq!(admit_bound(&ctx, true), Err(QdnfError::Denied));
    }

    #[test]
    fn connection_id_is_not_a_grant() {
        assert!(!connection_id_is_grant());
    }

    #[test]
    fn needs_human_policy_admits_as_needs_human() {
        let ctx = bind(PolicyOutcome::NeedsHuman, PROFILE_QPR_PQ_1, digest(1)).unwrap();
        assert_eq!(admit_bound(&ctx, true), Err(QdnfError::NeedsHuman));
    }

    #[test]
    fn zero_operation_is_incomplete() {
        assert_eq!(
            bind_complete(
                OperationId::ZERO,
                digest(1),
                digest(2),
                digest(3),
                digest(4),
                PROFILE_QPR_PQ_1,
                PolicyOutcome::Allow,
            ),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn zero_route_service_or_purpose_is_incomplete() {
        assert_eq!(
            bind_complete(
                operation(1),
                digest(1),
                StrongDigest::ZERO,
                digest(3),
                digest(4),
                PROFILE_QPR_PQ_1,
                PolicyOutcome::Allow,
            ),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(
            bind_complete(
                operation(1),
                digest(1),
                digest(2),
                StrongDigest::ZERO,
                digest(4),
                PROFILE_QPR_PQ_1,
                PolicyOutcome::Allow,
            ),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(
            bind_complete(
                operation(1),
                digest(1),
                digest(2),
                digest(3),
                StrongDigest::ZERO,
                PROFILE_QPR_PQ_1,
                PolicyOutcome::Allow,
            ),
            Err(QdnfError::Incomplete)
        );
    }

    #[test]
    fn challenge_and_error_map_without_grant_override() {
        let challenge = bind(PolicyOutcome::Challenge, PROFILE_QPR_PQ_1, digest(1)).unwrap();
        assert_eq!(admit_bound(&challenge, true), Err(QdnfError::Challenge));
        let error = bind(PolicyOutcome::Error, PROFILE_QPR_PQ_1, digest(1)).unwrap();
        assert_eq!(admit_bound(&error, true), Err(QdnfError::Unauthorized));
        let incomplete = bind(PolicyOutcome::Incomplete, PROFILE_QPR_PQ_1, digest(1)).unwrap();
        assert_eq!(admit_bound(&incomplete, true), Err(QdnfError::Incomplete));
    }
}
