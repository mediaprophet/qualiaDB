//! Admission helpers: essential reclaim floor and transient unknown-sender charge.
//!
//! Unknown senders charge the ledger transient pool first (`verified_peer = false`).
//! Data-class work cannot consume the essential floor reserved for cancel,
//! completion, and reclamation progress.

use crate::net::qdnf::errors::QdnfError;

use super::ledger::{ReservationLedger, ResourceBudget};
use super::scheduler::WorkClass;

pub struct ResourceGovernor {
    essential_floor_bytes: u64,
}

impl ResourceGovernor {
    pub const fn new(essential_floor_bytes: u64) -> Self {
        Self {
            essential_floor_bytes,
        }
    }

    /// Charge an unknown sender against the transient pool (not a verified peer).
    pub fn charge_unknown(
        ledger: &mut ReservationLedger,
        add: ResourceBudget,
    ) -> Result<crate::net::peer::runtime::ReservationHandle, QdnfError> {
        ledger.reserve(add, false)
    }

    /// Admit `add` then reserve it. Control may use the essential floor; other
    /// classes must leave that floor free on the host.
    pub fn admit_and_reserve(
        &self,
        ledger: &mut ReservationLedger,
        add: ResourceBudget,
        class: WorkClass,
        verified_peer: bool,
    ) -> Result<crate::net::peer::runtime::ReservationHandle, QdnfError> {
        if class != WorkClass::Control {
            let remaining = ledger.host_remaining_bytes();
            if remaining.saturating_sub(add.bytes) < self.essential_floor_bytes {
                return Err(QdnfError::BudgetExhausted);
            }
        }
        ledger.reserve(add, verified_peer)
    }
}

impl Default for ResourceGovernor {
    fn default() -> Self {
        Self::new(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ledger(bytes: u64) -> ReservationLedger {
        let cap = ResourceBudget {
            bytes,
            work: 16,
            io: 16,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    #[test]
    fn unknown_sender_charges_transient_pool() {
        let mut ledger = ledger(100);
        let add = ResourceBudget {
            bytes: 60,
            work: 1,
            io: 1,
        };
        ResourceGovernor::charge_unknown(&mut ledger, add).unwrap();
        assert_eq!(ledger.transient_used().bytes, 60);
        assert_eq!(
            ResourceGovernor::charge_unknown(&mut ledger, add),
            Err(QdnfError::BudgetExhausted)
        );
    }

    #[test]
    fn data_cannot_consume_essential_floor() {
        let mut ledger = ledger(100);
        let gov = ResourceGovernor::new(20);
        let add = ResourceBudget {
            bytes: 90,
            work: 1,
            io: 1,
        };
        assert_eq!(
            gov.admit_and_reserve(&mut ledger, add, WorkClass::Background, true),
            Err(QdnfError::BudgetExhausted)
        );
        gov.admit_and_reserve(&mut ledger, add, WorkClass::Control, true)
            .unwrap();
        assert_eq!(ledger.used().host.bytes, 90);
    }
}
