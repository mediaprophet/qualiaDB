//! Protected contact states and reserved help (NET-05.17 / NET-05.20).
//!
//! Request → Consent → Active. Request cannot skip Consent. Blocked and
//! Suspended deny even when a current grant would Allow and the peer paid.
//! Mandate revoke uses the current generation, then Blocked. Two help slots
//! stay independent of the 16 contact slots. Packages remain open.

use crate::net::qdnf::authority::{admit_service, ContactState, PolicyOutcome, TemporalGrant};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::policy::gate_channel;
use crate::net::qdnf::types::StrongDigest;

/// Occupied contact slots. A 17th distinct peer is [`QdnfError::Capacity`].
pub const CONTACT_SLOTS: usize = 16;
/// Revocation / help capacity. Bulk contact fill must not consume these.
pub const REVOKED_HELP_SLOTS: usize = 2;

/// One protected-person contact. `paid` is observational only.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContactSlot {
    pub peer: StrongDigest,
    pub state: ContactState,
    /// Zero means no mandate is recorded.
    pub mandate_generation: u64,
    pub purpose: StrongDigest,
    pub paid: bool,
    pub revoked: bool,
}

/// Sixteen contact slots plus two reserved help/revocation slots.
pub struct ContactTable {
    contacts: [Option<ContactSlot>; CONTACT_SLOTS],
    help: [Option<StrongDigest>; REVOKED_HELP_SLOTS],
}

impl ContactTable {
    pub const fn new() -> Self {
        Self {
            contacts: [None; CONTACT_SLOTS],
            help: [None; REVOKED_HELP_SLOTS],
        }
    }

    pub fn contact_occupied(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < CONTACT_SLOTS {
            if self.contacts[i].is_some() {
                n += 1;
            }
            i += 1;
        }
        n
    }

    pub fn help_occupied(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < REVOKED_HELP_SLOTS {
            if self.help[i].is_some() {
                n += 1;
            }
            i += 1;
        }
        n
    }

    fn index_of(&self, peer: StrongDigest) -> Result<usize, QdnfError> {
        let mut i = 0usize;
        while i < CONTACT_SLOTS {
            if let Some(slot) = self.contacts[i] {
                if slot.peer == peer {
                    return Ok(i);
                }
            }
            i += 1;
        }
        Err(QdnfError::Unauthorized)
    }

    pub fn get(&self, peer: StrongDigest) -> Result<ContactSlot, QdnfError> {
        let i = self.index_of(peer)?;
        self.contacts[i].ok_or(QdnfError::Unauthorized)
    }

    /// Occupy a Request slot. Distinct 17th peer → Capacity.
    pub fn insert_request(
        &mut self,
        peer: StrongDigest,
        purpose: StrongDigest,
        paid: bool,
        mandate_generation: u64,
    ) -> Result<(), QdnfError> {
        if peer == StrongDigest::ZERO {
            return Err(QdnfError::Malformed);
        }
        let mut free = None;
        let mut i = 0usize;
        while i < CONTACT_SLOTS {
            match self.contacts[i] {
                Some(slot) if slot.peer == peer => return Err(QdnfError::Conflict),
                Some(_) => {}
                None => {
                    if free.is_none() {
                        free = Some(i);
                    }
                }
            }
            i += 1;
        }
        let idx = free.ok_or(QdnfError::Capacity)?;
        self.contacts[idx] = Some(ContactSlot {
            peer,
            state: ContactState::Request,
            mandate_generation,
            purpose,
            paid,
            revoked: false,
        });
        Ok(())
    }

    /// Request → Consent. Already Consent/Active is idempotent.
    pub fn consent(&mut self, peer: StrongDigest) -> Result<(), QdnfError> {
        let i = self.index_of(peer)?;
        let slot = self.contacts[i].as_mut().ok_or(QdnfError::Unauthorized)?;
        match slot.state {
            ContactState::Request | ContactState::Consent => {
                slot.state = ContactState::Consent;
                Ok(())
            }
            ContactState::Active => Ok(()),
            ContactState::Suspended | ContactState::Blocked => Err(QdnfError::Denied),
        }
    }

    /// Consent → Active. Request cannot skip Consent.
    pub fn activate(&mut self, peer: StrongDigest) -> Result<(), QdnfError> {
        let i = self.index_of(peer)?;
        let slot = self.contacts[i].as_mut().ok_or(QdnfError::Unauthorized)?;
        match slot.state {
            ContactState::Request => Err(QdnfError::Unauthorized),
            ContactState::Consent | ContactState::Active => {
                slot.state = ContactState::Active;
                Ok(())
            }
            ContactState::Suspended | ContactState::Blocked => Err(QdnfError::Denied),
        }
    }

