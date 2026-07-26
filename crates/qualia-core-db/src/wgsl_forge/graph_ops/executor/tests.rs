use super::*;
use crate::wgsl_forge::ir::graph::{ComputeGraph, DType, OpNode, Shape, TensorRef};
use crate::wgsl_forge::Schedule;

/// The composed CPU oracle for softmax is a valid probability distribution
/// (non-negative, sums to 1) and matches a direct reference.
#[test]
fn softmax_cpu_oracle_is_a_distribution() {
    let x: Vec<f32> = vec![1.0, 2.0, 3.0, 0.5, -1.0, 4.0, 2.5, 0.0];
    let g = softmax_graph(x.len() as u32).unwrap();
    let out = execute_graph_cpu(&g, &[x.clone()]).unwrap();
    let sum: f32 = out.iter().sum();
    assert!((sum - 1.0).abs() < 1e-5, "softmax must sum to 1, got {sum}");
    assert!(out.iter().all(|&p| p >= 0.0));
    // Direct reference.
    let m = x.iter().cloned().fold(f32::MIN, f32::max);
    let exps: Vec<f32> = x.iter().map(|&v| (v - m).exp()).collect();
    let z: f32 = exps.iter().sum();
    for (o, e) in out.iter().zip(exps.iter()) {
        assert!((o - e / z).abs() < 1e-5);
    }
}

/// The composed CPU oracle for RMSNorm scales `x` by `1/rms(x)`.
#[test]
fn rmsnorm_cpu_oracle_matches_reference() {
    let x: Vec<f32> = vec![3.0, 4.0, 0.0, 0.0]; // rms = sqrt((9+16)/4) = 2.5
    let g = rmsnorm_graph(x.len() as u32).unwrap();
    let out = execute_graph_cpu(&g, &[x.clone()]).unwrap();
    let ms: f32 = x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32;
    let inv = ms.sqrt().recip();
    for (o, xi) in out.iter().zip(x.iter()) {
        assert!((o - xi * inv).abs() < 1e-5, "{o} vs {}", xi * inv);
    }
}

/// The composed CPU oracle for the SwiGLU-FFN block matches a hand-written reference.
#[test]
fn swiglu_ffn_cpu_oracle_matches_reference() {
    let (seq, dim, ffn) = (2usize, 3usize, 4usize);
    let x: Vec<f32> = (0..seq * dim).map(|i| (i as f32) * 0.1 - 0.3).collect();
    let wg: Vec<f32> = (0..dim * ffn).map(|i| (i as f32) * 0.05 - 0.2).collect();
    let wu: Vec<f32> = (0..dim * ffn).map(|i| (i as f32) * 0.03 - 0.1).collect();
    let wd: Vec<f32> = (0..ffn * dim).map(|i| (i as f32) * 0.02 - 0.1).collect();
    let g = swiglu_ffn_graph(seq as u32, dim as u32, ffn as u32).unwrap();
    let out = execute_graph_cpu(&g, &[x.clone(), wg.clone(), wu.clone(), wd.clone()]).unwrap();

    // Reference: gate = x·Wg, up = x·Wu, h = silu(gate)·up, y = h·Wd.
    let mm = |a: &[f32], b: &[f32], m: usize, k: usize, n: usize| {
        let mut c = vec![0.0f32; m * n];
        for i in 0..m {
            for j in 0..n {
                let mut acc = 0.0;
                for kk in 0..k {
                    acc += a[i * k + kk] * b[kk * n + j];
                }
                c[i * n + j] = acc;
            }
        }
        c
    };
    let gate = mm(&x, &wg, seq, dim, ffn);
    let up = mm(&x, &wu, seq, dim, ffn);
    let h: Vec<f32> = gate
        .iter()
        .zip(up.iter())
        .map(|(&g, &u)| (g / (1.0 + (-g).exp())) * u)
        .collect();
    let y = mm(&h, &wd, seq, ffn, dim);
    assert_eq!(out.len(), y.len());
    for (o, r) in out.iter().zip(y.iter()) {
        assert!((o - r).abs() < 1e-5, "{o} vs {r}");
    }
}

/// GPU certify: the full multi-node graph executed on the A2000 (intermediates kept
/// device-side) must match the composed CPU oracle within f32 tolerance — for softmax,
/// RMSNorm, and the SwiGLU-FFN block. Run by the orchestrator.
#[test]
#[serial_test::serial(gpu)]
fn execute_graph_gpu_matches_cpu_oracle() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    // softmax (1024-wide → exercises grid-stride reduce + broadcast + elementwise chain)
    {
        let n = 1024usize;
        let x: Vec<f32> = (0..n).map(|i| ((i * 13 % 97) as f32) * 0.1 - 5.0).collect();
        let g = softmax_graph(n as u32).unwrap();
        let gpu = execute_graph(&g, &[x.clone()]).expect("softmax gpu");
        let cpu = execute_graph_cpu(&g, &[x]).unwrap();
        for (a, b) in gpu.iter().zip(cpu.iter()) {
            assert!((a - b).abs() <= 1e-4, "softmax: {a} vs {b}");
        }
        assert!((gpu.iter().sum::<f32>() - 1.0).abs() < 1e-3);
    }
    // RMSNorm
    {
        let n = 768usize;
        let x: Vec<f32> = (0..n).map(|i| ((i * 7 % 31) as f32) * 0.2 - 3.0).collect();
        let g = rmsnorm_graph(n as u32).unwrap();
        let gpu = execute_graph(&g, &[x.clone()]).expect("rmsnorm gpu");
        let cpu = execute_graph_cpu(&g, &[x]).unwrap();
        for (a, b) in gpu.iter().zip(cpu.iter()) {
            assert!(
                (a - b).abs() <= 1e-3 * b.abs().max(1.0),
                "rmsnorm: {a} vs {b}"
            );
        }
    }
    // SwiGLU-FFN block (the LLM workhorse) — MatMul + Elementwise multi-node DAG.
    {
        let (seq, dim, ffn) = (8u32, 64u32, 128u32);
        let x: Vec<f32> = (0..seq * dim)
            .map(|i| ((i % 17) as f32) * 0.05 - 0.4)
            .collect();
        let wg: Vec<f32> = (0..dim * ffn)
            .map(|i| ((i % 13) as f32) * 0.02 - 0.12)
            .collect();
        let wu: Vec<f32> = (0..dim * ffn)
            .map(|i| ((i % 11) as f32) * 0.015 - 0.07)
            .collect();
        let wd: Vec<f32> = (0..ffn * dim)
            .map(|i| ((i % 7) as f32) * 0.01 - 0.03)
            .collect();
        let g = swiglu_ffn_graph(seq, dim, ffn).unwrap();
        let ext = vec![x, wg, wu, wd];
        let gpu = execute_graph(&g, &ext).expect("ffn gpu");
        let cpu = execute_graph_cpu(&g, &ext).unwrap();
        assert_eq!(gpu.len(), cpu.len());
        for (a, b) in gpu.iter().zip(cpu.iter()) {
            assert!((a - b).abs() <= 1e-2 * b.abs().max(1.0), "ffn: {a} vs {b}");
        }
    }
}

