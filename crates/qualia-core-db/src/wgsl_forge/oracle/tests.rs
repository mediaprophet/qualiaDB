use super::reference::gelu;
use super::*;
use crate::wgsl_forge::execute::WgpuComputeContext;
use crate::wgsl_forge::{BuiltinKernel, Schedule, ValidationLevel};

#[test]
fn oracle_vectors_are_reproducible() {
    let first = OracleCase::affine(17, 42, 1.5, -0.25);
    let second = OracleCase::affine(17, 42, 1.5, -0.25);
    assert_eq!(first, second);
    assert!(compare_f32(
        &first.expected,
        &second.expected,
        OracleTolerance::default()
    )
    .passed());
}

#[test]
fn rayprobe_cpu_reference_is_sane() {
    let scene = rayprobe_scene();
    let rays = rayprobe_rays();
    let hits = rayprobe_cpu(&rays, &scene);
    assert_eq!(hits.len(), 12);
    // First 7 rays hit the z=2 quad (committed t = 3.0); last 5 miss (-1.0).
    for (i, &t) in hits.iter().enumerate() {
        if i < 7 {
            assert!((t - 3.0).abs() < 1.0e-4, "ray {i} expected t=3, got {t}");
        } else {
            assert_eq!(t, -1.0, "ray {i} expected a miss");
        }
    }
}

#[test]
#[serial_test::serial(gpu)]
fn rayprobe_certifies_on_real_gpu() {
    // Feasibility + correctness: build a BLAS/TLAS and run the emitted ray_probe
    // shader, checking committed hit distances against the Möller–Trumbore CPU
    // reference. This is the gate for whether wgpu 29.0.3 ray-query *executes*.
    if !crate::gpu_context::experimental_features_allowed() {
        return;
    }
    let Ok(mut context) = WgpuComputeContext::new(1024 * 1024) else {
        return;
    };
    if !context.constraints.supports_rt_cores {
        return;
    }
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    let (_, eval) = evaluate_rayprobe(&mut context, schedule, 1, 3).expect("ray-probe evaluation");
    assert!(
        eval.oracle.passed(),
        "ray-probe GPU/oracle mismatch: {:?}",
        eval.oracle
    );
}

#[test]
#[serial_test::serial(gpu)]
fn rayprobe_certify_builtin_on_real_gpu() {
    // Full certification pipeline: certify_builtin -> evaluate_builtin ->
    // evaluate_rayprobe -> manifest, proving RayProbe is a first-class
    // certifiable builtin (not just a standalone test).
    if !crate::gpu_context::experimental_features_allowed() {
        return;
    }
    let Ok(mut context) = WgpuComputeContext::new(1024 * 1024) else {
        return;
    };
    if !context.constraints.supports_rt_cores {
        return;
    }
    let schedule = Schedule {
        workgroup_size: 64,
        ..Default::default()
    };
    let manifest = certify_builtin(&mut context, BuiltinKernel::RayProbe, schedule, 12, 1, 3)
        .expect("ray-probe certification");
    assert_eq!(manifest.validation_level, ValidationLevel::Certified);
    assert!(manifest.oracle.as_ref().is_some_and(|o| o.passed()));
}

#[test]
fn comparator_reports_tail_and_numeric_errors() {
    let report = compare_f32(&[1.0, 2.0, 3.0], &[1.0, 2.1], OracleTolerance::default());
    assert_eq!(report.mismatch_count, 2);
    assert_eq!(report.first_mismatch, Some(1));
}

#[test]
fn nan_never_certifies() {
    assert!(!compare_f32(&[f32::NAN], &[f32::NAN], OracleTolerance::default()).passed());
}

#[test]
fn topk_cpu_matches_bruteforce_full_blocks() {
    let block_size = 8usize;
    let k = 3usize;
    let blocks = 4usize;
    let length = block_size * blocks;
    let input = topk_inputs(length, 99);
    let got = topk_cpu(&input, length, k, block_size);
    assert_eq!(got.len(), blocks * k);
    for b in 0..blocks {
        let mut block = input[b * block_size..(b + 1) * block_size].to_vec();
        block.sort_by(|a, c| c.partial_cmp(a).unwrap());
        for i in 0..k {
            assert_eq!(got[b * k + i], block[i], "block {b} rank {i}");
        }
        for i in 1..k {
            assert!(got[b * k + i] <= got[b * k + i - 1], "descending order");
        }
    }
}

