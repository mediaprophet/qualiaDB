//! Heterogeneous MoE expert offload manager and streaming cache (Work Package F10).
//!
//! Manages compressed host-resident expert weights and bounded GPU VRAM staging slots,
//! with bandwidth-adaptive CPU/GPU hybrid partitioning for the RTX A2000 12GB budget.

/// Maximum number of active expert slots in GPU VRAM simultaneously.
pub const DEFAULT_GPU_EXPERT_SLOTS: usize = 32;

/// Device capability tier for personal computing local AI across Windows, Linux, and macOS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersonalHardwareTier {
    /// Workstation / Unified Studio (>= 32 GiB): All 256 experts resident in VRAM / unified memory (zero PCIe paging).
    WorkstationResident,
    /// Personal Discrete GPU (8–24 GiB VRAM, e.g. RTX A2000, 3060/4060, Radeon on Vulkan/CUDA): Dynamic LRU staging ring.
    PersonalGpuStaging,
    /// Apple Silicon Unified Memory (macOS Metal 16–36 GiB): Zero-copy shared pool with direct compute shader dispatch.
    AppleUnifiedMemory,
    /// Constrained Edge (< 8 GiB VRAM or CPU-only): On-demand hybrid CPU/GPU streaming.
    ConstrainedEdge,
}

/// Compute principled slot capacity given detected system memory and model footprint.
///
/// This principle-based calculation works across Vulkan/wgpu, CUDA, DirectML, and Apple Metal:
/// - Determines the available accelerator headroom after subtracting backbone and KV cache budgets.
/// - Slices headroom into slots of `bytes_per_expert`.
/// - Selects the appropriate `PersonalHardwareTier` without hardcoding specific GPU models.
pub fn compute_principled_slot_capacity(
    available_accelerator_bytes: u64,
    backbone_bytes: u64,
    kv_cache_bytes: u64,
    bytes_per_expert: usize,
    total_experts: usize,
    is_unified_memory: bool,
) -> (usize, PersonalHardwareTier) {
    if is_unified_memory {
        let total_required = backbone_bytes
            .saturating_add((total_experts * bytes_per_expert) as u64)
            .saturating_add(kv_cache_bytes);
        if available_accelerator_bytes >= total_required {
            return (total_experts, PersonalHardwareTier::AppleUnifiedMemory);
        }
    }

    let reserved = backbone_bytes.saturating_add(kv_cache_bytes);
    let expert_headroom = available_accelerator_bytes.saturating_sub(reserved);
    let computed_slots = if bytes_per_expert > 0 {
        (expert_headroom / bytes_per_expert as u64) as usize
    } else {
        0
    };

    if computed_slots >= total_experts && total_experts > 0 {
        (total_experts, PersonalHardwareTier::WorkstationResident)
    } else if computed_slots >= 8 {
        (computed_slots.min(total_experts), PersonalHardwareTier::PersonalGpuStaging)
    } else {
        (computed_slots.max(4).min(total_experts), PersonalHardwareTier::ConstrainedEdge)
    }
}

/// Outcome of accessing an expert in the offload cache.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotAccessOutcome {
    /// Expert already resident in GPU slot `slot_id`.
    Hit { slot_id: usize },
    /// Expert missing from GPU; should evict `evict_slot_id` (resident: `evicted_expert_id`) to fetch `expert_id`.
    MissFetch { evict_slot_id: usize, evicted_expert_id: Option<u16> },
    /// Expert missing and transfer latency exceeds compute; assigned to host CPU execution.
    MissComputeCpu,
}

/// Telemetry counters for MoE offload and residency.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExpertCacheTelemetry {
    pub hits: u64,
    pub misses: u64,
    pub gpu_fetches: u64,
    pub cpu_evaluations: u64,
    pub bytes_transferred: u64,
}

/// Metadata for a single GPU expert staging slot.
#[derive(Debug, Clone, Copy, Default)]
struct SlotEntry {
    expert_id: Option<u16>,
    last_access_tick: u64,
    is_pinned: bool,
}