/// Device-unification cert (LLM-on-forge Phase 1a): the forge running on the **process-wide
/// shared GPU device** ([`crate::gpu_context::shared_gpu`], the device that owns the LLM
/// weights + KV cache) produces results identical (within f32 tol) to the composed CPU oracle —
/// for a full **faithful decode block** (RMSNorm·eps → scaled attention → residual → SwiGLU-FFN
/// → residual). Proves `WgpuComputeContext::from_device` + `ForgeGraphExecutor::on_shared_gpu`
/// run real multi-node graphs correctly on the shared device, not a second one.
#[test]
#[serial_test::serial(gpu)]
fn shared_device_executor_matches_cpu_oracle() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let mut exec =
        ForgeGraphExecutor::on_shared_gpu().expect("forge executor on shared_gpu device");

    // The forge must report the SAME adapter as the process-wide shared device — i.e. it did
    // not silently spin up a second adapter/device.
    let shared = crate::gpu_context::shared_gpu();
    let forge_adapter = &exec.context().adapter;
    assert_eq!(
        forge_adapter.vendor, shared.adapter_caps.vendor,
        "forge ran on a different vendor than shared_gpu"
    );
    assert_eq!(
        forge_adapter.device, shared.adapter_caps.device,
        "forge ran on a different device than shared_gpu"
    );

    // softmax (1024-wide) on the shared device matches the oracle and is a distribution.
    {
        let n = 1024usize;
        let x: Vec<f32> = (0..n).map(|i| ((i * 13 % 97) as f32) * 0.1 - 5.0).collect();
        let g = softmax_graph(n as u32).unwrap();
        let gpu = exec.run(&g, &[x.clone()]).expect("softmax shared-gpu");
        let cpu = execute_graph_cpu(&g, &[x]).unwrap();
        for (a, b) in gpu.iter().zip(cpu.iter()) {
            assert!((a - b).abs() <= 1e-4, "softmax(shared): {a} vs {b}");
        }
        assert!((gpu.iter().sum::<f32>() - 1.0).abs() < 1e-3);
    }

    // Full faithful decode block on the SAME held executor (the decode-step usage pattern):
    // externals = [x, Kᵀ, V, Wg, Wu, Wd, inv_scale, eps].
    {
        let (d, kv, ffn) = (64u32, 32u32, 128u32);
        let inv_scale = 1.0f32 / (d as f32).sqrt();
        let eps = 1e-5f32;
        let x: Vec<f32> = (0..d).map(|i| ((i % 17) as f32) * 0.05 - 0.4).collect();
        let kt: Vec<f32> = (0..d * kv)
            .map(|i| ((i * 5 % 7) as f32) * 0.03 - 0.09)
            .collect();
        let v: Vec<f32> = (0..kv * d)
            .map(|i| ((i * 3 % 5) as f32) * 0.04 - 0.08)
            .collect();
        let wg: Vec<f32> = (0..d * ffn)
            .map(|i| ((i % 13) as f32) * 0.02 - 0.12)
            .collect();
        let wu: Vec<f32> = (0..d * ffn)
            .map(|i| ((i % 11) as f32) * 0.015 - 0.07)
            .collect();
        let wd: Vec<f32> = (0..ffn * d)
            .map(|i| ((i % 7) as f32) * 0.01 - 0.03)
            .collect();
        let ext = vec![x, kt, v, wg, wu, wd, vec![inv_scale], vec![eps]];
        let g = decode_block_graph(d, kv, ffn).unwrap();
        let gpu = exec.run(&g, &ext).expect("decode-block shared-gpu");
        let cpu = execute_graph_cpu(&g, &ext).unwrap();
        assert_eq!(gpu.len(), cpu.len());
        for (a, b) in gpu.iter().zip(cpu.iter()) {
            assert!(
                (a - b).abs() <= 1e-2 * b.abs().max(1.0),
                "decode-block(shared): {a} vs {b}"
            );
        }
    }
}

