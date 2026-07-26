use super::config::{PagedKvConfig, INVALID_BLOCK};
use super::pool::{BlockPool, CopyOnWrite, PoolError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TableError {
    InvalidConfig,
    OutputTooSmall,
    OutOfRange,
    TargetNotEmpty,
    Pool(PoolError),
}

impl From<PoolError> for TableError {
    fn from(value: PoolError) -> Self {
        Self::Pool(value)
    }
}

/// Fill the production single-sequence GPU table with deterministic identity pages.
///
/// Layout is `[layer][logical_page] -> physical_page`. With identity pages the byte layout is
/// compatible with the former dense `[layer][token][K|V]` arena, which enables staged rollout and
/// differential testing while kernels consume the real block table.
pub fn fill_identity_block_table(
    config: &PagedKvConfig,
    out: &mut [u32],
) -> Result<usize, TableError> {
    if !config.is_valid() {
        return Err(TableError::InvalidConfig);
    }
    let required = config.required_single_sequence_blocks() as usize;
    if out.len() < required {
        return Err(TableError::OutputTooSmall);
    }
    for (index, entry) in out[..required].iter_mut().enumerate() {
        *entry = index as u32;
    }
    Ok(required)
}

/// Cold-owned immutable block table uploaded by a prepared GPU plan.
#[derive(Debug)]
pub struct GpuBlockTablePlan {
    config: PagedKvConfig,
    entries: Vec<u32>,
}

impl GpuBlockTablePlan {
    pub fn identity(config: PagedKvConfig) -> Result<Self, TableError> {
        let mut entries = vec![INVALID_BLOCK; config.required_single_sequence_blocks() as usize];
        fill_identity_block_table(&config, &mut entries)?;
        Ok(Self { config, entries })
    }

    pub fn config(&self) -> PagedKvConfig {
        self.config
    }

    pub fn entries(&self) -> &[u32] {
        &self.entries
    }
}

/// One sequence's logical-page ownership. Capacity is fixed at construction.
#[derive(Debug)]
pub struct SequenceBlockTable {
    entries: Vec<u32>,
}

impl SequenceBlockTable {
    pub fn new(logical_pages: u32) -> Self {
        Self {
            entries: vec![INVALID_BLOCK; logical_pages as usize],
        }
    }

    pub fn entries(&self) -> &[u32] {
        &self.entries
    }

    pub fn get(&self, logical_page: u32) -> Option<u32> {
        self.entries
            .get(logical_page as usize)
            .copied()
            .filter(|block| *block != INVALID_BLOCK)
    }

    pub fn ensure_writable(
        &mut self,
        logical_page: u32,
        pool: &mut BlockPool,
    ) -> Result<CopyOnWrite, TableError> {
        let entry = self
            .entries
            .get_mut(logical_page as usize)
            .ok_or(TableError::OutOfRange)?;
        let action = pool.writable(*entry)?;
        *entry = match action {
            CopyOnWrite::Existing(block) | CopyOnWrite::Allocated(block) => block,
            CopyOnWrite::Copy { destination, .. } => destination,
        };
        Ok(action)
    }

    pub fn fork_into(&self, target: &mut Self, pool: &mut BlockPool) -> Result<(), TableError> {
        if self.entries.len() != target.entries.len() {
            return Err(TableError::OutOfRange);
        }
        if target.entries.iter().any(|block| *block != INVALID_BLOCK) {
            return Err(TableError::TargetNotEmpty);
        }
        let mut retained = 0usize;
        for &block in &self.entries {
            if block != INVALID_BLOCK {
                if let Err(error) = pool.retain(block) {
                    for &rollback in &self.entries[..retained] {
                        if rollback != INVALID_BLOCK {
                            let _ = pool.release(rollback);
                        }
                    }
                    return Err(error.into());
                }
            }
            retained += 1;
        }
        target.entries.copy_from_slice(&self.entries);
        Ok(())
    }

    /// Attach graph/prompt-cache prefix pages to an empty sequence table.
    ///
    /// Reference counts are retained before publishing entries; failures roll back, so a denied
    /// or stale graph prefix cannot partially mutate request state.
    pub fn install_shared_prefix(
        &mut self,
        pages: &[u32],
        pool: &mut BlockPool,
    ) -> Result<(), TableError> {
        if pages.len() > self.entries.len() {
            return Err(TableError::OutOfRange);
        }
        if self.entries.iter().any(|block| *block != INVALID_BLOCK) {
            return Err(TableError::TargetNotEmpty);
        }
        for (index, &page) in pages.iter().enumerate() {
            if let Err(error) = pool.retain(page) {
                for &rollback in &pages[..index] {
                    let _ = pool.release(rollback);
                }
                return Err(error.into());
            }
        }
        self.entries[..pages.len()].copy_from_slice(pages);
        Ok(())
    }

    pub fn release_all(&mut self, pool: &mut BlockPool) -> Result<(), TableError> {
        for entry in &mut self.entries {
            if *entry != INVALID_BLOCK {
                pool.release(*entry)?;
                *entry = INVALID_BLOCK;
            }
        }
        Ok(())
    }
}
