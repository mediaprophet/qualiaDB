//! Dynamic Memory & GPU Compute Throughput Governor
//!
//! Balances rendering fidelity, voxel DAG editing, and raymarching against
//! concurrent local LLM inference and hardware limits.
//!
//! Integrates directly with QualiaDB's LLM engine and machine GPU profile.

use serde::{Deserialize, Serialize};

/// Mode of operation for the hardware governor.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum GovernorMode {
    /// Reactive closed-loop feedback controller (automated PID/VibeScript).
    AutomatedScripted,
    /// Direct user/operator manual override (fixed quotas and profiles).
    ManualOverride,
}

/// Execution state of the concurrent local LLM inference engine.
#[repr(u8)]
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum LlmWorkloadPhase {
    /// LLM is loaded in mmap cache, but no active generation or prompt ingestion is occurring.
    Idle = 0,
    /// LLM is ingesting prompt tokens: high compute saturation and memory bandwidth.
    PromptProcessing = 1,
    /// LLM is actively generating tokens autoregressively (e.g. 30–60 tok/s).
    ActiveDecode = 2,
}

/// System hardware memory class based on total accessible VRAM.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum HardwareMemoryClass {
    /// 2–6 GB (Shared system memory / iGPU / Mobile)
    IntegratedEntry,
    /// 8–12 GB (Mainstream discrete GPU / Laptop, e.g. RTX 4060)
    MainstreamDesktop,
    /// 16–24 GB (Enthusiast workstation, e.g. RTX 4090, Apple M3 Max)
    EnthusiastWorkstation,
    /// 48–192+ GB (Multi-GPU workstation / Server cluster / Apple M2 Ultra)
    EnterpriseMultiGpu,
}

/// Telemetry snapshot sampled from physical GPU hardware (NVML, DXGI, Vulkan, Metal, or WebGPU).
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct GpuTelemetrySample {
    /// Total installed physical VRAM in bytes
    pub vram_total_bytes: u64,
    /// Currently allocated VRAM across all OS processes in bytes
    pub vram_used_bytes: u64,
    /// Safe OS-recommended memory budget ceiling in bytes
    pub vram_budget_bytes: u64,
    /// GPU compute engine utilization percentage [0..100]
    pub compute_utilization_pct: u8,
    /// GPU junction temperature in degrees Celsius
    pub temperature_celsius: u8,
    /// Total instantaneous board power consumption in Watts
    pub board_power_watts: f32,
    /// Current execution phase of the local LLM inference engine
    pub llm_phase: LlmWorkloadPhase,
    /// Current token generation speed (tokens/sec)
    pub llm_decode_rate_toks: f32,
}

impl GpuTelemetrySample {
    /// Classifies the host hardware into its memory capability class.
    pub fn memory_class(&self) -> HardwareMemoryClass {
        const GB: u64 = 1024 * 1024 * 1024;
        if self.vram_total_bytes < 7 * GB {
            HardwareMemoryClass::IntegratedEntry
        } else if self.vram_total_bytes < 15 * GB {
            HardwareMemoryClass::MainstreamDesktop
        } else if self.vram_total_bytes < 32 * GB {
            HardwareMemoryClass::EnthusiastWorkstation
        } else {
            HardwareMemoryClass::EnterpriseMultiGpu
        }
    }

    /// Computes current VRAM pressure as a ratio [0.0..1.0+].
    #[inline]
    pub fn vram_pressure_ratio(&self) -> f32 {
        if self.vram_budget_bytes == 0 {
            0.0
        } else {
            self.vram_used_bytes as f32 / self.vram_budget_bytes as f32
        }
    }
}

/// Dynamic resource budget allocated to the terrain and environmental simulation.
#[derive(Debug, Copy, Clone, PartialEq, Serialize, Deserialize)]
pub struct TerrainBudget {
    /// Maximum byte budget for the GPU Ring (.m64 / .e64 tile cache)
    pub gpu_ring_budget_bytes: u64,
    /// Maximum raymarching step count per pixel (32..256)
    pub max_raymarching_steps: u32,
    /// Maximum compute queue execution time per frame (milliseconds)
    pub compute_time_budget_ms: f32,
    /// Active radius in meters for high-detail Sparse Voxel DAG deformation
    pub svo_voxel_radius_m: f32,
    /// Whether real-time digging/soil displacement is permitted this frame
    pub allow_subsurface_voxel_edit: bool,
    /// Whether cache eviction is demanded due to high memory pressure
    pub demand_cache_flush: bool,
}