#[test]
fn topk_cpu_handles_partial_tail_block() {
    let block_size = 8usize;
    let k = 2usize;
    let length = block_size * 2 + 3; // two full blocks + a 3-element tail
    let input = topk_inputs(length, 7);
    let got = topk_cpu(&input, length, k, block_size);
    assert_eq!(got.len(), 3 * k); // tail counts as a third block
    let mut tail = input[16..19].to_vec();
    tail.sort_by(|a, c| c.partial_cmp(a).unwrap());
    assert_eq!(got[2 * k], tail[0]);
    assert_eq!(got[2 * k + 1], tail[1]);
}

#[test]
fn ffn_cpu_zero_weights_yield_zero() {
    // gelu(0) = 0, so all-zero w1 forces every output to 0 regardless of w2.
    let out = ffn_cpu(
        &[1.0, 2.0, 3.0],
        &vec![0.0; 4 * 3],
        &vec![0.5; 2 * 4],
        3,
        4,
        2,
    );
    assert_eq!(out, vec![0.0, 0.0]);
}

#[test]
fn ffn_cpu_matches_single_neuron() {
    // input=[2], w1=[3], w2=[4]: out = 4 * gelu(3*2).
    let out = ffn_cpu(&[2.0], &[3.0], &[4.0], 1, 1, 1);
    assert!((out[0] - 4.0 * gelu(6.0)).abs() < 1e-6);
}

#[test]
fn ffn_tensors_are_deterministic() {
    assert_eq!(ffn_tensors(8, 16, 4, 7), ffn_tensors(8, 16, 4, 7));
}

#[test]
fn p64_project_cpu_matches_manual() {
    let record = p64_records(1, 5)[0];
    let words: &[u32; 16] = bytemuck::cast_ref(&record);
    let mut weights = vec![0.0f32; 16];
    weights[0] = 2.0;
    weights[5] = -1.0;
    let out = p64_project_cpu(&[record], &weights);
    assert_eq!(out, vec![2.0 * words[0] as f32 - words[5] as f32]);
}

#[test]
fn ternary_gemv_cpu_matches_hand_checked_2x4() {
    // A 2x4 case computed by hand. K=4 => 1 u32 word per row.
    // Row 0 codes [1,2,0,1] -> ternary [+1,-1,0,+1]:
    //   1<<0 | 2<<2 | 0<<4 | 1<<6 = 1 + 8 + 0 + 64 = 73.
    // Row 1 codes [2,1,1,3] -> ternary [-1,+1,+1,0] (3 -> 0.0):
    //   2<<0 | 1<<2 | 1<<4 | 3<<6 = 2 + 4 + 16 + 192 = 214.
    // x = [1,2,3,4], scale = [2.0, 10.0].
    //   out[0] = 2.0  * ( +1*1 -1*2 +0*3 +1*4 ) = 2.0  * 3 = 6.0
    //   out[1] = 10.0 * ( -1*1 +1*2 +1*3 +0*4 ) = 10.0 * 4 = 40.0
    let x = [1.0f32, 2.0, 3.0, 4.0];
    let w_packed = [73u32, 214u32];
    let scale = [2.0f32, 10.0];
    let out = ternary_gemv_cpu(&x, &w_packed, &scale, 2, 4);
    assert_eq!(out, vec![6.0, 40.0]);
}

#[test]
fn ternary_gemv_tensors_are_deterministic_and_decode_in_range() {
    // The generated tensors must be reproducible and the packed codes must only
    // ever decode to {0, +1, -1} (never the unused code 3), so GPU == CPU.
    let (x0, w0, s0) = ternary_gemv_tensors(7, 40, 123);
    let (x1, w1, s1) = ternary_gemv_tensors(7, 40, 123);
    assert_eq!((x0.clone(), w0.clone(), s0.clone()), (x1, w1, s1));
    assert_eq!(x0.len(), 40);
    assert_eq!(s0.len(), 7);
    assert_eq!(w0.len(), 7 * 40usize.div_ceil(TERNARY_CODES_PER_WORD));
    for word in &w0 {
        for lane in 0..TERNARY_CODES_PER_WORD {
            let code = (word >> (lane * 2)) & 3;
            assert_ne!(
                code, 3,
                "generator must never emit the unused ternary code 3"
            );
        }
    }
}

#[test]
fn ternary_gemv_cpu_zero_codes_yield_zero() {
    // All-zero packed words => every ternary weight is 0.0 => output all zero
    // regardless of x and scale.
    let x = topk_inputs(20, 9);
    let scale = [3.0f32, -2.0, 7.0];
    let w_packed = vec![0u32; 3 * 20usize.div_ceil(TERNARY_CODES_PER_WORD)];
    let out = ternary_gemv_cpu(&x, &w_packed, &scale, 3, 20);
    assert_eq!(out, vec![0.0, 0.0, 0.0]);
}

