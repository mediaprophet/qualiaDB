//! Fixed-Capacity SOA Archetype Table (`table.rs`).
//!
//! Provides a zero-heap Structure-of-Arrays (SOA) component table [`SimTable<T, CAP>`] with:
//! - $O(1)$ sparse-to-dense entity index mapping (`sparse: [u32; CAP]`, `entities: [u32; CAP]`),
//! - Contiguous dense component array (`components: [T; CAP]`),
//! - Per-row `added_ticks` and `changed_ticks` ([`Tick`]) for zero-heap reactive queries,
//! - $O(1)$ `insert`, `remove` (swap-remove), `get`, and `get_mut` (which stamps `changed_tick`).

use crate::ecs::query::{Added, Changed};
use crate::ecs::tick::Tick;

/// Sentinel value indicating that an entity ID is not present in a [`SimTable`].
pub const INVALID_ROW: u32 = u32::MAX;

/// Errors returned by [`SimTable`] operations.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimTableError {
    /// Entity index is outside the table's `0..CAP` sparse index range.
    EntityOutOfBounds { entity: u32, cap: usize },
    /// Table has reached its fixed `CAP` row capacity.
    TableFull { cap: usize },
}

/// Fixed-capacity SOA component table with dense storage and per-row change-detection ticks.
#[derive(Clone, Debug)]
pub struct SimTable<T: Copy, const CAP: usize = 256> {
    /// Dense row index -> entity ID (`0..len` valid).
    pub entities: [u32; CAP],
    /// Entity ID -> dense row index (`INVALID_ROW` when absent).
    pub sparse: [u32; CAP],
    /// Dense contiguous component storage (`0..len` valid).
    pub components: [T; CAP],
    /// Per-dense-row tick when the component was inserted.
    pub added_ticks: [Tick; CAP],
    /// Per-dense-row tick when the component was last mutably accessed.
    pub changed_ticks: [Tick; CAP],
    /// Number of active dense rows.
    pub len: usize,
}

impl<T: Copy + Default, const CAP: usize> Default for SimTable<T, CAP> {
    fn default() -> Self {
        Self::new(T::default())
    }
}

impl<T: Copy, const CAP: usize> SimTable<T, CAP> {
    /// Creates an empty `SimTable` initialized with `fill_default`.
    pub const fn new(fill_default: T) -> Self {
        Self {
            entities: [0u32; CAP],
            sparse: [INVALID_ROW; CAP],
            components: [fill_default; CAP],
            added_ticks: [Tick::ZERO; CAP],
            changed_ticks: [Tick::ZERO; CAP],
            len: 0,
        }
    }

    /// Number of active entities stored in this table.
    #[inline]
    pub const fn len(&self) -> usize {
        self.len
    }

    /// Returns `true` if no entities are stored in this table.
    #[inline]
    pub const fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Maximum capacity of the table.
    #[inline]
    pub const fn capacity(&self) -> usize {
        CAP
    }

    /// Returns `true` if `entity` is stored in this table.
    #[inline]
    pub const fn contains(&self, entity: u32) -> bool {
        let idx = entity as usize;
        if idx >= CAP {
            return false;
        }
        let row = self.sparse[idx] as usize;
        row < self.len && self.entities[row] == entity
    }

    /// Inserts or overwrites the component for `entity` at `current_tick` in $O(1)$ time.
    ///
    /// If `entity` was not present, both `added_tick` and `changed_tick` are set to `current_tick`.
    /// If `entity` was already present, `changed_tick` is updated to `current_tick`.
    pub fn insert(
        &mut self,
        entity: u32,
        value: T,
        current_tick: Tick,
    ) -> Result<usize, SimTableError> {
        let e_idx = entity as usize;
        if e_idx >= CAP {
            return Err(SimTableError::EntityOutOfBounds { entity, cap: CAP });
        }

        let existing_row = self.sparse[e_idx];
        if existing_row != INVALID_ROW {
            let row = existing_row as usize;
            if row < self.len && self.entities[row] == entity {
                self.components[row] = value;
                self.changed_ticks[row] = current_tick;
                return Ok(row);
            }
        }

        if self.len >= CAP {
            return Err(SimTableError::TableFull { cap: CAP });
        }

        let row = self.len;
        self.len += 1;
        self.entities[row] = entity;
        self.sparse[e_idx] = row as u32;
        self.components[row] = value;
        self.added_ticks[row] = current_tick;
        self.changed_ticks[row] = current_tick;
        Ok(row)
    }

