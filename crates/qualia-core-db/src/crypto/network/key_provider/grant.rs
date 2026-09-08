//! Purpose-scoped grants. A provider is not a signing oracle.

use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::types::{KeyEpoch, KeyPurpose};

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ProviderGrant {
    pub epoch: KeyEpoch,
    pub cell_epoch: u64,
    pub authority_generation: u64,
    pub deadline_unix: u64,
    pub revoked: bool,
}

impl ProviderGrant {
    pub fn authorize(
        &self,
        purpose: KeyPurpose,
        controller: u64,
        now_unix: u64,
        authority_generation: u64,
    ) -> Result<(), CryptoError> {
        if self.revoked {
            return Err(CryptoError::Revoked);
        }
        if self.epoch.purpose != purpose {
            return Err(CryptoError::Unauthorized);
        }
        if self.epoch.controller != controller {
            return Err(CryptoError::Unauthorized);
        }
        if now_unix >= self.deadline_unix {
            return Err(CryptoError::Expired);
        }
        if self.authority_generation != authority_generation {
            return Err(CryptoError::StaleGeneration);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrong_purpose_is_rejected() {
        let grant = ProviderGrant {
            epoch: KeyEpoch {
                purpose: KeyPurpose::RouteUpdate,
                epoch: 1,
                controller: 9,
            },
            cell_epoch: 1,
            authority_generation: 1,
            deadline_unix: 100,
            revoked: false,
        };
        assert_eq!(
            grant.authorize(KeyPurpose::ControllerSign, 9, 10, 1),
            Err(CryptoError::Unauthorized)
        );
    }
}
