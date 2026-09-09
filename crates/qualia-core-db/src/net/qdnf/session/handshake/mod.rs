//! QSession handshake binding. No 0-RTT application data.

mod fragmented;
mod hello;

use crate::crypto::network::transcript::Transcript;
use crate::net::qdnf::authority::{
    ExecutionPermit, InstalledSessionKeys, PolicyOutcome, admit_service,
};
use crate::net::qdnf::crypto::finished::finished_mac;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{OperationId, StrongDigest};

pub use fragmented::{
    FragmentedHandshake, handshake_over_fragments, hello_fragment_payload_mtu, send_hello_fragments,
};
pub use hello::{
    CLIENT_HELLO_WIRE_LEN, SERVER_HELLO_WIRE_LEN, encode_client_hello_with_certs,
    encode_server_hello_with_certs,
};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionState {
    Idle = 0,
    Resolving = 1,
    RouteSelected = 2,
    LinkAuthenticated = 3,
    PersistentTargetVerified = 4,
    CapabilityNegotiated = 5,
    Active = 6,
    Draining = 7,
    Closed = 8,
}

/// Application session binding. Fields are private; Active requires a permit.
///
/// ```compile_fail
/// let _ = qualia_core_db::net::qdnf::session::SessionBinding {
///     operation: todo!(),
///     target: todo!(),
///     dni_digest: todo!(),
///     purpose: todo!(),
///     policy: todo!(),
///     state: todo!(),
/// };
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionBinding {
    operation: OperationId,
    target: StrongDigest,
    dni_digest: StrongDigest,
    purpose: StrongDigest,
    policy: PolicyOutcome,
    state: SessionState,
}

impl SessionBinding {
    pub fn from_permit(
        permit: &ExecutionPermit,
        keys: &InstalledSessionKeys,
        now_unix: u64,
    ) -> Result<Self, QdnfError> {
        permit.current_at(now_unix)?;
        permit.require_keys(keys)?;
        if permit.operation().is_zero()
            || permit.recipient().is_zero()
            || permit.purpose().is_zero()
        {
            return Err(QdnfError::Unauthorized);
        }
        Ok(Self {
            operation: permit.operation(),
            target: permit.recipient(),
            dni_digest: permit.instrument(),
            purpose: permit.purpose(),
            policy: PolicyOutcome::Allow,
            state: SessionState::Active,
        })
    }

    /// Component-test constructor. Rejects zero identity and forged Active+Allow
    /// unless `policy` is not Allow or `state` is not Active.
    #[cfg(test)]
    pub(crate) fn recorded(
        operation: OperationId,
        target: StrongDigest,
        dni_digest: StrongDigest,
        purpose: StrongDigest,
        policy: PolicyOutcome,
        state: SessionState,
    ) -> Result<Self, QdnfError> {
        if state == SessionState::Active && policy == PolicyOutcome::Allow {
            return Err(QdnfError::Unauthorized);
        }
        Ok(Self {
            operation,
            target,
            dni_digest,
            purpose,
            policy,
            state,
        })
    }

    #[inline]
    pub const fn operation(self) -> OperationId {
        self.operation
    }

    #[inline]
    pub const fn target(self) -> StrongDigest {
        self.target
    }

    #[inline]
    pub const fn dni_digest(self) -> StrongDigest {
        self.dni_digest
    }

    #[inline]
    pub const fn purpose(self) -> StrongDigest {
        self.purpose
    }

    #[inline]
    pub const fn policy(self) -> PolicyOutcome {
        self.policy
    }

    #[inline]
    pub const fn state(self) -> SessionState {
        self.state
    }

    pub fn admit_application(&self) -> Result<(), QdnfError> {
        if self.state != SessionState::Active {
            return Err(QdnfError::Unauthorized);
        }
        if self.operation.is_zero() || self.target.is_zero() || self.purpose.is_zero() {
            return Err(QdnfError::Unauthorized);
        }
        admit_service(self.policy, true)
    }

    #[cfg(test)]
    pub(crate) fn test_fixture(
        operation: OperationId,
        target: StrongDigest,
        dni_digest: StrongDigest,
        purpose: StrongDigest,
        policy: PolicyOutcome,
        state: SessionState,
    ) -> Self {
        Self {
            operation,
            target,
            dni_digest,
            purpose,
            policy,
            state,
        }
    }
}

pub fn bind_session_transcript(
    path_digest: &StrongDigest,
    target: &StrongDigest,
    purpose: &StrongDigest,
) -> Result<StrongDigest, QdnfError> {
    let mut t = Transcript::new();
    t.append(b"path", &path_digest.0)?;
    t.append(b"target", &target.0)?;
    t.append(b"purpose", &purpose.0)?;
    t.append(b"0rtt", &[0])?;
    Ok(t.digest())
}

pub fn confirm_finished(
    secret: &[u8; 32],
    digest: &StrongDigest,
    initiator_to_responder: bool,
) -> Result<StrongDigest, QdnfError> {
    finished_mac(secret, digest, initiator_to_responder)
}

pub const ZERO_RTT_APPLICATION: crate::net::qdnf::crypto::HandshakeState =
    crate::net::qdnf::crypto::HandshakeState::Idle;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_session_cannot_deliver() {
        let s = SessionBinding::recorded(
            OperationId([1u8; 16]),
            StrongDigest([2u8; 48]),
            StrongDigest([3u8; 48]),
            StrongDigest([4u8; 48]),
            PolicyOutcome::Allow,
            SessionState::LinkAuthenticated,
        )
        .unwrap();
        assert_eq!(s.admit_application(), Err(QdnfError::Unauthorized));
    }

    #[test]
    fn recorded_cannot_mint_active_allow() {
        assert_eq!(
            SessionBinding::recorded(
                OperationId([1u8; 16]),
                StrongDigest([2u8; 48]),
                StrongDigest([3u8; 48]),
                StrongDigest([4u8; 48]),
                PolicyOutcome::Allow,
                SessionState::Active,
            ),
            Err(QdnfError::Unauthorized)
        );
    }
}
