//! Thermal / VRAM policy stubs for vision (shared with SR plan Auto policy).

/// Coarse thermal hint from host (maps from ThermalGovernor when wired).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ThermalHint {
    Cool,
    Warm,
    Critical,
}

/// Whether GPU tile work is allowed under thermal state.
pub fn thermal_allows_gpu_tiles(hint: ThermalHint) -> bool {
    !matches!(hint, ThermalHint::Critical)
}

/// Soft VRAM budget for vision tiles (bytes). Host may override.
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
    /// Rough peak for float32 NCHW tile: c * h * w * 4 * scale² factors.
    pub fn estimate_resize_scratch(c: u32, h: u32, w: u32, scale: u32) -> u64 {
        let in_b = c as u64 * h as u64 * w as u64 * 4;
        let out_b = c as u64 * (h as u64 * scale as u64) * (w as u64 * scale as u64) * 4;
        in_b.saturating_add(out_b)
    }

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
        let b = VisionVramBudget {
            max_bytes: 1024,
        };
        assert!(!b.allows(VisionVramBudget::estimate_resize_scratch(3, 4096, 4096, 4)));
    }
}