#[test]
fn gemm_cpu_matches_hand_checked_2x3_3x2() {
    // A (2×3) · B (3×2) = C (2×2), all computed by hand. Row-major.
    //   A = [[1, 2, 3],      B = [[ 7,  8],
    //        [4, 5, 6]]           [ 9, 10],
    //                             [11, 12]]
    //   C[0][0] = 1*7 + 2*9  + 3*11 =  7 + 18 + 33 =  58
    //   C[0][1] = 1*8 + 2*10 + 3*12 =  8 + 20 + 36 =  64
    //   C[1][0] = 4*7 + 5*9  + 6*11 = 28 + 45 + 66 = 139
    //   C[1][1] = 4*8 + 5*10 + 6*12 = 32 + 50 + 72 = 154
    let a = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
    let b = [7.0f32, 8.0, 9.0, 10.0, 11.0, 12.0];
    let c = gemm_cpu(&a, &b, 2, 3, 2);
    assert_eq!(c, vec![58.0, 64.0, 139.0, 154.0]);
    // matmul_cpu is the n×n special case of gemm_cpu; a 2×2 identity confirms
    // the generalization is consistent with the pre-existing reference.
    let id = [1.0f32, 0.0, 0.0, 1.0];
    let mat = [2.0f32, 3.0, 4.0, 5.0];
    assert_eq!(matmul_cpu(&id, &mat, 2), gemm_cpu(&id, &mat, 2, 2, 2));
    assert_eq!(matmul_cpu(&id, &mat, 2), vec![2.0, 3.0, 4.0, 5.0]);
}

#[test]
fn gemm_tensors_are_deterministic() {
    assert_eq!(gemm_tensors(8, 16, 4, 7), gemm_tensors(8, 16, 4, 7));
    let (a, b) = gemm_tensors(8, 16, 4, 7);
    assert_eq!(a.len(), 8 * 16);
    assert_eq!(b.len(), 16 * 4);
}

#[test]
fn gemv_cpu_matches_hand_checked_2x3() {
    // A (2×3) · x (3) = y (2), computed by hand. Row-major.
    //   A = [[1, 2, 3],      x = [1, 1, 1]
    //        [4, 5, 6]]
    //   y[0] = 1*1 + 2*1 + 3*1 =  6
    //   y[1] = 4*1 + 5*1 + 6*1 = 15
    let a = [1.0f32, 2.0, 3.0, 4.0, 5.0, 6.0];
    let x = [1.0f32, 1.0, 1.0];
    let y = gemv_cpu(&a, &x, 2, 3);
    assert_eq!(y, vec![6.0, 15.0]);
    // GEMV is the N-column special case of GEMM with N_gemm = 1; a 2×3 · 3×1
    // GEMM must produce the same vector, confirming the references are consistent.
    let x_col = gemm_cpu(&a, &x, 2, 3, 1);
    assert_eq!(x_col, y);
}

#[test]
fn gemv_tensors_are_deterministic() {
    assert_eq!(gemv_tensors(8, 16, 7), gemv_tensors(8, 16, 7));
    let (a, x) = gemv_tensors(8, 16, 7);
    assert_eq!(a.len(), 8 * 16);
    assert_eq!(x.len(), 16);
}

#[test]
fn dft_cpu_impulse_is_all_ones() {
    // A unit impulse at index 0 (x[0] = 1, rest 0) has a flat spectrum:
    // X[k] = sum_j x[j] e^{-2pi i kj/N} = x[0] = 1 for every k. So every
    // output bin is exactly (1, 0). Hand-verified, exact.
    let n = 8usize;
    let mut input = vec![0.0f32; 2 * n];
    input[0] = 1.0; // real impulse at j=0
    let out = dft_cpu(&input, n);
    assert_eq!(out.len(), 2 * n);
    for k in 0..n {
        assert!((out[2 * k] - 1.0).abs() < 1e-5, "bin {k} real should be 1");
        assert!(out[2 * k + 1].abs() < 1e-5, "bin {k} imag should be 0");
    }
}

