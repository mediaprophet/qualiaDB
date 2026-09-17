//! Named falsifiable experiments from the fabric design §7.

use super::carrier::PathClass;
use super::connect::{connect, drive, Fabric};
use super::contact::ContactDescriptor;
use super::intent::{ProtectionPolicy, Purpose};
use super::kernel::{FabricState, KernelEffect, KernelEvent};
use super::lease::RelayLease;
use super::receipt::ReceiptStatus;
use crate::net::peer::runtime::ResourceBudget;

fn budget() -> ResourceBudget {
    ResourceBudget {
        bytes: 1024,
        work: 4,
        io: 4,
    }
}

/// Valid signature, old generation: do not probe the locator.
pub fn compromised_discovery_stale() -> KernelEffect {
    let mut f = Fabric::new();
    connect(
        &mut f,
        [9u8; 32],
        Purpose::ordinary(),
        ProtectionPolicy::RELAY_ONLY,
        budget(),
        0,
        8_000,
    )
    .unwrap();
    let last = ContactDescriptor::mailbox([9u8; 32], 5, 2_000_000_000);
    f.kernel.note_contact(&last);
    let stale = ContactDescriptor::mailbox([9u8; 32], 3, 2_000_000_000);
    assert!(stale.is_stale(f.kernel.last_contact_generation));
    f.kernel.step(
        KernelEvent::Descriptor {
            stale: stale.is_stale(f.kernel.last_contact_generation),
            expired: false,
            direct_locator: stale.is_direct_locator(),
        },
        1,
    )
}

/// Lease expiry mid-transfer queues the remainder and does not enlarge the cap.
pub fn lease_expiry_mid_transfer() -> (u64, FabricState) {
    let mut lease = RelayLease::grant(3, 1, [1u8; 32], [2u8; 32], 32, 100, 1, false);
    assert!(lease.charge(16, 10).is_ok());
    lease.expiry_ms = 10;
    assert!(lease.charge(8, 11).is_err());
    assert_eq!(lease.remaining_bytes, 16);
    let mut f = Fabric::new();
    connect(
        &mut f,
        [2u8; 32],
        Purpose::ordinary(),
        ProtectionPolicy::RELAY_ONLY,
        budget(),
        0,
        8_000,
    )
    .unwrap();
    drive(&mut f, KernelEvent::LeaseLive, 1);
    drive(&mut f, KernelEvent::LeaseExpired, 11);
    (lease.remaining_bytes, f.kernel.state)
}

/// Interface change while direct probing is forbidden: probe count stays zero.
pub fn network_switch_relay_only() -> u32 {
    let mut f = Fabric::new();
    connect(
        &mut f,
        [4u8; 32],
        Purpose::ordinary(),
        ProtectionPolicy::RELAY_ONLY,
        budget(),
        0,
        8_000,
    )
    .unwrap();
    drive(
        &mut f,
        KernelEvent::Descriptor {
            stale: false,
            expired: false,
            direct_locator: false,
        },
        1,
    );
    drive(&mut f, KernelEvent::LeaseLive, 1);
    drive(
        &mut f,
        KernelEvent::PathValidated {
            class: PathClass::Relayed,
        },
        2,
    );
    let before = f.kernel.direct_probe_count;
    drive(&mut f, KernelEvent::NetworkGenerationChanged, 3);
    assert_eq!(before, 0);
    f.kernel.direct_probe_count
}

/// Queued mutation after grant revocation cannot commit.
pub fn revoked_grant_replay() -> ReceiptStatus {
    let mut f = Fabric::new();
    connect(
        &mut f,
        [8u8; 32],
        Purpose::ordinary(),
        ProtectionPolicy::ISOLATED,
        budget(),
        0,
        8_000,
    )
    .unwrap();
    let i = f.enqueue(77, [0x11u8; 32], [8u8; 32], 500).unwrap();
    drive(&mut f, KernelEvent::GrantRevoked, 2);
    f.replay(i, 77, &[0x11u8; 32], 10).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn experiment_compromised_discovery() {
        assert_eq!(compromised_discovery_stale(), KernelEffect::Deny);
    }

    #[test]
    fn experiment_lease_expiry() {
        let (left, state) = lease_expiry_mid_transfer();
        assert_eq!(left, 16);
        assert_eq!(state, FabricState::OfflineQueued);
    }

    #[test]
    fn experiment_network_switch() {
        assert_eq!(network_switch_relay_only(), 0);
    }

    #[test]
    fn experiment_revoked_replay() {
        assert_eq!(revoked_grant_replay(), ReceiptStatus::Denied);
    }
}
