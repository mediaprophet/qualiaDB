//! Funded network roles (E18.1). Zero funding source is Malformed. No mandatory wallet.

use crate::net::peer::runtime::ResourceBudget;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Activated service role. Scope is the variant; budget is admitted separately.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FundedRole {
    Routing = 1,
    Relay = 2,
    Resolution = 3,
    Custody = 4,
    Storage = 5,
    Compute = 6,
}

/// Admitted funded role. Funding source is a SHA-384 digest, never a wallet possession proof.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RoleGrant {
    pub role: FundedRole,
    pub budget: ResourceBudget,
    pub funding_source: StrongDigest,
}

/// Activate a role with an explicit budget and accepted funding source.
pub fn activate_role(
    role: FundedRole,
    budget: ResourceBudget,
    funding_source: StrongDigest,
) -> Result<RoleGrant, QdnfError> {
    if funding_source.is_zero() {
        return Err(QdnfError::Malformed);
    }
    Ok(RoleGrant {
        role,
        budget,
        funding_source,
    })
}

/// Natural persons are not required to hold a wallet to receive service.
#[inline]
pub fn natural_person_wallet_required() -> bool {
    false
}

/// Routing turn-on requires a routing grant, a funding source, and remaining budget.
pub fn role_admits_router(grant: &RoleGrant) -> Result<(), QdnfError> {
    if grant.role != FundedRole::Routing {
        return Err(QdnfError::Denied);
    }
    if grant.funding_source.is_zero() {
        return Err(QdnfError::Malformed);
    }
    if grant.budget.bytes == 0 && grant.budget.work == 0 && grant.budget.io == 0 {
        return Err(QdnfError::BudgetExhausted);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::digest::sha384;

    #[test]
    fn zero_funding_source_is_malformed() {
        let budget = ResourceBudget {
            bytes: 64,
            work: 1,
            io: 1,
        };
        assert_eq!(
            activate_role(FundedRole::Routing, budget, StrongDigest::ZERO),
            Err(QdnfError::Malformed)
        );
        let grant = activate_role(FundedRole::Relay, budget, sha384(b"commons-fund")).unwrap();
        assert_eq!(grant.role, FundedRole::Relay);
        assert_eq!(grant.budget.bytes, 64);
        assert_ne!(grant.funding_source, StrongDigest::ZERO);
        assert!(!natural_person_wallet_required());
    }

    #[test]
    fn all_roles_activate_with_funding() {
        let src = sha384(b"fund-1");
        let budget = ResourceBudget::ZERO;
        let roles = [
            FundedRole::Routing,
            FundedRole::Relay,
            FundedRole::Resolution,
            FundedRole::Custody,
            FundedRole::Storage,
            FundedRole::Compute,
        ];
        let mut i = 0;
        while i < roles.len() {
            assert!(activate_role(roles[i], budget, src).is_ok());
            i += 1;
        }
    }

    #[test]
    fn routing_grant_requires_budget_and_role() {
        let src = sha384(b"fund-2");
        let budget = ResourceBudget {
            bytes: 8,
            work: 1,
            io: 1,
        };
        let routing = activate_role(FundedRole::Routing, budget, src).unwrap();
        assert!(role_admits_router(&routing).is_ok());
        let relay = activate_role(FundedRole::Relay, budget, src).unwrap();
        assert_eq!(role_admits_router(&relay), Err(QdnfError::Denied));
        let empty = activate_role(FundedRole::Routing, ResourceBudget::ZERO, src).unwrap();
        assert_eq!(role_admits_router(&empty), Err(QdnfError::BudgetExhausted));
    }
}
