use super::*;
use serde::{Deserialize, Serialize};

// ───────────────────────────────────────────────────────────────────────────
//  Reserve-mode budget query
// ───────────────────────────────────────────────────────────────────────────

/// Device availability for Reserve-mode filtering.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeviceAvailability {
    pub cpu: bool,
    pub simd: bool,
    pub wgpu: bool,
    pub cuda: bool,
    pub wasm: bool,
    pub exact: bool,
}

impl Default for DeviceAvailability {
    fn default() -> Self {
        Self {
            cpu: true,
            simd: true,
            wgpu: false,
            cuda: false,
            wasm: false,
            exact: true,
        }
    }
}

impl DeviceAvailability {
    fn supports(&self, backend: Backend) -> bool {
        match backend {
            Backend::Scalar => self.cpu,
            Backend::Simd => self.simd,
            Backend::Wgpu => self.wgpu,
            Backend::Cuda => self.cuda,
            Backend::Wasm => self.wasm,
            Backend::Exact => self.exact,
        }
    }
}

/// Given a device availability mask, return the backends runnable on
/// this device for the given op. Never returns an empty list — if all
/// GPU backends are unavailable, the CPU/WASM fallback is returned.
pub fn reserve_budget_query(op: &str, device: &DeviceAvailability) -> Vec<Backend> {
    GEOMETRY_OP_MANIFESTS
        .iter()
        .find(|m| m.op == op)
        .map(|m| {
            let runnable: Vec<Backend> = m
                .backends
                .iter()
                .copied()
                .filter(|b| device.supports(*b))
                .collect();

            if runnable.is_empty() {
                // Fallback: return deterministic backends regardless of mask.
                m.backends
                    .iter()
                    .copied()
                    .filter(|b| b.is_deterministic_fallback())
                    .collect()
            } else {
                runnable
            }
        })
        .unwrap_or_default()
}
