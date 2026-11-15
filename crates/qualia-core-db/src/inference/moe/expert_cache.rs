//! Host memory observations over canonical expert-cache mechanics.
#[cfg(test)]
use super::placement::ExpertPlacementPolicy;
use etalon_inference::moe::offload::MoeOffloadManager as CoreManager;
pub use etalon_inference::moe::offload::{
    compute_principled_slot_capacity, query_dynamic_hardware_tier, DynamicVramStatus,
    ExpertCacheTelemetry, ExpertResidencyProfile, PersonalHardwareTier, SlotAccessOutcome,
    DEFAULT_GPU_EXPERT_SLOTS,
};
#[derive(Debug)]
pub struct MoeOffloadManager(CoreManager);
impl core::ops::Deref for MoeOffloadManager {
    type Target = CoreManager;
    fn deref(&self) -> &CoreManager {
        &self.0
    }
}
impl core::ops::DerefMut for MoeOffloadManager {
    fn deref_mut(&mut self) -> &mut CoreManager {
        &mut self.0
    }
}
impl MoeOffloadManager {
    pub fn new(slots: usize, bytes_per_expert: usize) -> Self {
        Self(CoreManager::new(slots, bytes_per_expert))
    }
    pub fn with_tier(slots: usize, bytes_per_expert: usize, tier: PersonalHardwareTier) -> Self {
        Self(CoreManager::with_tier(slots, bytes_per_expert, tier))
    }
    pub fn from_residency_profile(profile: ExpertResidencyProfile) -> (Self, PersonalHardwareTier) {
        let (manager, tier) = CoreManager::from_residency_profile(profile);
        (Self(manager), tier)
    }
    /// Create an offload manager by dynamically querying the host accelerator budget.
    pub fn from_dynamic_query(
        backbone_bytes: u64,
        kv_cache_bytes: u64,
        bytes_per_expert: usize,
        total_experts: usize,
    ) -> Self {
        let (available_bytes, is_unified) = query_dynamic_accelerator_budget();
        let profile = ExpertResidencyProfile {
            available_accelerator_bytes: available_bytes,
            backbone_bytes,
            kv_cache_bytes,
            bytes_per_expert,
            total_experts,
            is_unified_memory: is_unified,
        };
        let (manager, _) = Self::from_residency_profile(profile);
        manager
    }
}

/// Query dynamic accelerator budget and memory architecture (discrete vs unified).
///
/// Reads environment overrides (`QUALIA_VRAM_OVERRIDE_BYTES`, `QUALIA_VRAM_OVERRIDE_MB`, `QUALIA_FORCE_UNIFIED_MEMORY`),
/// falling back to platform defaults (e.g. Apple Silicon unified memory on macOS or RTX A2000 discrete budget on Windows/Linux).
pub fn query_dynamic_accelerator_budget() -> (u64, bool) {
    let is_unified = std::env::var("QUALIA_FORCE_UNIFIED_MEMORY")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or_else(|_| cfg!(target_os = "macos"));

    if let Ok(bytes_str) = std::env::var("QUALIA_VRAM_OVERRIDE_BYTES") {
        if let Ok(bytes) = bytes_str.trim().parse::<u64>() {
            return (bytes, is_unified);
        }
    }

    if let Ok(mb_str) = std::env::var("QUALIA_VRAM_OVERRIDE_MB") {
        if let Ok(mb) = mb_str.trim().parse::<u64>() {
            return (mb.saturating_mul(1024 * 1024), is_unified);
        }
    }

    if is_unified {
        // macOS Apple Silicon default: 16 GiB unified pool available for compute
        (16 * 1024 * 1024 * 1024, true)
    } else {
        // Personal Discrete GPU default (e.g. RTX A2000 12GB with ~11.5 GiB usable VRAM headroom)
        (11_500 * 1024 * 1024, false)
    }
}

