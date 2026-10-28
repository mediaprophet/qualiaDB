//! Fixed-capacity zero-heap CPU/GPU residency budget governor (`BS-30`, `AR-04`, `ENG-07`).
//!
//! Enforces bounded CPU and GPU VRAM resident byte budgets for streamed assets
//! and sub-ranges. Unpinned entries are evicted using least-recently-used (LRU)
//! ordering on tick updates. Pinned slots are immune to eviction.
//! Oversized requests and headroom exhaustion fail closed without mutating pinned state.

use core::fmt;

/// Maximum byte capacities for resident CPU heap/staging and GPU VRAM buffers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResidencyPoolCaps {
    pub max_cpu_bytes: u64,
    pub max_gpu_vram_bytes: u64,
}

/// A resident asset slot tracking resource identity, footprint, and LRU recency.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ResidencySlot {
    pub resource_hash: u64,
    pub cpu_bytes: u64,
    pub gpu_vram_bytes: u64,
    pub last_used_tick: u64,
    pub pinned: bool,
    pub active: bool,
}

/// Typed error returned during residency evaluation, admission, or release.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidencyError {
    ResidencyBudgetExceeded {
        requested_cpu: u64,
        requested_gpu: u64,
        available_cpu: u64,
        available_gpu: u64,
    },
    SlotCapacityFull,
    ResourceNotFound(u64),
    ResourcePinned(u64),
    Cancelled,
}

impl fmt::Display for ResidencyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ResidencyBudgetExceeded { requested_cpu, requested_gpu, available_cpu, available_gpu } => {
                write!(
                    f,
                    "residency budget exceeded: requested (CPU {requested_cpu}, GPU {requested_gpu}) > available (CPU {available_cpu}, GPU {available_gpu})"
                )
            }
            Self::SlotCapacityFull => write!(f, "residency governor slot capacity full"),
            Self::ResourceNotFound(hash) => write!(f, "resource {hash:#018x} not found in residency pool"),
            Self::ResourcePinned(hash) => write!(f, "resource {hash:#018x} is pinned and cannot be evicted"),
            Self::Cancelled => write!(f, "residency admission cancelled"),
        }
    }
}

impl std::error::Error for ResidencyError {}

/// Fixed-capacity Tier-1 zero-heap governor for CPU and GPU VRAM residency.
#[derive(Debug, Clone, Copy)]
pub struct ResidencyGovernor<const MAX_SLOTS: usize> {
    caps: ResidencyPoolCaps,
    current_cpu_bytes: u64,
    current_gpu_vram_bytes: u64,
    slots: [ResidencySlot; MAX_SLOTS],
}

impl<const MAX_SLOTS: usize> ResidencyGovernor<MAX_SLOTS> {
    /// Creates a new governor with specified capacity caps and an empty slot array.
    pub const fn new(caps: ResidencyPoolCaps) -> Self {
        Self {
            caps,
            current_cpu_bytes: 0,
            current_gpu_vram_bytes: 0,
            slots: [ResidencySlot {
                resource_hash: 0,
                cpu_bytes: 0,
                gpu_vram_bytes: 0,
                last_used_tick: 0,
                pinned: false,
                active: false,
            }; MAX_SLOTS],
        }
    }

    #[inline]
    pub const fn caps(&self) -> ResidencyPoolCaps {
        self.caps
    }

    #[inline]
    pub const fn current_cpu_bytes(&self) -> u64 {
        self.current_cpu_bytes
    }

    #[inline]
    pub const fn current_gpu_vram_bytes(&self) -> u64 {
        self.current_gpu_vram_bytes
    }

    #[inline]
    pub fn occupied_slots(&self) -> usize {
        let mut count = 0;
        for s in &self.slots {
            if s.active {
                count += 1;
            }
        }
        count
    }

    #[inline]
    pub fn pinned_slots(&self) -> usize {
        let mut count = 0;
        for s in &self.slots {
            if s.active && s.pinned {
                count += 1;
            }
        }
        count
    }

    #[inline]
    fn find_slot(&self, resource_hash: u64) -> Option<usize> {
        for (i, slot) in self.slots.iter().enumerate() {
            if slot.active && slot.resource_hash == resource_hash {
                return Some(i);
            }
        }
        None
    }

    #[inline]
    pub fn contains(&self, resource_hash: u64) -> bool {
        self.find_slot(resource_hash).is_some()
    }

