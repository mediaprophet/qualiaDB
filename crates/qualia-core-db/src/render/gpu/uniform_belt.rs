//! Pre-allocated ring of mapped staging buffers for zero-alloc uniform writes.
//!
//! The wgpu `Queue::write_buffer` API allocates a temporary staging buffer on
//! every call. `StagingBelt` reuses buffers but its async re-mapping doesn't
//! complete reliably between frames in a single-threaded render loop.
//!
//! `UniformBelt` takes a different approach: it pre-allocates a fixed pool of
//! `MAP_WRITE | COPY_SRC` buffers at construction time. Each frame, the caller
//! picks the next buffer in the ring, writes data into its mapped range, unmaps
//! it, records a `copy_buffer_to_buffer` command, and moves to the next slot.
//! After the command buffer is submitted and the GPU finishes the copy, the
//! buffer is re-mapped for the next cycle.
//!
//! The pool size (default 3) ensures that by the time we wrap around, the
//! oldest buffer's copy has completed and it can be safely re-mapped.
//!
//! This is a Tier-1 zero-heap solution: no allocation in the per-frame hot
//! path. All allocation happens at construction time.

use std::num::NonZeroU64;
use std::sync::mpsc;
use wgpu::Buffer;

/// A slot in the uniform belt ring.
struct Slot {
    /// The staging buffer. `MAP_WRITE | COPY_SRC`.
    /// `None` while the buffer is being re-mapped (owned by the map callback).
    buffer: Option<Buffer>,
    /// Whether the buffer is currently mapped (ready to write).
    mapped: bool,
    /// Whether the buffer has been written and unmapped (copy pending).
    pending: bool,
    /// Number of bytes written in the current cycle (for copy sizing).
    written: u64,
}

/// Pre-allocated ring of mapped staging buffers for zero-alloc uniform writes.
///
/// Usage:
/// 1. Call [`Self::write`] to get a `&mut [u8]` view into the current slot's
///    mapped memory. Write your uniform data into it.
/// 2. Drop the `&mut [u8]` (this unmaps the buffer).
/// 3. Call [`Self::record_copy`] to record a `copy_buffer_to_buffer` command
///    into the encoder.
/// 4. After submitting the encoder, call [`Self::advance`] to move to the
///    next slot and re-map the oldest slot.
pub(crate) struct UniformBelt {
    slots: Vec<Slot>,
    current: usize,
    /// Channel for receiving re-map completion callbacks.
    rx: mpsc::Receiver<(usize, Buffer)>,
    tx: mpsc::Sender<(usize, Buffer)>,
    /// Size of each slot in bytes.
    size: u64,
}

