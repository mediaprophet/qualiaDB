//! Bounded neighbor table. Capacity failure does not allocate another flight.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::LinkId;

use super::adjacency::{Adjacency, AdjacencyState};

pub const MAX_NEIGHBORS: usize = 32;

pub struct NeighborTable {
    slots: [Option<Adjacency>; MAX_NEIGHBORS],
}

impl NeighborTable {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_NEIGHBORS],
        }
    }

    pub fn insert(&mut self, adj: Adjacency) -> Result<usize, QdnfError> {
        for slot in self.slots.iter() {
            if let Some(existing) = slot {
                if existing.remote.0 == adj.remote.0 {
                    return Ok(0);
                }
            }
        }
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(adj);
                return Ok(i);
            }
        }
        Err(QdnfError::Capacity)
    }

    /// Neighbor insert after a verified challenge. Unverified beacons occupy
    /// no extra slot. The table cap is still [`MAX_NEIGHBORS`].
    pub fn insert_verified(&mut self, adj: Adjacency, verified: bool) -> Result<usize, QdnfError> {
        if !verified {
            return Err(QdnfError::Unauthorized);
        }
        self.insert(adj)
    }

    pub fn forwarding(&self, remote: LinkId) -> Option<&Adjacency> {
        self.slots
            .iter()
            .flatten()
            .find(|a| a.remote.0 == remote.0 && a.state == AdjacencyState::Adjacent)
    }

    pub fn len(&self) -> usize {
        self.slots.iter().filter(|s| s.is_some()).count()
    }
}

impl Default for NeighborTable {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::types::ObservedLocator;

    #[test]
    fn full_table_does_not_allocate() {
        let mut table = NeighborTable::new();
        for i in 0..MAX_NEIGHBORS {
            let mut remote = LinkId::ZERO;
            remote.0[0] = i as u8 + 1;
            table
                .insert(Adjacency {
                    local: LinkId::ZERO,
                    remote,
                    observed_peer: ObservedLocator::EMPTY,
                    state: AdjacencyState::Adjacent,
                    generation: 1,
                    mtu: 1280,
                })
                .unwrap();
        }
        let mut extra = LinkId::ZERO;
        extra.0[0] = 99;
        assert_eq!(
            table.insert(Adjacency {
                local: LinkId::ZERO,
                remote: extra,
                observed_peer: ObservedLocator::EMPTY,
                state: AdjacencyState::Adjacent,
                generation: 1,
                mtu: 1280,
            }),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn unverified_challenge_does_not_insert() {
        let mut table = NeighborTable::new();
        assert_eq!(
            table.insert_verified(
                Adjacency {
                    local: LinkId::ZERO,
                    remote: LinkId([1u8; 16]),
                    observed_peer: ObservedLocator::EMPTY,
                    state: AdjacencyState::BeaconSeen,
                    generation: 1,
                    mtu: 1280,
                },
                false,
            ),
            Err(QdnfError::Unauthorized)
        );
        assert_eq!(table.len(), 0);
    }
}