    /// Removes `entity` from the table in $O(1)$ time via dense swap-remove.
    pub fn remove(&mut self, entity: u32) -> Option<T> {
        let e_idx = entity as usize;
        if e_idx >= CAP {
            return None;
        }
        let row = self.sparse[e_idx] as usize;
        if row >= self.len || self.entities[row] != entity {
            return None;
        }

        let removed_val = self.components[row];
        let last_row = self.len - 1;

        if row != last_row {
            let moved_entity = self.entities[last_row];
            self.entities[row] = moved_entity;
            self.components[row] = self.components[last_row];
            self.added_ticks[row] = self.added_ticks[last_row];
            self.changed_ticks[row] = self.changed_ticks[last_row];
            self.sparse[moved_entity as usize] = row as u32;
        }

        self.sparse[e_idx] = INVALID_ROW;
        self.len = last_row;
        Some(removed_val)
    }

    /// Shared $O(1)$ component lookup by entity ID.
    #[inline]
    pub fn get(&self, entity: u32) -> Option<&T> {
        let e_idx = entity as usize;
        if e_idx >= CAP {
            return None;
        }
        let row = self.sparse[e_idx] as usize;
        if row < self.len && self.entities[row] == entity {
            Some(&self.components[row])
        } else {
            None
        }
    }

    /// Mutable $O(1)$ component lookup by entity ID; stamps `changed_ticks[row] = current_tick`.
    #[inline]
    pub fn get_mut(&mut self, entity: u32, current_tick: Tick) -> Option<&mut T> {
        let e_idx = entity as usize;
        if e_idx >= CAP {
            return None;
        }
        let row = self.sparse[e_idx] as usize;
        if row < self.len && self.entities[row] == entity {
            self.changed_ticks[row] = current_tick;
            Some(&mut self.components[row])
        } else {
            None
        }
    }

    /// Returns `(added_tick, changed_tick)` for `entity`.
    #[inline]
    pub fn get_ticks(&self, entity: u32) -> Option<(Tick, Tick)> {
        let e_idx = entity as usize;
        if e_idx >= CAP {
            return None;
        }
        let row = self.sparse[e_idx] as usize;
        if row < self.len && self.entities[row] == entity {
            Some((self.added_ticks[row], self.changed_ticks[row]))
        } else {
            None
        }
    }

    /// Returns the dense slice of active `(entity_id)`s (`&[u32]`).
    #[inline]
    pub fn active_entities(&self) -> &[u32] {
        &self.entities[..self.len]
    }

    /// Returns the dense slice of active components (`&[T]`).
    #[inline]
    pub fn active_components(&self) -> &[T] {
        &self.components[..self.len]
    }

    /// Returns the mutable dense slice of active components (`&mut [T]`) and stamps all active rows
    /// with `current_tick`.
    #[inline]
    pub fn active_components_mut(&mut self, current_tick: Tick) -> &mut [T] {
        let mut i = 0usize;
        while i < self.len {
            self.changed_ticks[i] = current_tick;
            i += 1;
        }
        &mut self.components[..self.len]
    }

    /// Zero-heap iterator over `(entity_id, &T)` for components changed after `last_run`.
    #[inline]
    pub fn iter_changed(&self, last_run: Tick, this_run: Tick) -> Changed<'_, T, CAP> {
        Changed::new(self, last_run, this_run)
    }

    /// Zero-heap iterator over `(entity_id, &T)` for components added after `last_run`.
    #[inline]
    pub fn iter_added(&self, last_run: Tick, this_run: Tick) -> Added<'_, T, CAP> {
        Added::new(self, last_run, this_run)
    }
}
