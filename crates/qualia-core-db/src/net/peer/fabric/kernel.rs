//! Bounded fabric kernel. Same admitted inputs produce the same effects.

use super::carrier::{prohibited, PathClass};
use super::contact::ContactDescriptor;
use super::intent::ConnectionIntent;
use super::lease::RelayLease;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FabricError {
    PolicyDenied,
    StaleDescriptor,
    Expired,
    Capacity,
    LeaseDead,
    GrantRevoked,
    Illegal,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FabricState {
    Idle,
    IntentLive,
    ContactResolved,
    LeaseHeld,
    PathLive,
    SessionLive,
    OfflineQueued,
    Closed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelEvent {
    IntentAdmitted,
    Descriptor {
        stale: bool,
        expired: bool,
        direct_locator: bool,
    },
    LeaseLive,
    LeaseExpired,
    PathValidated {
        class: PathClass,
    },
    NetworkGenerationChanged,
    GrantRevoked,
    SessionAuthenticated,
    Deadline,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KernelEffect {
    None,
    Probe {
        class: PathClass,
    },
    Exclude {
        class: PathClass,
    },
    QueueOffline,
    AdmitSession,
    Close,
    Deny,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Kernel {
    pub state: FabricState,
    pub generation: u32,
    pub last_contact_generation: u32,
    pub grant_live: bool,
    pub grant_generation: u32,
    pub direct_probe_count: u32,
    pub selected: Option<PathClass>,
    intent: Option<ConnectionIntent>,
    lease_live: bool,
}

impl Kernel {
    pub const fn new() -> Self {
        Self {
            state: FabricState::Idle,
            generation: 1,
            last_contact_generation: 0,
            grant_live: true,
            grant_generation: 1,
            direct_probe_count: 0,
            selected: None,
            intent: None,
            lease_live: false,
        }
    }

    pub const fn intent(&self) -> Option<ConnectionIntent> {
        self.intent
    }

    pub fn admit(&mut self, intent: ConnectionIntent, now_ms: u64) -> Result<(), FabricError> {
        if self.state != FabricState::Idle && self.state != FabricState::Closed {
            return Err(FabricError::Illegal);
        }
        if intent.expired(now_ms) {
            return Err(FabricError::Expired);
        }
        self.intent = Some(intent);
        self.state = FabricState::IntentLive;
        self.grant_live = true;
        Ok(())
    }

    pub fn note_contact(&mut self, d: &ContactDescriptor) {
        if d.generation > self.last_contact_generation {
            self.last_contact_generation = d.generation;
        }
    }

    pub fn step(&mut self, event: KernelEvent, now_ms: u64) -> KernelEffect {
        let Some(intent) = self.intent else {
            return KernelEffect::Deny;
        };
        if intent.expired(now_ms) {
            self.state = FabricState::OfflineQueued;
            return KernelEffect::QueueOffline;
        }
        let disclosure = intent.protection.disclosure;
        match event {
            KernelEvent::IntentAdmitted => {
                if self.state == FabricState::IntentLive {
                    KernelEffect::None
                } else {
                    KernelEffect::Deny
                }
            }
            KernelEvent::Descriptor {
                stale,
                expired,
                direct_locator,
            } => {
                if stale || expired {
                    self.state = FabricState::OfflineQueued;
                    return KernelEffect::Deny;
                }
                if direct_locator && prohibited(disclosure, PathClass::DirectV6) {
                    return KernelEffect::Exclude {
                        class: PathClass::DirectV6,
                    };
                }
                self.state = FabricState::ContactResolved;
                if prohibited(disclosure, PathClass::Relayed) {
                    self.state = FabricState::OfflineQueued;
                    KernelEffect::QueueOffline
                } else {
                    KernelEffect::Probe {
                        class: PathClass::Relayed,
                    }
                }
            }
            KernelEvent::LeaseLive => {
                self.lease_live = true;
                self.state = FabricState::LeaseHeld;
                KernelEffect::None
            }
            KernelEvent::LeaseExpired => {
                self.lease_live = false;
                self.selected = None;
                self.state = FabricState::OfflineQueued;
                KernelEffect::QueueOffline
            }
            KernelEvent::PathValidated { class } => {
                if prohibited(disclosure, class) {
                    return KernelEffect::Exclude { class };
                }
                if class.discloses_peer_ip() {
                    self.direct_probe_count = self.direct_probe_count.saturating_add(1);
                }
                self.selected = Some(class);
                self.state = FabricState::PathLive;
                KernelEffect::None
            }
            KernelEvent::NetworkGenerationChanged => {
                self.generation = self.generation.wrapping_add(1);
                if self.generation == 0 {
                    self.generation = 1;
                }
                self.selected = None;
                if prohibited(disclosure, PathClass::DirectV6) {
                    if self.lease_live {
                        self.state = FabricState::LeaseHeld;
                        KernelEffect::Probe {
                            class: PathClass::Relayed,
                        }
                    } else if prohibited(disclosure, PathClass::Relayed) {
                        self.state = FabricState::OfflineQueued;
                        KernelEffect::QueueOffline
                    } else {
                        KernelEffect::Probe {
                            class: PathClass::Relayed,
                        }
                    }
                } else {
                    KernelEffect::Probe {
                        class: PathClass::Relayed,
                    }
                }
            }
            KernelEvent::GrantRevoked => {
                self.grant_live = false;
                self.state = FabricState::Closed;
                KernelEffect::Deny
            }
            KernelEvent::SessionAuthenticated => {
                if self.state != FabricState::PathLive || !self.grant_live {
                    return KernelEffect::Deny;
                }
                self.state = FabricState::SessionLive;
                KernelEffect::AdmitSession
            }
            KernelEvent::Deadline => {
                self.state = FabricState::OfflineQueued;
                KernelEffect::QueueOffline
            }
        }
    }

    pub fn lease_still_usable(&self, lease: &RelayLease, now_ms: u64) -> bool {
        self.lease_live && lease.live(now_ms)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::fabric::intent::{ProtectionPolicy, Purpose};
    use crate::net::peer::runtime::ResourceBudget;

    fn intent(p: ProtectionPolicy) -> ConnectionIntent {
        ConnectionIntent::new(
            [1u8; 32],
            Purpose::ordinary(),
            p,
            ResourceBudget {
                bytes: 4096,
                work: 8,
                io: 8,
            },
            0,
            10_000,
        )
    }

    #[test]
    fn stale_descriptor_denies_without_direct_probe() {
        let mut k = Kernel::new();
        k.admit(intent(ProtectionPolicy::RELAY_ONLY), 0).unwrap();
        k.last_contact_generation = 5;
        let e = k.step(
            KernelEvent::Descriptor {
                stale: true,
                expired: false,
                direct_locator: true,
            },
            1,
        );
        assert_eq!(e, KernelEffect::Deny);
        assert_eq!(k.direct_probe_count, 0);
    }

    #[test]
    fn relay_only_excludes_validated_direct() {
        let mut k = Kernel::new();
        k.admit(intent(ProtectionPolicy::RELAY_ONLY), 0).unwrap();
        k.step(
            KernelEvent::Descriptor {
                stale: false,
                expired: false,
                direct_locator: false,
            },
            1,
        );
        k.step(KernelEvent::LeaseLive, 1);
        let e = k.step(
            KernelEvent::PathValidated {
                class: PathClass::DirectV6,
            },
            2,
        );
        assert_eq!(
            e,
            KernelEffect::Exclude {
                class: PathClass::DirectV6
            }
        );
        assert_eq!(k.direct_probe_count, 0);
        assert_ne!(k.state, FabricState::PathLive);
    }
}
