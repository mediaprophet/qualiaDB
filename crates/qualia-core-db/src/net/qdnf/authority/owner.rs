//! Owner-held authority records and generation-bearing mutation handles.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, ProfileId, StrongDigest};

use super::digest_bind::AuthorityBinding;
use super::permit::ExecutionPermit;
use super::policy::{ContactState, TemporalGrant};
use super::verified::{AuthorisedContact, VerifiedCredential};

const MAX_RECORDS: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Record {
    occupied: bool,
    generation: Generation,
    principal: StrongDigest,
    recipient: StrongDigest,
    purpose: StrongDigest,
    instrument: StrongDigest,
    expires_unix: u64,
    contact: ContactState,
    revoked: bool,
}

impl Record {
    const EMPTY: Self = Self {
        occupied: false,
        generation: Generation::ZERO,
        principal: StrongDigest::ZERO,
        recipient: StrongDigest::ZERO,
        purpose: StrongDigest::ZERO,
        instrument: StrongDigest::ZERO,
        expires_unix: 0,
        contact: ContactState::Request,
        revoked: false,
    };
}

/// Handle for an asynchronous mutation. Validated against owner-held records.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MutationHandle {
    slot: u8,
    generation: Generation,
    principal: StrongDigest,
}

impl MutationHandle {
    pub fn matches_permit(&self, permit: &ExecutionPermit) -> Result<(), QdnfError> {
        if self.generation != permit.identity_generation() {
            return Err(QdnfError::StaleGeneration);
        }
        if self.principal != permit.principal() {
            return Err(QdnfError::Unauthorized);
        }
        Ok(())
    }
}

/// Single owner of verified grants, contacts and permits for this process.
pub struct AuthorityOwner {
    records: [Record; MAX_RECORDS],
    next_generation: Generation,
}

impl AuthorityOwner {
    pub const fn new() -> Self {
        Self {
            records: [Record::EMPTY; MAX_RECORDS],
            next_generation: Generation::ZERO,
        }
    }

    fn bump(&mut self) -> Result<Generation, QdnfError> {
        self.next_generation = self.next_generation.next()?;
        Ok(self.next_generation)
    }

    fn free_slot(&self) -> Result<usize, QdnfError> {
        let mut i = 0;
        while i < MAX_RECORDS {
            if !self.records[i].occupied {
                return Ok(i);
            }
            i += 1;
        }
        Err(QdnfError::Capacity)
    }

    /// Install a local verified grant after hashing the original bytes.
    pub fn install_grant(
        &mut self,
        binding: AuthorityBinding,
        now_unix: u64,
        expires_unix: u64,
        contact: ContactState,
    ) -> Result<(VerifiedCredential, AuthorisedContact, MutationHandle), QdnfError> {
        if expires_unix <= now_unix {
            return Err(QdnfError::Expired);
        }
        if contact != ContactState::Active {
            return Err(QdnfError::Unauthorized);
        }
        let slot = self.free_slot()?;
        let generation = self.bump()?;
        self.records[slot] = Record {
            occupied: true,
            generation,
            principal: binding.principal,
            recipient: binding.recipient,
            purpose: binding.purpose,
            instrument: binding.instrument,
            expires_unix,
            contact,
            revoked: false,
        };
        let cred = VerifiedCredential::new(
            binding.principal,
            binding.instrument,
            binding.instrument,
            generation,
        )?;
        let authorised =
            AuthorisedContact::new(binding.principal, binding.recipient, contact, generation)?;
        let handle = MutationHandle {
            slot: slot as u8,
            generation,
            principal: binding.principal,
        };
        Ok((cred, authorised, handle))
    }

    pub fn revoke(&mut self, handle: MutationHandle) -> Result<(), QdnfError> {
        let rec = self.record_mut(handle)?;
        rec.revoked = true;
        rec.contact = ContactState::Blocked;
        Ok(())
    }

    pub fn issue_permit(
        &self,
        handle: MutationHandle,
        binding: AuthorityBinding,
        now_unix: u64,
        resource_ceiling: u64,
    ) -> Result<ExecutionPermit, QdnfError> {
        let rec = self.record(handle)?;
        if rec.revoked {
            return Err(QdnfError::Revoked);
        }
        if rec.contact != ContactState::Active {
            return Err(QdnfError::Denied);
        }
        if now_unix >= rec.expires_unix {
            return Err(QdnfError::Expired);
        }
        if rec.principal != binding.principal
            || rec.recipient != binding.recipient
            || rec.purpose != binding.purpose
            || rec.instrument != binding.instrument
        {
            return Err(QdnfError::Unauthorized);
        }
        ExecutionPermit::issue(
            binding.operation,
            binding.principal,
            binding.recipient,
            binding.purpose,
            binding.instrument,
            rec.generation,
            rec.generation,
            rec.expires_unix,
            resource_ceiling,
        )
    }

