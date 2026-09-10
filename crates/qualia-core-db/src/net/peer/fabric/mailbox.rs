//! Invitation-scoped private mailbox. Not a public DHT.

use super::carrier::prohibited;
use super::contact::{ContactDescriptor, LocatorKind};
use super::intent::PeerId;
use super::kernel::FabricError;
use super::wire::{decode_contact, encode_contact};
use crate::net::peer::connectivity::policy::Disclosure;

pub const MAX_SLOTS: usize = 16;

#[derive(Debug, Clone, Copy)]
pub struct PrivateMailbox {
    slots: [Option<ContactDescriptor>; MAX_SLOTS],
    last_gen: [u32; MAX_SLOTS],
    len: usize,
}

impl PrivateMailbox {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_SLOTS],
            last_gen: [0; MAX_SLOTS],
            len: 0,
        }
    }

    fn find(&self, key: &PeerId) -> Option<usize> {
        let mut i = 0;
        while i < self.len {
            if let Some(d) = self.slots[i] {
                if &d.contact_key == key {
                    return Some(i);
                }
            }
            i += 1;
        }
        None
    }

    pub fn publish(&mut self, d: ContactDescriptor) -> Result<(), FabricError> {
        if d.contact_key == [0u8; 32] {
            return Err(FabricError::Illegal);
        }
        if let Some(i) = self.find(&d.contact_key) {
            if d.generation < self.last_gen[i] {
                return Err(FabricError::StaleDescriptor);
            }
            self.slots[i] = Some(d);
            self.last_gen[i] = d.generation;
            return Ok(());
        }
        if self.len >= MAX_SLOTS {
            return Err(FabricError::Capacity);
        }
        self.slots[self.len] = Some(d);
        self.last_gen[self.len] = d.generation;
        self.len += 1;
        Ok(())
    }

    pub fn resolve(
        &self,
        key: &PeerId,
        now_unix: u32,
        disclosure: Disclosure,
    ) -> Result<ContactDescriptor, FabricError> {
        let i = self.find(key).ok_or(FabricError::Illegal)?;
        let d = self.slots[i].ok_or(FabricError::Illegal)?;
        if d.is_expired(now_unix) {
            return Err(FabricError::Expired);
        }
        if d.is_stale(self.last_gen[i]) {
            return Err(FabricError::StaleDescriptor);
        }
        if d.kind == LocatorKind::Direct && prohibited(disclosure, super::carrier::PathClass::DirectV6)
        {
            return Err(FabricError::PolicyDenied);
        }
        Ok(d)
    }

    pub fn ingest_wire(
        &mut self,
        bytes: &[u8],
        now_unix: u32,
        disclosure: Disclosure,
    ) -> Result<ContactDescriptor, FabricError> {
        let d = decode_contact(bytes)?;
        if d.is_expired(now_unix) {
            return Err(FabricError::Expired);
        }
        self.publish(d)?;
        self.resolve(&d.contact_key, now_unix, disclosure)
    }

    pub fn encode_current(&self, key: &PeerId, out: &mut [u8]) -> Result<usize, FabricError> {
        let i = self.find(key).ok_or(FabricError::Illegal)?;
        let d = self.slots[i].ok_or(FabricError::Illegal)?;
        encode_contact(&d, out).map_err(FabricError::from)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stale_and_direct_forbidden() {
        let mut m = PrivateMailbox::new();
        let key = [7u8; 32];
        m.publish(ContactDescriptor::mailbox(key, 5, 2_000_000_000))
            .unwrap();
        let old = ContactDescriptor::mailbox(key, 3, 2_000_000_000);
        assert_eq!(m.publish(old), Err(FabricError::StaleDescriptor));
        let mut direct = ContactDescriptor::mailbox(key, 6, 2_000_000_000);
        direct.kind = LocatorKind::Direct;
        m.publish(direct).unwrap();
        assert_eq!(
            m.resolve(&key, 10, Disclosure::ApprovedRelaysOnly),
            Err(FabricError::PolicyDenied)
        );
        assert!(m.resolve(&key, 10, Disclosure::DirectPermitted).is_ok());
        let expired = ContactDescriptor::mailbox([8u8; 32], 1, 5);
        m.publish(expired).unwrap();
        assert_eq!(
            m.resolve(&[8u8; 32], 10, Disclosure::ApprovedRelaysOnly),
            Err(FabricError::Expired)
        );
        let mut wire = [0u8; 256];
        let n = m
            .encode_current(&key, &mut wire)
            .expect("direct locator still stored");
        let mut other = PrivateMailbox::new();
        assert_eq!(
            other.ingest_wire(&wire[..n], 10, Disclosure::ApprovedRelaysOnly),
            Err(FabricError::PolicyDenied)
        );
        assert!(!crate::net::peer::fabric::intent::ProtectionPolicy::RELAY_ONLY.public_dht);
    }
}
