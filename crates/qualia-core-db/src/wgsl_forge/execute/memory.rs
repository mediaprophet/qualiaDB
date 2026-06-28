use super::super::ForgeError;

/// Defines the topology of the underlying memory allocator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTopology {
    /// Memory is shared between host and device (e.g., Apple Silicon, APUs).
    /// Zero-copy mapping is preferred.
    Unified { zero_copy: bool },
    /// Memory requires a staging buffer (PCIe transfer to discrete VRAM).
    Discrete { staging_required: bool },
}

/// A lightweight, heap-free pointer representing a contiguous memory slice on the device.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BufferView {
    /// Offset in bytes from the start of the slab.
    pub offset: usize,
    /// Length in bytes of this slice.
    pub length_bytes: usize,
    /// Pipeline binding slot.
    pub binding: u32,
    /// Pipeline bind group.
    pub group: u32,
}

/// A topology-aware ring buffer designed to eliminate heap allocations 
/// during hot-path kernel dispatch. Maintains strict bounds invariant
/// so the write_head never laps the read_head.
#[derive(Debug)]
pub struct QualiaSlabAllocator {
    pub topology: MemoryTopology,
    capacity_bytes: u64,
    read_count: u64,
    write_count: u64,
}

impl QualiaSlabAllocator {
    pub fn new(topology: MemoryTopology, capacity_bytes: usize) -> Self {
        Self {
            topology,
            capacity_bytes: capacity_bytes as u64,
            read_count: 0,
            write_count: 0,
        }
    }

    /// Returns the capacity in bytes.
    pub fn capacity(&self) -> usize {
        self.capacity_bytes as usize
    }

    /// Allocates a transient memory slice within the slab.
    ///
    /// The allocator bumps the write_count. If the end of the slab is reached, 
    /// it wraps around to 0. If the new allocation would lap the read_count, 
    /// it returns an OutOfMemory error.
    pub fn allocate_transient(&mut self, size_bytes: usize, binding: u32, group: u32) -> Result<BufferView, ForgeError> {
        let size_u64 = size_bytes as u64;
        if size_u64 > self.capacity_bytes {
            return Err(ForgeError::GpuValidation("Allocation exceeds total slab capacity".to_string()));
        }

        let mut offset = self.write_count % self.capacity_bytes;
        let mut padding = 0;
        
        // Handle wrap-around padding
        if offset + size_u64 > self.capacity_bytes {
            padding = self.capacity_bytes - offset;
            offset = 0;
        }

        // Check if allocation + padding laps the read head
        if self.write_count + padding + size_u64 - self.read_count > self.capacity_bytes {
             return Err(ForgeError::GpuValidation("Write head lapped read head in slab allocator".to_string()));
        }

        self.write_count += padding + size_u64;

        Ok(BufferView {
            offset: offset as usize,
            length_bytes: size_bytes,
            binding,
            group,
        })
    }

    /// Advances the read head to free up space. 
    /// This should be called *after* a device synchronization fence guarantees
    /// the buffer region is no longer in flight.
    pub fn advance_read_head(&mut self, new_head_offset: usize) {
        let current_offset = (self.read_count % self.capacity_bytes) as usize;
        let advance = if new_head_offset >= current_offset {
            new_head_offset - current_offset
        } else {
            (self.capacity_bytes as usize) - current_offset + new_head_offset
        };
        self.read_count += advance as u64;
    }

    /// Clears all allocations, resetting the read head to the write head.
    /// MUST only be called when all device operations are fully synchronized and complete.
    pub fn clear(&mut self) {
        self.read_count = self.write_count;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_allocates_and_wraps() {
        let mut allocator = QualiaSlabAllocator::new(MemoryTopology::Unified { zero_copy: true }, 100);
        
        // Allocate 60 bytes
        let view1 = allocator.allocate_transient(60, 0, 0).unwrap();
        assert_eq!(view1.offset, 0);
        assert_eq!(allocator.write_count, 60);

        // Attempting to allocate 50 bytes should wrap, but it would lap the read_count (0), so it fails
        let err = allocator.allocate_transient(50, 1, 0);
        assert!(err.is_err());

        // Advance the read head to 60 (meaning view1 is freed)
        allocator.advance_read_head(60);

        // Now the 50 byte allocation should wrap to 0 and succeed.
        // It will add 40 bytes of padding to wrap around, plus the 50 bytes.
        let view2 = allocator.allocate_transient(50, 1, 0).unwrap();
        assert_eq!(view2.offset, 0);
        assert_eq!(allocator.write_count, 150);

        // Another 10 byte allocation should succeed
        let view3 = allocator.allocate_transient(10, 2, 0).unwrap();
        assert_eq!(view3.offset, 50);
        assert_eq!(allocator.write_count, 160);
        
        // Another 1 byte allocation should fail since write_count (160) - read_count (60) == 100
        let err2 = allocator.allocate_transient(1, 3, 0);
        assert!(err2.is_err());
    }
}
