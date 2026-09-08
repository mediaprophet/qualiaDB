//! NaturalAgent is not inferred from a verified route or instrument.

use crate::net::qdnf::errors::QdnfError;

/// Network-facing referent. Distinct from identifier-fabric planes.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NetworkReferent {
    NaturalAgent = 1,
    VerifiedRoute = 2,
    Instrument = 3,
}

/// A verified route or instrument cannot be treated as a NaturalAgent.
#[inline]
pub fn as_natural_agent(referent: NetworkReferent) -> Result<(), QdnfError> {
    match referent {
        NetworkReferent::NaturalAgent => Ok(()),
        NetworkReferent::VerifiedRoute | NetworkReferent::Instrument => {
            Err(QdnfError::Unauthorized)
        }
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
    fn explicit_natural_agent_is_not_rewritten() {
        assert!(as_natural_agent(NetworkReferent::NaturalAgent).is_ok());
    }
}
