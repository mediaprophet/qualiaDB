//! NaturalAgent is not inferred from a verified route, instrument, AI agent,
//! machine device, organization, or role capacity (IDF-02).

use crate::net::qdnf::errors::QdnfError;

/// Network-facing referent. Distinct from identifier-fabric planes.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkReferent {
    NaturalAgent = 1,
    VerifiedRoute = 2,
    Instrument = 3,
    AiAgent = 4,
    MachineDevice = 5,
    Organization = 6,
    RoleCapacity = 7,
}

impl NetworkReferent {
    pub const ALL: [Self; 7] = [
        Self::NaturalAgent,
        Self::VerifiedRoute,
        Self::Instrument,
        Self::AiAgent,
        Self::MachineDevice,
        Self::Organization,
        Self::RoleCapacity,
    ];
}

/// A verified route, instrument, AI agent, device, organization, or role capacity
/// cannot be treated as a NaturalAgent.
#[inline]
pub fn as_natural_agent(referent: NetworkReferent) -> Result<(), QdnfError> {
    match referent {
        NetworkReferent::NaturalAgent => Ok(()),
        NetworkReferent::VerifiedRoute
        | NetworkReferent::Instrument
        | NetworkReferent::AiAgent
        | NetworkReferent::MachineDevice
        | NetworkReferent::Organization
        | NetworkReferent::RoleCapacity => Err(QdnfError::Unauthorized),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verified_route_is_not_a_natural_agent() {
        assert_eq!(
            as_natural_agent(NetworkReferent::VerifiedRoute),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn instrument_is_not_a_natural_agent() {
        assert_eq!(
            as_natural_agent(NetworkReferent::Instrument),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn ai_agent_device_org_and_role_are_not_natural_agents() {
        for referent in [
            NetworkReferent::AiAgent,
            NetworkReferent::MachineDevice,
            NetworkReferent::Organization,
            NetworkReferent::RoleCapacity,
        ] {
            assert_eq!(as_natural_agent(referent), Err(QdnfError::Unauthorized));
        }
    }

    #[test]
    fn explicit_natural_agent_is_not_rewritten() {
        assert!(as_natural_agent(NetworkReferent::NaturalAgent).is_ok());
    }
}
