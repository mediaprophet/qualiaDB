//! Integrated handshake failure gate: entropy, 0-RTT/downgrade, and complete-path
//! classification via `crypto::network::malformed` (not duplicated here).

use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::pq_handshake::application_data_allowed;

use super::HandshakeState;

/// Qualified profile: 0-RTT application data is never admitted.
pub fn admit_early_application_data() -> Result<(), CryptoError> {
    debug_assert!(super::FORBIDDEN_ZERO_RTT);
    Err(CryptoError::Downgrade)
}

/// Application data is admitted only after the handshake reaches `Traffic`.
pub fn qualified_application_data(state: HandshakeState) -> Result<(), CryptoError> {
    if application_data_allowed(state) {
        Ok(())
    } else {
        Err(CryptoError::Downgrade)
    }
}

/// Production application-data gate. Same outcome as [`qualified_application_data`].
#[inline]
pub fn qualified_handshake_gate(state: HandshakeState) -> Result<(), CryptoError> {
    qualified_application_data(state)
}

/// Fill 32 bytes from the OS CSPRNG. All-zero output or getrandom failure is
/// [`CryptoError::EntropyFailure`]. Never panics.
pub fn entropy_fill(out: &mut [u8; 32]) -> Result<(), CryptoError> {
    match getrandom::fill(out.as_mut_slice()) {
        Ok(()) => {
            if out.iter().all(|b| *b == 0) {
                *out = [0u8; 32];
                Err(CryptoError::EntropyFailure)
            } else {
                Ok(())
            }
        }
        Err(_) => {
            *out = [0u8; 32];
            Err(CryptoError::EntropyFailure)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crypto::network::kem::MlKem768Secret;
    use crate::crypto::network::malformed::reject_all_zero_dh;
    use crate::net::qdnf::crypto::handshake::{initiator_share, responder_complete};

    #[test]
    fn admit_early_application_data_is_downgrade() {
        assert_eq!(admit_early_application_data(), Err(CryptoError::Downgrade));
    }

    #[test]
    fn all_zero_dh_already_rejected() {
        let (_sk, pk) = MlKem768Secret::generate().unwrap();
        let i_x = [11u8; 32];
        let mut ishare = initiator_share(&i_x, &pk).unwrap();
        ishare.x25519_pk = [0u8; 32];
        assert_eq!(
            reject_all_zero_dh(&ishare.x25519_pk),
            Err(CryptoError::CryptoFailure)
        );
        match responder_complete(&ishare, &[12u8; 32]) {
            Err(e) => assert_eq!(e, CryptoError::CryptoFailure),
            Ok(_) => panic!("all-zero peer DH public was accepted"),
        }
        assert!(initiator_share(&[0u8; 32], &pk).is_err());
    }

    #[test]
    fn qualified_application_data_idle_is_downgrade_traffic_is_ok() {
        assert_eq!(
            qualified_application_data(HandshakeState::Idle),
            Err(CryptoError::Downgrade)
        );
        assert_eq!(
            qualified_handshake_gate(HandshakeState::Idle),
            Err(CryptoError::Downgrade)
        );
        assert_eq!(qualified_application_data(HandshakeState::Traffic), Ok(()));
        assert_eq!(qualified_handshake_gate(HandshakeState::Traffic), Ok(()));
        assert_eq!(
            qualified_application_data(HandshakeState::Finished),
            Err(CryptoError::Downgrade)
        );
    }

    #[test]
    fn entropy_fill_produces_non_all_zero_or_entropy_failure() {
        let mut out = [0u8; 32];
        match entropy_fill(&mut out) {
            Ok(()) => {
                assert!(out.iter().any(|b| *b != 0));
            }
            Err(e) => {
                assert_eq!(e, CryptoError::EntropyFailure);
            }
        }
    }
}
