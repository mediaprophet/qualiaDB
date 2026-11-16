//! Optional bounded GPU timestamp ranges; readback runs only on diagnostic requests.
use crate::gpu_context::{global_vram_ledger, VramReservation, VramResourceClass};
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
#[path = "gpu_timing_math.rs"]
mod math;
use math::decode;
const SLOTS: usize = 4;
const MARKERS: usize = 6;
const BYTES: u64 = (MARKERS * 8) as u64;
#[derive(Clone, Copy, PartialEq)]
enum State {
    Free,
    Recording,
    Recorded,
    Mapping,
}
struct Slot {
    queries: etalon::QuerySet,
    resolve: etalon::Buffer,
    readback: etalon::Buffer,
    signal: Arc<AtomicU8>,
    state: State,
    frame: u64,
    written: u32,
}
#[derive(Clone, Copy, Default)]
pub struct TimingSnapshot {
    pub available: bool,
    pub enabled: bool,
    pub completed_frames: u64,
    pub skipped_frames: u64,
    pub readback_errors: u64,
    pub latest_frame: u64,
    pub latest_ms: [f64; MARKERS],
    pub p50_ms: [f64; MARKERS],
    pub p95_ms: [f64; MARKERS],
    pub samples: usize,
    pub observed_frame: u64,
    pub oldest_sample_frame: u64,
}
pub(super) struct PassTimings {
    slots: Option<[Slot; SLOTS]>,
    _reservation: Option<VramReservation<'static>>,
    active: Option<usize>,
    frame: u64,
    period_ns: f64,
    snapshot: TimingSnapshot,
    history: [[f64; MARKERS]; 16],
    history_frames: [u64; 16],
    adapter_info: Option<etalon::AdapterInfo>,
}
impl PassTimings {
    pub fn new(device: &etalon::Device, queue: &etalon::Queue) -> Self {
        let period_ns = f64::from(queue.get_timestamp_period());
        let reservation = if device
            .features()
            .contains(etalon::Features::TIMESTAMP_QUERY)
            && period_ns.is_finite()
            && period_ns > 0.0
        {
            global_vram_ledger()
                .try_reserve_graphics(
                    VramResourceClass::FrameTarget,
                    SLOTS as u64 * (etalon::QUERY_RESOLVE_BUFFER_ALIGNMENT + BYTES * 2),
                )
                .ok()
        } else {
            None
        };
        let slots = reservation.as_ref().map(|_| {
            std::array::from_fn(|_| Slot {
                queries: device.create_query_set(&etalon::QuerySetDescriptor {
                    label: Some("portal-pass-timing"),
                    ty: etalon::QueryType::Timestamp,
                    count: MARKERS as u32,
                }),
                resolve: device.create_buffer(&etalon::BufferDescriptor {
                    label: Some("portal-pass-timing-resolve"),
                    size: etalon::QUERY_RESOLVE_BUFFER_ALIGNMENT,
                    usage: etalon::BufferUsages::QUERY_RESOLVE | etalon::BufferUsages::COPY_SRC,
                    mapped_at_creation: false,
                }),
                readback: device.create_buffer(&etalon::BufferDescriptor {
                    label: Some("portal-pass-timing-readback"),
                    size: BYTES,
                    usage: etalon::BufferUsages::COPY_DST | etalon::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                }),
                signal: Arc::new(AtomicU8::new(0)),
                state: State::Free,
                frame: 0,
                written: 0,
            })
        });
        let available = slots.is_some();
        Self {
            slots,
            _reservation: reservation,
            active: None,
            frame: 0,
            period_ns,
            snapshot: TimingSnapshot {
                available,
                ..Default::default()
            },
            history: [[0.0; MARKERS]; 16],
            history_frames: [0; 16],
            adapter_info: None,
        }
    }
    pub fn set_adapter_info(&mut self, info: etalon::AdapterInfo) {
        self.adapter_info = Some(info);
    }
    pub fn set_enabled(&mut self, enabled: bool) {
        self.snapshot.enabled = enabled;
    }
    pub fn begin_frame(&mut self) {
        let Some(slots) = self.slots.as_mut() else {
            return;
        };
        if let Some(index) = self.active.take() {
            slots[index].state = State::Free;
        }
        self.frame = self.frame.saturating_add(1);
        if !self.snapshot.enabled {
            return;
        }
        let Some(index) = slots.iter().position(|s| s.state == State::Free) else {
            self.snapshot.skipped_frames += 1;
            return;
        };
        let slot = &mut slots[index];
        slot.state = State::Recording;
        slot.frame = self.frame;
        slot.written = 0;
        self.active = Some(index);
    }
    pub fn marker(&mut self, encoder: &mut etalon::CommandEncoder, marker: u32) {
        let Some(index) = self.active else {
            return;
        };
        let slot = &mut self.slots.as_mut().unwrap()[index];
        if marker as usize >= MARKERS || slot.written & (1 << marker) != 0 {
            return;
        }
        // Standard WebGPU exposes pass timestamps, not encoder write_timestamp.
        drop(encoder.begin_compute_pass(&etalon::ComputePassDescriptor {
            label: Some("portal-pass-timing-boundary"),
            timestamp_writes: Some(etalon::ComputePassTimestampWrites {
                query_set: &slot.queries,
                beginning_of_pass_write_index: Some(marker),
                end_of_pass_write_index: None,
            }),
        }));
        slot.written |= 1 << marker;
    }
    pub fn resolve(&mut self, encoder: &mut etalon::CommandEncoder) {
        let Some(index) = self.active.take() else {
            return;
        };
        let slot = &mut self.slots.as_mut().unwrap()[index];
        if slot.written != (1 << MARKERS) - 1 {
            slot.state = State::Free;
            return;
        }
        encoder.resolve_query_set(&slot.queries, 0..MARKERS as u32, &slot.resolve, 0);
        encoder.copy_buffer_to_buffer(&slot.resolve, 0, &slot.readback, 0, BYTES);
        slot.state = State::Recorded;
    }
    /// Cold diagnostics only: WebGPU map callbacks allocate in the provider API.
    pub fn collect(&mut self, device: &etalon::Device) -> TimingSnapshot {
        #[cfg(not(target_arch = "wasm32"))]
        let _ = device.poll(etalon::PollType::Poll);
        #[cfg(target_arch = "wasm32")]
        let _ = device;
        self.snapshot.observed_frame = self.frame;
        let Some(slots) = self.slots.as_mut() else {
            return self.snapshot;
        };
        for slot in slots {
            if slot.state == State::Mapping {
                let status = slot.signal.load(Ordering::Acquire);
                if status == 0 {
                    continue;
                }
                let sample = if status == 1 {
                    slot.readback
                        .slice(..)
                        .get_mapped_range()
                        .ok()
                        .and_then(|view| decode(&view, self.period_ns))
                } else {
                    None
                };
                slot.readback.unmap();
                slot.state = State::Free;
                if let Some(ms) = sample {
                    self.history[self.snapshot.completed_frames as usize % 16] = ms;
                    self.history_frames[self.snapshot.completed_frames as usize % 16] = slot.frame;
                    self.snapshot.completed_frames += 1;
                    self.snapshot.samples = self.snapshot.completed_frames.min(16) as usize;
                    if slot.frame > self.snapshot.latest_frame {
                        self.snapshot.latest_ms = ms;
                        self.snapshot.latest_frame = slot.frame;
                    }
                } else {
                    self.snapshot.readback_errors += 1;
                }
            }
            if slot.state == State::Recorded {
                slot.signal.store(0, Ordering::Release);
                let signal = Arc::clone(&slot.signal);
                slot.readback
                    .slice(..)
                    .map_async(etalon::MapMode::Read, move |result| {
                        signal.store(if result.is_ok() { 1 } else { 2 }, Ordering::Release);
                    });
                slot.state = State::Mapping;
            }
        }
        let count = self.snapshot.samples;
        if count > 0 {
            self.snapshot.oldest_sample_frame = *self.history_frames[..count].iter().min().unwrap();
            for pass in 0..MARKERS {
                let mut ordered = [0.0f64; 16];
                for (i, out) in ordered[..count].iter_mut().enumerate() {
                    *out = self.history[i][pass];
                }
                ordered[..count].sort_unstable_by(f64::total_cmp);
                self.snapshot.p50_ms[pass] = ordered[(count * 50).div_ceil(100) - 1];
                self.snapshot.p95_ms[pass] = ordered[(count * 95).div_ceil(100) - 1];
            }
        }
        self.snapshot
    }
}
impl Drop for PassTimings {
    fn drop(&mut self) {
        if let Some(slots) = &self.slots {
            for slot in slots {
                if slot.state == State::Mapping {
                    slot.readback.unmap();
                }
            }
        }
    }
}
impl super::PortalGpu {
    pub fn gpu_timing_adapter_info(&self) -> Option<&etalon::AdapterInfo> {
        self.pass_timings.adapter_info.as_ref()
    }
    pub fn set_gpu_pass_timing_enabled(&mut self, enabled: bool) {
        self.pass_timings.set_enabled(enabled);
    }
    pub fn gpu_pass_timing_snapshot(&mut self) -> TimingSnapshot {
        self.pass_timings.collect(&self.device)
    }
}
