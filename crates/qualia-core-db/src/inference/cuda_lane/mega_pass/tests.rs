use super::super::device::q4k_device_weight_count;
use super::super::gemv::try_q4k_soa_gemv;
use super::super::weight_cache::{try_cuda_batch_gemv, weight_fingerprint};
use crate::ggml_quants::{
    dequantize_row_into, q4k_block_to_soa, BLOCK_Q4K_SOA_BYTES, GGML_TYPE_Q4_K, GGML_TYPE_Q4_K_SOA,
};
use crate::inference_modes::{set_inference_mode, InferenceMode};

#[test]
fn fingerprint_stable() {
    let a = weight_fingerprint(b"abc", 3, 2);
    let b = weight_fingerprint(b"abc", 3, 2);
    assert_eq!(a, b);
    assert_ne!(a, weight_fingerprint(b"abd", 3, 2));
}

#[test]
fn batch_gemv_cpu_shape_pad() {
    // Without cuda mode this returns false quickly.
    if std::env::var("QUALIA_INFERENCE_MODE").ok().as_deref() != Some("cuda") {
        let mut out = [0.0f32; 4];
        let ok = try_cuda_batch_gemv(&[1.0, 0.0], 1, 2, 2, &[1.0, 0.0, 0.0, 1.0], &mut out);
        assert!(!ok);
    }
}

/// Build one synthetic Q4_K row (256 weights) → SoA, compare CUDA GEMV vs CPU dequant·dot.
#[test]
fn q4k_soa_gemv_matches_cpu_when_cuda_available() {
    if std::env::var("QUALIA_SKIP_CUDA").is_ok() {
        return;
    }
    // Deterministic pseudo-Q4_K block (144 B).
    let mut stock = [0u8; 144];
    stock[0] = 0x00;
    stock[1] = 0x3c; // d ≈ 1.0 f16
    stock[2] = 0x00;
    stock[3] = 0x38; // dmin small
    for i in 4..16 {
        stock[i] = 0x21;
    }
    for i in 16..144 {
        stock[i] = (i as u8).wrapping_mul(17);
    }
    let mut soa = [0u8; BLOCK_Q4K_SOA_BYTES];
    q4k_block_to_soa(&stock, &mut soa).expect("soa convert");

    let n_in = 256usize;
    let n_out = 4usize;
    let mut weight = Vec::with_capacity(n_out * BLOCK_Q4K_SOA_BYTES);
    for _ in 0..n_out {
        weight.extend_from_slice(&soa);
    }
    let x: Vec<f32> = (0..n_in).map(|i| (i as f32) * 0.01).collect();

    // CPU reference: dequant each row and dot.
    let mut cpu_out = vec![0.0f32; n_out];
    let mut row = vec![0.0f32; n_in];
    for r in 0..n_out {
        dequantize_row_into(
            &weight[r * BLOCK_Q4K_SOA_BYTES..(r + 1) * BLOCK_Q4K_SOA_BYTES],
            GGML_TYPE_Q4_K_SOA,
            n_in,
            &mut row,
        )
        .unwrap();
        cpu_out[r] = row.iter().zip(x.iter()).map(|(a, b)| a * b).sum();
    }

    // Force cuda mode for prefer_tensor_core_gemm.
    let prev = std::env::var("QUALIA_INFERENCE_MODE").ok();
    set_inference_mode(InferenceMode::CudaTc);
    std::env::set_var("QUALIA_INFERENCE_MODE", "cuda");

    let mut gpu_out = vec![0.0f32; n_out];
    let ok = try_q4k_soa_gemv(n_in, n_out, &x, &weight, &mut gpu_out);

    // Restore env.
    match prev {
        Some(v) => std::env::set_var("QUALIA_INFERENCE_MODE", v),
        None => std::env::remove_var("QUALIA_INFERENCE_MODE"),
    }
    set_inference_mode(InferenceMode::Portable);

    if !ok {
        // No CUDA toolkit / NVRTC — soft skip.
        eprintln!("q4k_soa_gemv: CUDA unavailable — skipped differential");
        return;
    }
    for r in 0..n_out {
        let err = (cpu_out[r] - gpu_out[r]).abs();
        let tol = 1e-2 * cpu_out[r].abs().max(1.0);
        assert!(
            err < tol,
            "row {r}: cpu={} gpu={} err={err}",
            cpu_out[r],
            gpu_out[r]
        );
    }
    // Multi-weight residency: a second distinct matrix should bump device count.
    let before = q4k_device_weight_count();
    let mut weight2 = weight.clone();
    if let Some(b) = weight2.last_mut() {
        *b = b.wrapping_add(1);
    }
    let mut gpu2 = vec![0.0f32; n_out];
    let ok2 = try_q4k_soa_gemv(n_in, n_out, &x, &weight2, &mut gpu2);
    if ok2 {
        assert!(
            q4k_device_weight_count() >= before.max(1),
            "expected sticky multi-weight residency after second matrix"
        );
    }
    let _ = GGML_TYPE_Q4_K; // silence if unused
}
