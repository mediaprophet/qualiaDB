//! Bounded process-local authentication state; never serialized to the journal.
use super::*;

/// Independent live sessions. Capacity failure never retires another live actor.
pub const MAX_NATIVE_SESSIONS: usize = 16;
/// Outstanding one-use challenges, independently cancellable and expiring.
pub const MAX_PENDING_SESSION_CHALLENGES: usize = 32;

pub(super) struct Pending {
    pub challenge: SessionChallenge,
    pub subject: String,
    pub expires: u64,
    pub admission: [u8; 32],
}
pub(super) struct Session {
    pub handle: SessionHandle,
    pub subject: String,
    pub expires: u64,
    // QDNF retains revoked slots. One existing grant owner per live session
    // bounds residency and allows slot reuse without recycling another grant.
    pub grants: GrantOwner,
    pub grant: MutationHandle,
    pub binding: AuthorityBinding,
    pub authentication_ref: String,
}

impl State {
    pub(super) fn session(&self, handle: &SessionHandle) -> Result<&Session, String> {
        self.sessions
            .iter()
            .flatten()
            .find(|s| &s.handle == handle)
            .ok_or_else(|| "native authenticated session unavailable/foreign".into())
    }

    pub(super) fn expire_sessions(&mut self, now: u64) -> Result<(), String> {
        for pending in &mut self.pending {
            if pending.as_ref().is_some_and(|p| now >= p.expires) {
                *pending = None;
            }
        }
        for slot in &mut self.sessions {
            if let Some(session) = slot.as_mut().filter(|s| now >= s.expires) {
                session
                    .grants
                    .revoke(session.grant)
                    .map_err(|e| format!("native expired grant revoke: {e:?}"))?;
                *slot = None;
            }
        }
        Ok(())
    }

    /// Called only after a signed adopted admission successor is durable.
    /// Dropping the exclusively owned grant tables retires all their permits.
    pub(super) fn invalidate_sessions(&mut self) {
        for slot in &mut self.sessions {
            *slot = None;
        }
        for slot in &mut self.pending {
            *slot = None;
        }
    }
}
