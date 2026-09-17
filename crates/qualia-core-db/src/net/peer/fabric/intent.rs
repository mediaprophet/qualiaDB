//! Connection intent: who, why, under which constraints. Not reachability.

use crate::net::peer::connectivity::policy::Disclosure;
use crate::net::peer::runtime::ResourceBudget;
use crate::q_hash;

pub type PeerId = [u8; 32];

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PurposeClass {
    Ordinary = 1,
    Clinical = 2,
    Infrastructure = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Purpose {
    pub iri_hash: u64,
    pub class: PurposeClass,
}

impl Purpose {
    pub const fn ordinary() -> Self {
        Self {
            iri_hash: q_hash("q42:purpose/ordinary"),
            class: PurposeClass::Ordinary,
        }
    }

    pub const fn clinical() -> Self {
        Self {
            iri_hash: q_hash("q42:purpose/clinical"),
            class: PurposeClass::Clinical,
        }
    }
}

/// Disclosure plus whether public DHT publication is admitted for this call.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ProtectionPolicy {
    pub disclosure: Disclosure,
    pub require_e2e_session: bool,
    pub public_dht: bool,
}

impl ProtectionPolicy {
    pub const ORDINARY: Self = Self {
        disclosure: Disclosure::DirectPermitted,
        require_e2e_session: true,
        public_dht: false,
    };

    pub const RELAY_ONLY: Self = Self {
        disclosure: Disclosure::ApprovedRelaysOnly,
        require_e2e_session: true,
        public_dht: false,
    };

    pub const ISOLATED: Self = Self {
        disclosure: Disclosure::Isolated,
        require_e2e_session: true,
        public_dht: false,
    };
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionIntent {
    pub peer: PeerId,
    pub purpose: Purpose,
    pub authority: u64,
    pub protection: ProtectionPolicy,
    pub deadline_ms: u64,
    pub budget: ResourceBudget,
    pub created_ms: u64,
}

impl ConnectionIntent {
    pub const fn new(
        peer: PeerId,
        purpose: Purpose,
        protection: ProtectionPolicy,
        budget: ResourceBudget,
        now_ms: u64,
        deadline_ms: u64,
    ) -> Self {
        Self {
            peer,
            purpose,
            authority: 0,
            protection,
            deadline_ms,
            budget,
            created_ms: now_ms,
        }
    }

    pub const fn expired(&self, now_ms: u64) -> bool {
        now_ms >= self.deadline_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordinary_does_not_enable_public_dht() {
        assert!(!ProtectionPolicy::ORDINARY.public_dht);
        assert!(!ProtectionPolicy::RELAY_ONLY.public_dht);
        assert_ne!(Purpose::ordinary().iri_hash, Purpose::clinical().iri_hash);
    }
}
