use super::config::INVALID_BLOCK;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoolError {
    Exhausted,
    InvalidBlock,
    ReferenceOverflow,
    DoubleRelease,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CopyOnWrite {
    Existing(u32),
    Allocated(u32),
    Copy { source: u32, destination: u32 },
}

/// Fixed-capacity physical-page allocator with reference counts.
///
/// Both vectors are fully sized during cold construction. Hot operations only mutate elements or
/// the existing vector length; `free` has capacity `physical_blocks`, so `push` cannot reallocate.
#[derive(Debug)]
pub struct BlockPool {
    free: Vec<u32>,
    refs: Vec<u32>,
}

impl BlockPool {
    pub fn new(physical_blocks: u32) -> Self {
        let mut free = Vec::with_capacity(physical_blocks as usize);
        free.extend((0..physical_blocks).rev());
        Self {
            free,
            refs: vec![0; physical_blocks as usize],
        }
    }

    pub fn allocate(&mut self) -> Result<u32, PoolError> {
        let block = self.free.pop().ok_or(PoolError::Exhausted)?;
        self.refs[block as usize] = 1;
        Ok(block)
    }

    pub fn retain(&mut self, block: u32) -> Result<(), PoolError> {
        let count = self
            .refs
            .get_mut(block as usize)
            .ok_or(PoolError::InvalidBlock)?;
        if *count == 0 {
            return Err(PoolError::InvalidBlock);
        }
        *count = count.checked_add(1).ok_or(PoolError::ReferenceOverflow)?;
        Ok(())
    }

    pub fn release(&mut self, block: u32) -> Result<(), PoolError> {
        let count = self
            .refs
            .get_mut(block as usize)
            .ok_or(PoolError::InvalidBlock)?;
        if *count == 0 {
            return Err(PoolError::DoubleRelease);
        }
        *count -= 1;
        if *count == 0 {
            debug_assert!(self.free.len() < self.free.capacity());
            self.free.push(block);
        }
        Ok(())
    }

    pub fn ref_count(&self, block: u32) -> Option<u32> {
        self.refs.get(block as usize).copied()
    }

    pub fn free_count(&self) -> usize {
        self.free.len()
    }

    pub fn writable(&mut self, current: u32) -> Result<CopyOnWrite, PoolError> {
        if current == INVALID_BLOCK {
            return self.allocate().map(CopyOnWrite::Allocated);
        }
        match self.ref_count(current) {
            Some(1) => Ok(CopyOnWrite::Existing(current)),
            Some(count) if count > 1 => {
                let destination = self.allocate()?;
                self.release(current)?;
                Ok(CopyOnWrite::Copy {
                    source: current,
                    destination,
                })
            }
            _ => Err(PoolError::InvalidBlock),
        }
    }
}
