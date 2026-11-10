//! Residency metadata is maintained by Etalon; Qualia owns resource policy and lifetime.

pub use etalon_resources::residency::{
    ResidencyError, ResidencyGovernor, ResidencyPoolCaps, ResidencySlot,
};

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
    use crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc;

    #[test]
    fn admit_within_caps_and_lru_eviction() {
        let caps = ResidencyPoolCaps {
            max_cpu_bytes: 1000,
            max_gpu_vram_bytes: 2000,
        };
        let mut gov = ResidencyGovernor::<3>::new(caps);

        // Admit slot 1 at tick 10 (400 CPU, 800 GPU)
        gov.admit_or_evict_lru_zero_heap(0x101, 400, 800, false, 10)
            .unwrap();
        // Admit slot 2 at tick 20 (400 CPU, 800 GPU)
        gov.admit_or_evict_lru_zero_heap(0x102, 400, 800, false, 20)
            .unwrap();

        assert_eq!(gov.occupied_slots(), 2);
        assert_eq!(gov.current_cpu_bytes(), 800);
        assert_eq!(gov.current_gpu_vram_bytes(), 1600);

        // Touch slot 1 at tick 30 (making slot 2 the LRU candidate with tick 20)
        assert!(gov.touch(0x101, 30));

        // Admit slot 3 needing 400 CPU, 800 GPU: exceeds total CPU (800+400=1200 > 1000)
        // Must evict slot 2 (tick 20), keeping slot 1 (tick 30)
        gov.admit_or_evict_lru_zero_heap(0x103, 400, 800, false, 40)
            .unwrap();

        assert_eq!(gov.occupied_slots(), 2);
        assert!(gov.contains(0x101));
        assert!(!gov.contains(0x102));
        assert!(gov.contains(0x103));
    }

    #[test]
    fn mandatory_negative_request_exceeds_caps() {
        let caps = ResidencyPoolCaps {
            max_cpu_bytes: 500,
            max_gpu_vram_bytes: 1000,
        };
        let mut gov = ResidencyGovernor::<4>::new(caps);

        // CPU exceeds total cap
        let err_cpu = gov
            .admit_or_evict_lru_zero_heap(0x1, 501, 100, false, 1)
            .unwrap_err();
        match err_cpu {
            ResidencyError::ResidencyBudgetExceeded { requested_cpu, .. } => {
                assert_eq!(requested_cpu, 501)
            }
            other => panic!("unexpected: {other:?}"),
        }

        // GPU exceeds total cap
        let err_gpu = gov
            .admit_or_evict_lru_zero_heap(0x2, 100, 1001, false, 1)
            .unwrap_err();
        match err_gpu {
            ResidencyError::ResidencyBudgetExceeded { requested_gpu, .. } => {
                assert_eq!(requested_gpu, 1001)
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn mandatory_negative_pinned_headroom_fails_closed() {
        let caps = ResidencyPoolCaps {
            max_cpu_bytes: 1000,
            max_gpu_vram_bytes: 2000,
        };
        let mut gov = ResidencyGovernor::<4>::new(caps);

        // Pin resource 0xAAA with 800 CPU, 1500 GPU
        gov.admit_or_evict_lru_zero_heap(0xAAA, 800, 1500, true, 10)
            .unwrap();
        // Unpinned resource 0xBBB with 100 CPU, 200 GPU
        gov.admit_or_evict_lru_zero_heap(0xBBB, 100, 200, false, 20)
            .unwrap();

        // New request: 300 CPU. Even evicting 0xBBB leaves only 200 CPU available (1000 - 800)
        let err = gov
            .admit_or_evict_lru_zero_heap(0xCCC, 300, 100, false, 30)
            .unwrap_err();
        match err {
            ResidencyError::ResidencyBudgetExceeded {
                requested_cpu,
                available_cpu,
                ..
            } => {
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
        let caps = ResidencyPoolCaps {
            max_cpu_bytes: 1000,
            max_gpu_vram_bytes: 1000,
        };
        let mut gov = ResidencyGovernor::<4>::new(caps);

        gov.admit_or_evict_lru_zero_heap(0x123, 250, 250, false, 1)
            .unwrap();
        assert!(gov.contains(0x123));

        gov.cancel_pending_hydration(0x123).unwrap();
        assert!(!gov.contains(0x123));
        assert_eq!(gov.current_cpu_bytes(), 0);
        assert_eq!(gov.current_gpu_vram_bytes(), 0);

        assert_eq!(
            gov.release_resource(0x999),
            Err(ResidencyError::ResourceNotFound(0x999))
        );
    }

    #[test]
    #[cfg(any(not(target_arch = "wasm32"), feature = "wasm-scientific"))]
    fn zero_alloc_residency_governor() {
        let caps = ResidencyPoolCaps {
            max_cpu_bytes: 2048,
            max_gpu_vram_bytes: 4096,
        };
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