    #[inline]
    pub fn get_slot(&self, resource_hash: u64) -> Option<&ResidencySlot> {
        self.find_slot(resource_hash).map(|i| &self.slots[i])
    }

    /// Touches a resident resource to refresh its LRU recency tick.
    pub fn touch(&mut self, resource_hash: u64, current_tick: u64) -> bool {
        if let Some(idx) = self.find_slot(resource_hash) {
            self.slots[idx].last_used_tick = current_tick;
            true
        } else {
            false
        }
    }

    /// Tier-1 zero-heap admission with deterministic LRU eviction.
    ///
    /// Fails closed with `ResidencyBudgetExceeded` if requested bytes exceed pool caps
    /// or if existing pinned allocations leave insufficient headroom.
    pub fn admit_or_evict_lru_zero_heap(
        &mut self,
        resource_hash: u64,
        cpu_bytes: u64,
        gpu_vram_bytes: u64,
        pinned: bool,
        current_tick: u64,
    ) -> Result<usize, ResidencyError> {
        // Step 1: Reject immediately if single request exceeds total pool capacity
        if cpu_bytes > self.caps.max_cpu_bytes || gpu_vram_bytes > self.caps.max_gpu_vram_bytes {
            return Err(ResidencyError::ResidencyBudgetExceeded {
                requested_cpu: cpu_bytes,
                requested_gpu: gpu_vram_bytes,
                available_cpu: self.caps.max_cpu_bytes.saturating_sub(self.current_cpu_bytes),
                available_gpu: self.caps.max_gpu_vram_bytes.saturating_sub(self.current_gpu_vram_bytes),
            });
        }

        // Step 2: Handle already resident resource
        if let Some(idx) = self.find_slot(resource_hash) {
            let slot = &mut self.slots[idx];
            if slot.cpu_bytes == cpu_bytes && slot.gpu_vram_bytes == gpu_vram_bytes {
                slot.last_used_tick = current_tick;
                if pinned {
                    slot.pinned = true;
                }
                return Ok(idx);
            }
            // Size footprint changed: release old allocation first
            self.current_cpu_bytes = self.current_cpu_bytes.saturating_sub(slot.cpu_bytes);
            self.current_gpu_vram_bytes = self.current_gpu_vram_bytes.saturating_sub(slot.gpu_vram_bytes);
            slot.active = false;
        }

        // Step 3: Compute total pinned usage to check whether eviction can satisfy request
        let mut pinned_cpu: u64 = 0;
        let mut pinned_gpu: u64 = 0;
        let mut pinned_count: usize = 0;
        let mut active_count: usize = 0;

        for slot in &self.slots {
            if slot.active {
                active_count += 1;
                if slot.pinned {
                    pinned_cpu = pinned_cpu.saturating_add(slot.cpu_bytes);
                    pinned_gpu = pinned_gpu.saturating_add(slot.gpu_vram_bytes);
                    pinned_count += 1;
                }
            }
        }

        let max_available_cpu = self.caps.max_cpu_bytes.saturating_sub(pinned_cpu);
        let max_available_gpu = self.caps.max_gpu_vram_bytes.saturating_sub(pinned_gpu);

        if cpu_bytes > max_available_cpu || gpu_vram_bytes > max_available_gpu {
            return Err(ResidencyError::ResidencyBudgetExceeded {
                requested_cpu: cpu_bytes,
                requested_gpu: gpu_vram_bytes,
                available_cpu: max_available_cpu,
                available_gpu: max_available_gpu,
            });
        }

        if pinned_count >= MAX_SLOTS {
            return Err(ResidencyError::SlotCapacityFull);
        }

        // Step 4: Evict unpinned LRU slots until enough space and slots are available
        while self.current_cpu_bytes.saturating_add(cpu_bytes) > self.caps.max_cpu_bytes
            || self.current_gpu_vram_bytes.saturating_add(gpu_vram_bytes) > self.caps.max_gpu_vram_bytes
            || active_count >= MAX_SLOTS
        {
            let mut lru_idx: Option<usize> = None;
            let mut oldest_tick: u64 = u64::MAX;

            for (i, slot) in self.slots.iter().enumerate() {
                if slot.active && !slot.pinned && slot.last_used_tick < oldest_tick {
                    oldest_tick = slot.last_used_tick;
                    lru_idx = Some(i);
                }
            }

            match lru_idx {
                Some(idx) => {
                    let evict = &mut self.slots[idx];
                    self.current_cpu_bytes = self.current_cpu_bytes.saturating_sub(evict.cpu_bytes);
                    self.current_gpu_vram_bytes = self.current_gpu_vram_bytes.saturating_sub(evict.gpu_vram_bytes);
                    evict.active = false;
                    active_count -= 1;
                }
                None => {
                    return Err(ResidencyError::ResidencyBudgetExceeded {
                        requested_cpu: cpu_bytes,
                        requested_gpu: gpu_vram_bytes,
                        available_cpu: self.caps.max_cpu_bytes.saturating_sub(self.current_cpu_bytes),
                        available_gpu: self.caps.max_gpu_vram_bytes.saturating_sub(self.current_gpu_vram_bytes),
                    });
                }
            }
        }

        // Step 5: Place into first inactive slot
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if !slot.active {
                *slot = ResidencySlot {
                    resource_hash,
                    cpu_bytes,
                    gpu_vram_bytes,
                    last_used_tick: current_tick,
                    pinned,
                    active: true,
                };
                self.current_cpu_bytes = self.current_cpu_bytes.saturating_add(cpu_bytes);
                self.current_gpu_vram_bytes = self.current_gpu_vram_bytes.saturating_add(gpu_vram_bytes);
                return Ok(i);
            }
        }

