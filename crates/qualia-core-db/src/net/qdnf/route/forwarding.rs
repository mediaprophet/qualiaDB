//! Immutable forwarding generation. Incomplete computation is not published.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::LinkId;

use super::spf::SpfTable;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ForwardingGeneration {
    pub id: u64,
    pub table: SpfTable,
    pub published: bool,
}

impl ForwardingGeneration {
    pub const fn empty(id: u64) -> Self {
        Self {
            id,
            table: SpfTable::EMPTY,
            published: false,
        }
    }

    pub fn publish(&mut self, table: SpfTable) -> Result<(), QdnfError> {
        if table.nodes == 0 {
            return Err(QdnfError::Incomplete);
        }
        self.table = table;
        self.published = true;
        Ok(())
    }

    pub fn lookup_next(&self, dest_node: u8) -> Result<u8, QdnfError> {
        if !self.published {
            return Err(QdnfError::Incomplete);
        }
        for i in 0..self.table.nodes {
            if self.table.dest[i] == dest_node {
                if self.table.hop_counts[i] == 0 {
                    return Ok(dest_node);
                }
                return Ok(self.table.hops[i][0].node);
            }
        }
        Err(QdnfError::NoRoute)
    }
}

pub fn decrement_hop(hop_limit: u8) -> Result<u8, QdnfError> {
    if hop_limit == 0 {
        Err(QdnfError::HopLimit)
    } else {
        Ok(hop_limit - 1)
    }
}

pub fn require_adjacency(neighbors: &[LinkId], next: LinkId) -> Result<(), QdnfError> {
    if neighbors.iter().any(|n| n.0 == next.0) {
        Ok(())
    } else {
        Err(QdnfError::NoRoute)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpublished_table_cannot_forward() {
        let gen = ForwardingGeneration::empty(1);
        assert_eq!(gen.lookup_next(1), Err(QdnfError::Incomplete));
    }
}
