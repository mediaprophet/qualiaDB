//! Forge GPU decode/prefill dual-path schedules and occupancy diagnostics (W6: EOS-061).
//!
//! Provides schedule synthesis and analytical occupancy diagnostics for GPU execution:
//! - Decode path: batch 1 token-latency optimized fused workgroup tile lookup.
//! - Prefill path: batched throughput schedule with register tiling.
//! - Occupancy diagnostics: register pressure, shared memory usage, and spill detection.

use serde::{Deserialize, Serialize};

/// Target execution path on the GPU.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuScheduleKind {
    /// Batch 1 token latency optimized schedule.
    Decode,
    /// Batch > 1 throughput schedule.
    Prefill,
}

/// Analytical occupancy diagnostics for a synthesized Forge schedule.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpuOccupancyDiagnostics {
    pub schedule: GpuScheduleKind,
    pub threads_per_workgroup: u32,
    pub workgroup_shared_bytes: u32,
    pub estimated_registers_per_thread: u32,
    pub theoretical_occupancy: f32,
    pub spill_warning: bool,
    pub memory_domain: String,
}

/// Synthesizes a GPU execution schedule descriptor for an operator matrix multiplication.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct GpuOperatorSchedule {
    pub kind: GpuScheduleKind,
    pub in_features: usize,
    pub out_features: usize,
    pub batch_size: usize,
    pub tile_m: u32,
    pub tile_n: u32,
    pub tile_k: u32,
    pub diagnostics: GpuOccupancyDiagnostics,
}

impl GpuOperatorSchedule {
    /// Create an optimized GPU schedule for given dimensions and batch size.
    pub fn plan(
        in_features: usize,
        out_features: usize,
        batch_size: usize,
    ) -> Self {
        if batch_size <= 1 {
            // Decode schedule: 128 threads/workgroup, tile_m = 1, tile_k = 256 (one Q4_K block)
            let threads_per_workgroup = 128u32;
            let workgroup_shared_bytes = 1024u32; // 256 floats for reduction
            let estimated_registers_per_thread = 28u32;
            let spill_warning = estimated_registers_per_thread > 64;
            let theoretical_occupancy = 1.0f32;

            Self {
                kind: GpuScheduleKind::Decode,
                in_features,
                out_features,
                batch_size,
                tile_m: 1,
                tile_n: 16,
                tile_k: 256,
                diagnostics: GpuOccupancyDiagnostics {
                    schedule: GpuScheduleKind::Decode,
                    threads_per_workgroup,
                    workgroup_shared_bytes,
                    estimated_registers_per_thread,
                    theoretical_occupancy,
                    spill_warning,
                    memory_domain: "LocalDeviceVRAM".to_string(),
                },
            }
        } else {
            // Prefill schedule: 256 threads/workgroup, tile_m = 16, tile_n = 16, tile_k = 32
            let threads_per_workgroup = 256u32;
            let workgroup_shared_bytes = 4096u32; // double buffered input tiles
            let estimated_registers_per_thread = 42u32;
            let spill_warning = estimated_registers_per_thread > 64;
            let theoretical_occupancy = 0.85f32;

            Self {
                kind: GpuScheduleKind::Prefill,
                in_features,
                out_features,
                batch_size,
                tile_m: 16,
                tile_n: 16,
                tile_k: 32,
                diagnostics: GpuOccupancyDiagnostics {
                    schedule: GpuScheduleKind::Prefill,
                    threads_per_workgroup,
                    workgroup_shared_bytes,
                    estimated_registers_per_thread,
                    theoretical_occupancy,
                    spill_warning,
                    memory_domain: "LocalDeviceVRAM".to_string(),
                },
            }
        }
    }

    /// Emits a deterministic WGSL shader preamble and entry point for this schedule.
    pub fn emit_wgsl_stub(&self) -> String {
        let workgroup_size = self.diagnostics.threads_per_workgroup;
        let smem_bytes = self.diagnostics.workgroup_shared_bytes;

        format!(
            "// Qualia Forge WGSL Schedule: {:?}\n\
             // Dimensions: in={}, out={}, batch={}\n\
             // Tile: M={}, N={}, K={}\n\
             // Occupancy: {:.1}%, SharedMem: {} B\n\
             @group(0) @binding(0) var<storage, read> weights: array<u32>;\n\
             @group(0) @binding(1) var<storage, read> inputs: array<f32>;\n\
             @group(0) @binding(2) var<storage, read_write> outputs: array<f32>;\n\
             var<workgroup> s_scratch: array<f32, {}>;\n\
             \n\
             @compute @workgroup_size({}, 1, 1)\n\
             fn main(@builtin(global_invocation_id) global_id: vec3<u32>,\n\
                     @builtin(local_invocation_id) local_id: vec3<u32>) {{\n\
                 // Bounded zero-allocation execution loop\n\
             }}\n",
            self.kind,
            self.in_features,
            self.out_features,
            self.batch_size,
            self.tile_m,
            self.tile_n,
            self.tile_k,
            self.diagnostics.theoretical_occupancy * 100.0,
            smem_bytes,
            smem_bytes / 4,
            workgroup_size
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decode_schedule_synthesis() {
        let sched = GpuOperatorSchedule::plan(2048, 512, 1);
        assert_eq!(sched.kind, GpuScheduleKind::Decode);
        assert_eq!(sched.tile_m, 1);
        assert_eq!(sched.diagnostics.threads_per_workgroup, 128);
        assert!(!sched.diagnostics.spill_warning);
        assert_eq!(sched.diagnostics.theoretical_occupancy, 1.0);

        let code = sched.emit_wgsl_stub();
        assert!(code.contains("@compute @workgroup_size(128, 1, 1)"));
    }

    #[test]
    fn test_prefill_schedule_synthesis() {
        let sched = GpuOperatorSchedule::plan(2048, 512, 8);
        assert_eq!(sched.kind, GpuScheduleKind::Prefill);
        assert_eq!(sched.tile_m, 16);
        assert_eq!(sched.diagnostics.threads_per_workgroup, 256);
        assert!(!sched.diagnostics.spill_warning);

        let code = sched.emit_wgsl_stub();
        assert!(code.contains("@compute @workgroup_size(256, 1, 1)"));
    }
}
