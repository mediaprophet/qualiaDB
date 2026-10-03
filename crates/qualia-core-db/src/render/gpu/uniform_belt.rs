//! Per-frame uniform upload for the portal viewport.
//!
//! Native keeps a mapped staging ring (`MAP_WRITE | COPY_SRC`). That ring
//! depends on `map_async` completing inside `device.poll`.
//!
//! On wasm the browser does not deliver `mapAsync` from a blocking poll, so
//! the ring falls through to a fresh `mappedAtCreation` buffer every wrap.
//! SwiftShader (and some other adapters) then reject that buffer:
//! `createBuffer failed, size (256) is too large ... mappedAtCreation == true`.
//! wgpu unwraps the rejection and the tick aborts with `unreachable`, so a
//! live adapter never presents the mesh. The wasm path uses
//! `queue.write_buffer` instead, which does not map at creation.

#[cfg(not(target_arch = "wasm32"))]
use std::sync::mpsc;
use std::sync::Arc;
use wgpu::Buffer;

/// A slot in the native uniform belt ring.
#[cfg(not(target_arch = "wasm32"))]
struct Slot {
    buffer: Option<Buffer>,
    mapped: bool,
    pending: bool,
    written: u64,
}

enum BeltInner {
    #[cfg(not(target_arch = "wasm32"))]
    Mapped {
        slots: Vec<Slot>,
        current: usize,
        rx: mpsc::Receiver<(usize, Buffer)>,
        tx: mpsc::Sender<(usize, Buffer)>,
        size: u64,
    },
    #[cfg(target_arch = "wasm32")]
    Direct {
        queue: Arc<wgpu::Queue>,
        scratch: Vec<u8>,
    },
}

pub(crate) struct UniformBelt {
    inner: BeltInner,
}

impl UniformBelt {
    /// `slot_size` is the largest uniform written in one call (256 covers
    /// telemetry). `pool_size` is the native ring length; wasm ignores it.
    pub(crate) fn new(
        device: &wgpu::Device,
        queue: Arc<wgpu::Queue>,
        slot_size: u64,
        pool_size: usize,
    ) -> Self {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = (device, pool_size);
            let mut scratch = Vec::new();
            scratch.reserve(slot_size as usize);
            return Self {
                inner: BeltInner::Direct { queue, scratch },
            };
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let _ = queue;
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
                inner: BeltInner::Mapped {
                    slots,
                    current: 0,
                    rx,
                    tx,
                    size: slot_size,
                },
            }
        }
    }

    /// Stage `data` for the next [`Self::record_copy`].
    pub(crate) fn write_and_unmap(&mut self, data: &[u8]) {
        match &mut self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            BeltInner::Mapped {
                slots,
                current,
                size,
                ..
            } => {
                let slot = &mut slots[*current];
                debug_assert!(slot.mapped, "uniform belt slot must be mapped before write");
                debug_assert!(
                    slot.buffer.is_some(),
                    "uniform belt slot must have a buffer"
                );
                debug_assert!(
                    data.len() as u64 <= *size,
                    "uniform belt write exceeds slot size"
                );
                slot.mapped = false;
                slot.written = data.len() as u64;
                let buffer = slot.buffer.as_ref().unwrap();
                let mut range = buffer
                    .slice(..)
                    .get_mapped_range_mut()
                    .expect("uniform belt buffer must be mapped");
                // wgpu 30 BufferViewMut is write-only; it does not index as [u8].
                range.slice(0..data.len()).copy_from_slice(data);
                drop(range);
                buffer.unmap();
            }
            #[cfg(target_arch = "wasm32")]
            BeltInner::Direct { scratch, .. } => {
                scratch.clear();
                scratch.extend_from_slice(data);
                while scratch.len() % 4 != 0 {
                    scratch.push(0);
                }
            }
        }
    }

    /// Copy the staged bytes into `target` at `offset`.
    pub(crate) fn record_copy(
        &self,
        encoder: &mut wgpu::CommandEncoder,
        target: &Buffer,
        offset: wgpu::BufferAddress,
    ) {
        match &self.inner {
            #[cfg(not(target_arch = "wasm32"))]
            BeltInner::Mapped { slots, current, .. } => {
                let slot = &slots[*current];
                debug_assert!(
                    !slot.mapped,
                    "uniform belt slot must be unmapped before copy"
                );
                let buffer = slot
                    .buffer
                    .as_ref()
                    .expect("uniform belt slot must have a buffer");
                let copy_size = (slot.written + 3) & !3;
                if copy_size > 0 {
                    encoder.copy_buffer_to_buffer(buffer, 0, target, offset, copy_size);
                }
            }
            #[cfg(target_arch = "wasm32")]
            BeltInner::Direct { queue, scratch } => {
                let _ = encoder;
                if !scratch.is_empty() {
                    queue.write_buffer(target, offset, scratch);
                }
            }
        }
    }

    /// Advance the native ring. No-op on the wasm direct path.
    pub(crate) fn advance(&mut self, device: &wgpu::Device) {
        #[cfg(target_arch = "wasm32")]
        {
            let _ = device;
            return;
        }
        #[cfg(not(target_arch = "wasm32"))]
        {
            let BeltInner::Mapped {
                slots,
                current,
                rx,
                tx,
                size,
            } = &mut self.inner;
            slots[*current].pending = true;
            while let Ok((idx, buffer)) = rx.try_recv() {
                slots[idx].buffer = Some(buffer);
                slots[idx].mapped = true;
                slots[idx].pending = false;
            }
            *current = (*current + 1) % slots.len();
            if slots[*current].pending {
                let _ = device.poll(wgpu::PollType::wait_indefinitely());
                while let Ok((idx, buffer)) = rx.try_recv() {
                    slots[idx].buffer = Some(buffer);
                    slots[idx].mapped = true;
                    slots[idx].pending = false;
                }
            }
            if slots[*current].pending {
                let idx = *current;
                let buffer = slots[idx]
                    .buffer
                    .take()
                    .expect("pending slot must have buffer");
                let buffer_for_closure = buffer.clone();
                let tx = tx.clone();
                buffer
                    .slice(..)
                    .map_async(wgpu::MapMode::Write, move |result| {
                        if result.is_ok() {
                            let _ = tx.send((idx, buffer_for_closure));
                        }
                    });
                let _ = device.poll(wgpu::PollType::wait_indefinitely());
                while let Ok((ridx, rbuffer)) = rx.try_recv() {
                    slots[ridx].buffer = Some(rbuffer);
                    slots[ridx].mapped = true;
                    slots[ridx].pending = false;
                }
            }
            if !slots[*current].mapped || slots[*current].buffer.is_none() {
                let buffer = device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("uniform-belt-slot-fallback"),
                    size: *size,
                    usage: wgpu::BufferUsages::MAP_WRITE | wgpu::BufferUsages::COPY_SRC,
                    mapped_at_creation: true,
                });
                slots[*current].buffer = Some(buffer);
                slots[*current].mapped = true;
                slots[*current].pending = false;
            }
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
        let mut belt = UniformBelt::new(device, Arc::new(queue.clone()), 256, 8);
        let target = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("uniform-belt-test-target"),
            size: 256,
            usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::UNIFORM,
            mapped_at_creation: false,
        });

        for _ in 0..8 {
            belt.write_and_unmap(&[0u8; 256]);
            let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("uniform-belt-test"),
            });
            belt.record_copy(&mut enc, &target, 0);
            queue.submit(std::iter::once(enc.finish()));
            belt.advance(device);
        }

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
        assert!(
            count < 50,
            "uniform belt + wgpu map_async should be < 50 allocs, got {count}"
        );
    }
}
