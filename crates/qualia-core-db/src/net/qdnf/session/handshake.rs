//! QSession handshake binding. No 0-RTT application data.

use crate::crypto::network::transcript::Transcript;
use crate::net::qdnf::authority::{admit_service, PolicyOutcome};
use crate::net::qdnf::crypto::handshake::{finished_mac, HandshakeState};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{OperationId, StrongDigest};

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

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionBinding {
    pub operation: OperationId,
    pub target: StrongDigest,
    pub dni_digest: StrongDigest,
    pub purpose: StrongDigest,
    pub policy: PolicyOutcome,
    pub state: SessionState,
}

impl SessionBinding {
    pub fn admit_application(&self) -> Result<(), QdnfError> {
        if self.state != SessionState::Active {
            return Err(QdnfError::Unauthorized);
        }
        admit_service(self.policy, true)
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

pub fn confirm_finished(digest: &StrongDigest) -> StrongDigest {
    finished_mac(digest, b"qsession")
}

pub const ZERO_RTT_APPLICATION: HandshakeState = HandshakeState::Idle;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inactive_session_cannot_deliver() {
        let s = SessionBinding {
            operation: OperationId::ZERO,
            target: StrongDigest::ZERO,
            dni_digest: StrongDigest::ZERO,
            purpose: StrongDigest::ZERO,
            policy: PolicyOutcome::Allow,
            state: SessionState::LinkAuthenticated,
        };
        assert_eq!(s.admit_application(), Err(QdnfError::Unauthorized));
    }
}