/// Weight-residency cert + perf (LLM-on-forge Phase 1b): a decode block's FFN matrices
/// (Wg, Wu, Wd) are uploaded ONCE via `load_weights`, then `run_resident` is called repeatedly
/// with only the activations. Correctness: the resident path matches both the all-upload `run`
/// path **exactly** (same kernels, same bytes) and the composed CPU oracle, across multiple
/// calls (proving the resident weights survive the per-call transient-ring reset). Perf: prints
/// ms/call for resident vs all-upload and the per-call weight bytes no longer re-uploaded.
#[test]
#[serial_test::serial(gpu)]
fn resident_weights_decode_block() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    use std::time::Instant;
    let (d, kv, ffn) = (576u32, 128u32, 1536u32);
    let inv_scale = 1.0f32 / (d as f32).sqrt();
    let eps = 1e-5f32;
    let x: Vec<f32> = (0..d).map(|i| ((i % 17) as f32) * 0.05 - 0.4).collect();
    let kt: Vec<f32> = (0..d * kv)
        .map(|i| ((i * 5 % 7) as f32) * 0.03 - 0.09)
        .collect();
    let v: Vec<f32> = (0..kv * d)
        .map(|i| ((i * 3 % 5) as f32) * 0.04 - 0.08)
        .collect();
    let wg: Vec<f32> = (0..d * ffn)
        .map(|i| ((i % 13) as f32) * 0.02 - 0.12)
        .collect();
    let wu: Vec<f32> = (0..d * ffn)
        .map(|i| ((i % 11) as f32) * 0.015 - 0.07)
        .collect();
    let wd: Vec<f32> = (0..ffn * d)
        .map(|i| ((i % 7) as f32) * 0.01 - 0.03)
        .collect();
    let g = decode_block_graph(d, kv, ffn).unwrap();

    // All-upload externals (the run() baseline) — every tensor provided.
    let full = vec![
        x.clone(),
        kt.clone(),
        v.clone(),
        wg.clone(),
        wu.clone(),
        wd.clone(),
        vec![inv_scale],
        vec![eps],
    ];
    // Resident-activation externals: indices 3,4,5 (Wg,Wu,Wd) are resident → empty placeholders.
    let acts = vec![
        x.clone(),
        kt.clone(),
        v.clone(),
        vec![],
        vec![],
        vec![],
        vec![inv_scale],
        vec![eps],
    ];

    let mut exec = ForgeGraphExecutor::new().expect("forge executor");
    // Upload the FFN weight matrices once into the persistent region.
    let resident = exec
        .load_weights(&[(3, wg.clone()), (4, wu.clone()), (5, wd.clone())])
        .expect("load_weights");
    assert_eq!(resident.len(), 3);
    let resident_bytes = exec.context().resident_weight_bytes();

    let cpu = execute_graph_cpu(&g, &full).unwrap();
    let upload_ref = exec.run(&g, &full).expect("run all-upload");

    // Resident path matches the all-upload path EXACTLY (identical kernels + bytes), and the
    // CPU oracle, on every one of several calls (resident weights persist across runs).
    for call in 0..3 {
        let res = exec
            .run_resident(&g, &acts, &resident)
            .expect("run_resident");
        assert_eq!(res.len(), upload_ref.len());
        for (a, b) in res.iter().zip(upload_ref.iter()) {
            assert_eq!(a, b, "resident != all-upload on call {call}");
        }
        for (a, b) in res.iter().zip(cpu.iter()) {
            assert!(
                (a - b).abs() <= 1e-2 * b.abs().max(1.0),
                "resident != oracle: {a} vs {b}"
            );
        }
    }

    // Perf: time resident vs all-upload (after warmup).
    let iters = 50;
    for _ in 0..5 {
        let _ = exec.run(&g, &full).unwrap();
        let _ = exec.run_resident(&g, &acts, &resident).unwrap();
    }
    let t0 = Instant::now();
    for _ in 0..iters {
        let _ = exec.run(&g, &full).unwrap();
    }
    let upload_ms = t0.elapsed().as_secs_f64() * 1e3 / iters as f64;
    let t1 = Instant::now();
    for _ in 0..iters {
        let _ = exec.run_resident(&g, &acts, &resident).unwrap();
    }
    let resident_ms = t1.elapsed().as_secs_f64() * 1e3 / iters as f64;
    let saved_bytes = (wg.len() + wu.len() + wd.len()) * std::mem::size_of::<f32>();
    println!(
        "[weight residency] decode block d={d} kv={kv} ffn={ffn} | resident {resident_ms:.3} ms/call vs all-upload {upload_ms:.3} ms/call | weights {resident_bytes} B resident, {saved_bytes} B/call NOT re-uploaded. Correctness: resident==all-upload (exact) + matches CPU oracle across 3 calls."
    );
}

// ── P4b: attention + GatherDequant + decode block ────────────────────────────────

/// Row-major matmul helper for the test references.
fn ref_mm(a: &[f32], b: &[f32], m: usize, k: usize, n: usize) -> Vec<f32> {
    let mut c = vec![0.0f32; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut acc = 0.0f32;
            for kk in 0..k {
                acc += a[i * k + kk] * b[kk * n + j];
            }
            c[i * n + j] = acc;
        }
    }
    c
}

fn ref_softmax(s: &[f32]) -> Vec<f32> {
    let m = s.iter().cloned().fold(f32::MIN, f32::max);
    let e: Vec<f32> = s.iter().map(|&v| (v - m).exp()).collect();
    let z: f32 = e.iter().sum();
    e.iter().map(|&x| x / z).collect()
}

fn ref_rmsnorm(x: &[f32], eps: f32) -> Vec<f32> {
    let ms = x.iter().map(|v| v * v).sum::<f32>() / x.len() as f32;
    let inv = (ms + eps).sqrt().recip();
    x.iter().map(|&v| v * inv).collect()
}