/// Manager for heterogeneous GPU expert staging and host-RAM offload.
#[derive(Debug)]
pub struct MoeOffloadManager {
    slots: Vec<SlotEntry>,
    current_tick: u64,
    bytes_per_expert: usize,
    prefer_cpu_hybrid_threshold: f32,
    pub telemetry: ExpertCacheTelemetry,
}

impl MoeOffloadManager {
    /// Create a new offload manager with `num_gpu_slots` staging capacity.
    pub fn new(num_gpu_slots: usize, bytes_per_expert: usize) -> Self {
        let capacity = if num_gpu_slots > 0 { num_gpu_slots } else { DEFAULT_GPU_EXPERT_SLOTS };
        Self {
            slots: vec![SlotEntry::default(); capacity],
            current_tick: 0,
            bytes_per_expert,
            prefer_cpu_hybrid_threshold: 0.5,
            telemetry: ExpertCacheTelemetry::default(),
        }
    }

    /// Access or schedule an expert for evaluation.
    ///
    /// Checks GPU residency. If resident, returns `Hit`.
    /// If not resident and `allow_cpu_hybrid` is enabled and transfer queue is contested, returns `MissComputeCpu`.
    /// Otherwise returns `MissFetch` identifying the least-recently-used slot for replacement.
    pub fn resolve_expert(
        &mut self,
        expert_id: u16,
        allow_cpu_hybrid: bool,
        estimated_pcie_queue_depth: usize,
    ) -> SlotAccessOutcome {
        self.current_tick = self.current_tick.wrapping_add(1);

        // 1. Check for residency hit
        for (slot_idx, slot) in self.slots.iter_mut().enumerate() {
            if slot.expert_id == Some(expert_id) {
                slot.last_access_tick = self.current_tick;
                self.telemetry.hits += 1;
                return SlotAccessOutcome::Hit { slot_id: slot_idx };
            }
        }

        self.telemetry.misses += 1;

        // 2. Calibrated CPU hybrid evaluation check
        let queue_ratio = estimated_pcie_queue_depth as f32 / self.slots.len().max(1) as f32;
        if allow_cpu_hybrid && queue_ratio >= self.prefer_cpu_hybrid_threshold {
            self.telemetry.cpu_evaluations += 1;
            return SlotAccessOutcome::MissComputeCpu;
        }

        // 3. Find LRU slot to evict
        let mut lru_slot_idx = 0;
        let mut min_tick = u64::MAX;

        for (idx, slot) in self.slots.iter().enumerate() {
            if !slot.is_pinned && slot.last_access_tick < min_tick {
                min_tick = slot.last_access_tick;
                lru_slot_idx = idx;
                if slot.expert_id.is_none() {
                    // Empty slot preferred immediately
                    break;
                }
            }
        }

        let evicted_expert_id = self.slots[lru_slot_idx].expert_id;
        self.slots[lru_slot_idx].expert_id = Some(expert_id);
        self.slots[lru_slot_idx].last_access_tick = self.current_tick;

        self.telemetry.gpu_fetches += 1;
        self.telemetry.bytes_transferred += self.bytes_per_expert as u64;

        SlotAccessOutcome::MissFetch {
            evict_slot_id: lru_slot_idx,
            evicted_expert_id,
        }
    }

    /// Pin an expert into GPU VRAM (e.g. shared expert that must never be evicted).
    pub fn pin_expert(&mut self, slot_id: usize, expert_id: u16) {
        if slot_id < self.slots.len() {
            self.slots[slot_id].expert_id = Some(expert_id);
            self.slots[slot_id].is_pinned = true;
            self.slots[slot_id].last_access_tick = u64::MAX;
        }
    }

    /// Number of available staging slots in GPU memory.
    pub fn capacity(&self) -> usize {
        self.slots.len()
    }

    /// CPU hybrid dispatch threshold (queue depth / slot capacity).
    pub fn prefer_cpu_hybrid_threshold(&self) -> f32 {
        self.prefer_cpu_hybrid_threshold
    }

