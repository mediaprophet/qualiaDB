//! Hot exact-key coalescing, stable secondary partitioning, acyclic commitments
//! (E07.6).
//!
//! A single exact key is not split by re-hashing its identity. Replica count is
//! 1..=3. Large postings partition on the full 48-byte record id. Parent
//! pointers that would cycle are Conflict.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::StrongDigest;

/// Admitted replica count for a hot exact key.
pub const MAX_HOT_REPLICAS: u8 = 3;
/// Small parent table for acyclic primary/secondary/checkpoint commitments.
pub const PARENT_CAP: u8 = 8;
const NO_PARENT: u8 = 0xff;

/// Secondary partition of a full record identity.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Partition {
    pub part: u8,
    pub of: u8,
}

/// Parent pointers for an 8-node commitment DAG. `0xff` means no parent.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct ParentTable {
    parent: [u8; 8],
}

impl ParentTable {
    pub const fn new() -> Self {
        Self {
            parent: [NO_PARENT; 8],
        }
    }

    #[inline]
    pub const fn parent_of(&self, node: u8) -> Option<u8> {
        if node >= PARENT_CAP {
            return None;
        }
        let p = self.parent[node as usize];
        if p == NO_PARENT {
            None
        } else {
            Some(p)
        }
    }
}

/// Coalesce identical scoped reads onto one replica in `1..=replicas`.
///
/// `replicas` must be 1..=3. Same key always maps to the same replica index.
pub fn coalesce_hot(key: &StrongDigest, replicas: u8) -> Result<u8, QdnfError> {
    if replicas < 1 || replicas > MAX_HOT_REPLICAS {
        return Err(QdnfError::Range);
    }
    let slot = mix_key(key) % (replicas as u32);
    Ok(slot as u8 + 1)
}

/// Stable full-record partition. `parts == 0` is Range. Same key → same part.
pub fn partition_record(key: &StrongDigest, parts: u8) -> Result<Partition, QdnfError> {
    if parts == 0 {
        return Err(QdnfError::Range);
    }
    let part = (mix_key(key) % (parts as u32)) as u8;
    Ok(Partition { part, of: parts })
}

fn mix_key(key: &StrongDigest) -> u32 {
    let mut acc = 2166136261u32;
    let mut i = 0usize;
    while i < 48 {
        acc = acc.wrapping_mul(16777619).wrapping_add(key.0[i] as u32);
        i += 1;
    }
    acc
}

/// Bind `child` to `parent` (0..8). A cycle is Conflict.
pub fn insert_parent(table: &mut ParentTable, child: u8, parent: u8) -> Result<(), QdnfError> {
    if child >= PARENT_CAP || parent >= PARENT_CAP {
        return Err(QdnfError::Range);
    }
    if child == parent {
        return Err(QdnfError::Conflict);
    }
    let mut cur = parent;
    let mut steps = 0u8;
    while steps < PARENT_CAP {
        if cur == child {
            return Err(QdnfError::Conflict);
        }
        let next = table.parent[cur as usize];
        if next == NO_PARENT {
            break;
        }
        cur = next;
        steps = steps + 1;
    }
    table.parent[child as usize] = parent;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_tail(b47: u8) -> StrongDigest {
        let mut k = StrongDigest::ZERO;
        k.0[0] = 0xab;
        k.0[47] = b47;
        k
    }

    #[test]
    fn coalesce_rejects_zero_and_more_than_three() {
        let k = StrongDigest::ZERO;
        assert_eq!(coalesce_hot(&k, 0), Err(QdnfError::Range));
        assert_eq!(coalesce_hot(&k, 4), Err(QdnfError::Range));
        let r = coalesce_hot(&k, 3).unwrap();
        assert!(r >= 1 && r <= 3);
    }

    #[test]
    fn coalesce_is_stable_for_same_key() {
        let k = key_tail(0x11);
        assert_eq!(coalesce_hot(&k, 2).unwrap(), coalesce_hot(&k, 2).unwrap());
    }

    #[test]
    fn partition_is_stable_for_same_key() {
        let k = key_tail(0x22);
        let a = partition_record(&k, 7).unwrap();
        let b = partition_record(&k, 7).unwrap();
        assert_eq!(a, b);
        assert_eq!(a.of, 7);
        assert!(a.part < 7);
    }

    #[test]
    fn partition_uses_later_bytes() {
        let a = key_tail(0x01);
        let b = key_tail(0x02);
        assert_eq!(a.0[0], b.0[0]);
        let pa = partition_record(&a, 16).unwrap();
        let pb = partition_record(&b, 16).unwrap();
        assert_eq!(pa.of, pb.of);
        assert_ne!(pa.part, pb.part);
    }

    #[test]
    fn partition_zero_parts_is_range() {
        assert_eq!(
            partition_record(&StrongDigest::ZERO, 0),
            Err(QdnfError::Range)
        );
    }

    #[test]
    fn parent_cycle_is_conflict() {
        let mut t = ParentTable::new();
        insert_parent(&mut t, 1, 0).unwrap();
        insert_parent(&mut t, 2, 1).unwrap();
        assert_eq!(insert_parent(&mut t, 0, 2), Err(QdnfError::Conflict));
        assert_eq!(insert_parent(&mut t, 3, 3), Err(QdnfError::Conflict));
        assert_eq!(t.parent_of(1), Some(0));
        assert_eq!(t.parent_of(0), None);
    }

    #[test]
    fn parent_out_of_table_is_range() {
        let mut t = ParentTable::new();
        assert_eq!(insert_parent(&mut t, 8, 0), Err(QdnfError::Range));
        assert_eq!(insert_parent(&mut t, 0, 8), Err(QdnfError::Range));
    }

    #[test]
    fn acyclic_chain_is_ok() {
        let mut t = ParentTable::new();
        insert_parent(&mut t, 2, 1).unwrap();
        insert_parent(&mut t, 1, 0).unwrap();
        assert_eq!(t.parent_of(2), Some(1));
        assert_eq!(t.parent_of(1), Some(0));
    }
}
