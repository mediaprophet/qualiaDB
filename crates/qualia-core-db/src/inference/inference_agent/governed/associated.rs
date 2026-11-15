//! Native actor and policy acceptance share one explicit linearization boundary.
use super::{
    controller::{InstalledController, PolicyTokenSession},
    ActorPolicyScope, InferenceAdmission, PolicyError, TokenDecision, TokenProposal,
};
use crate::poet_host::native_authority::{
    inference::AuthenticatedInferenceAssociation, NativeAuthority,
};
use std::sync::Arc;

/// Receipt of an accepted record, not a reusable permission or restored state.
#[derive(Clone, Copy, Debug)]
pub struct AcceptedToken {
    pub proposal: TokenProposal,
    pub key: etalon_inference_contract::CandidateKey,
}

/// Created only from installed policy AND an actual current native association.
/// No public constructor accepts copied evidence. This owns no numerical state.
pub struct GovernedTokenSession {
    inner: PolicyTokenSession,
    owner: Arc<NativeAuthority>,
    association: AuthenticatedInferenceAssociation,
    terminal: bool,
}
impl GovernedTokenSession {
    pub(crate) fn admit(
        controller: Arc<InstalledController>,
        request: InferenceAdmission,
        session_id: u64,
        clock_override: Option<u64>,
        owner: Arc<NativeAuthority>,
        association: AuthenticatedInferenceAssociation,
    ) -> Result<Self, PolicyError> {
        let inner = owner
            .with_inference_association_guard(&association, |view| {
                let _gate = controller.authority.lock()?;
                let e = view
                    .revalidate_current()
                    .map_err(|_| PolicyError::Revoked)?;
                let real_now = clock(None)?;
                PolicyTokenSession::admit(
                    Arc::clone(&controller),
                    request,
                    ActorPolicyScope::from_evidence(e),
                    session_id,
                    clock_override.unwrap_or(real_now),
                )
            })
            .map_err(|_| PolicyError::Revoked)??;
        Ok(Self {
            inner,
            owner,
            association,
            terminal: false,
        })
    }
    pub fn prepare(&mut self, proposal: TokenProposal) -> Result<TokenDecision, PolicyError> {
        self.prepare_clock(proposal, None)
    }
    #[cfg(test)]
    pub(super) fn prepare_at(
        &mut self,
        proposal: TokenProposal,
        now: u64,
    ) -> Result<TokenDecision, PolicyError> {
        self.prepare_clock(proposal, Some(now))
    }
    fn prepare_clock(
        &mut self,
        proposal: TokenProposal,
        clock_override: Option<u64>,
    ) -> Result<TokenDecision, PolicyError> {
        if self.terminal {
            return Err(PolicyError::Revoked);
        }
        self.terminal = true;
        let controller = Arc::clone(&self.inner.controller);
        let result = self
            .owner
            .with_inference_association_guard(&self.association, |view| {
                let _gate = controller.authority.lock()?;
                let e = view
                    .revalidate_current()
                    .map_err(|_| PolicyError::Revoked)?;
                if ActorPolicyScope::from_evidence(e) != self.inner.actor {
                    return Err(PolicyError::Scope);
                }
                let now = clock(clock_override)?;
                self.inner.prepare(proposal, now)
            })
            .map_err(|_| PolicyError::Revoked)
            .and_then(|r| r);
        if result.is_err() {
            self.cancel();
        } else {
            self.terminal = false;
        }
        result
    }
    /// Enqueue exactly one accepted record under NativeAuthority -> policy gate.
    /// The caller drains UI/user callbacks AFTER this returns, outside all locks.
    /// Revocation linearizes before refusal or after this accepted record; it
    /// cannot retroactively retract an earlier acceptance. Occupied output refuses.
    pub fn accept_into(
        &mut self,
        decision: &TokenDecision,
        out: &mut Option<AcceptedToken>,
    ) -> Result<(), PolicyError> {
        self.accept_into_clock(decision, None, out)
    }
    #[cfg(test)]
    pub(super) fn accept_into_at(
        &mut self,
        decision: &TokenDecision,
        now: u64,
        out: &mut Option<AcceptedToken>,
    ) -> Result<(), PolicyError> {
        self.accept_into_clock(decision, Some(now), out)
    }
    fn accept_into_clock(
        &mut self,
        decision: &TokenDecision,
        clock_override: Option<u64>,
        out: &mut Option<AcceptedToken>,
    ) -> Result<(), PolicyError> {
        if out.is_some() {
            self.cancel();
            return Err(PolicyError::Capacity);
        }
        self.accept_clock(decision, clock_override, || {
            *out = Some(AcceptedToken {
                proposal: decision.proposal(),
                key: decision.key(),
            });
            Ok(())
        })
    }
    /// Internal bounded state/record enqueue only, NEVER arbitrary user callbacks,
    /// host/vault calls, I/O, await, or reentrant authority operations. Failure or
    /// unwind leaves this session terminal; the caller must restore enrolled
    /// state or discard it. Already published effects cannot be retracted here.
    #[cfg(test)]
    pub(super) fn accept_with_at(
        &mut self,
        decision: &TokenDecision,
        now: u64,
        enqueue: impl FnOnce() -> Result<(), PolicyError>,
    ) -> Result<(), PolicyError> {
        self.accept_clock(decision, Some(now), enqueue)
    }
    fn accept_clock(
        &mut self,
        decision: &TokenDecision,
        clock_override: Option<u64>,
        enqueue: impl FnOnce() -> Result<(), PolicyError>,
    ) -> Result<(), PolicyError> {
        if self.terminal {
            return Err(PolicyError::Revoked);
        }
        // Sticky before entering any fallible code; also survives unwinding.
        self.terminal = true;
        let controller = Arc::clone(&self.inner.controller);
        let result = self
            .owner
            .with_inference_association_guard(&self.association, |view| {
                let _gate = controller.authority.lock()?;
                let e = view
                    .revalidate_current()
                    .map_err(|_| PolicyError::Revoked)?;
                if ActorPolicyScope::from_evidence(e) != self.inner.actor {
                    return Err(PolicyError::Scope);
                }
                let now = clock(clock_override)?;
                controller.live(now)?;
                if now >= self.inner.actor.valid_until {
                    return Err(PolicyError::Expired);
                }
                self.inner.commit(decision, now)?;
                enqueue()
            })
            .map_err(|_| PolicyError::Revoked)
            .and_then(|r| r);
        match result {
            Ok(()) => {
                self.terminal = false;
                Ok(())
            }
            Err(error) => {
                self.inner.cancel();
                Err(error)
            }
        }
    }
    pub fn cancel(&mut self) {
        self.terminal = true;
        self.inner.cancel();
    }
    pub fn is_terminal(&self) -> bool {
        self.terminal
    }
    pub fn snapshot(&self) -> etalon_inference_contract::SessionSnapshot {
        self.inner.snapshot()
    }
}

// Production callers cannot supply time. Read after acquiring both acceptance
// locks so waiting on a lock cannot extend an expired policy window.
fn clock(test_override: Option<u64>) -> Result<u64, PolicyError> {
    if let Some(now) = test_override {
        return Ok(now);
    }
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .map_err(|_| PolicyError::Clock)
}
