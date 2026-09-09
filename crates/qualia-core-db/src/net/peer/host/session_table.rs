//! Bounded session table. Multiple live sessions; close releases the ledger handle.

use crate::net::peer::runtime::{ReservationHandle, ReservationLedger};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::streams::StreamFrame;
use crate::net::qdnf::session::{ProtectedAckSession, SessionBinding, StreamState};
use crate::net::qdnf::types::Generation;

/// Occupied application sessions per peer. A fifth insert is [`QdnfError::Capacity`].
pub const MAX_SESSIONS: usize = 4;

/// Copyable slot+generation index. The table owns the reservation handle.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SessionHandle {
    slot: u8,
    generation: Generation,
}

impl SessionHandle {
    #[inline]
    pub const fn generation(self) -> Generation {
        self.generation
    }

    #[inline]
    pub const fn slot(self) -> u8 {
        self.slot
    }
}

struct SessionSlot {
    generation: Generation,
    binding: Option<SessionBinding>,
    protection: Option<ProtectedAckSession>,
    reservation: Option<ReservationHandle>,
    stream: StreamState,
}

impl SessionSlot {
    const fn empty() -> Self {
        Self {
            generation: Generation::ZERO,
            binding: None,
            protection: None,
            reservation: None,
            stream: StreamState::new(),
        }
    }

    fn occupied(&self) -> bool {
        self.protection.is_some() && self.reservation.is_some()
    }
}

pub struct SessionTable {
    slots: [SessionSlot; MAX_SESSIONS],
}

impl SessionTable {
    pub const fn new() -> Self {
        Self {
            slots: [
                SessionSlot::empty(),
                SessionSlot::empty(),
                SessionSlot::empty(),
                SessionSlot::empty(),
            ],
        }
    }

    pub fn live_count(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_SESSIONS {
            if self.slots[i].occupied() {
                n = n.saturating_add(1);
            }
            i = i.saturating_add(1);
        }
        n
    }

    #[inline]
    pub fn is_full(&self) -> bool {
        self.live_count() >= MAX_SESSIONS
    }

    pub fn first_live(&self) -> Option<SessionHandle> {
        let mut i = 0usize;
        while i < MAX_SESSIONS {
            if self.slots[i].occupied() {
                return Some(SessionHandle {
                    slot: i as u8,
                    generation: self.slots[i].generation,
                });
            }
            i = i.saturating_add(1);
        }
        None
    }

    pub(crate) fn free_index(&self) -> Result<usize, QdnfError> {
        let mut i = 0usize;
        while i < MAX_SESSIONS {
            if !self.slots[i].occupied() {
                return Ok(i);
            }
            i = i.saturating_add(1);
        }
        Err(QdnfError::Capacity)
    }

    fn slot_mut(&mut self, h: SessionHandle) -> Result<&mut SessionSlot, QdnfError> {
        let i = h.slot as usize;
        if i >= MAX_SESSIONS {
            return Err(QdnfError::Range);
        }
        let slot = &mut self.slots[i];
        if !slot.occupied() {
            return Err(QdnfError::Closed);
        }
        if slot.generation != h.generation {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(slot)
    }

    fn slot(&self, h: SessionHandle) -> Result<&SessionSlot, QdnfError> {
        let i = h.slot as usize;
        if i >= MAX_SESSIONS {
            return Err(QdnfError::Range);
        }
        let slot = &self.slots[i];
        if !slot.occupied() {
            return Err(QdnfError::Closed);
        }
        if slot.generation != h.generation {
            return Err(QdnfError::StaleGeneration);
        }
        Ok(slot)
    }

    /// Occupies one vacancy. Caller must [`Self::is_full`] before reserving so a
    /// rejected insert cannot drop an unreleased handle.
    pub fn insert(
        &mut self,
        binding: SessionBinding,
        protection: ProtectedAckSession,
        reservation: ReservationHandle,
    ) -> Result<SessionHandle, QdnfError> {
        let i = self.free_index()?;
        let generation = self.slots[i].generation.next()?;
        self.slots[i] = SessionSlot {
            generation,
            binding: Some(binding),
            protection: Some(protection),
            reservation: Some(reservation),
            stream: StreamState::new(),
        };
        Ok(SessionHandle {
            slot: i as u8,
            generation,
        })
    }

    pub fn get_mut(&mut self, h: SessionHandle) -> Result<&mut ProtectedAckSession, QdnfError> {
        self.slot_mut(h)?
            .protection
            .as_mut()
            .ok_or(QdnfError::Unauthorized)
    }

    pub fn binding(&self, h: SessionHandle) -> Result<SessionBinding, QdnfError> {
        self.slot(h)?.binding.ok_or(QdnfError::Unauthorized)
    }

    /// Advance the application send offset only after the bearer accepted the frame.
    pub fn commit_send_offset(
        &mut self,
        h: SessionHandle,
        payload_len: u16,
    ) -> Result<u64, QdnfError> {
        let slot = self.slot_mut(h)?;
        let offset = slot.stream.next_offset;
        slot.stream.accept(StreamFrame {
            stream_id: 0,
            offset,
            fin: false,
            declared_final: None,
            len: payload_len,
        })?;
        Ok(slot.stream.next_offset)
    }

    pub fn send_offset(&self, h: SessionHandle) -> Result<u64, QdnfError> {
        Ok(self.slot(h)?.stream.next_offset)
    }

    /// Handle-based release. Does not accept caller-supplied byte amounts.
    pub fn close(
        &mut self,
        h: SessionHandle,
        ledger: &mut ReservationLedger,
    ) -> Result<(), QdnfError> {
        let i = h.slot as usize;
        if i >= MAX_SESSIONS {
            return Err(QdnfError::Range);
        }
        if !self.slots[i].occupied() {
            return Err(QdnfError::Closed);
        }
        if self.slots[i].generation != h.generation {
            return Err(QdnfError::StaleGeneration);
        }
        let reservation = self.slots[i].reservation.take().ok_or(QdnfError::Closed)?;
        ledger.release(reservation)?;
        let keep = self.slots[i].generation;
        self.slots[i] = SessionSlot::empty();
        self.slots[i].generation = keep;
        Ok(())
    }
}

impl Default for SessionTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::runtime::ResourceBudget;
    use crate::net::qdnf::authority::PolicyOutcome;
    use crate::net::qdnf::session::SessionState;
    use crate::net::qdnf::types::{OperationId, StrongDigest};

