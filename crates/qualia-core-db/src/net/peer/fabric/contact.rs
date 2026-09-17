//! Private contact descriptor. Discovery, not traversal, and not concealment.

use super::intent::PeerId;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocatorKind {
    /// Relationship-scoped mailbox. No access-network IP.
    Mailbox = 1,
    /// Approved relay hint. Not a peer IP.
    RelayHint = 2,
    /// Direct locator. Only usable when disclosure permits probing.
    Direct = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContactDescriptor {
    pub contact_key: PeerId,
    pub generation: u32,
    pub expiry_unix: u32,
    pub network_generation: u32,
    pub kind: LocatorKind,
    pub locator: [u8; 16],
}

impl ContactDescriptor {
    pub const fn mailbox(contact_key: PeerId, generation: u32, expiry_unix: u32) -> Self {
        Self {
            contact_key,
            generation,
            expiry_unix,
            network_generation: 1,
            kind: LocatorKind::Mailbox,
            locator: [0u8; 16],
        }
    }

    pub const fn is_expired(&self, now_unix: u32) -> bool {
        now_unix >= self.expiry_unix
    }

    /// A valid signature on an older generation is stale, not current.
    pub const fn is_stale(&self, last_known_generation: u32) -> bool {
        last_known_generation != 0 && self.generation < last_known_generation
    }

    pub const fn is_direct_locator(&self) -> bool {
        matches!(self.kind, LocatorKind::Direct)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_generation_is_stale() {
        let d = ContactDescriptor::mailbox([7u8; 32], 3, 2_000_000_000);
        assert!(d.is_stale(5));
        assert!(!d.is_stale(3));
        assert!(!d.is_stale(0));
        assert!(d.is_expired(2_000_000_000));
        assert!(!d.is_expired(1_700_000_000));
    }
}
