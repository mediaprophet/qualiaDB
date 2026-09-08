//! Old/new forwarding-generation overlap. Incomplete tables never go live.

use crate::net::qdnf::errors::QdnfError;

use super::forwarding::ForwardingGeneration;
use super::spf::SpfTable;

/// Charged old generation plus the candidate new table. Never a third live gen.
pub const MAX_LIVE_GENERATIONS: usize = 2;

/// Pair of immutable forwarding generations with a reader charge on `old`.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GenerationPair {
    old: ForwardingGeneration,
    new: ForwardingGeneration,
    old_readers: u8,
}

impl GenerationPair {
    pub const fn empty() -> Self {
        Self {
            old: ForwardingGeneration::empty(0),
            new: ForwardingGeneration::empty(0),
            old_readers: 0,
        }
    }

    /// QDNF has no default-route or LIG/DNS fallback.
    pub const fn has_default_route() -> bool {
        false
    }

    /// Install a computed table as `new`. Incomplete (`nodes == 0`) does not publish.
    pub fn install_new(&mut self, table: SpfTable, id: u64) -> Result<(), QdnfError> {
        Self::cancelled_computation_cannot_publish(&table)?;
        let mut gen = ForwardingGeneration::empty(id);
        gen.publish(table)?;
        self.new = gen;
        Ok(())
    }

    /// Atomic: unpublished `new` cannot activate. If `old_readers == 0`, swap
    /// `new` → `old`. If readers remain, keep `old` until `release_old()`.
    pub fn activate_new(&mut self) -> Result<(), QdnfError> {
        if !self.new.published {
            return Err(QdnfError::Incomplete);
        }
        if self.old_readers == 0 {
            self.old = self.new;
            self.new = ForwardingGeneration::empty(0);
        }
        Ok(())
    }

    /// Charge a reader on the retained old generation. Saturates at 255 → Capacity.
    pub fn retain_old(&mut self) -> Result<(), QdnfError> {
        match self.old_readers.checked_add(1) {
            Some(n) => {
                self.old_readers = n;
                Ok(())
            }
            None => Err(QdnfError::Capacity),
        }
    }

    /// Drop one old-generation reader. Zero charges → DoubleRelease.
    pub fn release_old(&mut self) -> Result<(), QdnfError> {
        if self.old_readers == 0 {
            return Err(QdnfError::DoubleRelease);
        }
        self.old_readers -= 1;
        if self.old_readers == 0 && self.new.published {
            self.old = self.new;
            self.new = ForwardingGeneration::empty(0);
        }
        Ok(())
    }

    /// Prefer published `new`; else published `old`; else Incomplete.
    pub fn lookup(&self, dest_node: u8) -> Result<u8, QdnfError> {
        if self.new.published {
            return self.new.lookup_next(dest_node);
        }
        if self.old.published {
            return self.old.lookup_next(dest_node);
        }
        Err(QdnfError::Incomplete)
    }

    /// Cancelled / empty computation cannot publish a live generation.
    pub fn cancelled_computation_cannot_publish(table: &SpfTable) -> Result<(), QdnfError> {
        if table.nodes == 0 {
            Err(QdnfError::Incomplete)
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::route::forwarding::{decrement_hop, require_adjacency};
    use crate::net::qdnf::route::spf::{compute_spf, LinkMetric};
    use crate::net::qdnf::types::LinkId;

    fn abc_links() -> [LinkMetric; 2] {
        [
            LinkMetric {
                from: 0,
                to: 1,
                cost: 1,
                bidirectional: true,
            },
            LinkMetric {
                from: 1,
                to: 2,
                cost: 1,
                bidirectional: true,
            },
        ]
    }

    fn compute_abc() -> SpfTable {
        let mut table = SpfTable::EMPTY;
        compute_spf(0, &abc_links(), &mut table).expect("A-B-C SPF");
        table
    }

    #[test]
    fn unpublished_empty_cannot_activate() {
        let mut pair = GenerationPair::empty();
        assert_eq!(pair.activate_new(), Err(QdnfError::Incomplete));
        assert_eq!(pair.lookup(2), Err(QdnfError::Incomplete));
    }

    #[test]
    fn install_abc_lookup_dest_two_is_next_hop_one() {
        let mut pair = GenerationPair::empty();
        pair.install_new(compute_abc(), 1).unwrap();
        assert_eq!(pair.lookup(2), Ok(1));
    }

    #[test]
    fn retain_old_then_activate_new_still_allows_lookup() {
        let mut pair = GenerationPair::empty();
        pair.install_new(compute_abc(), 1).unwrap();
        pair.retain_old().unwrap();
        pair.activate_new().unwrap();
        assert_eq!(pair.lookup(2), Ok(1));
        pair.release_old().unwrap();
        assert_eq!(pair.lookup(2), Ok(1));
        assert_eq!(pair.release_old(), Err(QdnfError::DoubleRelease));
    }

    #[test]
    fn cancelled_empty_table_does_not_publish() {
        let mut pair = GenerationPair::empty();
        assert_eq!(
            GenerationPair::cancelled_computation_cannot_publish(&SpfTable::EMPTY),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(
            pair.install_new(SpfTable::EMPTY, 9),
            Err(QdnfError::Incomplete)
        );
        assert_eq!(pair.lookup(1), Err(QdnfError::Incomplete));
        assert_eq!(pair.activate_new(), Err(QdnfError::Incomplete));
    }

    #[test]
    fn retain_old_capacity_at_255() {
        let mut pair = GenerationPair::empty();
        pair.old_readers = 255;
        assert_eq!(pair.retain_old(), Err(QdnfError::Capacity));
    }

    #[test]
    fn a_to_c_via_b_forward_without_default_route() {
        let mut table = SpfTable::EMPTY;
        compute_spf(0, &abc_links(), &mut table).unwrap();
        let mut pair = GenerationPair::empty();
        pair.install_new(table, 1).unwrap();
        assert_eq!(pair.lookup(2), Ok(1));
        assert_eq!(decrement_hop(1), Ok(0));
        assert_eq!(decrement_hop(0), Err(QdnfError::HopLimit));

        let one_sided = [LinkMetric {
            from: 0,
            to: 1,
            cost: 1,
            bidirectional: false,
        }];
        let mut ignored = SpfTable::EMPTY;
        compute_spf(0, &one_sided, &mut ignored).unwrap();
        assert!(ignored.dest.iter().take(ignored.nodes).all(|d| *d == 0));

        assert!(!GenerationPair::has_default_route());
        assert_eq!(MAX_LIVE_GENERATIONS, 2);

        let neighbors = [LinkId([1u8; 16])];
        require_adjacency(&neighbors, LinkId([1u8; 16])).unwrap();
        assert_eq!(
            require_adjacency(&neighbors, LinkId([2u8; 16])),
            Err(QdnfError::NoRoute)
        );
    }
}