#[cfg(test)]
mod tests {
    use super::super::placement::ExpertPlacementCalibration;
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
    fn measured_placement_overrides_legacy_queue_ratio() {
        let mut manager = MoeOffloadManager::new(8, 240 * 1024 * 1024);
        manager.set_placement_policy(ExpertPlacementPolicy::from_calibration(
            ExpertPlacementCalibration {
                transfer_bytes_per_second: 1_000_000_000,
                cpu_experts_per_second: 20.0,
                gpu_experts_per_second: 1_000.0,
                gpu_launch_overhead_ns: 100_000,
                max_inflight_transfers: 4,
            },
            1,
        ));
        assert_eq!(
            manager.resolve_expert(7, true, 0),
            SlotAccessOutcome::MissComputeCpu
        );
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

    #[test]
    fn profile_constructs_manager_at_measured_slot_capacity() {
        let profile = ExpertResidencyProfile {
            available_accelerator_bytes: 11_500 * 1024 * 1024,
            backbone_bytes: 2_800 * 1024 * 1024,
            kv_cache_bytes: 1_024 * 1024 * 1024,
            bytes_per_expert: 240 * 1024 * 1024,
            total_experts: 256,
            is_unified_memory: false,
        };
        let (manager, tier) = MoeOffloadManager::from_residency_profile(profile);
        assert_eq!(tier, PersonalHardwareTier::PersonalGpuStaging);
        assert_eq!(manager.capacity(), 31);
    }

    #[test]
    fn dynamic_query_and_vram_status_testing() {
        let (budget, is_unified) = query_dynamic_accelerator_budget();
        assert!(budget > 0);

        let (slots, tier) = query_dynamic_hardware_tier(
            budget,
            2_800 * 1024 * 1024,
            1_024 * 1024 * 1024,
            240 * 1024 * 1024,
            256,
            is_unified,
        );
        assert!(slots >= 4);
        assert_ne!(tier, PersonalHardwareTier::WorkstationResident);

        let mut manager = MoeOffloadManager::from_dynamic_query(
            2_800 * 1024 * 1024,
            1_024 * 1024 * 1024,
            240 * 1024 * 1024,
            256,
        );
        assert_eq!(manager.capacity(), slots);

        // Record some hits/misses to check telemetry in DynamicVramStatus
        let outcome1 = manager.resolve_expert(10, false, 0);
        assert!(matches!(outcome1, SlotAccessOutcome::MissFetch { .. }));

        let outcome2 = manager.resolve_expert(10, false, 0);
        assert!(matches!(outcome2, SlotAccessOutcome::Hit { slot_id: 0 }));

        let status = manager.query_vram_status(budget);
        assert_eq!(status.total_slots, slots);
        assert_eq!(status.hits, 1);
        assert_eq!(status.misses, 1);
        assert!((status.hit_rate - 0.5).abs() < 1e-6);
        assert_eq!(status.hardware_tier, manager.hardware_tier());
    }

    #[test]
    fn exercise_staging_fallback_and_release_under_routed_workload() {
        // Staging capacity = 4 slots (RTX A2000 constrained edge setup)
        let mut manager = MoeOffloadManager::new(4, 240 * 1024 * 1024);

        // Pin shared expert (ID 999) in slot 0: must never be evicted
        manager.pin_expert(0, 999);

        // Step 1: Experts 10 and 11 miss -> fetch into slots 1 and 2
        let out_1a = manager.resolve_expert(10, false, 0);
        assert!(matches!(
            out_1a,
            SlotAccessOutcome::MissFetch {
                evict_slot_id: 1,
                ..
            }
        ));
        let out_1b = manager.resolve_expert(11, false, 0);
        assert!(matches!(
            out_1b,
            SlotAccessOutcome::MissFetch {
                evict_slot_id: 2,
                ..
            }
        ));

        // Step 2: Expert 10 hits in slot 1; Expert 12 misses -> fetches into slot 3
        let out_2a = manager.resolve_expert(10, false, 0);
        assert_eq!(out_2a, SlotAccessOutcome::Hit { slot_id: 1 });
        let out_2b = manager.resolve_expert(12, false, 0);
        assert!(matches!(
            out_2b,
            SlotAccessOutcome::MissFetch {
                evict_slot_id: 3,
                ..
            }
        ));

        // Step 3: Expert 11 hits in slot 2; Expert 13 misses -> evicts LRU (slot 1 or 3, NEVER slot 0)
        let out_3a = manager.resolve_expert(11, false, 0);
        assert_eq!(out_3a, SlotAccessOutcome::Hit { slot_id: 2 });
        let out_3b = manager.resolve_expert(13, false, 0);
        if let SlotAccessOutcome::MissFetch { evict_slot_id, .. } = out_3b {
            assert_ne!(
                evict_slot_id, 0,
                "Pinned shared expert in slot 0 must NEVER be evicted"
            );
        } else {
            panic!("Expected MissFetch");
        }

        // Step 4: Access pinned shared expert (always a hit in slot 0)
        let out_4a = manager.resolve_expert(999, false, 0);
        assert_eq!(out_4a, SlotAccessOutcome::Hit { slot_id: 0 });

        // Step 5: High PCIe contention triggers hybrid CPU execution fallback
        let out_5a = manager.resolve_expert(14, true, 8);
        assert_eq!(out_5a, SlotAccessOutcome::MissComputeCpu);

        // Step 6: Normal query without CPU hybrid falls back to fetching into GPU
        let out_6a = manager.resolve_expert(14, false, 0);
        assert!(matches!(out_6a, SlotAccessOutcome::MissFetch { .. }));

        // Verify telemetry integrity
        let telem = manager.telemetry;
        assert!(telem.hits >= 3);
        assert!(telem.misses >= 5);
        assert!(telem.gpu_fetches >= 4);
        assert_eq!(telem.cpu_evaluations, 1);
        assert!(telem.bytes_transferred >= 4 * 240 * 1024 * 1024);
    }
}