/// The decode-step **scaled** attention graph's composed CPU oracle matches an independent
/// `softmax((q·Kᵀ)/√d)·V` reference — with the 1/√d score scaling.
#[test]
fn attention_cpu_oracle_matches_reference() {
    let (d, kv) = (4usize, 6usize);
    let inv_scale = 1.0f32 / (d as f32).sqrt();
    let q: Vec<f32> = (0..d).map(|i| (i as f32) * 0.2 - 0.3).collect();
    let kt: Vec<f32> = (0..d * kv)
        .map(|i| ((i * 5 % 7) as f32) * 0.1 - 0.25)
        .collect();
    let v: Vec<f32> = (0..kv * d)
        .map(|i| ((i * 3 % 5) as f32) * 0.15 - 0.2)
        .collect();
    let g = attention_graph(d as u32, kv as u32).unwrap();
    let out = execute_graph_cpu(&g, &[q.clone(), kt.clone(), v.clone(), vec![inv_scale]]).unwrap();
    // Reference: scores = (q·Kᵀ)/√d [1,kv]; probs = softmax(scores); out = probs·V [1,d].
    let scores: Vec<f32> = ref_mm(&q, &kt, 1, d, kv)
        .iter()
        .map(|s| s * inv_scale)
        .collect();
    let probs = ref_softmax(&scores);
    let want = ref_mm(&probs, &v, 1, kv, d);
    assert_eq!(out.len(), d);
    for (o, w) in out.iter().zip(&want) {
        assert!((o - w).abs() < 1e-5, "{o} vs {w}");
    }
}

// ── Real multi-head (GQA) decode layer ───────────────────────────────────────────

/// Deterministic externals for [`decode_layer_graph`] (head-major K/V cache layout).
fn decode_layer_externals(
    n_heads: u32,
    n_kv_heads: u32,
    head_dim: u32,
    seq: u32,
    ffn: u32,
) -> Vec<Vec<f32>> {
    let d = (n_heads * head_dim) as usize;
    let (hd, seqn, ffnn, n_kv) = (
        head_dim as usize,
        seq as usize,
        ffn as usize,
        n_kv_heads as usize,
    );
    let gen = |len: usize, salt: usize| -> Vec<f32> {
        (0..len)
            .map(|i| (((i * 7 + salt * 13) % 23) as f32) * 0.05 - 0.55)
            .collect()
    };
    let inv_scale = 1.0f32 / (head_dim as f32).sqrt();
    vec![
        gen(d, 1),                // x  [1,d]
        gen(n_kv * hd * seqn, 2), // Kt [n_kv, head_dim, seq]
        gen(n_kv * seqn * hd, 3), // V  [n_kv, seq, head_dim]
        gen(d * d, 4),            // Wq [d,d]
        gen(d * d, 5),            // Wo [d,d]
        gen(d * ffnn, 6),         // Wg [d,ffn]
        gen(d * ffnn, 7),         // Wu [d,ffn]
        gen(ffnn * d, 8),         // Wd [ffn,d]
        gen(d, 9),                // attn_norm [d]
        gen(d, 10),               // ffn_norm  [d]
        vec![inv_scale],          // inv_scale
        vec![1e-5],               // eps
    ]
}

/// Independent hand-written reference for the real decode layer (mirrors the math, not the
/// graph code): RMSNorm → Q-proj → RoPE(q) → per-head GQA `softmax(q_h·Kᵀ_h/√hd)·V_h` →
/// `o_h·Wo_h` summed → +x → RMSNorm → SwiGLU → +.
#[allow(clippy::too_many_arguments)]
fn ref_decode_layer(
    ext: &[Vec<f32>],
    n_heads: u32,
    n_kv_heads: u32,
    head_dim: u32,
    seq: u32,
    ffn: u32,
    pos: u32,
    mode: u32,
    base: f32,
) -> Vec<f32> {
    use crate::wgsl_forge::graph_ops::stencil::{rope_cpu, RopeConfig, RopeMode};
    let d = (n_heads * head_dim) as usize;
    let (hd, seqn, ffnn) = (head_dim as usize, seq as usize, ffn as usize);
    let group = (n_heads / n_kv_heads) as usize;
    let (x, kt, v, wq, wo, wg, wu, wd) = (
        &ext[0], &ext[1], &ext[2], &ext[3], &ext[4], &ext[5], &ext[6], &ext[7],
    );
    let attn_norm = &ext[8];
    let ffn_norm = &ext[9];
    let inv_scale = ext[10][0];
    let eps = ext[11][0];
    let n1: Vec<f32> = ref_rmsnorm(x, eps)
        .iter()
        .zip(attn_norm)
        .map(|(a, w)| a * w)
        .collect();
    let q = ref_mm(&n1, wq, 1, d, d);
    let rmode = if mode == 0 {
        RopeMode::Interleaved
    } else {
        RopeMode::Neox
    };
    let q = rope_cpu(
        &q,
        &RopeConfig {
            head_dim,
            pos,
            mode: rmode,
            theta_base: base,
        },
    )
    .unwrap();
    let mut attn = vec![0.0f32; d];
    for h in 0..n_heads as usize {
        let kh = h / group;
        let q_h = &q[h * hd..(h + 1) * hd];
        let kt_h = &kt[kh * hd * seqn..(kh + 1) * hd * seqn];
        let v_h = &v[kh * seqn * hd..(kh + 1) * seqn * hd];
        let scores: Vec<f32> = ref_mm(q_h, kt_h, 1, hd, seqn)
            .iter()
            .map(|s| s * inv_scale)
            .collect();
        let probs = ref_softmax(&scores);
        let o_h = ref_mm(&probs, v_h, 1, seqn, hd);
        let wo_h = &wo[h * hd * d..(h + 1) * hd * d];
        let part = ref_mm(&o_h, wo_h, 1, hd, d);
        for (a, p) in attn.iter_mut().zip(&part) {
            *a += *p;
        }
    }
    let res1: Vec<f32> = x.iter().zip(&attn).map(|(a, b)| a + b).collect();
    let n2: Vec<f32> = ref_rmsnorm(&res1, eps)
        .iter()
        .zip(ffn_norm)
        .map(|(a, w)| a * w)
        .collect();
    let gate = ref_mm(&n2, wg, 1, d, ffnn);
    let up = ref_mm(&n2, wu, 1, d, ffnn);
    let hsl: Vec<f32> = gate
        .iter()
        .zip(&up)
        .map(|(g, u)| (g / (1.0 + (-g).exp())) * u)
        .collect();
    let ffn_out = ref_mm(&hsl, wd, 1, ffnn, d);
    res1.iter().zip(&ffn_out).map(|(a, b)| a + b).collect()
}

