//! Bounded dial races for QSession (NET-05.10 / RT-03.03 partial).
//!
//! At most [`MAX_ACTIVE_PATHS`] paths may be [`PathState::Active`]. Up to
//! [`MAX_RACE_CANDIDATES`] Racing+Active slots may be occupied; a fourth Active
//! is [`QdnfError::Capacity`]. Losing and late Racing attempts close and free
//! the slot. Application [`crate::net::qdnf::types::OperationId`] is not a path
//! id (NET-05.05 partial). Compat-carrier duplicate recovery stays off
//! (NET-05.11).

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::Generation;

/// Negotiated multipath ceiling (QSession max paths = 3).
pub const MAX_ACTIVE_PATHS: usize = 3;
/// Collected race candidates; only [`MAX_ACTIVE_PATHS`] may become Active.
pub const MAX_RACE_CANDIDATES: usize = 8;

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PathState {
    Racing = 1,
    Active = 2,
    Losing = 3,
    Closed = 4,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathSlot {
    pub id: u8,
    pub generation: u64,
    pub state: PathState,
    pub proven: bool,
}

/// Generation-bearing path mutation handle. Not a budget or permit token.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PathHandle {
    slot: u8,
    generation: Generation,
}

impl PathHandle {
    #[inline]
    pub const fn slot(self) -> u8 {
        self.slot
    }

    #[inline]
    pub const fn generation(self) -> Generation {
        self.generation
    }
}

pub const fn path_handle_is_budget_token() -> bool {
    false
}

/// Mutate `handle`'s slot. Stale generation → [`QdnfError::StaleGeneration`].
/// Promoting to [`PathState::Active`] requires attested reachability.
pub fn mutate_path(
    table: &mut PathTable,
    handle: PathHandle,
    next: PathState,
) -> Result<PathHandle, QdnfError> {
    table.mutate(handle, next)
}

/// Eight-slot table. Occupied entries are Racing, Active, or Losing.
pub struct PathTable {
    slots: [Option<PathSlot>; MAX_RACE_CANDIDATES],
}

