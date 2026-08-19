//! Isolated Q4_K SoA GEMV microbench (CPU oracle + optional CUDA).

use std::time::Instant;

use crate::ggml_quants::{
    dequantize_row_into, q4k_block_to_soa, BLOCK_Q4K_SOA_BYTES, GGML_TYPE_Q4_K_SOA,
};
use crate::inference_kernel_parity::{max_abs_err, max_ulp_diff};
use crate::inference_modes::{set_inference_mode, InferenceMode};

#[derive(Debug, Clone)]
pub struct MicrobenchResult {
    pub n_in: usize,
    pub n_out: usize,
    pub cpu_ms: f64,
    pub cpu_gflops: f64,
    pub cuda_ms: Option<f64>,
    pub cuda_ok: bool,
    pub max_abs_err: f32,
    pub max_ulp: u64,
    pub notes: String,
}

fn synth_soa_weight(n_in: usize, n_out: usize) -> (Vec<u8>, Vec<f32>) {
    let n_in = n_in.max(256) & !255;
    let mut stock = [0u8; 144];
    stock[0] = 0x00;
    stock[1] = 0x3c;
    stock[2] = 0x00;
    stock[3] = 0x38;
    for i in 4..144 {
        stock[i] = (i as u8).wrapping_mul(17);
    }
    let mut soa = [0u8; BLOCK_Q4K_SOA_BYTES];
    q4k_block_to_soa(&stock, &mut soa).expect("soa");
    let n_blocks = n_in / 256;
    let mut weight = Vec::with_capacity(n_out * n_blocks * BLOCK_Q4K_SOA_BYTES);
    for _ in 0..n_out {
        for _ in 0..n_blocks {
            weight.extend_from_slice(&soa);
        }
    }
    let x: Vec<f32> = (0..n_in).map(|i| (i as f32) * 0.01).collect();
    (weight, x)
}

fn cpu_gemv(n_in: usize, n_out: usize, weight: &[u8], x: &[f32], out: &mut [f32]) {
    let row_bytes = (n_in / 256) * BLOCK_Q4K_SOA_BYTES;
    let mut row = vec![0.0f32; n_in];
    for r in 0..n_out {
        dequantize_row_into(
            &weight[r * row_bytes..(r + 1) * row_bytes],
            GGML_TYPE_Q4_K_SOA,
            n_in,
            &mut row,
        )
        .unwrap();
        out[r] = row.iter().zip(x.iter()).map(|(a, b)| a * b).sum();
    }
}

/// Run CPU Q4 SoA GEMV; if CUDA mode available, differential vs GPU.
pub fn run_q4k_soa_microbench(n_in: usize, n_out: usize) -> MicrobenchResult {
    let n_in = n_in.max(256) & !255;
    let n_out = n_out.max(1).min(512);
    let (weight, x) = synth_soa_weight(n_in, n_out);
    let mut cpu_out = vec![0.0f32; n_out];

    // Warm (untimed) — no Instant here (avoids unused `t0` if warm is dropped).
    cpu_gemv(n_in, n_out, &weight, &x, &mut cpu_out);
    let cpu_start = Instant::now();
    cpu_gemv(n_in, n_out, &weight, &x, &mut cpu_out);
    let cpu_ms = cpu_start.elapsed().as_secs_f64() * 1e3;
    let flops = (n_in as f64) * (n_out as f64) * 2.0;
    let cpu_gflops = (flops / (cpu_ms / 1e3).max(1e-9)) / 1e9;

    let mut cuda_ms = None;
    let mut cuda_ok = false;
    let mut max_abs = 0.0f32;
    let mut max_ulp = 0u64;

    // CUDA differential when available.
    let prev = std::env::var("QUALIA_INFERENCE_MODE").ok();
    set_inference_mode(InferenceMode::CudaTc);
    std::env::set_var("QUALIA_INFERENCE_MODE", "cuda");
    let mut gpu_out = vec![0.0f32; n_out];
    #[cfg(feature = "cuda")]
    let ok = crate::try_q4k_soa_gemv(n_in, n_out, &x, &weight, &mut gpu_out);
    #[cfg(not(feature = "cuda"))]
    let ok = false;
    // Single assignment (not `mut notes` + overwrite) so unused_assignments stays quiet.
    let notes = if ok {
        // Warm GPU path untimed; second call times sticky weights.
        let cuda_start = Instant::now();
        #[cfg(feature = "cuda")]
        let ok2 = crate::try_q4k_soa_gemv(n_in, n_out, &x, &weight, &mut gpu_out);
        #[cfg(not(feature = "cuda"))]
        let ok2 = false;
        cuda_ms = Some(cuda_start.elapsed().as_secs_f64() * 1e3);
        cuda_ok = ok2;
        max_abs = max_abs_err(&cpu_out, &gpu_out);
        max_ulp = max_ulp_diff(&cpu_out, &gpu_out);
        #[cfg(feature = "cuda")]
        let wcount = crate::q4k_device_weight_count();
        #[cfg(not(feature = "cuda"))]
        let wcount = 0usize;
        format!("cuda differential max_abs={max_abs:.6e} max_ulp={max_ulp} weights={wcount}")
    } else {
        "cuda unavailable or prefer_tensor_core_gemm false - cpu only".into()
    };
    match prev {
        Some(v) => std::env::set_var("QUALIA_INFERENCE_MODE", v),
        None => std::env::remove_var("QUALIA_INFERENCE_MODE"),
    }
    set_inference_mode(InferenceMode::Portable);

    MicrobenchResult {
        n_in,
        n_out,
        cpu_ms,
        cpu_gflops,
        cuda_ms,
        cuda_ok,
        max_abs_err: max_abs,
        max_ulp,
        notes,
    }
}

impl MicrobenchResult {
    pub fn format_report(&self) -> String {
        format!(
            "Q4_K SoA GEMV microbench\n  shape:     {} x {}\n  cpu_ms:    {:.4}  ({:.2} GFLOP/s)\n  cuda_ms:   {}\n  cuda_ok:   {}\n  max_abs:   {:.6e}\n  max_ulp:   {}\n  {}\n",
            self.n_in,
            self.n_out,
            self.cpu_ms,
            self.cpu_gflops,
            self.cuda_ms
                .map(|m| format!("{m:.4}"))
                .unwrap_or_else(|| "—".into()),
            self.cuda_ok,
            self.max_abs_err,
            self.max_ulp,
            self.notes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[serial_test::serial]
    fn micro_cpu_runs() {
        let r = run_q4k_soa_microbench(256, 4);
        assert!(r.cpu_ms > 0.0);
        assert!(r.cpu_gflops > 0.0);
    }
}
