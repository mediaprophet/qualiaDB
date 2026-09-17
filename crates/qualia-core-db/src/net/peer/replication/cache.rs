//! Canonical QSync cache keys (E06.3).
//!
//! Key = SHA-384 transcript over scope || source_generation || policy_generation
//! || abi_tag. Lossy eviction is never evidence of absence. A miss is
//! [`QdnfError::Incomplete`], never EmptyInSnapshot.

use crate::crypto::network::transcript::Transcript;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

pub const MAX_CACHE: usize = 16;

const CACHE_DOMAIN: &[u8] = b"qdnf:sync:cache-key:v1-pq";

/// Lookup of a generation-bound cache entry.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CacheLookup {
    Hit(StrongDigest),
    Miss,
    UntrustedMiss,
}

#[derive(Clone, Copy)]
struct Slot {
    occupied: bool,
    scope: u64,
    abi_tag: u64,
    source_generation: Generation,
    policy_generation: Generation,
    key: StrongDigest,
    value: StrongDigest,
}

/// Sixteen-slot generation-bound cache. Not a completeness oracle.
pub struct CacheTable {
    slots: [Slot; MAX_CACHE],
}

impl CacheTable {
    pub const fn new() -> Self {
        Self {
            slots: [Slot {
                occupied: false,
                scope: 0,
                abi_tag: 0,
                source_generation: Generation::ZERO,
                policy_generation: Generation::ZERO,
                key: StrongDigest::ZERO,
                value: StrongDigest::ZERO,
            }; MAX_CACHE],
        }
    }
}

impl Default for CacheTable {
    fn default() -> Self {
        Self::new()
    }
}

/// SHA-384 transcript key over scope and the two generations plus ABI tag.
pub fn cache_key(
    scope: u64,
    source_generation: Generation,
    policy_generation: Generation,
    abi_tag: u64,
) -> Result<StrongDigest, QdnfError> {
    let mut t = Transcript::new();
    t.append(b"v", CACHE_DOMAIN)?;
    t.append(b"scope", &scope.to_be_bytes())?;
    t.append(b"source_generation", &source_generation.0.to_be_bytes())?;
    t.append(b"policy_generation", &policy_generation.0.to_be_bytes())?;
    t.append(b"abi_tag", &abi_tag.to_be_bytes())?;
    Ok(t.digest())
}

/// Insert `value` under the canonical key. Same key+value is idempotent.
///
/// [`QdnfError::Conflict`] if the key is occupied with a different value.
/// [`QdnfError::Capacity`] at sixteen distinct keys.
pub fn insert(
    table: &mut CacheTable,
    scope: u64,
    source_generation: Generation,
    policy_generation: Generation,
    abi_tag: u64,
    value: StrongDigest,
) -> Result<StrongDigest, QdnfError> {
    if value == StrongDigest::ZERO {
        return Err(QdnfError::Malformed);
    }
    let key = cache_key(scope, source_generation, policy_generation, abi_tag)?;
    if let Some(i) = find_key(table, &key) {
        if table.slots[i].value != value {
            return Err(QdnfError::Conflict);
        }
        return Ok(key);
    }
    let i = find_free(table).ok_or(QdnfError::Capacity)?;
    table.slots[i] = Slot {
        occupied: true,
        scope,
        abi_tag,
        source_generation,
        policy_generation,
        key,
        value,
    };
    Ok(key)
}

/// Look up by live generations. Generation mismatch on the same scope/ABI
/// is [`CacheLookup::UntrustedMiss`]. Absence after eviction is [`CacheLookup::Miss`].
pub fn lookup(
    table: &CacheTable,
    scope: u64,
    source_generation: Generation,
    policy_generation: Generation,
    abi_tag: u64,
) -> Result<CacheLookup, QdnfError> {
    let key = cache_key(scope, source_generation, policy_generation, abi_tag)?;
    if let Some(i) = find_key(table, &key) {
        return Ok(CacheLookup::Hit(table.slots[i].value));
    }
    if let Some(i) = find_scope_abi(table, scope, abi_tag) {
        let s = &table.slots[i];
        if s.source_generation == source_generation && s.policy_generation == policy_generation {
            return Ok(CacheLookup::Hit(s.value));
        }
        return Ok(CacheLookup::UntrustedMiss);
    }
    Ok(CacheLookup::Miss)
}