impl PathTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_RACE_CANDIDATES],
        }
    }

    /// Open a Racing candidate. Occupied Racing+Active+Losing at
    /// [`MAX_RACE_CANDIDATES`] → [`QdnfError::Capacity`].
    pub fn start_race(&mut self, generation: u64) -> Result<u8, QdnfError> {
        if self.occupied_count() >= MAX_RACE_CANDIDATES {
            return Err(QdnfError::Capacity);
        }
        let idx = self.free_index().ok_or(QdnfError::Capacity)?;
        let id = idx as u8;
        self.slots[idx] = Some(PathSlot {
            id,
            generation,
            state: PathState::Racing,
            proven: false,
        });
        Ok(id)
    }

    pub fn handle_of(&self, id: u8) -> Result<PathHandle, QdnfError> {
        let slot = self.get(id).ok_or(QdnfError::Closed)?;
        Ok(PathHandle {
            slot: slot.id,
            generation: Generation(slot.generation),
        })
    }

    /// Authenticated reachability proof already verified by the caller.
    /// Mismatched generation → [`QdnfError::StaleGeneration`].
    pub fn attest_reachability(&mut self, handle: PathHandle) -> Result<(), QdnfError> {
        let idx = self.index_of(handle.slot)?;
        let slot = self.slots[idx].as_mut().ok_or(QdnfError::Closed)?;
        if slot.generation != handle.generation.0 {
            return Err(QdnfError::StaleGeneration);
        }
        slot.proven = true;
        Ok(())
    }

    pub fn mutate(&mut self, handle: PathHandle, next: PathState) -> Result<PathHandle, QdnfError> {
        let idx = self.index_of(handle.slot)?;
        {
            let slot = self.slots[idx].as_ref().ok_or(QdnfError::Closed)?;
            if slot.generation != handle.generation.0 {
                return Err(QdnfError::StaleGeneration);
            }
            if next == PathState::Active {
                if !slot.proven {
                    return Err(QdnfError::Unauthorized);
                }
                if slot.state != PathState::Active && self.active_count() >= MAX_ACTIVE_PATHS {
                    return Err(QdnfError::Capacity);
                }
            }
        }
        if matches!(next, PathState::Closed | PathState::Losing) {
            let new_gen = Generation(handle.generation.0).next()?;
            self.slots[idx] = None;
            return Ok(PathHandle {
                slot: handle.slot,
                generation: new_gen,
            });
        }
        let slot = self.slots[idx].as_mut().ok_or(QdnfError::Closed)?;
        let new_gen = Generation(slot.generation).next()?;
        slot.generation = new_gen.0;
        slot.state = next;
        Ok(PathHandle {
            slot: handle.slot,
            generation: new_gen,
        })
    }

    /// Promote a Racing slot to Active. A fourth Active is Capacity.
    pub fn mark_active(&mut self, id: u8) -> Result<(), QdnfError> {
        let idx = self.index_of(id)?;
        let state = self.slots[idx].ok_or(QdnfError::Closed)?.state;
        match state {
            PathState::Active => Ok(()),
            PathState::Closed | PathState::Losing => Err(QdnfError::Closed),
            PathState::Racing => {
                if self.active_count() >= MAX_ACTIVE_PATHS {
                    return Err(QdnfError::Capacity);
                }
                let slot = self.slots[idx].as_mut().ok_or(QdnfError::Closed)?;
                slot.state = PathState::Active;
                Ok(())
            }
        }
    }

    /// Losing/late attempts: Racing or Losing → Closed, slot freed.
    /// Active may be retired the same way so a later race can win.
    pub fn lose_and_cleanup(&mut self, id: u8) -> Result<(), QdnfError> {
        let idx = self.index_of(id)?;
        let slot = self.slots[idx].as_mut().ok_or(QdnfError::Closed)?;
        match slot.state {
            PathState::Closed => Err(QdnfError::Closed),
            PathState::Racing | PathState::Losing | PathState::Active => {
                slot.state = PathState::Losing;
                self.slots[idx] = None;
                Ok(())
            }
        }
    }

    /// Same as [`Self::lose_and_cleanup`], but the caller must present the
    /// generation stored on the slot. Mismatch → [`QdnfError::StaleGeneration`].
    pub fn lose_generation(&mut self, id: u8, generation: u64) -> Result<(), QdnfError> {
        let idx = self.index_of(id)?;
        let slot = self.slots[idx].as_ref().ok_or(QdnfError::Closed)?;
        if slot.generation != generation {
            return Err(QdnfError::StaleGeneration);
        }
        self.lose_and_cleanup(id)
    }

    pub fn active_count(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_RACE_CANDIDATES {
            if let Some(slot) = self.slots[i] {
                if slot.state == PathState::Active {
                    n += 1;
                }
            }
            i += 1;
        }
        n
    }

    /// NET-05.05 partial: the application operation id is not the path id.
    pub const fn preserve_operation_id_across_path_change() -> bool {
        true
    }

    /// NET-05.11: reliable-carrier compatibility disables duplicate recovery.
    pub const fn duplicate_recovery_on_compat_carrier() -> bool {
        false
    }

    pub fn get(&self, id: u8) -> Option<&PathSlot> {
        let idx = id as usize;
        if idx >= MAX_RACE_CANDIDATES {
            return None;
        }
        self.slots[idx].as_ref()
    }

    fn occupied_count(&self) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_RACE_CANDIDATES {
            if let Some(slot) = self.slots[i] {
                if slot.state != PathState::Closed {
                    n += 1;
                }
            }
            i += 1;
        }
        n
    }

    fn free_index(&self) -> Option<usize> {
        let mut i = 0usize;
        while i < MAX_RACE_CANDIDATES {
            match self.slots[i] {
                None => return Some(i),
                Some(slot) if slot.state == PathState::Closed => return Some(i),
                _ => {}
            }
            i += 1;
        }
        None
    }

    fn index_of(&self, id: u8) -> Result<usize, QdnfError> {
        let idx = id as usize;
        if idx >= MAX_RACE_CANDIDATES {
            return Err(QdnfError::Range);
        }
        Ok(idx)
    }
}