    /// Active → Suspended.
    pub fn suspend(&mut self, peer: StrongDigest) -> Result<(), QdnfError> {
        let i = self.index_of(peer)?;
        let slot = self.contacts[i].as_mut().ok_or(QdnfError::Unauthorized)?;
        match slot.state {
            ContactState::Active | ContactState::Suspended => {
                slot.state = ContactState::Suspended;
                Ok(())
            }
            ContactState::Blocked => Err(QdnfError::Denied),
            ContactState::Request | ContactState::Consent => Err(QdnfError::Unauthorized),
        }
    }

    /// Any live slot → Blocked. Payment on the slot is left observational.
    pub fn block(&mut self, peer: StrongDigest) -> Result<(), QdnfError> {
        let i = self.index_of(peer)?;
        let slot = self.contacts[i].as_mut().ok_or(QdnfError::Unauthorized)?;
        slot.state = ContactState::Blocked;
        Ok(())
    }
}

impl Default for ContactTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Payment never unblocks, unsuspends, or un-revokes.
#[inline]
pub fn payment_bypasses_block() -> bool {
    false
}

/// Group membership is not private-message consent (NET-05.19 claim only).
#[inline]
pub fn group_membership_is_private_consent() -> bool {
    false
}

/// Admit service for a contact using the current operation-specific grant.
///
/// `paid` is ignored for Blocked, Suspended, and Revoked. Purpose on the
/// grant must match `purpose` and the stored slot purpose.
pub fn admit_contact(
    table: &ContactTable,
    peer: StrongDigest,
    purpose: StrongDigest,
    paid: bool,
    now: u64,
    grant: &TemporalGrant,
) -> Result<(), QdnfError> {
    let _ = paid;
    debug_assert!(!payment_bypasses_block());
    let slot = table.get(peer)?;
    if slot.revoked {
        return Err(QdnfError::Revoked);
    }
    if matches!(slot.state, ContactState::Blocked | ContactState::Suspended) {
        return gate_channel(PolicyOutcome::Allow, true, slot.state);
    }
    if grant.purpose_digest != purpose || slot.purpose != purpose {
        return Err(QdnfError::Unauthorized);
    }
    if slot.state != ContactState::Active {
        return Err(QdnfError::Unauthorized);
    }
    let current = match grant.current_at(now) {
        Ok(()) => true,
        Err(QdnfError::Expired) => false,
        Err(e) => return Err(e),
    };
    gate_channel(PolicyOutcome::Allow, current, slot.state)?;
    admit_service(PolicyOutcome::Allow, current)
}

/// Intermediary path: any Blocked or Suspended hop is Denied.
/// A later hop cannot authorize the path by paying.
pub fn admit_path(states: &[ContactState]) -> Result<(), QdnfError> {
    if states.is_empty() {
        return Err(QdnfError::Unauthorized);
    }
    let mut i = 0usize;
    while i < states.len() {
        match states[i] {
            ContactState::Blocked | ContactState::Suspended => return Err(QdnfError::Denied),
            ContactState::Request | ContactState::Consent | ContactState::Active => {}
        }
        i += 1;
    }
    Ok(())
}

/// Guardian / organizational mandate revoke. Stale generation is unchanged.
/// Current generation sets Blocked (guardian revoke) and marks revoked.
pub fn revoke_mandate(
    table: &mut ContactTable,
    peer: StrongDigest,
    generation: u64,
) -> Result<(), QdnfError> {
    let i = table.index_of(peer)?;
    let slot = table.contacts[i].as_mut().ok_or(QdnfError::Unauthorized)?;
    if slot.mandate_generation == 0 || slot.mandate_generation != generation {
        return Err(QdnfError::StaleGeneration);
    }
    slot.state = ContactState::Blocked;
    slot.revoked = true;
    Ok(())
}