/// The real multi-head (GQA) decode-layer graph's **composed CPU oracle** matches an
/// INDEPENDENT hand-written reference — proving the graph computes a real decode layer (not
/// merely that the GPU matches its own oracle). Covers both RoPE conventions + GQA (2:1).
#[test]
fn decode_layer_cpu_oracle_matches_reference() {
    let (n_heads, n_kv_heads, head_dim, seq, ffn) = (2u32, 1u32, 8u32, 4u32, 16u32);
    let (pos, base) = (3u32, 10000.0f32);
    let ext = decode_layer_externals(n_heads, n_kv_heads, head_dim, seq, ffn);
    for mode in [0u32, 1u32] {
        let g =
            decode_layer_graph(n_heads, n_kv_heads, head_dim, seq, ffn, pos, mode, base).unwrap();
        let cpu = execute_graph_cpu(&g, &ext).unwrap();
        let want = ref_decode_layer(
            &ext, n_heads, n_kv_heads, head_dim, seq, ffn, pos, mode, base,
        );
        assert_eq!(cpu.len(), (n_heads * head_dim) as usize);
        for (a, b) in cpu.iter().zip(&want) {
            assert!(
                (a - b).abs() <= 1e-4 * b.abs().max(1.0),
                "decode-layer oracle vs ref (mode {mode}): {a} vs {b}"
            );
        }
    }
}

/// GPU certify: the real multi-head decode layer on the A2000 matches its composed CPU oracle,
/// for both RoPE conventions, at a realistic shape (4 heads, 2 kv-heads = GQA 2:1, head_dim 16).
#[test]
#[serial_test::serial(gpu)]
fn decode_layer_gpu_matches_cpu_oracle() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let (n_heads, n_kv_heads, head_dim, seq, ffn) = (4u32, 2u32, 16u32, 8u32, 32u32);
    let (pos, base) = (5u32, 10000.0f32);
    let ext = decode_layer_externals(n_heads, n_kv_heads, head_dim, seq, ffn);
    for mode in [0u32, 1u32] {
        let g =
            decode_layer_graph(n_heads, n_kv_heads, head_dim, seq, ffn, pos, mode, base).unwrap();
        let gpu = execute_graph(&g, &ext).expect("decode-layer gpu");
        let cpu = execute_graph_cpu(&g, &ext).unwrap();
        assert_eq!(gpu.len(), (n_heads * head_dim) as usize);
        for (a, b) in gpu.iter().zip(&cpu) {
            assert!(
                (a - b).abs() <= 1e-2 * b.abs().max(1.0),
                "decode-layer gpu vs oracle (mode {mode}): {a} vs {b}"
            );
        }
    }
}

// ── MatMul.trans_b (native [out,in] weight layout) ───────────────────────────────

fn trans_b_graph(m: u32, n: u32, k: u32) -> ComputeGraph {
    let mut g = ComputeGraph::new();
    let s = Schedule::default();
    let a = TensorRef::input(0, Shape::new(&[m, k]), DType::F32);
    let b = TensorRef::input(1, Shape::new(&[n, k]), DType::F32);
    let out = g
        .push(
            OpNode::MatMul {
                m,
                n,
                k,
                tc: false,
                trans_b: true,
            },
            &[a, b],
            Shape::new(&[m, n]),
            DType::F32,
            s,
        )
        .unwrap();
    g.mark_output(out);
    g
}

/// `MatMul.trans_b` CPU oracle (B bound `[n,k]`) matches an independent `A·Bᵀ` reference —
/// proving the previously-silently-dropped `trans_b` flag now computes.
#[test]
fn matmul_trans_b_cpu_oracle_matches_reference() {
    let (m, n, k) = (2usize, 3usize, 4usize);
    let a: Vec<f32> = (0..m * k).map(|i| (i as f32) * 0.2 - 0.5).collect();
    let b_nk: Vec<f32> = (0..n * k).map(|i| (i as f32) * 0.1 - 0.2).collect();
    let cpu = execute_graph_cpu(
        &trans_b_graph(m as u32, n as u32, k as u32),
        &[a.clone(), b_nk.clone()],
    )
    .unwrap();
    let mut want = vec![0.0f32; m * n];
    for i in 0..m {
        for j in 0..n {
            let mut acc = 0.0f32;
            for kk in 0..k {
                acc += a[i * k + kk] * b_nk[j * k + kk];
            }
            want[i * n + j] = acc;
        }
    }
    for (c, w) in cpu.iter().zip(&want) {
        assert!((c - w).abs() < 1e-5, "trans_b cpu {c} vs ref {w}");
    }
}

