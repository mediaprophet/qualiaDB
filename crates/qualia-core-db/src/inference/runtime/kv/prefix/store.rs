use crate::inference::runtime::graph_assist::{
    PrefixIdentity, PrefixPageRegistry, PrefixPageSet, RegistryError,
};

use super::super::paged::{BlockPool, PoolError, SequenceBlockTable, TableError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrefixKvError {
    Registry(RegistryError),
    Pool(PoolError),
    Table(TableError),
}

impl From<RegistryError> for PrefixKvError {
    fn from(value: RegistryError) -> Self {
        Self::Registry(value)
    }
}

impl From<PoolError> for PrefixKvError {
    fn from(value: PoolError) -> Self {
        Self::Pool(value)
    }
}

impl From<TableError> for PrefixKvError {
    fn from(value: TableError) -> Self {
        Self::Table(value)
    }
}

/// Fixed-capacity graph-prefix store with explicit physical-page ownership.
pub struct PrefixKvStore<const ENTRIES: usize, const PAGES: usize> {
    registry: PrefixPageRegistry<ENTRIES, PAGES>,
}

impl<const ENTRIES: usize, const PAGES: usize> PrefixKvStore<ENTRIES, PAGES> {
    pub const fn new() -> Self {
        Self {
            registry: PrefixPageRegistry::new(),
        }
    }

    pub fn get(&self, identity: PrefixIdentity) -> Option<PrefixPageSet<PAGES>> {
        self.registry.get(identity).copied()
    }

    /// Publish pages and retain one pool reference for the registry.
    ///
    /// Retains happen before publication. A failed retain or registry insert rolls back, and an
    /// update/eviction releases the old registry ownership only after the new set is visible.
    pub fn publish(
        &mut self,
        set: PrefixPageSet<PAGES>,
        pool: &mut BlockPool,
    ) -> Result<(), PrefixKvError> {
        let pages = set.page_slice();
        for (index, &page) in pages.iter().enumerate() {
            if let Err(error) = pool.retain(page) {
                for &rollback in &pages[..index] {
                    let _ = pool.release(rollback);
                }
                return Err(error.into());
            }
        }
        let replaced = match self.registry.insert_replacing(set) {
            Ok(replaced) => replaced,
            Err(error) => {
                for &page in pages {
                    let _ = pool.release(page);
                }
                return Err(error.into());
            }
        };
        if let Some(old) = replaced {
            for &page in old.page_slice() {
                pool.release(page)?;
            }
        }
        Ok(())
    }

    /// Attach a matching prefix to an empty request table.
    pub fn attach(
        &self,
        identity: PrefixIdentity,
        table: &mut SequenceBlockTable,
        pool: &mut BlockPool,
    ) -> Result<Option<u32>, PrefixKvError> {
        let Some(set) = self.get(identity) else {
            return Ok(None);
        };
        table.install_shared_prefix(set.page_slice(), pool)?;
        Ok(Some(set.token_count))
    }

    /// Evict a prefix and release the registry's page references.
    pub fn remove(
        &mut self,
        identity: PrefixIdentity,
        pool: &mut BlockPool,
    ) -> Result<bool, PrefixKvError> {
        let Some(removed) = self.registry.remove_entry(identity) else {
            return Ok(false);
        };
        for &page in removed.page_slice() {
            pool.release(page)?;
        }
        Ok(true)
    }
}

impl<const ENTRIES: usize, const PAGES: usize> Default for PrefixKvStore<ENTRIES, PAGES> {
    fn default() -> Self {
        Self::new()
    }
}
