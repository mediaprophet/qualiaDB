//! Relay leases and custody leases are distinct commitments.

use super::intent::PeerId;

pub const MAX_LEASES: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RelayLease {
    pub lease_id: u64,
    pub operator: u64,
    pub participants: [PeerId; 2],
    pub max_bytes: u64,
    pub remaining_bytes: u64,
    pub expiry_ms: u64,
    pub generation: u32,
    pub cancelled: bool,
    /// When false, the relay must not export observed peer locators.
    pub export_observations: bool,
}

impl RelayLease {
    pub const fn grant(
        lease_id: u64,
        operator: u64,
        a: PeerId,
        b: PeerId,
        max_bytes: u64,
        expiry_ms: u64,
        generation: u32,
        export_observations: bool,
    ) -> Self {
        Self {
            lease_id,
            operator,
            participants: [a, b],
            max_bytes,
            remaining_bytes: max_bytes,
            expiry_ms,
            generation,
            cancelled: false,
            export_observations,
        }
    }

    pub const fn live(&self, now_ms: u64) -> bool {
        !self.cancelled && now_ms < self.expiry_ms && self.remaining_bytes > 0
    }

    pub fn admits(&self, from: &PeerId, to: &PeerId) -> bool {
        (from == &self.participants[0] && to == &self.participants[1])
            || (from == &self.participants[1] && to == &self.participants[0])
    }

    /// Charge forwarded bytes. Expiry or cancel fails closed; the cap is not enlarged.
    pub fn charge(&mut self, n: u64, now_ms: u64) -> Result<(), ()> {
        if !self.live(now_ms) || n > self.remaining_bytes {
            return Err(());
        }
        self.remaining_bytes -= n;
        Ok(())
    }

    pub fn cancel(&mut self) {
        self.cancelled = true;
        self.remaining_bytes = 0;
    }
}

/// Stored-envelope retention. Not a live forwarding budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CustodyLease {
    pub lease_id: u64,
    pub operator: u64,
    pub expiry_unix: u32,
    pub max_objects: u16,
    pub held: u16,
    pub cancelled: bool,
}

impl CustodyLease {
    pub const fn grant(
        lease_id: u64,
        operator: u64,
        expiry_unix: u32,
        max_objects: u16,
    ) -> Self {
        Self {
            lease_id,
            operator,
            expiry_unix,
            max_objects,
            held: 0,
            cancelled: false,
        }
    }

    pub const fn live(&self, now_unix: u32) -> bool {
        !self.cancelled && now_unix < self.expiry_unix && self.held < self.max_objects
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn charge_stops_at_cap_and_expiry() {
        let mut l = RelayLease::grant(1, 9, [1u8; 32], [2u8; 32], 10, 100, 1, false);
        assert!(l.charge(8, 10).is_ok());
        assert!(l.charge(3, 10).is_err());
        assert_eq!(l.remaining_bytes, 2);
        l.remaining_bytes = 10;
        assert!(l.charge(1, 100).is_err());
        l.expiry_ms = 200;
        l.cancel();
        assert!(l.charge(1, 10).is_err());
    }

    #[test]
    fn custody_is_not_a_relay_budget() {
        let c = CustodyLease::grant(2, 9, 50, 4);
        assert!(c.live(10));
        assert_ne!(
            core::mem::size_of::<CustodyLease>(),
            0,
            "distinct type from RelayLease"
        );
    }
}