/// GPU certify `trans_b` two ways on the A2000: vs the composed CPU oracle, AND vs the **plain
/// GEMM run on an explicitly-transposed B** (a different kernel path) — the gold cross-check.
#[test]
#[serial_test::serial(gpu)]
fn matmul_trans_b_gpu_matches_plain_on_transposed() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let (m, n, k) = (3usize, 5usize, 4usize);
    let a: Vec<f32> = (0..m * k).map(|i| (i as f32) * 0.1 - 0.3).collect();
    let b_nk: Vec<f32> = (0..n * k)
        .map(|i| ((i * 3 % 7) as f32) * 0.05 - 0.15)
        .collect();
    let g = trans_b_graph(m as u32, n as u32, k as u32);
    let gpu = execute_graph(&g, &[a.clone(), b_nk.clone()]).expect("trans_b gpu");
    let cpu = execute_graph_cpu(&g, &[a.clone(), b_nk.clone()]).unwrap();
    for (gp, cp) in gpu.iter().zip(&cpu) {
        assert!(
            (gp - cp).abs() <= 1e-4 * cp.abs().max(1.0),
            "trans_b gpu {gp} vs cpu {cp}"
        );
    }
    // Cross-check: plain GEMM on B explicitly transposed to [k,n] must match.
    let mut b_kn = vec![0.0f32; k * n];
    for j in 0..n {
        for kk in 0..k {
            b_kn[kk * n + j] = b_nk[j * k + kk];
        }
    }
    let mut g2 = ComputeGraph::new();
    let s = Schedule::default();
    let a2 = TensorRef::input(0, Shape::new(&[m as u32, k as u32]), DType::F32);
    let b2 = TensorRef::input(1, Shape::new(&[k as u32, n as u32]), DType::F32);
    let out2 = g2
        .push(
            OpNode::MatMul {
                m: m as u32,
                n: n as u32,
                k: k as u32,
                tc: false,
                trans_b: false,
            },
            &[a2, b2],
            Shape::new(&[m as u32, n as u32]),
            DType::F32,
            s,
        )
        .unwrap();
    g2.mark_output(out2);
    let plain = execute_graph(&g2, &[a, b_kn]).expect("plain gpu");
    for (t, p) in gpu.iter().zip(&plain) {
        assert!(
            (t - p).abs() <= 1e-4 * p.abs().max(1.0),
            "trans_b {t} vs plain-on-transposed {p}"
        );
    }
}

/// The `{GatherDequant → MatMul}` graph dequantizes a ternary weight on the fly and
/// matmuls it; its CPU oracle matches `x · (scale ⊙ vals)` from the *known* ternary
/// values (an independent reference, not the same unpack code).
#[test]
fn dequant_matmul_cpu_oracle_matches_reference() {
    use crate::wgsl_forge::graph_ops::gather_dequant::pack_ternary_as_words;
    let (rows, cols) = (5usize, 8usize);
    let vals: Vec<f32> = (0..rows * cols)
        .map(|i| match (i * 7) % 3 {
            0 => 1.0,
            1 => -1.0,
            _ => 0.0,
        })
        .collect();
    let scale: Vec<f32> = (0..rows).map(|r| 0.5 + r as f32 * 0.1).collect();
    let packed = pack_ternary_as_words(&vals, rows, cols);
    let x: Vec<f32> = (0..rows).map(|i| (i as f32) * 0.3 - 0.6).collect();
    let g = dequant_matmul_graph(rows as u32, cols as u32).unwrap();
    let out = execute_graph_cpu(&g, &[x.clone(), packed, scale.clone()]).unwrap();
    // Independent reference W[r,c] = scale[r]*vals[r,c]; y = x·W.
    let w: Vec<f32> = (0..rows * cols)
        .map(|i| scale[i / cols] * vals[i])
        .collect();
    let want = ref_mm(&x, &w, 1, rows, cols);
    assert_eq!(out.len(), cols);
    for (o, r) in out.iter().zip(&want) {
        assert!((o - r).abs() < 1e-5, "{o} vs {r}");
    }
}

/// The full decode-block graph's composed CPU oracle matches an independent
/// `x + attn(RMSNorm(x)); + SwiGLU(RMSNorm(·))` reference (both residuals).
#[test]
fn decode_block_cpu_oracle_matches_reference() {
    let (d, kv, ffn) = (4usize, 5usize, 6usize);
    let inv_scale = 1.0f32 / (d as f32).sqrt();
    let eps = 1e-5f32;
    let x: Vec<f32> = (0..d).map(|i| (i as f32) * 0.2 - 0.3).collect();
    let kt: Vec<f32> = (0..d * kv)
        .map(|i| ((i * 5 % 7) as f32) * 0.1 - 0.25)
        .collect();
    let v: Vec<f32> = (0..kv * d)
        .map(|i| ((i * 3 % 5) as f32) * 0.15 - 0.2)
        .collect();
    let wg: Vec<f32> = (0..d * ffn)
        .map(|i| ((i % 11) as f32) * 0.03 - 0.15)
        .collect();
    let wu: Vec<f32> = (0..d * ffn)
        .map(|i| ((i % 7) as f32) * 0.02 - 0.07)
        .collect();
    let wd: Vec<f32> = (0..ffn * d)
        .map(|i| ((i % 5) as f32) * 0.04 - 0.08)
        .collect();
    let g = decode_block_graph(d as u32, kv as u32, ffn as u32).unwrap();
    let ext = vec![
        x.clone(),
        kt.clone(),
        v.clone(),
        wg.clone(),
        wu.clone(),
        wd.clone(),
        vec![inv_scale],
        vec![eps],
    ];
    let out = execute_graph_cpu(&g, &ext).unwrap();

    // Reference (with 1/√d attention scale + RMSNorm eps).
    let n1 = ref_rmsnorm(&x, eps);
    let scores: Vec<f32> = ref_mm(&n1, &kt, 1, d, kv)
        .iter()
        .map(|s| s * inv_scale)
        .collect();
    let probs = ref_softmax(&scores);
    let attn = ref_mm(&probs, &v, 1, kv, d);
    let res1: Vec<f32> = x.iter().zip(&attn).map(|(a, b)| a + b).collect();
    let n2 = ref_rmsnorm(&res1, eps);
    let gate = ref_mm(&n2, &wg, 1, d, ffn);
    let up = ref_mm(&n2, &wu, 1, d, ffn);
    let h: Vec<f32> = gate
        .iter()
        .zip(&up)
        .map(|(&gv, &uv)| (gv / (1.0 + (-gv).exp())) * uv)
        .collect();
    let ffn_out = ref_mm(&h, &wd, 1, ffn, d);
    let want: Vec<f32> = res1.iter().zip(&ffn_out).map(|(a, b)| a + b).collect();

    assert_eq!(out.len(), d);
    for (o, w) in out.iter().zip(&want) {
        assert!((o - w).abs() < 1e-5, "{o} vs {w}");
    }
}

