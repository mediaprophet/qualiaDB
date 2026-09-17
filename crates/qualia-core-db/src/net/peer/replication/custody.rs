//! Durable ciphertext custody states (E11.5).
//!
//! Length or digest without bytes is never [`CustodyState::Stored`].
//! [`CustodyState::Delivered`] is not [`CustodyState::ApplicationAcked`].

use crate::net::qdnf::errors::QdnfError;

/// Explicit custody lifecycle. Delivered is not application-acked.
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CustodyState {
    Accepted = 1,
    Stored = 2,
    Delivered = 3,
    ApplicationAcked = 4,
}

/// Mark stored only when the bytes themselves are present.
///
/// `len_only` or `!bytes_present` → [`QdnfError::Incomplete`], never Stored.
pub fn mark_stored(len_only: bool, bytes_present: bool) -> Result<CustodyState, QdnfError> {
    if len_only || !bytes_present {
        return Err(QdnfError::Incomplete);
    }
    Ok(CustodyState::Stored)
}

/// Accept ciphertext identity (digest/length may be known; bytes not yet).
#[inline]
pub fn mark_accepted() -> CustodyState {
    CustodyState::Accepted
}

/// Transport delivery. Requires Stored. Does not application-ack.
pub fn mark_delivered(state: CustodyState) -> Result<CustodyState, QdnfError> {
    match state {
        CustodyState::Stored | CustodyState::Delivered => Ok(CustodyState::Delivered),
        CustodyState::ApplicationAcked => Ok(CustodyState::ApplicationAcked),
        CustodyState::Accepted => Err(QdnfError::Incomplete),
    }
}

/// Application acknowledgement. Requires Delivered. Distinct from Delivered.
pub fn mark_application_acked(state: CustodyState) -> Result<CustodyState, QdnfError> {
    match state {
        CustodyState::Delivered | CustodyState::ApplicationAcked => {
            Ok(CustodyState::ApplicationAcked)
        }
        CustodyState::Accepted | CustodyState::Stored => Err(QdnfError::Incomplete),
    }
}

/// Delivered is not application-acknowledged.
#[inline]
pub fn delivered_is_application_acked() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn len_only_is_not_stored() {
        assert_eq!(mark_stored(true, false), Err(QdnfError::Incomplete));
        assert_eq!(mark_stored(true, true), Err(QdnfError::Incomplete));
        assert_eq!(mark_stored(false, false), Err(QdnfError::Incomplete));
        assert_eq!(mark_stored(false, true).unwrap(), CustodyState::Stored);
        assert_ne!(mark_accepted(), CustodyState::Stored);
    }

    #[test]
    fn delivered_is_not_application_acked() {
        let stored = mark_stored(false, true).unwrap();
        let delivered = mark_delivered(stored).unwrap();
        assert_eq!(delivered, CustodyState::Delivered);
        assert_ne!(delivered, CustodyState::ApplicationAcked);
        assert!(!delivered_is_application_acked());
        assert_eq!(
            mark_application_acked(CustodyState::Stored),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(
            mark_application_acked(delivered).unwrap(),
            CustodyState::ApplicationAcked
        );
        assert_eq!(
            mark_delivered(CustodyState::Accepted),
            Err(QdnfError::Incomplete)
        );
    }
}