    /// Set the CPU hybrid dispatch threshold.
    pub fn set_prefer_cpu_hybrid_threshold(&mut self, threshold: f32) {
        self.prefer_cpu_hybrid_threshold = threshold;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_moe_offload_manager_lru_and_hits() {
        let mut manager = MoeOffloadManager::new(2, 1024);

        // Access expert 10 -> MissFetch into slot 0
        let res1 = manager.resolve_expert(10, false, 0);
        assert_eq!(
            res1,
            SlotAccessOutcome::MissFetch {
                evict_slot_id: 0,
                evicted_expert_id: None,
            }
        );

        // Access expert 20 -> MissFetch into slot 1
        let res2 = manager.resolve_expert(20, false, 0);
        assert_eq!(
            res2,
            SlotAccessOutcome::MissFetch {
                evict_slot_id: 1,
                evicted_expert_id: None,
            }
        );

        // Access expert 10 -> Hit in slot 0
        let res3 = manager.resolve_expert(10, false, 0);
        assert_eq!(res3, SlotAccessOutcome::Hit { slot_id: 0 });

        // Access expert 30 -> Slot 1 is LRU (expert 20 was accessed before expert 10)
        let res4 = manager.resolve_expert(30, false, 0);
        assert_eq!(
            res4,
            SlotAccessOutcome::MissFetch {
                evict_slot_id: 1,
                evicted_expert_id: Some(20),
            }
        );

        assert_eq!(manager.telemetry.hits, 1);
        assert_eq!(manager.telemetry.misses, 3);
        assert_eq!(manager.telemetry.gpu_fetches, 3);
        assert_eq!(manager.telemetry.bytes_transferred, 3072);
    }

    #[test]
    fn test_cpu_hybrid_dispatch_on_contention() {
        let mut manager = MoeOffloadManager::new(2, 1024);
        let res = manager.resolve_expert(99, true, 4);
        assert_eq!(res, SlotAccessOutcome::MissComputeCpu);
        assert_eq!(manager.telemetry.cpu_evaluations, 1);
    }

    #[test]
    fn test_principled_hardware_tier_computation() {
        let bytes_per_exp = 240 * 1024 * 1024; // 240 MiB per expert
        let backbone = 2800 * 1024 * 1024; // 2.8 GiB
        let kv = 1024 * 1024 * 1024; // 1 GiB

        // 1. Personal GPU 12GB (e.g. RTX A2000, 3060)
        let (slots_12g, tier_12g) = compute_principled_slot_capacity(
            11_500 * 1024 * 1024,
            backbone,
            kv,
            bytes_per_exp,
            256,
            false,
        );
        assert_eq!(tier_12g, PersonalHardwareTier::PersonalGpuStaging);
        assert_eq!(slots_12g, 31);

        // 2. Apple Silicon 64GB Unified Memory (Mac Studio)
        let (slots_mac, tier_mac) = compute_principled_slot_capacity(
            64 * 1024 * 1024 * 1024,
            backbone,
            kv,
            bytes_per_exp,
            256,
            true,
        );
        assert_eq!(tier_mac, PersonalHardwareTier::AppleUnifiedMemory);
        assert_eq!(slots_mac, 256);

        // 3. Workstation 80GB VRAM (A100/H100)
        let (slots_workstation, tier_workstation) = compute_principled_slot_capacity(
            80 * 1024 * 1024 * 1024,
            backbone,
            kv,
            bytes_per_exp,
            256,
            false,
        );
        assert_eq!(tier_workstation, PersonalHardwareTier::WorkstationResident);
        assert_eq!(slots_workstation, 256);

        // 4. Constrained Edge 5GB VRAM
        let (slots_edge, tier_edge) = compute_principled_slot_capacity(
            5 * 1024 * 1024 * 1024,
            backbone,
            kv,
            bytes_per_exp,
            256,
            false,
        );
        assert_eq!(tier_edge, PersonalHardwareTier::ConstrainedEdge);
        assert_eq!(slots_edge, 5);
    }
}