/// Reserved help/revocation admit. Independent of the 16 contact slots.
/// Does not grant private-message consent.
pub fn admit_help(table: &mut ContactTable, peer: StrongDigest) -> Result<(), QdnfError> {
    if peer == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    debug_assert!(!group_membership_is_private_consent());
    let mut free = None;
    let mut i = 0usize;
    while i < REVOKED_HELP_SLOTS {
        match table.help[i] {
            Some(existing) if existing == peer => return Ok(()),
            Some(_) => {}
            None => {
                if free.is_none() {
                    free = Some(i);
                }
            }
        }
        i += 1;
    }
    let idx = free.ok_or(QdnfError::Capacity)?;
    table.help[idx] = Some(peer);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::types::ProfileId;

    fn digest(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d
    }

    fn grant(purpose: StrongDigest, expires_unix: u64) -> TemporalGrant {
        TemporalGrant {
            purpose_digest: purpose,
            audience_digest: StrongDigest::ZERO,
            authority_generation: 1,
            not_before_unix: 0,
            expires_unix,
            profile: ProfileId::QDNF_CRYPTO_1,
        }
    }

    fn active_peer(table: &mut ContactTable, tag: u8, mandate: u64, paid: bool) -> StrongDigest {
        let peer = digest(tag);
        let purpose = digest(200);
        table
            .insert_request(peer, purpose, paid, mandate)
            .expect("request");
        table.consent(peer).expect("consent");
        table.activate(peer).expect("active");
        peer
    }

    #[test]
    fn request_cannot_activate_without_consent() {
        let mut table = ContactTable::new();
        let peer = digest(1);
        let purpose = digest(200);
        table
            .insert_request(peer, purpose, false, 0)
            .expect("request");
        assert_eq!(table.activate(peer), Err(QdnfError::Unauthorized));
        assert_eq!(table.get(peer).unwrap().state, ContactState::Request);
        table.consent(peer).expect("consent");
        table.activate(peer).expect("after consent");
        assert_eq!(table.get(peer).unwrap().state, ContactState::Active);
    }

    #[test]
    fn blocked_paid_allow_is_denied() {
        let mut table = ContactTable::new();
        let peer = active_peer(&mut table, 2, 0, true);
        table.block(peer).expect("block");
        let purpose = digest(200);
        let g = grant(purpose, 1_000);
        assert_eq!(
            admit_contact(&table, peer, purpose, true, 10, &g),
            Err(QdnfError::Denied)
        );
        assert!(!payment_bypasses_block());
    }

    #[test]
    fn suspended_paid_is_denied() {
        let mut table = ContactTable::new();
        let peer = active_peer(&mut table, 3, 0, true);
        table.suspend(peer).expect("suspend");
        let purpose = digest(200);
        let g = grant(purpose, 1_000);
        assert_eq!(
            admit_contact(&table, peer, purpose, true, 10, &g),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn active_current_grant_allow_admits() {
        let mut table = ContactTable::new();
        let peer = active_peer(&mut table, 4, 0, false);
        let purpose = digest(200);
        let g = grant(purpose, 1_000);
        assert!(admit_contact(&table, peer, purpose, false, 10, &g).is_ok());
    }

    #[test]
    fn expired_grant_is_expired_when_active() {
        let mut table = ContactTable::new();
        let peer = active_peer(&mut table, 5, 0, false);
        let purpose = digest(200);
        let g = grant(purpose, 50);
        assert_eq!(
            admit_contact(&table, peer, purpose, false, 50, &g),
            Err(QdnfError::Expired)
        );
        assert_eq!(
            admit_contact(&table, peer, purpose, true, 80, &g),
            Err(QdnfError::Expired)
        );
    }

    #[test]
    fn path_with_blocked_intermediary_is_denied() {
        let blocked = [
            ContactState::Active,
            ContactState::Blocked,
            ContactState::Active,
        ];
        assert_eq!(admit_path(&blocked), Err(QdnfError::Denied));
        let suspended = [ContactState::Active, ContactState::Suspended];
        assert_eq!(admit_path(&suspended), Err(QdnfError::Denied));
        let ok = [ContactState::Active, ContactState::Active];
        assert!(admit_path(&ok).is_ok());
    }

    #[test]
    fn mandate_revoke_then_admit_is_revoked() {
        let mut table = ContactTable::new();
        let peer = active_peer(&mut table, 6, 7, false);
        revoke_mandate(&mut table, peer, 7).expect("revoke");
        assert_eq!(table.get(peer).unwrap().state, ContactState::Blocked);
        let purpose = digest(200);
        let g = grant(purpose, 1_000);
        assert_eq!(
            admit_contact(&table, peer, purpose, true, 10, &g),
            Err(QdnfError::Revoked)
        );
    }

    #[test]
    fn stale_mandate_generation_is_stale() {
        let mut table = ContactTable::new();
        let peer = active_peer(&mut table, 7, 9, false);
        assert_eq!(
            revoke_mandate(&mut table, peer, 8),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(table.get(peer).unwrap().state, ContactState::Active);
        let purpose = digest(200);
        let g = grant(purpose, 1_000);
        assert!(admit_contact(&table, peer, purpose, false, 10, &g).is_ok());
    }

    #[test]
    fn seventeenth_contact_is_capacity_help_still_admits() {
        let mut table = ContactTable::new();
        let purpose = digest(200);
        let mut tag = 1u8;
        while tag <= CONTACT_SLOTS as u8 {
            table
                .insert_request(digest(tag), purpose, false, 0)
                .expect("fill");
            tag += 1;
        }
        assert_eq!(table.contact_occupied(), CONTACT_SLOTS);
        assert_eq!(table.help_occupied(), 0);
        assert_eq!(
            table.insert_request(digest(17), purpose, false, 0),
            Err(QdnfError::Capacity)
        );
        assert_eq!(table.contact_occupied(), CONTACT_SLOTS);
        assert!(admit_help(&mut table, digest(100)).is_ok());
        assert_eq!(table.help_occupied(), 1);
        assert_eq!(table.contact_occupied(), CONTACT_SLOTS);
    }

    #[test]
    fn payment_does_not_bypass_block() {
        assert!(!payment_bypasses_block());
        assert!(!group_membership_is_private_consent());
    }
}