    pub fn current_grant(&self, handle: MutationHandle) -> Result<TemporalGrant, QdnfError> {
        let rec = self.record(handle)?;
        if rec.revoked {
            return Err(QdnfError::Revoked);
        }
        Ok(TemporalGrant {
            purpose_digest: rec.purpose,
            audience_digest: rec.recipient,
            authority_generation: rec.generation.0,
            not_before_unix: 0,
            expires_unix: rec.expires_unix,
            profile: ProfileId::QPR_PQ_1,
        })
    }

    pub fn validate_handle(&self, handle: MutationHandle) -> Result<(), QdnfError> {
        let _ = self.record(handle)?;
        Ok(())
    }

    fn record(&self, handle: MutationHandle) -> Result<&Record, QdnfError> {
        let i = handle.slot as usize;
        if i >= MAX_RECORDS {
            return Err(QdnfError::Range);
        }
        let rec = &self.records[i];
        if !rec.occupied {
            return Err(QdnfError::Incomplete);
        }
        if rec.generation != handle.generation {
            return Err(QdnfError::StaleGeneration);
        }
        if rec.principal != handle.principal {
            return Err(QdnfError::Unauthorized);
        }
        Ok(rec)
    }

    fn record_mut(&mut self, handle: MutationHandle) -> Result<&mut Record, QdnfError> {
        let i = handle.slot as usize;
        if i >= MAX_RECORDS {
            return Err(QdnfError::Range);
        }
        let rec = &mut self.records[i];
        if !rec.occupied {
            return Err(QdnfError::Incomplete);
        }
        if rec.generation != handle.generation {
            return Err(QdnfError::StaleGeneration);
        }
        if rec.principal != handle.principal {
            return Err(QdnfError::Unauthorized);
        }
        Ok(rec)
    }
}

impl Default for AuthorityOwner {
    fn default() -> Self {
        Self::new()
    }
}

/// Helper used by native pairing: hash two controller byte strings into a binding.
pub fn binding_for_controllers(
    local: &[u8],
    remote: &[u8],
    purpose: &[u8],
    operation: &[u8],
) -> Result<AuthorityBinding, QdnfError> {
    let instrument = sha384(b"qpr-pq-1/known-peer-grant");
    super::digest_bind::bind_authority_digests(
        local,
        remote,
        &instrument.0,
        b"qpr-scope/local",
        purpose,
        operation,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::authority::DecodedClaim;
    use crate::net::qdnf::authority::PolicyOutcome;
    use crate::net::qdnf::types::OperationId;

    #[test]
    fn decoded_allow_cannot_issue_permit() {
        let owner = AuthorityOwner::new();
        let mut claim = DecodedClaim::empty();
        claim.asserted_outcome = PolicyOutcome::Allow;
        claim.signed_flag = true;
        assert_eq!(claim.admit_service(), Err(QdnfError::Unauthorized));
        let _ = owner;
    }

    #[test]
    fn stale_handle_is_rejected() {
        let mut owner = AuthorityOwner::new();
        let binding =
            binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a").unwrap();
        let (_c, _k, handle) = owner
            .install_grant(binding, 10, 100, ContactState::Active)
            .unwrap();
        let mut stale = handle;
        stale.generation = Generation(99);
        assert_eq!(
            owner.issue_permit(stale, binding, 20, 256),
            Err(QdnfError::StaleGeneration)
        );
    }

    #[test]
    fn revoked_grant_cannot_issue_permit() {
        let mut owner = AuthorityOwner::new();
        let binding =
            binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a").unwrap();
        let (_c, _k, handle) = owner
            .install_grant(binding, 10, 100, ContactState::Active)
            .unwrap();
        owner.revoke(handle).unwrap();
        assert_eq!(
            owner.issue_permit(handle, binding, 20, 256),
            Err(QdnfError::Revoked)
        );
    }

    #[test]
    fn current_grant_issues_permit() {
        let mut owner = AuthorityOwner::new();
        let binding =
            binding_for_controllers(b"did:q42:a", b"did:q42:b", b"q42:QSync/1", b"op-a").unwrap();
        let (cred, contact, handle) = owner
            .install_grant(binding, 10, 100, ContactState::Active)
            .unwrap();
        let permit = owner.issue_permit(handle, binding, 20, 256).unwrap();
        assert!(permit.matches_credential(&cred).is_ok());
        assert!(permit.matches_contact(&contact).is_ok());
        assert_ne!(permit.operation(), OperationId::ZERO);
    }
}
