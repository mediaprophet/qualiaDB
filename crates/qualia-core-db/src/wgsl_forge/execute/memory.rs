use super::super::ForgeError;

/// Defines the topology of the underlying memory allocator.
///
/// # Implementation status (honest scope, plan §2)
///
/// Plan §2 calls for *differentiated* memory paths: *"larger persistent slabs with
/// zero-copy for unified memory, pinned staging rings with async `copy_buffer` for
/// discrete PCIe devices."* What is **implemented and verified** today is the
/// classification plus the topology-aware, lap-safe ring/slab allocator
/// ([`QualiaSlabAllocator`], proven not to lap its read head under sustained
/// dispatch — see `sustained_dispatch_never_laps_read_head`).
///
/// What is **NOT yet implemented** is the *differentiation* of the physical copy
/// path by topology. The `zero_copy` / `staging_required` flags are descriptive
/// tags only: the wgpu backend ([`super::wgpu`]) uses the **same** uniform path on
/// both topologies — `queue.write_buffer` for host→device uploads and
/// `copy_buffer_to_buffer` for readback — regardless of the variant here. That
/// path is correct on both unified and discrete hardware; it is simply not yet
/// *optimised* per topology.
///
/// This is a deliberate measurement-honesty decision: the development host is a
/// discrete-only NVIDIA RTX A2000 (no unified memory), so a unified zero-copy
/// persistent-mapped path cannot be exercised or verified here, and shipping
/// unverified memory-mapping code would over-claim. The differentiated paths
/// (zero-copy persistent-mapped slabs for unified; pinned staging ring + async
/// copy for discrete) are documented future work, to be built and benchmarked on
/// unified-memory hardware where the benefit is actually measurable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoryTopology {
    /// Memory is shared between host and device (e.g., Apple Silicon, APUs).
    /// Zero-copy mapping is *preferred in principle* but not yet implemented — see
    /// the type-level note above. Currently classified, not yet exploited.
    Unified { zero_copy: bool },
    /// Memory requires a staging buffer (PCIe transfer to discrete VRAM). A pinned
    /// staging ring with async `copy_buffer` is *intended* but not yet implemented
    /// — see the type-level note above. Currently classified, not yet exploited.
    Discrete { staging_required: bool },
}

/// How a [`BufferView`] is bound in a dispatch. Determines which physical slab
/// backs it: wgpu forbids a single buffer being bound as both read-only and
/// read-write storage in one dispatch, so read-write outputs live in a separate
/// buffer from read-only/uniform inputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BindingUsage {
    StorageRead,
    StorageReadWrite,
    Uniform,
    /// Read-only storage that lives in the **persistent weight region**, not the recycling
    /// transient ring. Backed by its own device buffer (`weight_slab`) and a write-once bump
    /// cursor, so a view with this usage survives [`super::wgpu::WgpuComputeContext::clear_transient_allocations`]
    /// — uploaded once (e.g. a decode layer's projection / FFN matrices) and referenced by offset
    /// across many `run`s. Bound exactly like [`Self::StorageRead`] in the shader (a distinct
    /// read-only storage buffer in the same bind group); only the backing buffer differs.
    StorageReadResident,
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
    /// How this view is bound (selects the backing slab on the wgpu backend).
    pub usage: BindingUsage,
}

/// WebGPU's maximum `min_{storage,uniform}_buffer_offset_alignment`. Aligning
/// every view to this value satisfies any conforming adapter's bind-group offset
/// requirement (the A2000 reports 256), so slab sub-ranges can be bound directly.
pub const DEFAULT_BINDING_ALIGNMENT: usize = 256;

/// A topology-aware ring buffer designed to eliminate heap allocations
/// during hot-path kernel dispatch. Maintains strict bounds invariant
/// so the write_head never laps the read_head.
///
/// Every allocation's start offset is rounded up to `alignment` bytes so the
/// resulting [`BufferView`] can be used directly as a wgpu bind-group entry
/// without violating `min_{storage,uniform}_buffer_offset_alignment`.
#[derive(Debug)]
pub struct QualiaSlabAllocator {
    pub topology: MemoryTopology,
    capacity_bytes: u64,
    alignment: u64,
    read_count: u64,
    write_count: u64,
}

impl QualiaSlabAllocator {
    pub fn new(topology: MemoryTopology, capacity_bytes: usize) -> Self {
        Self::new_with_alignment(topology, capacity_bytes, DEFAULT_BINDING_ALIGNMENT)
    }