        Err(ResidencyError::SlotCapacityFull)
    }

    /// Releases a resident resource and returns its byte allocations to the pool.
    pub fn release_resource(&mut self, resource_hash: u64) -> Result<(), ResidencyError> {
        for slot in self.slots.iter_mut() {
            if slot.active && slot.resource_hash == resource_hash {
                self.current_cpu_bytes = self.current_cpu_bytes.saturating_sub(slot.cpu_bytes);
                self.current_gpu_vram_bytes = self.current_gpu_vram_bytes.saturating_sub(slot.gpu_vram_bytes);
                slot.active = false;
                slot.pinned = false;
                return Ok(());
            }
        }
        Err(ResidencyError::ResourceNotFound(resource_hash))
    }

    /// Sets or clears the pinned status for a resident resource.
    pub fn pin_resource(&mut self, resource_hash: u64, pinned: bool) -> Result<(), ResidencyError> {
        for slot in self.slots.iter_mut() {
            if slot.active && slot.resource_hash == resource_hash {
                slot.pinned = pinned;
                return Ok(());
            }
        }
        Err(ResidencyError::ResourceNotFound(resource_hash))
    }

    /// Cancels a pending or in-flight hydration, releasing its slot immediately.
    #[inline]
    pub fn cancel_pending_hydration(&mut self, resource_hash: u64) -> Result<(), ResidencyError> {
        self.release_resource(resource_hash)
    }

    /// Evicts all unpinned resources (e.g. on stream abort or camera scene jump).
    pub fn cancel_all_unpinned(&mut self) -> usize {
        let mut evicted = 0;
        for slot in self.slots.iter_mut() {
            if slot.active && !slot.pinned {
                self.current_cpu_bytes = self.current_cpu_bytes.saturating_sub(slot.cpu_bytes);
                self.current_gpu_vram_bytes = self.current_gpu_vram_bytes.saturating_sub(slot.gpu_vram_bytes);
                slot.active = false;
                evicted += 1;
            }
        }
        evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn admit_within_caps_and_lru_eviction() {
        let caps = ResidencyPoolCaps { max_cpu_bytes: 1000, max_gpu_vram_bytes: 2000 };
        let mut gov = ResidencyGovernor::<3>::new(caps);

        // Admit slot 1 at tick 10 (400 CPU, 800 GPU)
        gov.admit_or_evict_lru_zero_heap(0x101, 400, 800, false, 10).unwrap();
        // Admit slot 2 at tick 20 (400 CPU, 800 GPU)
        gov.admit_or_evict_lru_zero_heap(0x102, 400, 800, false, 20).unwrap();

        assert_eq!(gov.occupied_slots(), 2);
        assert_eq!(gov.current_cpu_bytes(), 800);
        assert_eq!(gov.current_gpu_vram_bytes(), 1600);

        // Touch slot 1 at tick 30 (making slot 2 the LRU candidate with tick 20)
        assert!(gov.touch(0x101, 30));

        // Admit slot 3 needing 400 CPU, 800 GPU: exceeds total CPU (800+400=1200 > 1000)
        // Must evict slot 2 (tick 20), keeping slot 1 (tick 30)
        gov.admit_or_evict_lru_zero_heap(0x103, 400, 800, false, 40).unwrap();

        assert_eq!(gov.occupied_slots(), 2);
        assert!(gov.contains(0x101));
        assert!(!gov.contains(0x102));
        assert!(gov.contains(0x103));
    }

    #[test]
    fn mandatory_negative_request_exceeds_caps() {
        let caps = ResidencyPoolCaps { max_cpu_bytes: 500, max_gpu_vram_bytes: 1000 };
        let mut gov = ResidencyGovernor::<4>::new(caps);

        // CPU exceeds total cap
        let err_cpu = gov.admit_or_evict_lru_zero_heap(0x1, 501, 100, false, 1).unwrap_err();
        match err_cpu {
            ResidencyError::ResidencyBudgetExceeded { requested_cpu, .. } => assert_eq!(requested_cpu, 501),
            other => panic!("unexpected: {other:?}"),
        }

        // GPU exceeds total cap
        let err_gpu = gov.admit_or_evict_lru_zero_heap(0x2, 100, 1001, false, 1).unwrap_err();
        match err_gpu {
            ResidencyError::ResidencyBudgetExceeded { requested_gpu, .. } => assert_eq!(requested_gpu, 1001),
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn mandatory_negative_pinned_headroom_fails_closed() {
        let caps = ResidencyPoolCaps { max_cpu_bytes: 1000, max_gpu_vram_bytes: 2000 };
        let mut gov = ResidencyGovernor::<4>::new(caps);

        // Pin resource 0xAAA with 800 CPU, 1500 GPU
        gov.admit_or_evict_lru_zero_heap(0xAAA, 800, 1500, true, 10).unwrap();
        // Unpinned resource 0xBBB with 100 CPU, 200 GPU
        gov.admit_or_evict_lru_zero_heap(0xBBB, 100, 200, false, 20).unwrap();

        // New request: 300 CPU. Even evicting 0xBBB leaves only 200 CPU available (1000 - 800)
        let err = gov.admit_or_evict_lru_zero_heap(0xCCC, 300, 100, false, 30).unwrap_err();
        match err {
            ResidencyError::ResidencyBudgetExceeded { requested_cpu, available_cpu, .. } => {
                assert_eq!(requested_cpu, 300);
                assert_eq!(available_cpu, 200);
            }
            other => panic!("unexpected: {other:?}"),
        }

        // Pinned resource 0xAAA must NOT be evicted or mutated
        assert!(gov.contains(0xAAA));
        assert!(gov.get_slot(0xAAA).unwrap().pinned);
    }

    #[test]
    fn release_and_cancel_pending_hydration() {
        let caps = ResidencyPoolCaps { max_cpu_bytes: 1000, max_gpu_vram_bytes: 1000 };
        let mut gov = ResidencyGovernor::<4>::new(caps);

        gov.admit_or_evict_lru_zero_heap(0x123, 250, 250, false, 1).unwrap();
        assert!(gov.contains(0x123));

        gov.cancel_pending_hydration(0x123).unwrap();
        assert!(!gov.contains(0x123));
        assert_eq!(gov.current_cpu_bytes(), 0);
        assert_eq!(gov.current_gpu_vram_bytes(), 0);

        assert_eq!(gov.release_resource(0x999), Err(ResidencyError::ResourceNotFound(0x999)));
    }

    #[test]
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
    fn zero_alloc_residency_governor() {
        let caps = ResidencyPoolCaps { max_cpu_bytes: 2048, max_gpu_vram_bytes: 4096 };
        let mut gov = ResidencyGovernor::<8>::new(caps);

        assert_zero_alloc("residency_governor_admit_evict", || {
            let _ = gov.admit_or_evict_lru_zero_heap(0x100, 256, 512, false, 1);
            let _ = gov.admit_or_evict_lru_zero_heap(0x200, 512, 1024, true, 2);
            let _ = gov.touch(0x100, 3);
            let _ = gov.pin_resource(0x100, true);
            let _ = gov.release_resource(0x100);
        });
    }
}