/// GPU certify (A2000): attention, `{GatherDequant→MatMul}`, and the full decode block —
/// each executed device-side — match their composed CPU oracle within f32 tolerance.
#[test]
#[serial_test::serial(gpu)]
fn p4b_graphs_gpu_match_cpu_oracle() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    use crate::wgsl_forge::graph_ops::gather_dequant::pack_ternary_as_words;
    // Attention (decode).
    {
        let (d, kv) = (64usize, 96usize);
        let q: Vec<f32> = (0..d).map(|i| ((i * 7 % 23) as f32) * 0.05 - 0.5).collect();
        let kt: Vec<f32> = (0..d * kv)
            .map(|i| ((i % 19) as f32) * 0.02 - 0.18)
            .collect();
        let v: Vec<f32> = (0..kv * d)
            .map(|i| ((i % 13) as f32) * 0.03 - 0.18)
            .collect();
        let g = attention_graph(d as u32, kv as u32).unwrap();
        let ext = vec![q, kt, v, vec![1.0f32 / (d as f32).sqrt()]];
        let gpu = execute_graph(&g, &ext).expect("attn gpu");
        let cpu = execute_graph_cpu(&g, &ext).unwrap();
        for (a, b) in gpu.iter().zip(&cpu) {
            assert!((a - b).abs() <= 1e-3 * b.abs().max(1.0), "attn: {a} vs {b}");
        }
    }
    // {GatherDequant → MatMul}.
    {
        let (rows, cols) = (48usize, 64usize);
        let vals: Vec<f32> = (0..rows * cols)
            .map(|i| match (i * 7) % 3 {
                0 => 1.0,
                1 => -1.0,
                _ => 0.0,
            })
            .collect();
        let scale: Vec<f32> = (0..rows).map(|r| 0.25 + (r % 5) as f32 * 0.1).collect();
        let packed = pack_ternary_as_words(&vals, rows, cols);
        let x: Vec<f32> = (0..rows).map(|i| ((i % 9) as f32) * 0.1 - 0.4).collect();
        let g = dequant_matmul_graph(rows as u32, cols as u32).unwrap();
        let ext = vec![x, packed, scale];
        let gpu = execute_graph(&g, &ext).expect("dequant gpu");
        let cpu = execute_graph_cpu(&g, &ext).unwrap();
        for (a, b) in gpu.iter().zip(&cpu) {
            assert!(
                (a - b).abs() <= 1e-3 * b.abs().max(1.0),
                "dequant: {a} vs {b}"
            );
        }
    }
    // Full decode block.
    {
        let (d, kv, ffn) = (64u32, 80u32, 128u32);
        let mk = |n: usize, m: u32| {
            (0..n)
                .map(|i| ((i as u32 % m) as f32) * 0.01 - 0.1)
                .collect::<Vec<f32>>()
        };
        let ext = vec![
            mk(d as usize, 17),
            mk((d * kv) as usize, 19),
            mk((kv * d) as usize, 13),
            mk((d * ffn) as usize, 11),
            mk((d * ffn) as usize, 7),
            mk((ffn * d) as usize, 5),
            vec![1.0f32 / (d as f32).sqrt()],
            vec![1e-5f32],
        ];
        let g = decode_block_graph(d, kv, ffn).unwrap();
        let gpu = execute_graph(&g, &ext).expect("decode gpu");
        let cpu = execute_graph_cpu(&g, &ext).unwrap();
        assert_eq!(gpu.len(), cpu.len());
        for (a, b) in gpu.iter().zip(&cpu) {
            assert!(
                (a - b).abs() <= 1e-2 * b.abs().max(1.0),
                "decode: {a} vs {b}"
            );
        }
    }
}

