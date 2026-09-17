//! Full-strength digest bindings. Short URI hashes are lookup indexes only.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{OperationId, StrongDigest};

/// SHA-384 bindings for principal, recipient, instrument, scope and purpose.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AuthorityBinding {
    pub principal: StrongDigest,
    pub recipient: StrongDigest,
    pub instrument: StrongDigest,
    pub scope: StrongDigest,
    pub purpose: StrongDigest,
    pub operation: OperationId,
}

impl AuthorityBinding {
    pub fn from_bytes(
        principal: &[u8],
        recipient: &[u8],
        instrument: &[u8],
        scope: &[u8],
        purpose: &[u8],
        operation: &[u8],
    ) -> Result<Self, QdnfError> {
        if principal.is_empty()
            || recipient.is_empty()
            || instrument.is_empty()
            || scope.is_empty()
            || purpose.is_empty()
            || operation.is_empty()
        {
            return Err(QdnfError::Unauthorized);
        }
        let principal = sha384(principal);
        let recipient = sha384(recipient);
        let instrument = sha384(instrument);
        let scope = sha384(scope);
        let purpose = sha384(purpose);
        let op = sha384(operation);
        let mut operation = OperationId::ZERO;
        operation.0.copy_from_slice(&op.0[..16]);
        if principal.is_zero()
            || recipient.is_zero()
            || instrument.is_zero()
            || scope.is_zero()
            || purpose.is_zero()
            || operation.is_zero()
        {
            return Err(QdnfError::Unauthorized);
        }
        Ok(Self {
            principal,
            recipient,
            instrument,
            scope,
            purpose,
            operation,
        })
    }
}

pub fn bind_authority_digests(
    principal: &[u8],
    recipient: &[u8],
    instrument: &[u8],
    scope: &[u8],
    purpose: &[u8],
    operation: &[u8],
) -> Result<AuthorityBinding, QdnfError> {
    AuthorityBinding::from_bytes(principal, recipient, instrument, scope, purpose, operation)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_or_zero_inputs_are_rejected() {
        assert_eq!(
            bind_authority_digests(b"", b"r", b"i", b"s", b"p", b"o"),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn bindings_are_full_sha384_not_first_byte() {
        let a =
            bind_authority_digests(b"alice", b"bob", b"grant", b"care", b"chart", b"op-1").unwrap();
        let b =
            bind_authority_digests(b"alice", b"bob", b"grant", b"care", b"chart", b"op-2").unwrap();
        assert_ne!(a.operation, b.operation);
        assert_ne!(a.principal.0, [0u8; 48]);
        assert_ne!(a.principal, a.recipient);
    }
}
