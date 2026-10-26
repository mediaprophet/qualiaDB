use super::*;

impl NativeAuthority {
    pub fn begin_session(
        &self,
        subject: &str,
        lifetime_seconds: u64,
    ) -> Result<SessionChallenge, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let now = now()?;
        self.require_admission(&state, now)?;
        if lifetime_seconds == 0 || lifetime_seconds > 3600 {
            return Err("native session lifetime exceeds bounds".into());
        }
        state.expire_sessions(now)?;
        let slot = state
            .pending
            .iter()
            .position(Option::is_none)
            .ok_or("native pending challenge capacity exhausted")?;
        let admitted = state
            .durable
            .admission
            .snapshot
            .subjects
            .iter()
            .find(|s| s.subject == subject)
            .ok_or("subject has no signed project admission")?;
        if admitted.revoked || !live(now, admitted.valid_from_unix, admitted.valid_until_unix) {
            return Err("subject admission expired/revoked".into());
        }
        let expires = now
            .checked_add(lifetime_seconds)
            .ok_or("session expiry overflow")?
            .min(admitted.valid_until_unix)
            .min(state.durable.admission.snapshot.valid_until_unix);
        let nonce = random()?;
        if state
            .pending
            .iter()
            .flatten()
            .any(|p| p.challenge.nonce == nonce)
            || state
                .sessions
                .iter()
                .flatten()
                .any(|s| s.handle.nonce == nonce)
        {
            return Err("native session nonce collision".into());
        }
        let mut hash = Sha256::new();
        // Hash fixed-size fields plus bounded, length-delimited identifiers.
        hash.update(SESSION_CONTEXT);
        hash.update(self.incarnation);
        hash.update(nonce);
        hash.update(state.admission_digest);
        hash.update(admitted.controller_public_key);
        hash.update(expires.to_be_bytes());
        for value in [&self.anchor.project_id, &admitted.subject] {
            hash.update((value.len() as u64).to_be_bytes());
            hash.update(value.as_bytes());
        }
        let challenge = SessionChallenge {
            owner: self.incarnation,
            nonce,
            digest: hash.finalize().into(),
        };
        state.pending[slot] = Some(Pending {
            challenge: challenge.clone(),
            subject: subject.into(),
            expires,
            admission: state.admission_digest,
        });
        Ok(challenge)
    }
    pub fn complete_session(
        &self,
        challenge: SessionChallenge,
        signature: &[u8; 64],
    ) -> Result<SessionHandle, String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        let now = now()?;
        self.require_admission(&state, now)?;
        state.expire_sessions(now)?;
        // Match all opaque fields before consuming a slot. A foreign challenge
        // cannot consume another actor's proof. An invalid signature for this
        // exact challenge still consumes it once.
        if challenge.owner != self.incarnation {
            return Err("stale/foreign native session challenge".into());
        }
        let index = state
            .pending
            .iter()
            .position(|p| {
                p.as_ref().is_some_and(|p| {
                    p.challenge.owner == challenge.owner
                        && p.challenge.nonce == challenge.nonce
                        && p.challenge.digest == challenge.digest
                })
            })
            .ok_or("no matching pending native session challenge")?;
        let pending = state.pending[index]
            .take()
            .ok_or("native challenge unavailable")?;
        if now >= pending.expires || pending.admission != state.admission_digest {
            return Err("stale/foreign native session challenge".into());
        }
        let subject = state
            .durable
            .admission
            .snapshot
            .subjects
            .iter()
            .find(|s| s.subject == pending.subject)
            .ok_or("native subject admission disappeared")?;
        if subject.revoked || !live(now, subject.valid_from_unix, subject.valid_until_unix) {
            return Err("native subject admission expired/revoked".into());
        }
        ed25519::verify_with_context(
            &subject.controller_public_key,
            &challenge.digest,
            SESSION_CONTEXT,
            signature,
        )
        .map_err(|_| "native session possession proof verification failed")?;
        // The native grant is installed only AFTER the adopted key proof above.
        let binding = AuthorityBinding::from_bytes(
            &subject.controller_public_key,
            &self.anchor.public_key,
            &state.admission_digest,
            self.anchor.project_id.as_bytes(),
            b"poet/native-authoring/v1",
            &challenge.nonce,
        )
        .map_err(|e| format!("native grant binding: {e:?}"))?;
        let slot = state
            .sessions
            .iter()
            .position(Option::is_none)
            .ok_or("native authenticated session capacity exhausted")?;
        let mut grants = GrantOwner::new();
        let (_, _, grant) = grants
            .install_grant(binding, now, pending.expires, ContactState::Active)
            .map_err(|e| format!("native grant install: {e:?}"))?;
        // Shared generation denotes adopted admission, not the last login.
        // Publication guards also check their own opaque session handle.
        let handle = SessionHandle {
            owner: self.incarnation,
            nonce: challenge.nonce,
        };
        state.sessions[slot] = Some(Session {
            handle: handle.clone(),
            subject: pending.subject,
            expires: pending.expires,
            grants,
            grant,
            binding,
            authentication_ref: format!(
                "urn:qualia:native-session-proof:sha256:{}",
                hex::encode(challenge.digest)
            ),
        });
        Ok(handle)
    }
    /// Cancel only this owner's exact outstanding challenge. No live session,
    /// unrelated challenge or shared admission generation changes.
    pub fn cancel_session_challenge(&self, challenge: &SessionChallenge) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        if challenge.owner != self.incarnation {
            return Err("foreign native session challenge".into());
        }
        let slot = state
            .pending
            .iter()
            .position(|p| {
                p.as_ref().is_some_and(|p| {
                    p.challenge.owner == challenge.owner
                        && p.challenge.nonce == challenge.nonce
                        && p.challenge.digest == challenge.digest
                })
            })
            .ok_or("no matching pending native session challenge")?;
        state.pending[slot] = None;
        Ok(())
    }

    /// Revoke exactly this session. Other actors and sessions for the same actor
    /// keep their own proof, grant and expiry.
    pub fn disconnect(&self, handle: &SessionHandle) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "native authority poisoned")?;
        if handle.owner != self.incarnation {
            return Err("foreign native session".into());
        }
        let slot = state
            .sessions
            .iter()
            .position(|s| s.as_ref().is_some_and(|s| &s.handle == handle))
            .ok_or("foreign native session")?;
        let session = state.sessions[slot]
            .as_mut()
            .ok_or("native session unavailable")?;
        session
            .grants
            .revoke(session.grant)
            .map_err(|e| format!("native revoke: {e:?}"))?;
        state.sessions[slot] = None;
        Ok(())
    }
}