/// **Honest kernel-level uplift benchmark** — times one decode-block graph (≈SmolLM2-360M
/// dims: d=576, kv=128, ffn=1536) executed on the GPU vs the composed CPU oracle. Reports
/// wall-clock per call for two GPU paths so the throughput pass is attributable:
/// - **reused** — one [`ForgeGraphExecutor`] held across calls (`run` per step): context
///   reuse **+** single-encoder deferred submit (the realistic decode-step usage);
/// - **one-shot** — `execute_graph` per call (fresh device/slab each call): single-encoder
///   submit but no context reuse, so `reused` vs `one-shot` isolates the device-creation cost.
///
/// **Caveats (do not over-read):** this is ONE decode block, not a full L-layer model; it is
/// **not** end-to-end tokens/sec and does not include sampling, KV-cache management, or
/// host↔device transfer beyond the final readback. The `reused` path now records the whole
/// graph into one encoder + one submit **and** compiles each node's pipeline only once (the
/// context-level pipeline cache), so the warmup run pays compilation and the timed loop pays
/// only bind-group build + dispatch + readback — the realistic held-executor decode step.
#[test]
#[serial_test::serial(gpu)]
fn decode_block_kernel_uplift_bench() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    use std::time::Instant;
    let (d, kv, ffn) = (576u32, 128u32, 1536u32);
    let mk = |n: usize, m: u32, off: f32| {
        (0..n)
            .map(|i| ((i as u32 % m) as f32) * 0.001 - off)
            .collect::<Vec<f32>>()
    };
    let ext = vec![
        mk(d as usize, 97, 0.05),
        mk((d * kv) as usize, 89, 0.04),
        mk((kv * d) as usize, 83, 0.04),
        mk((d * ffn) as usize, 79, 0.03),
        mk((d * ffn) as usize, 73, 0.03),
        mk((ffn * d) as usize, 71, 0.03),
        vec![1.0f32 / (d as f32).sqrt()],
        vec![1e-5f32],
    ];
    let g = decode_block_graph(d, kv, ffn).unwrap();
    let nodes = g.nodes.len();
    let iters = 20;

    // ── Reused executor: context reuse + single-encoder submit (the decode-step path) ──
    let mut exec = ForgeGraphExecutor::new().expect("executor");
    let _ = exec.run(&g, &ext).expect("warmup reused"); // shader compile + first dispatch
    let t0 = Instant::now();
    for _ in 0..iters {
        let _ = exec.run(&g, &ext).expect("gpu reused");
    }
    let gpu_reuse_ms = t0.elapsed().as_secs_f64() * 1e3 / iters as f64;

    // ── One-shot: fresh device/slab per call (single-encoder submit, no context reuse) ──
    let _ = execute_graph(&g, &ext).expect("warmup one-shot");
    let t1 = Instant::now();
    for _ in 0..iters {
        let _ = execute_graph(&g, &ext).expect("gpu one-shot");
    }
    let gpu_oneshot_ms = t1.elapsed().as_secs_f64() * 1e3 / iters as f64;

    let c0 = Instant::now();
    for _ in 0..iters {
        let _ = execute_graph_cpu(&g, &ext).expect("cpu");
    }
    let cpu_ms = c0.elapsed().as_secs_f64() * 1e3 / iters as f64;

    let cached = exec.context().cached_pipeline_count();
    eprintln!(
        "[decode-block uplift] d={d} kv={kv} ffn={ffn} nodes={nodes} cached_pipelines={cached} | \
         GPU reused {gpu_reuse_ms:.3} ms/call (ctx reuse + 1 encoder + pipeline cache; ~{:.3} ms/node) | \
         GPU one-shot {gpu_oneshot_ms:.3} ms/call (fresh device/slab per call) | \
         CPU oracle {cpu_ms:.3} ms/call | ratio (reused vs CPU) {:.2}x. \
         NOT end-to-end tok/s; one block, not L layers.",
        gpu_reuse_ms / nodes as f64,
        cpu_ms / gpu_reuse_ms,
    );
}

/// The context-level pipeline cache amortizes shader compilation across `run()` calls: a
/// held [`ForgeGraphExecutor`] re-running the same graph compiles each distinct node kernel
/// exactly once, so the cache count is **stable** after the first run (and bounded by the
/// number of distinct kernels, well below the node count for a decode block with repeated
/// op-classes). This is what turns the per-call compile cost into a one-time warmup.
#[test]
#[serial_test::serial(gpu)]
fn pipeline_cache_amortizes_across_runs() {
    if !crate::wgsl_forge::test_gpu_available() {
        return;
    }
    let (d, kv, ffn) = (64u32, 80u32, 128u32);
    let mk = |n: usize, m: u32| {
        (0..n)
            .map(|i| ((i as u32 % m) as f32) * 0.01 - 0.1)
            .collect::<Vec<f32>>()
    };
    let ext = vec![
        mk(d as usize, 17),
        mk((d * kv) as usize, 19),
        mk((kv * d) as usize, 13),
        mk((d * ffn) as usize, 11),
        mk((d * ffn) as usize, 7),
        mk((ffn * d) as usize, 5),
        vec![1.0f32 / (d as f32).sqrt()],
        vec![1e-5f32],
    ];
    let g = decode_block_graph(d, kv, ffn).unwrap();
    let mut exec = ForgeGraphExecutor::new().expect("executor");
    let _ = exec.run(&g, &ext).expect("run 1");
    let after_first = exec.context().cached_pipeline_count();
    let _ = exec.run(&g, &ext).expect("run 2");
    let after_second = exec.context().cached_pipeline_count();
    // Stable: the second run compiled nothing new.
    assert_eq!(
        after_first, after_second,
        "cache must be stable across runs"
    );
    // Distinct kernels < node count (repeated op-classes share a pipeline).
    assert!(
        after_first > 0 && after_first <= g.nodes.len(),
        "cached={after_first} nodes={}",
        g.nodes.len()
    );
    // The result is unchanged across runs (cache returns the same pipeline).
    let a = exec.run(&g, &ext).unwrap();
    let b = exec.run(&g, &ext).unwrap();
    assert_eq!(a, b, "cached runs must be deterministic");
}
