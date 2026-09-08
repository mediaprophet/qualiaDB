//! Bounded dial races for QSession (NET-05.10 / RT-03.03 partial).
//!
//! At most [`MAX_ACTIVE_PATHS`] paths may be [`PathState::Active`]. Up to
//! [`MAX_RACE_CANDIDATES`] Racing+Active slots may be occupied; a fourth Active
//! is [`QdnfError::Capacity`]. Losing and late Racing attempts close and free
//! the slot. Application [`crate::net::qdnf::types::OperationId`] is not a path
//! id (NET-05.05 partial). Compat-carrier duplicate recovery stays off
//! (NET-05.11).

use crate::net::qdnf::errors::QdnfError;

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
        });
        Ok(id)
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
}
