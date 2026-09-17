//! Untrusted decoded claims. Setting fields never admits a service.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

use super::policy::PolicyOutcome;

/// Wire or caller-asserted claim. Not a credential and not a permit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DecodedClaim {
    pub asserted_principal: StrongDigest,
    pub asserted_recipient: StrongDigest,
    pub asserted_purpose: StrongDigest,
    pub asserted_outcome: PolicyOutcome,
    pub signed_flag: bool,
    pub supported_flag: bool,
}

impl DecodedClaim {
    pub const fn empty() -> Self {
        Self {
            asserted_principal: StrongDigest::ZERO,
            asserted_recipient: StrongDigest::ZERO,
            asserted_purpose: StrongDigest::ZERO,
            asserted_outcome: PolicyOutcome::Incomplete,
            signed_flag: false,
            supported_flag: false,
        }
    }

    /// Booleans and enum tags are not admission.
    pub fn admit_service(&self) -> Result<(), QdnfError> {
        let _ = (self.signed_flag, self.supported_flag, self.asserted_outcome);
        Err(QdnfError::Unauthorized)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_boolean_cannot_admit() {
        let mut claim = DecodedClaim::empty();
        claim.asserted_outcome = PolicyOutcome::Allow;
        claim.signed_flag = true;
        claim.supported_flag = true;
        assert_eq!(claim.admit_service(), Err(QdnfError::Unauthorized));
    }
}
