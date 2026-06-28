//! Simple roofline estimates per kernel (plan §6 / §10).
//!
//! For each kernel we estimate the FLOPs performed and the bytes moved over a
//! representative problem size, giving an arithmetic intensity (FLOP/byte) and a
//! memory-vs-compute classification.
//!
//! **Known limitations (honest, not stubs).** This is an *estimate only*; it never
//! rejects a schedule. wgpu does not expose device peak FLOPS or memory bandwidth, so
//! there is no device-relative roofline ceiling to reject against — a real
//! device-relative bound would require a calibration micro-benchmark. Likewise,
//! compute-unit-saturation pruning is not implemented because wgpu does not expose a
//! compute-unit (SM/CU) count. The classification here is therefore used to *explain*
//! why a schedule is (or isn't) worth pursuing and to drive the search-tree dump, not
//! to gate the search.

use serde::{Deserialize, Serialize};

use super::BuiltinKernel;

/// Whether a kernel is dominated by memory traffic or by arithmetic.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RooflineBound {
    Memory,
    Compute,
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct RooflineEstimate {
    pub flops: u64,
    pub bytes: u64,
    pub arithmetic_intensity: f64,
    pub bound: RooflineBound,
}

/// Crossover FLOP/byte below which a kernel is treated as memory-bound. Modern
/// discrete GPUs sit roughly in the 10–40 range; 10 is a conservative default
/// used when no per-device calibration is available.
pub const DEFAULT_BALANCE_FLOP_PER_BYTE: f64 = 10.0;

impl RooflineEstimate {
    pub fn new(flops: u64, bytes: u64) -> Self {
        let arithmetic_intensity = if bytes == 0 {
            0.0
        } else {
            flops as f64 / bytes as f64
        };
        let bound = if arithmetic_intensity < DEFAULT_BALANCE_FLOP_PER_BYTE {
            RooflineBound::Memory
        } else {
            RooflineBound::Compute
        };
        Self {
            flops,
            bytes,
            arithmetic_intensity,
            bound,
        }
    }
}

/// Roofline estimate for `kernel` over a representative size `n` (output
/// elements / records / rays, depending on the kernel).
pub fn roofline_for(kernel: BuiltinKernel, n: u64) -> RooflineEstimate {
    match kernel {
        // out[i] = in[i]*scale + bias: one FMA (2 FLOP); read + write one f32.
        BuiltinKernel::AffineF32 => RooflineEstimate::new(2 * n, 8 * n),
        // per record: 16 MACs (32 FLOP); read 64 bytes, write 4.
        BuiltinKernel::P64Project => RooflineEstimate::new(32 * n, 68 * n),
        // top-k: ~k tree passes per block; load-dominated, little arithmetic.
        BuiltinKernel::TopK => RooflineEstimate::new(4 * n, 4 * n),
        // ray-probe: per-ray traversal is variable; modelled as load-dominated.
        BuiltinKernel::RayProbe => RooflineEstimate::new(8 * n, 36 * n),
        // FFN (input=64, hidden=128): weights amortised (read once), so for large
        // n this is compute-bound — the desired roofline answer.
        BuiltinKernel::FusedFfn => {
            let input = 64u64;
            let hidden = 128u64;
            let flops = n * (hidden * (2 * input + 8) + 2 * hidden);
            let bytes = (input + hidden * input + n * hidden + n) * 4;
            RooflineEstimate::new(flops, bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_is_memory_bound_ffn_is_compute_bound() {
        assert_eq!(roofline_for(BuiltinKernel::AffineF32, 1_000_000).bound, RooflineBound::Memory);
        assert_eq!(roofline_for(BuiltinKernel::FusedFfn, 4096).bound, RooflineBound::Compute);
    }

    #[test]
    fn intensity_is_flops_over_bytes() {
        let estimate = roofline_for(BuiltinKernel::AffineF32, 10);
        assert!((estimate.arithmetic_intensity - 0.25).abs() < 1e-9);
    }
}