/// Drop the slot for `key`. Remaining lookups are [`CacheLookup::Miss`].
pub fn evict(table: &mut CacheTable, key: StrongDigest) -> Result<(), QdnfError> {
    let i = find_key(table, &key).ok_or(QdnfError::Incomplete)?;
    table.slots[i].occupied = false;
    Ok(())
}

/// Lossy eviction never becomes evidence of absence.
#[inline]
pub fn eviction_proves_absence() -> bool {
    false
}

/// A miss is Incomplete, never EmptyInSnapshot.
pub fn miss_is_incomplete(lookup: CacheLookup) -> Result<StrongDigest, QdnfError> {
    match lookup {
        CacheLookup::Hit(d) => Ok(d),
        CacheLookup::Miss | CacheLookup::UntrustedMiss => Err(QdnfError::Incomplete),
    }
}

fn find_key(table: &CacheTable, key: &StrongDigest) -> Option<usize> {
    let mut i = 0usize;
    while i < MAX_CACHE {
        if table.slots[i].occupied && table.slots[i].key == *key {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_free(table: &CacheTable) -> Option<usize> {
    let mut i = 0usize;
    while i < MAX_CACHE {
        if !table.slots[i].occupied {
            return Some(i);
        }
        i += 1;
    }
    None
}

fn find_scope_abi(table: &CacheTable, scope: u64, abi_tag: u64) -> Option<usize> {
    let mut i = 0usize;
    while i < MAX_CACHE {
        let s = &table.slots[i];
        if s.occupied && s.scope == scope && s.abi_tag == abi_tag {
            return Some(i);
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn value(tag: u8) -> StrongDigest {
        let mut d = StrongDigest::ZERO;
        d.0[0] = tag;
        d
    }

    #[test]
    fn eviction_is_miss_not_absence() {
        let mut table = CacheTable::new();
        let gen = Generation(1);
        let key = insert(&mut table, 7, gen, gen, 0xAB, value(1)).unwrap();
        assert_eq!(
            lookup(&table, 7, gen, gen, 0xAB).unwrap(),
            CacheLookup::Hit(value(1))
        );
        evict(&mut table, key).unwrap();
        let miss = lookup(&table, 7, gen, gen, 0xAB).unwrap();
        assert_eq!(miss, CacheLookup::Miss);
        assert!(!eviction_proves_absence());
        assert_eq!(miss_is_incomplete(miss), Err(QdnfError::Incomplete));
        assert_ne!(miss, CacheLookup::Hit(value(1)));
    }

    #[test]
    fn generation_mismatch_is_untrusted_miss() {
        let mut table = CacheTable::new();
        insert(&mut table, 7, Generation(1), Generation(1), 3, value(2)).unwrap();
        let stale = lookup(&table, 7, Generation(2), Generation(1), 3).unwrap();
        assert_eq!(stale, CacheLookup::UntrustedMiss);
        assert_eq!(miss_is_incomplete(stale), Err(QdnfError::Incomplete));
    }

    #[test]
    fn key_changes_with_each_generation_and_abi() {
        let a = cache_key(1, Generation(1), Generation(1), 1).unwrap();
        let b = cache_key(1, Generation(2), Generation(1), 1).unwrap();
        let c = cache_key(1, Generation(1), Generation(2), 1).unwrap();
        let d = cache_key(1, Generation(1), Generation(1), 2).unwrap();
        assert_ne!(a, b);
        assert_ne!(a, c);
        assert_ne!(a, d);
        assert_ne!(a, StrongDigest::ZERO);
    }

    #[test]
    fn seventeenth_insert_is_capacity() {
        let mut table = CacheTable::new();
        let gen = Generation(1);
        let mut i = 0u64;
        while i < MAX_CACHE as u64 {
            insert(&mut table, i, gen, gen, 1, value(1)).unwrap();
            i += 1;
        }
        assert_eq!(
            insert(&mut table, 99, gen, gen, 1, value(1)),
            Err(QdnfError::Capacity)
        );
    }
}
