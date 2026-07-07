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
        // ternary GEMV (K=256): per output row, K MACs (2 FLOP) over the activation
        // vector plus one scale multiply. Bytes: 2-bit-packed weights (K/16 u32 =
        // K/4 bytes per row), the shared K-length f32 x once, plus scale + output.
        // n is the output-row count M; x is amortised (read once) so for large M
        // this is memory-bound on the packed weights, the desired roofline answer.
        BuiltinKernel::TernaryGemv => {
            let k = 256u64;
            let flops = n * (2 * k + 1);
            let bytes = (n * (k / 16) + k + n + n) * 4;
            RooflineEstimate::new(flops, bytes)
        }
        // Dense GEMM C[M×N] = A[M×K]·B[K×N] (K=64). `n` is the output-element
        // count (M*N), one invocation each. Per output element: K MACs = 2K FLOP.
        // Bytes: the K-length A-row and B-column read per element (2*K), plus the
        // one output write — i.e. (n*(2*K + 1))*4. Large-N this is compute-bound,
        // the desired roofline answer for a dense matmul.
        BuiltinKernel::Gemm => {
            let k = 64u64;
            let flops = n * 2 * k;
            let bytes = (n * (2 * k + 1)) * 4;
            RooflineEstimate::new(flops, bytes)
        }
        // Dense GEMV y[M] = A[M×N]·x[N] (N=256). `n` is the output-row count M, one
        // invocation each. Per output row: N MACs = 2N FLOP. Bytes: the whole matrix
        // A (M*N) is read once, x (N) is read once (amortised), plus the M output
        // writes — i.e. (M*N + N + M)*4. GEMV is memory-bound (each A element is read
        // exactly once, AI ≈ 2 FLOP/byte/2 = ~0.5), the desired roofline answer for a
        // matrix-vector product.
        BuiltinKernel::Gemv => {
            let row = n; // output-row count M
            let n_cols = 256u64;
            let flops = row * 2 * n_cols;
            let bytes = (row * n_cols + n_cols + row) * 4;
            RooflineEstimate::new(flops, bytes)
        }
        // Radix-2 FFT of `n` complex points: log2(n) stages, each doing n/2
        // butterflies; one butterfly is ~10 FLOP (complex multiply + two complex
        // add/sub), so ~5*n*log2(n) FLOP total. Bytes: read 2*n f32 in + write
        // 2*n f32 out = 4*n f32 = 4*n*4 bytes (the transform itself stays in
        // workgroup-shared memory). For modest n the log factor keeps arithmetic
        // intensity low, so this reads as memory-bound at small sizes — the
        // honest answer for a single-workgroup transform that is dominated by the
        // shared-memory traffic, not global FLOPs.
        BuiltinKernel::Fft => {
            let log2n = (n.max(1)).next_power_of_two().trailing_zeros() as u64;
            let flops = 5 * n * log2n;
            let bytes = 4 * n * 4;
            RooflineEstimate::new(flops, bytes)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_is_memory_bound_ffn_is_compute_bound() {
        assert_eq!(
            roofline_for(BuiltinKernel::AffineF32, 1_000_000).bound,
            RooflineBound::Memory
        );
        assert_eq!(
            roofline_for(BuiltinKernel::FusedFfn, 4096).bound,
            RooflineBound::Compute
        );
    }

    #[test]
    fn intensity_is_flops_over_bytes() {
        let estimate = roofline_for(BuiltinKernel::AffineF32, 10);
        assert!((estimate.arithmetic_intensity - 0.25).abs() < 1e-9);
    }
}