#[test]
fn dft_cpu_dc_signal_concentrates_in_bin_zero() {
    // A constant real signal x[j] = 1 for all j has all its energy in bin 0:
    // X[0] = N, X[k>0] = 0. For N=4 input [1,0, 1,0, 1,0, 1,0]:
    //   X[0] = 1+1+1+1 = 4
    //   X[1] = 1 + e^{-i pi/2} + e^{-i pi} + e^{-i 3pi/2} = 1 - i - 1 + i = 0
    //   X[2], X[3] = 0 by the same cancellation.
    // Hand-verified -> expected interleaved [4,0, 0,0, 0,0, 0,0].
    let n = 4usize;
    let input = [1.0f32, 0.0, 1.0, 0.0, 1.0, 0.0, 1.0, 0.0];
    let out = dft_cpu(&input, n);
    assert!((out[0] - 4.0).abs() < 1e-5, "X[0] real should be 4");
    assert!(out[1].abs() < 1e-5, "X[0] imag should be 0");
    for k in 1..n {
        assert!(out[2 * k].abs() < 1e-5, "bin {k} real should be 0");
        assert!(out[2 * k + 1].abs() < 1e-5, "bin {k} imag should be 0");
    }
}

#[test]
fn fft_inputs_are_deterministic_interleaved() {
    let a = fft_inputs(16, 7);
    let b = fft_inputs(16, 7);
    assert_eq!(a, b);
    assert_eq!(a.len(), 2 * 16, "interleaved complex => 2*n f32");
}

#[test]
#[serial_test::serial(gpu)]
fn generated_fft_matches_oracle_on_real_gpu() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    // N=256-point forward FFT, one workgroup of 256 threads, checked against
    // the O(N²) DFT reference (same forward sign convention).
    let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
    let schedule = Schedule {
        workgroup_size: 256,
        items_per_invocation: 1,
        vector_width: 1,
        ..Default::default()
    };
    let (_, evaluation) = evaluate_fft(&mut context, schedule, 256, 2, 5).expect("fft evaluation");
    assert!(
        evaluation.oracle.passed(),
        "fft GPU/oracle mismatch: {:?}",
        evaluation.oracle
    );
}

#[test]
#[serial_test::serial(gpu)]
fn generated_ternary_gemv_matches_oracle_on_real_gpu() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
    let schedule = Schedule {
        workgroup_size: 64,
        items_per_invocation: 1,
        vector_width: 1,
        ..Default::default()
    };
    let (_, evaluation) = evaluate_ternary_gemv(&mut context, schedule, 256, 256, 2, 5)
        .expect("ternary-gemv evaluation");
    assert!(
        evaluation.oracle.passed(),
        "ternary-gemv GPU/oracle mismatch: {:?}",
        evaluation.oracle
    );
}

#[test]
#[serial_test::serial(gpu)]
fn generated_gemm_matches_oracle_on_real_gpu() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
    let schedule = Schedule {
        workgroup_size: 64,
        items_per_invocation: 1,
        vector_width: 1,
        ..Default::default()
    };
    let (_, evaluation) =
        evaluate_gemm(&mut context, schedule, 64, 64, 64, 2, 5).expect("gemm evaluation");
    assert!(
        evaluation.oracle.passed(),
        "gemm GPU/oracle mismatch: {:?}",
        evaluation.oracle
    );
}

#[test]
#[serial_test::serial(gpu)]
fn generated_gemv_matches_oracle_on_real_gpu() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
    let schedule = Schedule {
        workgroup_size: 64,
        items_per_invocation: 1,
        vector_width: 1,
        ..Default::default()
    };
    let (_, evaluation) =
        evaluate_gemv(&mut context, schedule, 256, 256, 2, 5).expect("gemv evaluation");
    assert!(
        evaluation.oracle.passed(),
        "gemv GPU/oracle mismatch: {:?}",
        evaluation.oracle
    );
}

#[test]
#[serial_test::serial(gpu)]
fn generated_p64_matches_oracle_on_real_gpu() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
    let schedule = Schedule {
        workgroup_size: 64,
        items_per_invocation: 1,
        vector_width: 1,
        ..Default::default()
    };
    let (_, evaluation) = evaluate_p64(&mut context, schedule, 1000, 2, 5).expect("p64 evaluation");
    assert!(
        evaluation.oracle.passed(),
        "p64 GPU/oracle mismatch: {:?}",
        evaluation.oracle
    );
}

#[test]
#[serial_test::serial(gpu)]
fn generated_ffn_matches_oracle_on_real_gpu() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut context = WgpuComputeContext::new(4 * 1024 * 1024).expect("adapter");
    let schedule = Schedule {
        workgroup_size: 64,
        items_per_invocation: 1,
        vector_width: 1,
        ..Default::default()
    };
    let (_, evaluation) =
        evaluate_ffn(&mut context, schedule, 64, 128, 256, 2, 5).expect("ffn evaluation");
    assert!(
        evaluation.oracle.passed(),
        "fused-ffn GPU/oracle mismatch: {:?}",
        evaluation.oracle
    );
}

