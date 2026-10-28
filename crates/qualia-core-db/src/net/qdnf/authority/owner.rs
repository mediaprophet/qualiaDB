//! Owner-held authority records and generation-bearing mutation handles.

use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, OperationId, ProfileId, StrongDigest};

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
    scope: StrongDigest,
    operation: OperationId,
    not_before_unix: u64,
    resource_ceiling: u64,
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
        scope: StrongDigest::ZERO,
        operation: OperationId::ZERO,
        not_before_unix: 0,
        resource_ceiling: 0,
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
    /// `Ok` when `permit` authorizes this handle; `QdnfError` otherwise.
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
    /// Creates a default `AuthorityOwner`.
    pub const fn new() -> Self {
        Self {
            records: [Record::EMPTY; MAX_RECORDS],
            next_generation: Generation::ZERO,
        }
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

    /// Compatibility admission with an unlimited per-permit resource ceiling.
    /// Scope and operation remain pinned. New bounded hosts should use
    /// `install_grant_with_ceiling`; neither route is a consumable budget ledger.
    pub fn install_grant(
        &mut self,
        binding: AuthorityBinding,
        now_unix: u64,
        expires_unix: u64,
        contact: ContactState,
    ) -> Result<(VerifiedCredential, AuthorisedContact, MutationHandle), QdnfError> {
        self.install_grant_with_ceiling(binding, now_unix, expires_unix, contact, u64::MAX)
    }

    /// Pin the exact admitted binding, validity start and per-permit ceiling.
    /// This trusted host entry point does not verify human consent by itself.
    pub fn install_grant_with_ceiling(
        &mut self,
        binding: AuthorityBinding,
        now_unix: u64,
        expires_unix: u64,
        contact: ContactState,
        resource_ceiling: u64,
    ) -> Result<(VerifiedCredential, AuthorisedContact, MutationHandle), QdnfError> {
        if expires_unix <= now_unix {
            return Err(QdnfError::Expired);
        }
        if contact != ContactState::Active {
            return Err(QdnfError::Unauthorized);
        }
        if binding.principal.is_zero()
            || binding.recipient.is_zero()
            || binding.purpose.is_zero()
            || binding.instrument.is_zero()
            || binding.scope.is_zero()
            || binding.operation.is_zero()
        {
            return Err(QdnfError::Unauthorized);
        }
        if binding.principal == binding.recipient {
            return Err(QdnfError::Conflict);
        }
        if resource_ceiling == 0 {
            return Err(QdnfError::Capacity);
        }
        let slot = self.free_slot()?;
        let generation = self.next_generation.next()?;
        let cred = VerifiedCredential::new(
            binding.principal,
            binding.instrument,
            binding.instrument,
            generation,
        )?;
        let authorised =
            AuthorisedContact::new(binding.principal, binding.recipient, contact, generation)?;
        // Every fallible admission check precedes publication/capacity use.
        let record = Record {
            occupied: true,
            generation,
            principal: binding.principal,
            recipient: binding.recipient,
            purpose: binding.purpose,
            instrument: binding.instrument,
            scope: binding.scope,
            operation: binding.operation,
            not_before_unix: now_unix,
            resource_ceiling,
            expires_unix,
            contact,
            revoked: false,
        };
        let handle = MutationHandle {
            slot: slot as u8,
            generation,
            principal: binding.principal,
        };
        self.records[slot] = record;
        self.next_generation = generation;
        Ok((cred, authorised, handle))
    }

    /// Revokes `handle`'s grant.
    pub fn revoke(&mut self, handle: MutationHandle) -> Result<(), QdnfError> {
        let rec = self.record_mut(handle)?;
        rec.revoked = true;
        rec.contact = ContactState::Blocked;
        Ok(())
    }

    /// Issues an `ExecutionPermit` for a handle under the current grant.
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
        if now_unix < rec.not_before_unix {
            return Err(QdnfError::Denied);
        }
        if rec.principal != binding.principal
            || rec.recipient != binding.recipient
            || rec.purpose != binding.purpose
            || rec.instrument != binding.instrument
            || rec.scope != binding.scope
            || rec.operation != binding.operation
        {
            return Err(QdnfError::Unauthorized);
        }
        if resource_ceiling > rec.resource_ceiling {
            return Err(QdnfError::BudgetExhausted);
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

    /// Active grant for `handle`; error when none.
    pub fn current_grant(&self, handle: MutationHandle) -> Result<TemporalGrant, QdnfError> {
        let rec = self.record(handle)?;
        if rec.revoked {
            return Err(QdnfError::Revoked);
        }
        Ok(TemporalGrant {
            purpose_digest: rec.purpose,
            audience_digest: rec.recipient,
            authority_generation: rec.generation.0,
            not_before_unix: rec.not_before_unix,
            expires_unix: rec.expires_unix,
            profile: ProfileId::QPR_PQ_1,
        })
    }

    /// Check record association/generation only; use `issue_permit` for live rights.
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
#[path = "owner_tests.rs"]
mod tests;
