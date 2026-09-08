//! Software key-provider backend. Hardware backends return capability errors.

use crate::crypto::network::ed25519;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::types::KeyPurpose;
use crate::crypto::network::x25519::X25519Secret;

use super::grant::ProviderGrant;

pub struct SoftwareBackend;

impl SoftwareBackend {
    pub fn sign_ed25519(
        grant: &ProviderGrant,
        seed: &[u8; 32],
        message: &[u8],
        context: &[u8],
        now_unix: u64,
        controller: u64,
        authority_generation: u64,
        sig_out: &mut [u8; 64],
    ) -> Result<(), CryptoError> {
        grant.authorize(
            KeyPurpose::ControllerSign,
            controller,
            now_unix,
            authority_generation,
        )?;
        ed25519::sign_with_context(seed, message, context, sig_out)
    }

    pub fn x25519_public(secret: &[u8; 32]) -> [u8; 32] {
        X25519Secret::from_bytes(*secret).public()
    }

    pub fn hardware_sign() -> Result<(), CryptoError> {
        super::hardware::HardwareBackend::sign()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hardware_sign_is_platform_unsupported() {
        assert_eq!(
            SoftwareBackend::hardware_sign(),
            Err(CryptoError::PlatformUnsupported)
        );
    }
}
