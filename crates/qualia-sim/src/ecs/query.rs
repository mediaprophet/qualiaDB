//! Zero-Heap Reactive ECS Filter Iterators (`query.rs`).
//!
//! Provides allocation-free [`Changed<T>`] and [`Added<T>`] iterators over a [`SimTable<T, CAP>`]
//! using per-row [`Tick`] comparisons (`Tick::is_newer_than`).

use crate::ecs::table::SimTable;
use crate::ecs::tick::Tick;

/// Zero-heap iterator yielding `(entity_id, &T)` for rows whose `changed_tick` is newer than `last_run`.
pub struct Changed<'a, T: Copy, const CAP: usize> {
    table: &'a SimTable<T, CAP>,
    cursor: usize,
    last_run: Tick,
    this_run: Tick,
}

impl<'a, T: Copy, const CAP: usize> Changed<'a, T, CAP> {
    /// Creates a new `Changed` filter iterator over `table`.
    #[inline]
    pub const fn new(table: &'a SimTable<T, CAP>, last_run: Tick, this_run: Tick) -> Self {
        Self {
            table,
            cursor: 0,
            last_run,
            this_run,
        }
    }
}

impl<'a, T: Copy, const CAP: usize> Iterator for Changed<'a, T, CAP> {
    type Item = (u32, &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.table.len {
            let row = self.cursor;
            self.cursor += 1;
            if self.table.changed_ticks[row].is_newer_than(self.last_run, self.this_run) {
                return Some((self.table.entities[row], &self.table.components[row]));
            }
        }
        None
    }
}

/// Zero-heap iterator yielding `(entity_id, &T)` for rows whose `added_tick` is newer than `last_run`.
pub struct Added<'a, T: Copy, const CAP: usize> {
    table: &'a SimTable<T, CAP>,
    cursor: usize,
    last_run: Tick,
    this_run: Tick,
}

impl<'a, T: Copy, const CAP: usize> Added<'a, T, CAP> {
    /// Creates a new `Added` filter iterator over `table`.
    #[inline]
    pub const fn new(table: &'a SimTable<T, CAP>, last_run: Tick, this_run: Tick) -> Self {
        Self {
            table,
            cursor: 0,
            last_run,
            this_run,
        }
    }
}

impl<'a, T: Copy, const CAP: usize> Iterator for Added<'a, T, CAP> {
    type Item = (u32, &'a T);

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        while self.cursor < self.table.len {
            let row = self.cursor;
            self.cursor += 1;
            if self.table.added_ticks[row].is_newer_than(self.last_run, self.this_run) {
                return Some((self.table.entities[row], &self.table.components[row]));
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sim_table_insert_remove_and_reactive_queries() {
        let mut table = SimTable::<[f32; 3], 16>::new([0.0; 3]);

        // Tick 1: insert entities 3 and 7.
        let t1 = Tick::new(1);
        table.insert(3, [1.0, 2.0, 3.0], t1).unwrap();
        table.insert(7, [4.0, 5.0, 6.0], t1).unwrap();
        assert_eq!(table.len(), 2);

        // Both are Added and Changed relative to Tick 0.
        assert_eq!(table.iter_added(Tick::ZERO, t1).count(), 2);
        assert_eq!(table.iter_changed(Tick::ZERO, t1).count(), 2);

        // Tick 2: mutate only entity 7.
        let t2 = Tick::new(2);
        table.get_mut(7, t2).unwrap()[0] = 99.0;

        // Relative to t1: 0 added, 1 changed (entity 7).
        assert_eq!(table.iter_added(t1, t2).count(), 0);
        let mut changed = table.iter_changed(t1, t2);
        let (ent, val) = changed.next().unwrap();
        assert_eq!(ent, 7);
        assert_eq!(val[0], 99.0);
        assert!(changed.next().is_none());

        // Swap-remove entity 3 and verify entity 7 remains intact.
        let removed = table.remove(3).unwrap();
        assert_eq!(removed, [1.0, 2.0, 3.0]);
        assert_eq!(table.len(), 1);
        assert!(!table.contains(3));
        assert!(table.contains(7));
        assert_eq!(table.get(7).unwrap()[0], 99.0);
    }
}