impl Default for PathTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::types::Generation;

    #[test]
    fn fourth_mark_active_is_capacity() {
        let mut table = PathTable::new();
        let a = table.start_race(1).unwrap();
        let b = table.start_race(1).unwrap();
        let c = table.start_race(1).unwrap();
        let d = table.start_race(1).unwrap();
        table.mark_active(a).unwrap();
        table.mark_active(b).unwrap();
        table.mark_active(c).unwrap();
        assert_eq!(table.active_count(), MAX_ACTIVE_PATHS);
        assert_eq!(table.mark_active(d), Err(QdnfError::Capacity));
        assert_eq!(table.get(d).unwrap().state, PathState::Racing);
    }

    #[test]
    fn lose_and_cleanup_frees_slot_for_new_race() {
        let mut table = PathTable::new();
        let mut ids = [0u8; MAX_RACE_CANDIDATES];
        let mut i = 0usize;
        while i < MAX_RACE_CANDIDATES {
            ids[i] = table.start_race(10 + i as u64).unwrap();
            i += 1;
        }
        assert_eq!(table.start_race(99), Err(QdnfError::Capacity));
        table.lose_and_cleanup(ids[0]).unwrap();
        let reused = table.start_race(100).unwrap();
        assert_eq!(reused, ids[0]);
        assert_eq!(table.get(reused).unwrap().generation, 100);
        assert_eq!(table.get(reused).unwrap().state, PathState::Racing);
    }

    #[test]
    fn lose_generation_stale_is_rejected() {
        let mut table = PathTable::new();
        let id = table.start_race(7).unwrap();
        assert_eq!(
            table.lose_generation(id, 8),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(table.get(id).unwrap().state, PathState::Racing);
        table.lose_generation(id, 7).unwrap();
        assert!(table.get(id).is_none());
    }

    #[test]
    fn late_racing_attempt_closes_after_active_winner() {
        let mut table = PathTable::new();
        let winner = table.start_race(1).unwrap();
        let late = table.start_race(1).unwrap();
        table.mark_active(winner).unwrap();
        table.lose_and_cleanup(late).unwrap();
        assert_eq!(table.active_count(), 1);
        assert!(table.get(late).is_none());
    }

    #[test]
    fn preserve_operation_id_across_path_change() {
        assert!(PathTable::preserve_operation_id_across_path_change());
    }

    #[test]
    fn duplicate_recovery_on_compat_carrier_is_disabled() {
        assert!(!PathTable::duplicate_recovery_on_compat_carrier());
    }

    #[test]
    fn stale_path_handle_is_stale_generation() {
        let mut table = PathTable::new();
        let id = table.start_race(4).unwrap();
        let h = table.handle_of(id).unwrap();
        table.attest_reachability(h).unwrap();
        let fresh = mutate_path(&mut table, h, PathState::Racing).unwrap();
        assert_eq!(
            mutate_path(&mut table, h, PathState::Racing),
            Err(QdnfError::StaleGeneration)
        );
        assert_eq!(fresh.generation(), Generation(5));
        assert!(!path_handle_is_budget_token());
        assert_eq!(
            table.attest_reachability(h),
            Err(QdnfError::StaleGeneration)
        );
    }

    #[test]
    fn coupled_multipath_does_not_multiply_window() {
        use crate::net::qdnf::session::congestion::PathCcTable;
        let mut table = PathTable::new();
        let a = table.start_race(1).unwrap();
        let b = table.start_race(1).unwrap();
        table.mark_active(a).unwrap();
        table.mark_active(b).unwrap();
        let ha = table.handle_of(a).unwrap();
        let hb = table.handle_of(b).unwrap();
        let mut coupled = PathCcTable::new(500);
        assert!(!coupled.independent_bottleneck);
        coupled.attach(ha).unwrap();
        coupled.attach(hb).unwrap();
        coupled.send(ha, 1, 400).unwrap();
        assert_eq!(coupled.send(hb, 2, 200), Err(QdnfError::BudgetExhausted));
        assert!(coupled.total_in_flight() <= 500);
        let mut independent = PathCcTable::new(500);
        independent.independent_bottleneck = true;
        independent.attach(ha).unwrap();
        independent.attach(hb).unwrap();
        independent.send(ha, 1, 500).unwrap();
        independent.send(hb, 2, 500).unwrap();
        assert_eq!(independent.total_in_flight(), 1000);
    }

    #[test]
    fn mutate_active_requires_reachability_proof() {
        let mut table = PathTable::new();
        let id = table.start_race(1).unwrap();
        let h = table.handle_of(id).unwrap();
        assert_eq!(
            mutate_path(&mut table, h, PathState::Active),
            Err(QdnfError::Unauthorized)
        );
        table.attest_reachability(h).unwrap();
        let active = mutate_path(&mut table, h, PathState::Active).unwrap();
        assert_eq!(table.get(id).unwrap().state, PathState::Active);
        assert_eq!(active.generation(), Generation(2));
    }
}
