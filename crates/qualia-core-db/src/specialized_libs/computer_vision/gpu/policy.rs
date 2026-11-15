//! Thermal / VRAM dynamic policy for vision workloads driven by the host ThermalGovernor.

use crate::inference::orchestrator::{ThermalGovernor, ThermalStatus};

/// Coarse thermal hint from host (maps from ThermalGovernor).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalHint {
    Cool,
    Warm,
    Critical,
}

impl From<ThermalStatus> for ThermalHint {
    fn from(status: ThermalStatus) -> Self {
        match status {
            ThermalStatus::Cool => Self::Cool,
            ThermalStatus::Warm => Self::Warm,
            ThermalStatus::Critical => Self::Critical,
        }
    }
}

/// Whether GPU tile work is allowed under thermal state.
pub fn thermal_allows_gpu_tiles(hint: ThermalHint) -> bool {
    !matches!(hint, ThermalHint::Critical)
}

/// Dynamic VRAM budget for vision tiles (bytes) derived from host thermal state.
#[derive(Debug, Clone, Copy)]
pub struct VisionVramBudget {
    pub max_bytes: u64,
}

impl Default for VisionVramBudget {
    fn default() -> Self {
        Self {
            max_bytes: 256 * 1024 * 1024, // 256 MiB soft default
        }
    }
}

impl VisionVramBudget {
    /// Calculate dynamic VRAM budget using the host device's ThermalGovernor
    pub fn from_governor(governor: &dyn ThermalGovernor) -> Self {
        let max_bytes = match governor.get_thermal_state() {
            ThermalStatus::Cool => 512 * 1024 * 1024,
            ThermalStatus::Warm => 256 * 1024 * 1024,
            ThermalStatus::Critical => 64 * 1024 * 1024,
        };
        Self { max_bytes }
    }

    /// Rough peak for float32 NCHW tile: c * h * w * 4 * scale² factors.
    pub fn estimate_resize_scratch(c: u32, h: u32, w: u32, scale: u32) -> u64 {
        let in_b = c as u64 * h as u64 * w as u64 * 4;
        let out_b = c as u64 * (h as u64 * scale as u64) * (w as u64 * scale as u64) * 4;
        in_b.saturating_add(out_b)
    }

    /// `true` when the policy allows the op.
    pub fn allows(&self, estimate: u64) -> bool {
        estimate <= self.max_bytes
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn critical_blocks_gpu() {
        assert!(!thermal_allows_gpu_tiles(ThermalHint::Critical));
        assert!(thermal_allows_gpu_tiles(ThermalHint::Cool));
    }

    #[test]
    fn budget_refuses_huge() {
        let b = VisionVramBudget { max_bytes: 1024 };
        assert!(!b.allows(VisionVramBudget::estimate_resize_scratch(3, 4096, 4096, 4)));
    }

    #[test]
    fn budget_scales_with_governor() {
        use crate::inference::orchestrator::NullThermalGovernor;
        let gov = NullThermalGovernor;
        let budget = VisionVramBudget::from_governor(&gov);
        assert_eq!(budget.max_bytes, 512 * 1024 * 1024);
    }
}