    fn ledger() -> ReservationLedger {
        let cap = ResourceBudget {
            bytes: 1024,
            work: 16,
            io: 16,
        };
        ReservationLedger::new(cap, cap, cap, cap, cap)
    }

    fn charge() -> ResourceBudget {
        ResourceBudget {
            bytes: 256,
            work: 1,
            io: 1,
        }
    }

    fn binding() -> SessionBinding {
        SessionBinding::test_fixture(
            OperationId([1u8; 16]),
            StrongDigest([2u8; 48]),
            StrongDigest([3u8; 48]),
            StrongDigest([4u8; 48]),
            PolicyOutcome::Allow,
            SessionState::Active,
        )
    }

    fn protection(tag: u8) -> ProtectedAckSession {
        let mut send = [tag; 32];
        let mut recv = [tag.wrapping_add(1); 32];
        send[0] = tag.max(1);
        recv[0] = tag.wrapping_add(40).max(2);
        ProtectedAckSession::from_keys(send, recv, Generation(1)).unwrap()
    }

    #[test]
    fn two_sessions_live_close_one_keeps_the_other() {
        let mut table = SessionTable::new();
        let mut ledger = ledger();
        let h1 = table
            .insert(
                binding(),
                protection(1),
                ledger.reserve(charge(), true).unwrap(),
            )
            .unwrap();
        let h2 = table
            .insert(
                binding(),
                protection(2),
                ledger.reserve(charge(), true).unwrap(),
            )
            .unwrap();
        assert_eq!(table.live_count(), 2);
        assert!(table.get_mut(h1).is_ok());
        assert!(table.get_mut(h2).is_ok());
        let h3 = table
            .insert(
                binding(),
                protection(3),
                ledger.reserve(charge(), true).unwrap(),
            )
            .unwrap();
        let h4 = table
            .insert(
                binding(),
                protection(4),
                ledger.reserve(charge(), true).unwrap(),
            )
            .unwrap();
        assert_eq!(table.live_count(), MAX_SESSIONS);
        assert!(table.is_full());
        assert_eq!(table.free_index(), Err(QdnfError::Capacity));
        table.close(h1, &mut ledger).unwrap();
        assert_eq!(table.live_count(), 3);
        assert_eq!(table.get_mut(h1).err(), Some(QdnfError::Closed));
        assert!(table.get_mut(h2).is_ok());
        assert!(table.get_mut(h3).is_ok());
        assert!(table.get_mut(h4).is_ok());
    }

    #[test]
    fn close_releases_ledger_budget() {
        let mut table = SessionTable::new();
        let mut ledger = ledger();
        let before = ledger.host_remaining_bytes();
        let h = table
            .insert(
                binding(),
                protection(1),
                ledger.reserve(charge(), true).unwrap(),
            )
            .unwrap();
        assert!(ledger.host_remaining_bytes() < before);
        table.close(h, &mut ledger).unwrap();
        assert_eq!(ledger.host_remaining_bytes(), before);
    }
}