#[test]
#[serial_test::serial(gpu)]
fn generated_topk_matches_oracle_on_real_gpu() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut context = WgpuComputeContext::new(1024 * 1024).expect("adapter");
    let schedule = Schedule {
        workgroup_size: 64,
        items_per_invocation: 1,
        vector_width: 1,
        ..Default::default()
    };
    let (_, evaluation) =
        evaluate_topk(&mut context, schedule, 64 * 10, 4, 2, 5).expect("topk evaluation");
    assert!(
        evaluation.oracle.passed(),
        "top-k GPU/oracle mismatch: {:?}",
        evaluation.oracle
    );
}

#[cfg(feature = "cuda")]
#[test]
#[serial_test::serial(gpu)]
fn affine_oracle_matches_across_cuda_backend() {
    if !crate::wgsl_forge::test_cuda_available() {
        return;
    }
    let report = evaluate_affine_cuda(4099).expect("cuda affine evaluation");
    assert!(
        report.passed(),
        "CUDA affine GPU/oracle mismatch: {report:?}"
    );
}

#[cfg(feature = "cuda")]
#[test]
#[serial_test::serial(gpu)]
fn ffn_oracle_matches_across_cuda_backend() {
    if !crate::wgsl_forge::test_cuda_available() {
        return;
    }
    let report = evaluate_ffn_cuda(64, 128, 256).expect("cuda ffn evaluation");
    assert!(report.passed(), "CUDA fused-ffn mismatch: {report:?}");
}

#[cfg(feature = "cuda")]
#[test]
#[serial_test::serial(gpu)]
fn topk_oracle_matches_across_cuda_backend() {
    if !crate::wgsl_forge::test_cuda_available() {
        return;
    }
    let report = evaluate_topk_cuda(64 * 10, 4).expect("cuda topk evaluation");
    assert!(report.passed(), "CUDA top-k mismatch: {report:?}");
}

#[cfg(feature = "cuda")]
#[test]
#[serial_test::serial(gpu)]
fn wmma_matmul_certifies_on_cuda_tensor_cores() {
    if !crate::wgsl_forge::test_cuda_available() {
        return;
    }
    // The genuine tensor-core path: f16-input WMMA GEMM through NVRTC, the
    // reduced-precision config wgpu 29's coopmat cannot run.
    let report = evaluate_matmul_tc_cuda().expect("cuda wmma evaluation");
    assert!(report.passed(), "CUDA WMMA C=A*B mismatch: {report:?}");
}

#[test]
#[serial_test::serial(gpu)]
fn coopmat_loadstore_roundtrips_on_real_gpu() {
    // Diagnostic: coopLoadT/coopStoreT round-trip correctly on the adapter (c == a).
    if !crate::gpu_context::experimental_features_allowed() {
        return;
    }
    let Ok(mut context) = WgpuComputeContext::new_for_coopmat(1024 * 1024) else {
        return;
    };
    if !context.constraints.supports_coopmat {
        return;
    }
    let report = evaluate_coopmat_loadstore(&mut context).expect("coopmat round-trip");
    assert!(
        report.passed(),
        "coopmat load/store round-trip mismatch: {report:?}"
    );
}

// NOTE: there is intentionally no GPU test asserting the *WGSL* coopmat
// `coopMultiplyAdd` (`evaluate_matmul_tc`) computes C = A * B. The emitter
// produces the correct, naga-validated all-f32 8x8x8 kernel
// (`cooperative_matrix_tile_validates`, above), but wgpu/naga 29.0.3's
// experimental cooperative-matrix *execution* path returns all-zeros from the
// multiply (the load/store round-trip works — see below), and no published
// wgpu release fixes it (29.0.3 is the newest on crates.io; the fix is on
// unreleased git main). The genuine tensor-core multiply is proven instead via
// CUDA WMMA (`wmma_matmul_certifies_on_cuda_tensor_cores`). When wgpu ships the
// coopmat execution fix, add the `evaluate_matmul_tc` assertion back here.

#[test]
#[serial_test::serial(gpu)]
fn generated_affine_certifies_on_real_gpu() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut context = WgpuComputeContext::new(1024 * 1024).expect("adapter");
    let manifest = certify_builtin(
        &mut context,
        BuiltinKernel::AffineF32,
        Schedule {
            workgroup_size: 64,
            items_per_invocation: 2,
            vector_width: 4,
            ..Default::default()
        },
        4_099,
        2,
        5,
    )
    .expect("certification");
    assert_eq!(manifest.validation_level, ValidationLevel::Certified);
    assert!(manifest.oracle.unwrap().passed());
}