    /// Constructs an allocator with an explicit binding alignment. The usable
    /// capacity is floored to a multiple of `alignment` so that wrapped offsets
    /// (computed modulo the capacity) remain aligned.
    pub fn new_with_alignment(
        topology: MemoryTopology,
        capacity_bytes: usize,
        alignment: usize,
    ) -> Self {
        let alignment = (alignment as u64).max(1);
        let capacity = (capacity_bytes as u64) / alignment * alignment;
        Self {
            topology,
            capacity_bytes: capacity,
            alignment,
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
    /// The start offset is aligned up to the binding alignment. The allocator
    /// bumps the write_count past any alignment/wrap padding plus the size. If
    /// the end of the slab is reached, it wraps around to 0 (itself aligned). If
    /// the new allocation would lap the read_count, it returns an error.
    pub fn allocate_transient(
        &mut self,
        size_bytes: usize,
        binding: u32,
        group: u32,
        usage: BindingUsage,
    ) -> Result<BufferView, ForgeError> {
        let size_u64 = size_bytes as u64;
        if size_u64 > self.capacity_bytes {
            return Err(ForgeError::GpuValidation(
                "Allocation exceeds total slab capacity".to_string(),
            ));
        }

        // Align the write head up to the binding alignment, accounting for the
        // padding bytes consumed.
        let aligned_write = align_up(self.write_count, self.alignment);
        let mut padding = aligned_write - self.write_count;
        let mut offset = aligned_write % self.capacity_bytes;

        // Handle wrap-around: pad out the slab tail and restart at 0 (aligned,
        // since the capacity is a multiple of the alignment).
        if offset + size_u64 > self.capacity_bytes {
            padding += self.capacity_bytes - offset;
            offset = 0;
        }

        // Check if allocation + padding laps the read head
        if self.write_count + padding + size_u64 - self.read_count > self.capacity_bytes {
            return Err(ForgeError::GpuValidation(
                "Write head lapped read head in slab allocator".to_string(),
            ));
        }

        self.write_count += padding + size_u64;

        Ok(BufferView {
            offset: offset as usize,
            length_bytes: size_bytes,
            binding,
            group,
            usage,
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

    /// Snapshot of the write cursor (absolute, not modulo capacity).
    pub fn write_checkpoint(&self) -> u64 {
        self.write_count
    }

    /// Rewind write/read heads to a prior checkpoint so later transient allocations
    /// reuse the same slab region **without** overwriting earlier persistent views
    /// (device bytes below the checkpoint remain intact). Sync before calling.
    pub fn restore_checkpoint(&mut self, write_count: u64) {
        self.write_count = write_count;
        self.read_count = write_count;
    }
}

fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment <= 1 {
        return value;
    }
    value.div_ceil(alignment) * alignment
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ring_buffer_allocates_and_wraps() {
        // Alignment 1 exercises the pure ring-buffer bookkeeping.
        let mut allocator = QualiaSlabAllocator::new_with_alignment(
            MemoryTopology::Unified { zero_copy: true },
            100,
            1,
        );

        // Allocate 60 bytes
        let view1 = allocator
            .allocate_transient(60, 0, 0, BindingUsage::StorageReadWrite)
            .unwrap();
        assert_eq!(view1.offset, 0);
        assert_eq!(allocator.write_count, 60);

        // Attempting to allocate 50 bytes should wrap, but it would lap the read_count (0), so it fails
        let err = allocator.allocate_transient(50, 1, 0, BindingUsage::StorageReadWrite);
        assert!(err.is_err());

        // Advance the read head to 60 (meaning view1 is freed)
        allocator.advance_read_head(60);

        // Now the 50 byte allocation should wrap to 0 and succeed.
        // It will add 40 bytes of padding to wrap around, plus the 50 bytes.
        let view2 = allocator
            .allocate_transient(50, 1, 0, BindingUsage::StorageReadWrite)
            .unwrap();
        assert_eq!(view2.offset, 0);
        assert_eq!(allocator.write_count, 150);

        // Another 10 byte allocation should succeed
        let view3 = allocator
            .allocate_transient(10, 2, 0, BindingUsage::StorageReadWrite)
            .unwrap();
        assert_eq!(view3.offset, 50);
        assert_eq!(allocator.write_count, 160);

        // Another 1 byte allocation should fail since write_count (160) - read_count (60) == 100
        let err2 = allocator.allocate_transient(1, 3, 0, BindingUsage::StorageReadWrite);
        assert!(err2.is_err());
    }

    #[test]
    fn sustained_dispatch_never_laps_read_head() {
        // Plan §10: under sustained, high-throughput async dispatch the read/write
        // heads must never lap. Model several in-flight buffers (the read head lags
        // the write head by a few "dispatches") and recycle the ring many times.
        let capacity = 1 << 14; // 16 KiB, a multiple of the 256-byte alignment
        let mut allocator = QualiaSlabAllocator::new(
            MemoryTopology::Discrete {
                staging_required: true,
            },
            capacity,
        );
        let chunk = 512usize;
        let in_flight = 4usize; // up to 4 dispatches outstanding (4*512 << 16 KiB)
        let mut pending: std::collections::VecDeque<usize> = std::collections::VecDeque::new();
        for i in 0..100_000 {
            let view = allocator
                .allocate_transient(chunk, 0, 0, BindingUsage::StorageReadWrite)
                .unwrap_or_else(|e| panic!("iteration {i} must fit in the ring: {e}"));
            assert_eq!(view.offset % DEFAULT_BINDING_ALIGNMENT, 0);
            assert!(view.offset + view.length_bytes <= allocator.capacity());
            pending.push_back(view.offset + chunk);
            // Once the pipeline is `in_flight` deep, the oldest dispatch completes
            // and its region is freed by advancing the read head past it.
            if pending.len() > in_flight {
                let freed = pending.pop_front().unwrap();
                allocator.advance_read_head(freed % allocator.capacity());
            }
        }
    }

    #[test]
    fn every_view_offset_is_binding_aligned() {
        // Mirrors the affine case (4099 f32 = 16396 bytes) that wgpu rejected at
        // offset 16396 for not respecting min_storage_buffer_offset_alignment.
        let mut allocator = QualiaSlabAllocator::new(
            MemoryTopology::Discrete {
                staging_required: true,
            },
            1 << 20,
        );
        let sizes = [16_396usize, 16_396, 16, 4, 65_537];
        let mut last_end = 0usize;
        for (binding, size) in sizes.into_iter().enumerate() {
            let view = allocator
                .allocate_transient(size, binding as u32, 0, BindingUsage::StorageReadWrite)
                .unwrap();
            assert_eq!(
                view.offset % DEFAULT_BINDING_ALIGNMENT,
                0,
                "offset {} not {}-aligned",
                view.offset,
                DEFAULT_BINDING_ALIGNMENT
            );
            assert!(view.offset >= last_end, "allocations must not overlap");
            last_end = view.offset + view.length_bytes;
        }
    }
}
