//! Read-only actual canonical handle headroom; numeric caps are not authority.
use super::ReservationLedger;
impl ReservationLedger {
    /// Vacancies in THIS actual ledger, including every retained external owner.
    pub fn available_handles(&self) -> usize {
        self.handles.iter().filter(|slot| !slot.occupied).count()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::ResourceBudget;
    #[test]
    fn counts_live_handles_not_budget_values() {
        let cap = ResourceBudget {
            bytes: 1000,
            work: 1000,
            io: 1000,
        };
        let mut ledger = ReservationLedger::new(cap, cap, cap, cap, cap);
        assert_eq!(ledger.available_handles(), 32);
        let handle = ledger
            .reserve(
                ResourceBudget {
                    bytes: 1,
                    work: 1,
                    io: 1,
                },
                true,
            )
            .unwrap();
        assert_eq!(ledger.available_handles(), 31);
        ledger.release(handle).unwrap();
        assert_eq!(ledger.available_handles(), 32);
    }
}