/// Hardware Governor configuration parameters.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GovernorConfig {
    pub mode: GovernorMode,
    /// Target frame rate in Hz (default: 60.0)
    pub target_fps: f32,
    /// Explicit manual memory cap for terrain (bytes, used in ManualOverride mode)
    pub manual_terrain_vram_cap: u64,
    /// Explicit manual compute time budget (ms, used in ManualOverride mode)
    pub manual_compute_time_budget_ms: f32,
    /// Explicit manual raymarching step limit (used in ManualOverride mode)
    pub manual_raymarching_steps: u32,
}

impl Default for GovernorConfig {
    fn default() -> Self {
        Self {
            mode: GovernorMode::AutomatedScripted,
            target_fps: 60.0,
            manual_terrain_vram_cap: 512 * 1024 * 1024,
            manual_compute_time_budget_ms: 6.0,
            manual_raymarching_steps: 96,
        }
    }
}

/// Hardware Governor managing VRAM allocation and compute queue time-slicing.
#[derive(Debug, Clone)]
pub struct VramAndComputeGovernor {
    pub config: GovernorConfig,
    /// Hysteresis tracker for consecutive high-pressure frames
    pub high_pressure_streak: u32,
}

impl VramAndComputeGovernor {
    pub fn new(config: GovernorConfig) -> Self {
        Self {
            config,
            high_pressure_streak: 0,
        }
    }