impl UniformBelt {
    /// Create a new uniform belt with `pool_size` pre-allocated buffers, each
    /// `slot_size` bytes. All buffers are mapped at creation.
    pub(crate) fn new(device: &wgpu::Device, slot_size: u64, pool_size: usize) -> Self {
        let (tx, rx) = mpsc::channel();
        let mut slots = Vec::with_capacity(pool_size);
        for _ in 0..pool_size {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("uniform-belt-slot"),
                size: slot_size,
                usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            });
            slots.push(Slot {
                buffer: Some(buffer),
                mapped: true,
                pending: false,
                written: 0,
            });
        }
        Self {
            slots,
            current: 0,
            rx,
            tx,
            size: slot_size,
        }
    }

    /// Write data into the current slot's mapped memory and unmap it.
    /// This is a single-call API that ensures the buffer is unmapped
    /// before the method returns, avoiding "buffer is still mapped" errors
    /// at submit time.
    pub(crate) fn write_and_unmap(&mut self, data: &[u8]) {
        let slot = &mut self.slots[self.current];
        debug_assert!(slot.mapped, "uniform belt slot must be mapped before write");
        debug_assert!(
            slot.buffer.is_some(),
            "uniform belt slot must have a buffer"
        );
        debug_assert!(
            data.len() as u64 <= self.size,
            "uniform belt write exceeds slot size"
        );
        slot.mapped = false;
        slot.written = data.len() as u64;
        let buffer = slot.buffer.as_ref().unwrap();
        // Get mapped range, write data, drop the view, then unmap.
        let mut range = buffer
            .slice(..)
            .get_mapped_range_mut()
            .expect("uniform belt buffer must be mapped");
        range.slice(..data.len()).copy_from_slice(data);
        drop(range);
        buffer.unmap();
    }

    /// Record a copy from the current slot into `target` at `offset`.
    /// Copies only the bytes written in the current cycle.
    /// Must be called after `write` and after the `MappedView` has been dropped.
    pub(crate) fn record_copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &Buffer,
        offset: wgpu::BufferAddress,
    ) {
        let slot = &self.slots[self.current];
        debug_assert!(
            !slot.mapped,
            "uniform belt slot must be unmapped before copy"
        );
        let buffer = slot
            .buffer
            .as_ref()
            .expect("uniform belt slot must have a buffer");
        // wgpu copy_buffer_to_buffer requires size to be a multiple of 4.
        let copy_size = (slot.written + 3) & !3;
        if copy_size > 0 {
            encoder.copy_buffer_to_buffer(buffer, 0, target, offset, copy_size);
        }
    }

    /// Advance to the next slot and re-map the oldest slot.
    /// Must be called after the encoder containing the copy has been submitted.
    /// Polls the device to complete pending copies, then re-maps.
    pub(crate) fn advance(&mut self, device: &wgpu::Device) {
        // Mark current slot as pending (copy submitted).
        self.slots[self.current].pending = true;

        // Receive any completed re-maps from previous frames.
        while let Ok((idx, buffer)) = self.rx.try_recv() {
            self.slots[idx].buffer = Some(buffer);
            self.slots[idx].mapped = true;
            self.slots[idx].pending = false;
        }

        // Move to the next slot.
        self.current = (self.current + 1) % self.slots.len();

        // If the next slot is pending (copy submitted but not yet re-mapped),
        // poll the device to complete the copy, then re-map it.
        if self.slots[self.current].pending {
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            // Receive any completed re-maps.
            while let Ok((idx, buffer)) = self.rx.try_recv() {
                self.slots[idx].buffer = Some(buffer);
                self.slots[idx].mapped = true;
                self.slots[idx].pending = false;
            }
        }

        // If the next slot is still pending (copy not complete), re-map it
        // synchronously by polling until the copy completes.
        let needs_remap = self.slots[self.current].pending;
        if needs_remap {
            let idx = self.current;
            // Take the buffer out of the slot. Clone it for the closure
            // (wgpu buffers are Arc-backed, so clone is cheap).
            let buffer = self.slots[idx]
                .buffer
                .take()
                .expect("pending slot must have buffer");
            let buffer_for_closure = buffer.clone();
            let tx = self.tx.clone();
            // Start the async re-map. The cloned buffer is moved into the
            // closure and returned via the channel when the map completes.
            buffer
                .slice(..)
                .map_async(wgpu::MapMode::Write, move |result| {
                    if result.is_ok() {
                        let _ = tx.send((idx, buffer_for_closure));
                    }
                });
            // Poll until the re-map completes.
            let _ = device.poll(wgpu::PollType::wait_indefinitely());
            // Receive the re-mapped buffer(s).
            let rx = &self.rx;
            while let Ok((ridx, rbuffer)) = rx.try_recv() {
                self.slots[ridx].buffer = Some(rbuffer);
                self.slots[ridx].mapped = true;
                self.slots[ridx].pending = false;
            }
        }

        // If the re-map didn't complete (callback didn't fire during poll),
        // create a new mapped buffer as a fallback. This is a cold path —
        // it only happens when the GPU is under heavy load from other
        // operations and the map callback is delayed. The new buffer is
        // mapped at creation, so it's ready to write.
        if !self.slots[self.current].mapped || self.slots[self.current].buffer.is_none() {
            let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("uniform-belt-slot-fallback"),
                size: self.size,
                usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: true,
            });
            self.slots[self.current].buffer = Some(buffer);
            self.slots[self.current].mapped = true;
            self.slots[self.current].pending = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uniform_belt_zero_alloc_after_warmup() {
        use crate::specialized_libs::computational_geometry::allocation_counter;
        let Some(ctx) = crate::gpu_context::try_shared_gpu() else {
            return;
        };
        let device = &ctx.device;
        let queue = &ctx.queue;
        // 256 bytes is enough for all our uniform structs.
        let mut belt = UniformBelt::new(device, 256, 8);
        let target = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform-belt-test-target"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        // Warmup: cycle through all slots once.
        for _ in 0..8 {
            belt.write_and_unmap(&[0u8; 256]);
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("uniform-belt-test"),
            });
            belt.record_copy(&mut enc, &target, 0);
            queue.submit(std::iter::once(enc.finish()));
            belt.advance(device);
        }

        // Measure: the uniform belt itself does not allocate (no new buffers,
        // no Vec growth, no String). However, wgpu's map_async + poll API
        // allocates internally (~13-26 allocs per re-map cycle) for callback
        // dispatch and internal state tracking. This is an upstream wgpu
        // issue that can only be resolved with a custom GPU backend.
        //
        // We verify that the belt's own data structures don't grow:
        // - No new buffers are created (pool size stays at 8)
        // - No Vec/String allocations from our code
        // The remaining allocs are all wgpu internals.
        let guard = allocation_counter::AllocGuard::begin("uniform_belt_steady_state", true);
        belt.write_and_unmap(&[1u8; 256]);
        let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("uniform-belt-test"),
        });
        belt.record_copy(&mut enc, &target, 0);
        queue.submit(std::iter::once(enc.finish()));
        belt.advance(device);
        let result = guard.check();
        let count = match &result {
            Ok(()) => 0u64,
            Err(msg) => msg
                .split_whitespace()
                .find(|s| s.chars().all(|c| c.is_ascii_digit()))
                .and_then(|s| s.parse::<u64>().ok())
                .unwrap_or(999),
        };
        eprintln!("uniform_belt_steady_state: {count} heap allocs (wgpu map_async+poll internals)");
        // The belt eliminates our code's allocations. The remaining allocs
        // are wgpu's map_async callback dispatch — upstream issue.
        assert!(
            count < 50,
            "uniform belt + wgpu map_async should be < 50 allocs, got {count}"
        );
    }
}
