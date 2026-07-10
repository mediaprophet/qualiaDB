//! Device peak calibration — real roofline anchors (plan L1.2).

use std::time::Instant;

use crate::device_benchmark::benchmark_devices;

#[derive(Debug, Clone)]
pub struct DeviceRoof {
    pub gemv_n: usize,
    pub best_label: String,
    pub best_backend: String,
    pub gemv_ms: f64,
    pub gemv_gflops: f64,
    pub upload_gbps: f64,
    pub balance_flop_per_byte: f64,
    pub notes: String,
}

/// Calibrate using the existing multi-circuit GEMV passport bench + derived balance point.
pub fn calibrate_device_roof(gemv_n: usize) -> DeviceRoof {
    let n = gemv_n.max(256).min(4096);
    let matrix = benchmark_devices(n);
    let best = matrix.best();
    let (label, backend, ms, gflops, up) = match best {
        Some(c) => (
            c.label.clone(),
            c.backend.clone(),
            c.ms_per_gemv,
            c.gflops,
            c.upload_gbps,
        ),
        None => (
            "none".into(),
            "none".into(),
            f64::INFINITY,
            0.0,
            0.0,
        ),
    };
    // Rough balance: if we measured G FLOP/s and U GB/s, balance ≈ G / U (FLOP/byte).
    // upload_gbps is host→device; for in-pool compute use gflops as primary signal.
    let balance = if up.is_finite() && up > 0.1 {
        // GFLOP/s / (GB/s) = FLOP/byte
        (gflops.max(0.01)) / up
    } else {
        // CPU in-pool: treat as compute-leaning default
        20.0
    };
    DeviceRoof {
        gemv_n: n,
        best_label: label,
        best_backend: backend,
        gemv_ms: ms,
        gemv_gflops: gflops,
        upload_gbps: up,
        balance_flop_per_byte: balance,
        notes: format!(
            "from benchmark_devices({n}); use balance_flop_per_byte for schedule classification"
        ),
    }
}

impl DeviceRoof {
    pub fn format_report(&self) -> String {
        format!(
            "Device roof calibration\n  gemv_n:        {}\n  best:          {} [{}]\n  gemv_ms:       {:.4}\n  gemv_gflops:   {:.2}\n  upload_gbps:   {:.2}\n  balance FLOP/B:{:.2}\n  {}\n",
            self.gemv_n,
            self.best_label,
            self.best_backend,
            self.gemv_ms,
            self.gemv_gflops,
            if self.upload_gbps.is_finite() {
                self.upload_gbps
            } else {
                -1.0
            },
            self.balance_flop_per_byte,
            self.notes
        )
    }
}

/// Quick CPU Q4 dequant·dot micro for intensity reference (no GPU).
pub fn cpu_q4_intensity_probe(n_in: usize, n_out: usize) -> (f64, f64) {
    use crate::ggml_quants::{
        dequantize_row_into, q4k_block_to_soa, GGML_TYPE_Q4_K_SOA, BLOCK_Q4K_SOA_BYTES,
    };
    let n_in = n_in.max(256) & !255; // multiple of 256
    let n_out = n_out.max(1).min(64);
    let mut stock = [0u8; 144];
    stock[0] = 0x00;
    stock[1] = 0x3c;
    for i in 4..144 {
        stock[i] = (i as u8).wrapping_mul(17);
    }
    let mut soa = [0u8; BLOCK_Q4K_SOA_BYTES];
    let _ = q4k_block_to_soa(&stock, &mut soa);
    let n_blocks = n_in / 256;
    let mut weight = Vec::with_capacity(n_out * n_blocks * BLOCK_Q4K_SOA_BYTES);
    for _ in 0..n_out {
        for _ in 0..n_blocks {
            weight.extend_from_slice(&soa);
        }
    }
    let x: Vec<f32> = (0..n_in).map(|i| (i as f32) * 0.001).collect();
    let mut row = vec![0.0f32; n_in];
    let t0 = Instant::now();
    let mut acc = 0.0f64;
    for r in 0..n_out {
        let off = r * n_blocks * BLOCK_Q4K_SOA_BYTES;
        dequantize_row_into(
            &weight[off..off + n_blocks * BLOCK_Q4K_SOA_BYTES],
            GGML_TYPE_Q4_K_SOA,
            n_in,
            &mut row,
        )
        .ok();
        acc += row.iter().zip(x.iter()).map(|(a, b)| (*a as f64) * (*b as f64)).sum::<f64>();
    }
    let secs = t0.elapsed().as_secs_f64().max(1e-9);
    let flops = (n_in as f64) * (n_out as f64) * 2.0; // mul+add
    let bytes = (weight.len() + n_in * 4 + n_out * 4) as f64;
    let gflops = (flops / secs) / 1e9;
    let intensity = flops / bytes;
    let _ = acc;
    (gflops, intensity)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_intensity_positive() {
        let (g, i) = cpu_q4_intensity_probe(256, 4);
        assert!(g > 0.0);
        assert!(i > 0.0);
    }
}