    /// Evaluates live telemetry and returns the optimal terrain execution budget.
    pub fn evaluate_budget(
        &mut self,
        telemetry: &GpuTelemetrySample,
        last_frame_time_ms: f32,
    ) -> TerrainBudget {
        if self.config.mode == GovernorMode::ManualOverride {
            return TerrainBudget {
                gpu_ring_budget_bytes: self.config.manual_terrain_vram_cap,
                max_raymarching_steps: self.config.manual_raymarching_steps,
                compute_time_budget_ms: self.config.manual_compute_time_budget_ms,
                svo_voxel_radius_m: 16.0,
                allow_subsurface_voxel_edit: true,
                demand_cache_flush: telemetry.vram_pressure_ratio() > 0.95,
            };
        }

        // --- Automated Scripted Mode ---
        let mem_class = telemetry.memory_class();
        let pressure = telemetry.vram_pressure_ratio();

        if pressure > 0.90 {
            self.high_pressure_streak = self.high_pressure_streak.saturating_add(1);
        } else {
            self.high_pressure_streak = 0;
        }

        // 1. Base GPU Ring Budget based on hardware memory tier
        let base_ring_bytes: u64 = match mem_class {
            HardwareMemoryClass::IntegratedEntry => 64 * 1024 * 1024,      // 64 MiB
            HardwareMemoryClass::MainstreamDesktop => 256 * 1024 * 1024,   // 256 MiB
            HardwareMemoryClass::EnthusiastWorkstation => 1024 * 1024 * 1024, // 1024 MiB
            HardwareMemoryClass::EnterpriseMultiGpu => 8192 * 1024 * 1024,  // 8192 MiB
        };

        // 2. Modulate based on LLM Phase and VRAM Pressure
        let (gpu_ring_budget_bytes, max_raymarching_steps, compute_time_budget_ms, svo_voxel_radius_m) =
            match telemetry.llm_phase {
                LlmWorkloadPhase::ActiveDecode => {
                    let ring = (base_ring_bytes / 2).max(64 * 1024 * 1024);
                    let steps = match mem_class {
                        HardwareMemoryClass::IntegratedEntry => 32,
                        HardwareMemoryClass::MainstreamDesktop => 64,
                        HardwareMemoryClass::EnthusiastWorkstation => 96,
                        HardwareMemoryClass::EnterpriseMultiGpu => 128,
                    };
                    let compute_ms = 2.5f32;
                    let svo_radius = 8.0f32;
                    (ring, steps, compute_ms, svo_radius)
                }
                LlmWorkloadPhase::PromptProcessing => {
                    let ring = (base_ring_bytes / 2).max(64 * 1024 * 1024);
                    let steps = 48;
                    let compute_ms = 1.0f32;
                    let svo_radius = 4.0f32;
                    (ring, steps, compute_ms, svo_radius)
                }
                LlmWorkloadPhase::Idle => {
                    let ring = base_ring_bytes;
                    let steps = match mem_class {
                        HardwareMemoryClass::IntegratedEntry => 48,
                        HardwareMemoryClass::MainstreamDesktop => 96,
                        HardwareMemoryClass::EnthusiastWorkstation => 160,
                        HardwareMemoryClass::EnterpriseMultiGpu => 256,
                    };
                    let compute_ms = 12.0f32;
                    let svo_radius = match mem_class {
                        HardwareMemoryClass::IntegratedEntry => 0.0,
                        HardwareMemoryClass::MainstreamDesktop => 12.0,
                        HardwareMemoryClass::EnthusiastWorkstation => 24.0,
                        HardwareMemoryClass::EnterpriseMultiGpu => 48.0,
                    };
                    (ring, steps, compute_ms, svo_radius)
                }
            };

        // 3. Frame-pacing feedback
        let target_frame_ms = 1000.0 / self.config.target_fps;
        let adjusted_compute_ms = if last_frame_time_ms > target_frame_ms * 1.1 {
            (compute_time_budget_ms * 0.75).max(1.5)
        } else {
            compute_time_budget_ms
        };

        let demand_cache_flush = self.high_pressure_streak >= 3 || pressure > 0.94;
        let allow_subsurface_voxel_edit =
            svo_voxel_radius_m > 0.0 && telemetry.llm_phase != LlmWorkloadPhase::PromptProcessing;

        TerrainBudget {
            gpu_ring_budget_bytes,
            max_raymarching_steps,
            compute_time_budget_ms: adjusted_compute_ms,
            svo_voxel_radius_m,
            allow_subsurface_voxel_edit,
            demand_cache_flush,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_governor_idle_desktop_scaling() {
        let mut gov = VramAndComputeGovernor::new(GovernorConfig::default());
        let telemetry = GpuTelemetrySample {
            vram_total_bytes: 12 * 1024 * 1024 * 1024, // 12 GB RTX 4070
            vram_used_bytes: 3 * 1024 * 1024 * 1024,
            vram_budget_bytes: 10 * 1024 * 1024 * 1024,
            compute_utilization_pct: 20,
            temperature_celsius: 52,
            board_power_watts: 85.0,
            llm_phase: LlmWorkloadPhase::Idle,
            llm_decode_rate_toks: 0.0,
        };

        let budget = gov.evaluate_budget(&telemetry, 16.0);
        assert_eq!(budget.gpu_ring_budget_bytes, 256 * 1024 * 1024);
        assert_eq!(budget.max_raymarching_steps, 96);
        assert!(budget.allow_subsurface_voxel_edit);
        assert!(!budget.demand_cache_flush);
    }

    #[test]
    fn test_governor_llm_active_decode_throttle() {
        let mut gov = VramAndComputeGovernor::new(GovernorConfig::default());
        let telemetry = GpuTelemetrySample {
            vram_total_bytes: 12 * 1024 * 1024 * 1024,
            vram_used_bytes: 7 * 1024 * 1024 * 1024,
            vram_budget_bytes: 10 * 1024 * 1024 * 1024,
            compute_utilization_pct: 75,
            temperature_celsius: 64,
            board_power_watts: 140.0,
            llm_phase: LlmWorkloadPhase::ActiveDecode,
            llm_decode_rate_toks: 42.5,
        };

        let budget = gov.evaluate_budget(&telemetry, 16.0);
        assert_eq!(budget.gpu_ring_budget_bytes, 128 * 1024 * 1024);
        assert_eq!(budget.max_raymarching_steps, 64);
        assert!(budget.compute_time_budget_ms <= 2.5);
    }
}
