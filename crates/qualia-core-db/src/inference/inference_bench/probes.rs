//! Correctness / parity / evaluation probes that drive the real engine on a
//! dedicated runtime: top-k A/B decode, single-decode metrics, GEMM parity
//! (Q8/F16), native perplexity, GPU KV readback, the AWQ α-sweep, exact-sampler
//! decode, and the speculative-decode verify probe. Pure code motion — unchanged.

use crate::llm_agent::{AgentBackend, LocalLlmAgent};

use super::metrics::tok_per_s;
use super::*;

/// A1a correctness: decode the same prompt with the GPU top-k path **off** then **on** (same resident
/// model, deterministic argmax) and return both strings. Since k=1 top-k == argmax, the texts must be
/// byte-identical — this verifies the GEMM→top-k wiring, not just the kernel (which is oracle-tested).
pub fn compare_topk_decode(
    model_path: &str,
    prompt: &str,
    decode_tokens: u32,
) -> Result<(String, String), String> {
    if !std::path::Path::new(model_path).exists() {
        return Err(format!("model not found: {model_path}"));
    }
    let agent = LocalLlmAgent::with_local_backend(
        "did:qualia:bench",
        AgentBackend::Local {
            model_path: model_path.to_string(),
            context_window: 4096,
            quantization: "auto".into(),
            vision_projector_path: None,
            modality: "text".into(),
            architecture: None,
        },
    );
    set_decode_budget_override(decode_tokens);
    let model_id = crate::q_hash(model_path);
    let _ = crate::resident_model::mount_resident_gguf(model_id, model_path, false);

    set_gpu_topk(false);
    let (off_text, _, _, _) = agent.infer_local_model_streaming::<fn(String)>(prompt, "", None);
    set_gpu_topk(true);
    let (on_text, _, _, _) = agent.infer_local_model_streaming::<fn(String)>(prompt, "", None);

    set_gpu_topk(false);
    set_decode_budget_override(0);
    crate::resident_model::clear_resident_model();
    Ok((off_text, on_text))
}

/// `compare_topk_decode` inside a fresh multi-thread runtime (residency mount needs `block_in_place`).
pub fn compare_topk_decode_blocking(
    model_path: &str,
    prompt: &str,
    decode_tokens: u32,
) -> Result<(String, String), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async { compare_topk_decode(model_path, prompt, decode_tokens) })
}

/// A1b: mount a model (auto-detecting `P64` vs GGUF by magic) and run ONE decode of `prompt` for
/// `decode_tokens`, returning `(text, decode_tok_s)`. For a ternary `.q42` the FFN routing follows
/// the global `set_ternary_ffn` toggle, so a caller can measure GPU-ON vs CPU-OFF on identical
/// weights. Caller sets the toggle before invoking. (Use the `_blocking` wrapper from sync code.)
#[cfg(not(target_arch = "wasm32"))]
pub fn decode_with_metrics(
    model_path: &str,
    prompt: &str,
    decode_tokens: u32,
) -> Result<(String, f64), String> {
    if !std::path::Path::new(model_path).exists() {
        return Err(format!("model not found: {model_path}"));
    }
    let is_q42 = {
        use std::io::Read;
        let mut buf = [0u8; 4];
        std::fs::File::open(model_path)
            .and_then(|mut f| f.read_exact(&mut buf))
            .map(|_| &buf == b"p64\0")
            .unwrap_or(false)
    };
    let agent = LocalLlmAgent::with_local_backend(
        "did:qualia:bench",
        AgentBackend::Local {
            model_path: model_path.to_string(),
            context_window: 4096,
            quantization: "auto".into(),
            vision_projector_path: None,
            modality: "text".into(),
            architecture: None,
        },
    );
    set_decode_budget_override(decode_tokens);
    let model_id = crate::q_hash(model_path);
    if is_q42 {
        crate::resident_model::mount_resident_q42(model_id, model_path, false)?;
    } else {
        let _ = crate::resident_model::mount_resident_gguf(model_id, model_path, false);
    }
    reset_phase_metrics();
    let (text, _, _, _) = agent.infer_local_model_streaming::<fn(String)>(prompt, "", None);
    let snap = phase_snapshot();
    let decode_tok_s = tok_per_s(snap.decode_tokens, snap.decode_ns);
    set_decode_budget_override(0);
    crate::resident_model::clear_resident_model();
    Ok((text, decode_tok_s))
}

/// W3 — GPU↔CPU GEMM parity probe (test/diagnostic). Builds a fresh engine, synthesizes a random
/// Q8_0 weight matrix (`n_out` rows × `n_in`; `n_in` must be a multiple of 32) + input from `seed`,
/// runs the GPU kernel and the CPU reference on **identical** bytes, and returns
/// `(max_abs_err, mean_abs_err, max_ulp, gpu_gemm_passes_profiled)`. A non-zero pass count proves the
/// GPU path actually executed — the engine readback falls back to CPU when no tokio handle is present,
/// so the `rt.enter()` below installs one to force the real GPU path.
#[cfg(not(target_arch = "wasm32"))]
pub fn gemm_parity_probe_blocking(
    n_in: usize,
    n_out: usize,
    seed: u64,
) -> Result<(f32, f64, u64, u64), String> {
    use crate::gguf_sharder::GgufTensorInfo;
    if n_in == 0 || n_out == 0 || n_in % 32 != 0 {
        return Err("n_in must be a non-zero multiple of 32 (Q8_0 block size); n_out > 0".into());
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let mut engine = rt
        .block_on(crate::gguf_bridge::QTensorEngine::try_new())
        .map_err(|e| format!("engine init: {e}"))?;
    let _guard = rt.enter(); // install a tokio handle on this thread so the GPU readback path runs

    // Deterministic LCG → values in [-1, 1).
    let mut s = seed | 1;
    let mut rng = move || -> f32 {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((s >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    };

    let row_bytes = crate::llm_kernel_parity::q8_0_bytes(n_in);
    let mut raw = vec![0u8; row_bytes * n_out];
    let mut row_f32 = vec![0f32; n_in];
    for r in 0..n_out {
        for x in row_f32.iter_mut() {
            *x = rng();
        }
        if !crate::llm_kernel_parity::quantize_q8_0_from_f32(
            &row_f32,
            &mut raw[r * row_bytes..(r + 1) * row_bytes],
        ) {
            return Err("q8_0 quantize failed".into());
        }
    }
    let input: Vec<f32> = (0..n_in).map(|_| rng()).collect();

    let info = GgufTensorInfo {
        dims: [n_in as u64, n_out as u64, 1, 1],
        n_dims: 2,
        ggml_type: crate::ggml_quants::GGML_TYPE_Q8_0,
        byte_offset: 0,
    };

    crate::llm_gpu_profiler::set_enabled(true);
    crate::llm_gpu_profiler::reset();
    let mut gpu_out = vec![0f32; n_out];
    let mut cpu_out = vec![0f32; n_out];
    let ok = engine.gemm_parity_probe(&info, &raw, &input, &mut gpu_out, &mut cpu_out, n_in, n_out);
    let calls = crate::llm_gpu_profiler::snapshot()
        .iter()
        .find(|t| matches!(t.phase, crate::llm_gpu_profiler::Phase::Gemm))
        .map(|t| t.calls)
        .unwrap_or(0);
    crate::llm_gpu_profiler::set_enabled(false);
    if !ok {
        return Err("gemm_parity_probe: GPU or CPU path returned false".into());
    }
    Ok((
        crate::llm_kernel_parity::max_abs_err(&gpu_out, &cpu_out),
        crate::llm_kernel_parity::mean_abs_err(&gpu_out, &cpu_out),
        crate::llm_kernel_parity::max_ulp_diff(&gpu_out, &cpu_out),
        calls,
    ))
}

/// W3/F16 — GPU↔CPU parity for the new **F16** GEMM path (`unpack2x16float` in the shader vs the CPU
/// `dequant_f16` reference). Synthesizes a random F16 weight matrix (`n_out` rows × `n_in`; no block
/// constraint) + input from `seed`, runs both on identical bytes, returns
/// `(max_abs_err, mean_abs_err, max_ulp, gpu_gemm_passes)`. Same witness rule as the Q8 probe.
#[cfg(not(target_arch = "wasm32"))]
pub fn gemm_parity_probe_f16_blocking(
    n_in: usize,
    n_out: usize,
    seed: u64,
) -> Result<(f32, f64, u64, u64), String> {
    use crate::gguf_sharder::GgufTensorInfo;
    if n_in == 0 || n_out == 0 {
        return Err("n_in and n_out must be > 0".into());
    }
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    let mut engine = rt
        .block_on(crate::gguf_bridge::QTensorEngine::try_new())
        .map_err(|e| format!("engine init: {e}"))?;
    let _guard = rt.enter();

    let mut s = seed | 1;
    let mut rng = move || -> f32 {
        s = s
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        ((s >> 40) as f32 / (1u64 << 24) as f32) * 2.0 - 1.0
    };

    let row_bytes = crate::llm_kernel_parity::f16_bytes(n_in);
    let mut raw = vec![0u8; row_bytes * n_out];
    let mut row_f32 = vec![0f32; n_in];
    for r in 0..n_out {
        for x in row_f32.iter_mut() {
            *x = rng();
        }
        if !crate::llm_kernel_parity::quantize_f16_from_f32(
            &row_f32,
            &mut raw[r * row_bytes..(r + 1) * row_bytes],
        ) {
            return Err("f16 quantize failed".into());
        }
    }
    let input: Vec<f32> = (0..n_in).map(|_| rng()).collect();

    let info = GgufTensorInfo {
        dims: [n_in as u64, n_out as u64, 1, 1],
        n_dims: 2,
        ggml_type: crate::ggml_quants::GGML_TYPE_F16,
        byte_offset: 0,
    };

    crate::llm_gpu_profiler::set_enabled(true);
    crate::llm_gpu_profiler::reset();
    let mut gpu_out = vec![0f32; n_out];
    let mut cpu_out = vec![0f32; n_out];
    let ok = engine.gemm_parity_probe(&info, &raw, &input, &mut gpu_out, &mut cpu_out, n_in, n_out);
    let calls = crate::llm_gpu_profiler::snapshot()
        .iter()
        .find(|t| matches!(t.phase, crate::llm_gpu_profiler::Phase::Gemm))
        .map(|t| t.calls)
        .unwrap_or(0);
    crate::llm_gpu_profiler::set_enabled(false);
    if !ok {
        return Err("gemm_parity_probe (f16): GPU or CPU path returned false".into());
    }
    Ok((
        crate::llm_kernel_parity::max_abs_err(&gpu_out, &cpu_out),
        crate::llm_kernel_parity::mean_abs_err(&gpu_out, &cpu_out),
        crate::llm_kernel_parity::max_ulp_diff(&gpu_out, &cpu_out),
        calls,
    ))
}

/// W1 — teacher-forced perplexity of `model_path` over the eval corpus, run through Qualia's **native**
/// engine (never an external runtime). For each corpus passage: `reset_kv_cache`, then per position
/// embed → `dispatch_transformer_forward` → `apply_output_norm_inplace` → `dispatch_output_logits_into`
/// → NLL of the true next token; PPL = `exp(ΣNLL / Σtokens)`. `max_tok` = 0 scores the whole passage,
/// >0 caps it (to bound the slow F16-on-CPU path for big models). Returns `(perplexity, tokens_scored)`.
/// Runs on a dedicated thread with a current-thread tokio runtime (mirrors the decode path) so the
/// engine's GPU readback works. Handles both GGUF and `.q42` containers.
#[cfg(not(target_arch = "wasm32"))]
pub fn perplexity_eval_blocking(model_path: &str, max_tok: usize) -> Result<(f64, usize), String> {
    use crate::gguf_bridge::QTensorEngine;
    use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};

    let corpus = crate::llm_eval::load_corpus().map_err(|e| format!("corpus load: {e}"))?;
    if corpus.is_empty() {
        return Err("eval corpus is empty".into());
    }
    let model_path = model_path.to_string();

    std::thread::spawn(move || -> Result<(f64, usize), String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let _g = rt.enter();

        let mut engine = QTensorEngine::new();
        let mut magic = [0u8; 4];
        let is_q42 = {
            use std::io::Read;
            std::fs::File::open(&model_path)
                .and_then(|mut f| f.read_exact(&mut magic))
                .map(|_| &magic == b"p64\0")
                .unwrap_or(false)
        };
        if is_q42 {
            let f = std::fs::File::open(&model_path).map_err(|e| e.to_string())?;
            let mmap = unsafe { memmap2::Mmap::map(&f) }.map_err(|e| e.to_string())?;
            engine
                .adopt_resident_p64_mmap(std::sync::Arc::new(mmap))
                .map_err(|e| format!("q42 adopt: {e}"))?;
        } else {
            engine.load_gguf(&model_path);
        }

        let mmap = engine
            .gguf_mmap
            .clone()
            .ok_or_else(|| "model did not memory-map (load failed)".to_string())?;
        let is_q42_mmap = mmap.len() >= 4 && mmap[0..4] == *b"p64\0";
        let tok = if is_q42_mmap {
            crate::p64_weight::P64TensorIndex::from_p64(&mmap)
                .ok()
                .and_then(|qi| GgufTokenizer::from_p64_section(qi.tokenizer_bytes(&mmap)))
                .unwrap_or_default()
        } else {
            GgufTokenizer::from_gguf(&mmap)
        };
        let tensor_idx = if is_q42_mmap {
            crate::p64_weight::P64TensorIndex::from_p64(&mmap)
                .map(|qi| qi.to_gguf_index())
                .map_err(|e| format!("q42 index: {e}"))?
        } else {
            GgufTensorIndex::from_gguf(&mmap)
        };

        let emb_dim = tensor_idx.emb_dim();
        if emb_dim == 0 {
            return Err("embedding dimension is 0 (tensor index parse failed)".into());
        }
        let vocab = tok.vocab_len().max(1) as usize;

        let mut emb_buf = vec![0f32; emb_dim.max(8192)];
        let mut scratch_a = vec![0f32; 16384];
        let mut scratch_b = vec![0f32; 16384];
        let mut logits = vec![0f32; vocab];
        let mmap_bytes: &[u8] = &mmap;

        let mut total_nll = 0.0f64;
        let mut total_tok = 0usize;
        for passage in &corpus {
            let toks = tok.encode(passage);
            if toks.len() < 2 {
                continue;
            }
            let limit = if max_tok > 0 {
                (max_tok + 1).min(toks.len())
            } else {
                toks.len()
            };
            engine.reset_kv_cache();
            for i in 0..limit - 1 {
                let n_emb = tensor_idx.dequantize_token_embedding_into(
                    mmap_bytes,
                    toks[i],
                    &mut emb_buf[..emb_dim],
                );
                if n_emb == 0 {
                    return Err(format!("embedding lookup failed for token {}", toks[i]));
                }
                // AWQ calibration: reset the per-forward layer cursor so the FFN hook tags layers
                // 0..n_layer-1 correctly (no-op when AWQ capture is off).
                crate::llm_awq::begin_forward();
                let _ = engine.dispatch_transformer_forward(
                    &tensor_idx,
                    &mut emb_buf[..emb_dim],
                    emb_dim,
                    &mut scratch_a,
                    &mut scratch_b,
                    i as u32,
                    0, // 0 = all layers (full model depth)
                );
                let _ =
                    engine.apply_output_norm_inplace(&tensor_idx, &mut emb_buf[..emb_dim], emb_dim);
                let n = engine.dispatch_output_logits_into(
                    &tensor_idx,
                    &emb_buf[..emb_dim],
                    emb_dim,
                    &mut logits,
                );
                if n == 0 {
                    return Err("output projection produced no logits".into());
                }
                let nll = crate::llm_eval::token_nll(&logits[..n], toks[i + 1] as usize);
                if nll.is_finite() {
                    total_nll += nll;
                    total_tok += 1;
                }
            }
        }
        if total_tok == 0 {
            return Err("no tokens scored".into());
        }
        Ok((crate::llm_eval::perplexity(total_nll, total_tok), total_tok))
    })
    .join()
    .map_err(|_| "perplexity eval thread panicked".to_string())?
}

/// GPU-readback KV capture for the W5b sparse-dictionary go/no-go — the independent route to the
/// CPU-reference hook. Loads the model with an **f32** KV cache, runs the REAL fast GPU decode forward
/// over the eval corpus, and after each passage reads the KV arena back from VRAM
/// ([`QTensorEngine::capture_kv_f32`]), accumulating up to `max_per_layer` K and V vectors per layer.
/// Because it samples the actual decode-path vectors (not the CPU reference), it cross-checks the hook
/// capture: if both agree, the measured KV geometry is trustworthy. Stops early once every layer's cap
/// is hit. Needs a GPU.
#[cfg(not(target_arch = "wasm32"))]
pub fn capture_kv_gpu_readback(
    model_path: &str,
    max_tok: usize,
    max_per_layer: usize,
) -> Result<crate::kv_capture::KvCapture, String> {
    use crate::gguf_bridge::QTensorEngine;
    use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};

    let corpus = crate::llm_eval::load_corpus().map_err(|e| format!("corpus load: {e}"))?;
    if corpus.is_empty() {
        return Err("eval corpus is empty".into());
    }
    let model_path = model_path.to_string();

    std::thread::spawn(move || -> Result<crate::kv_capture::KvCapture, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let _g = rt.enter();

        // Force an f32 KV layout so the readback decodes via k_index/v_index (int8 packs differently).
        let prev_int8 = kv_int8_enabled();
        set_kv_int8(false);

        let mut engine = QTensorEngine::new();
        let mut magic = [0u8; 4];
        let is_q42 = {
            use std::io::Read;
            std::fs::File::open(&model_path)
                .and_then(|mut f| f.read_exact(&mut magic))
                .map(|_| &magic == b"p64\0")
                .unwrap_or(false)
        };
        if is_q42 {
            let f = std::fs::File::open(&model_path).map_err(|e| e.to_string())?;
            let mmap = unsafe { memmap2::Mmap::map(&f) }.map_err(|e| e.to_string())?;
            engine
                .adopt_resident_p64_mmap(std::sync::Arc::new(mmap))
                .map_err(|e| format!("q42 adopt: {e}"))?;
        } else {
            engine.load_gguf(&model_path);
        }

        let mmap = engine
            .gguf_mmap
            .clone()
            .ok_or_else(|| "model did not memory-map (load failed)".to_string())?;
        let is_q42_mmap = mmap.len() >= 4 && mmap[0..4] == *b"p64\0";
        let tok = if is_q42_mmap {
            crate::p64_weight::P64TensorIndex::from_p64(&mmap)
                .ok()
                .and_then(|qi| GgufTokenizer::from_p64_section(qi.tokenizer_bytes(&mmap)))
                .unwrap_or_default()
        } else {
            GgufTokenizer::from_gguf(&mmap)
        };
        let tensor_idx = if is_q42_mmap {
            crate::p64_weight::P64TensorIndex::from_p64(&mmap)
                .map(|qi| qi.to_gguf_index())
                .map_err(|e| format!("q42 index: {e}"))?
        } else {
            GgufTensorIndex::from_gguf(&mmap)
        };

        let emb_dim = tensor_idx.emb_dim();
        if emb_dim == 0 {
            return Err("embedding dimension is 0".into());
        }
        let mut emb_buf = vec![0f32; emb_dim.max(8192)];
        let mut scratch_a = vec![0f32; 16384];
        let mut scratch_b = vec![0f32; 16384];
        let mmap_bytes: &[u8] = &mmap;

        let mut acc_k: Vec<Vec<Vec<f32>>> = Vec::new();
        let mut acc_v: Vec<Vec<Vec<f32>>> = Vec::new();
        let mut head_dim = 0usize;

        'corpus: for passage in &corpus {
            let toks = tok.encode(passage);
            if toks.len() < 2 {
                continue;
            }
            let limit = if max_tok > 0 {
                (max_tok + 1).min(toks.len())
            } else {
                toks.len()
            };
            engine.reset_kv_cache();
            for i in 0..limit - 1 {
                let n_emb = tensor_idx.dequantize_token_embedding_into(
                    mmap_bytes,
                    toks[i],
                    &mut emb_buf[..emb_dim],
                );
                if n_emb == 0 {
                    return Err(format!("embedding lookup failed for token {}", toks[i]));
                }
                let _ = engine.dispatch_transformer_forward(
                    &tensor_idx,
                    &mut emb_buf[..emb_dim],
                    emb_dim,
                    &mut scratch_a,
                    &mut scratch_b,
                    i as u32,
                    0,
                );
            }
            // Read this passage's KV back from VRAM and merge into the accumulator.
            if let Some(cap) = engine.capture_kv_f32((limit - 1) as u32, max_per_layer) {
                head_dim = cap.head_dim;
                if acc_k.is_empty() {
                    acc_k = vec![Vec::new(); cap.k.len()];
                    acc_v = vec![Vec::new(); cap.v.len()];
                }
                let mut all_full = true;
                for l in 0..cap.k.len().min(acc_k.len()) {
                    for vk in &cap.k[l] {
                        if acc_k[l].len() < max_per_layer {
                            acc_k[l].push(vk.clone());
                        }
                    }
                    for vv in &cap.v[l] {
                        if acc_v[l].len() < max_per_layer {
                            acc_v[l].push(vv.clone());
                        }
                    }
                    if acc_k[l].len() < max_per_layer {
                        all_full = false;
                    }
                }
                if all_full {
                    break 'corpus; // caps hit — stop early, don't burn the rest of the corpus
                }
            }
        }

        set_kv_int8(prev_int8);
        if head_dim == 0 {
            return Err("no KV captured via GPU readback (int8 layout, or empty forward)".into());
        }
        Ok(crate::kv_capture::KvCapture {
            head_dim,
            k: acc_k,
            v: acc_v,
        })
    })
    .join()
    .map_err(|_| "kv capture thread panicked".to_string())?
}

/// AWQ α-sweep on the ternary FFN (AWQ steps 1–3 end to end): capture activation salience from the Q8
/// reference at `gguf_path`, then for each α compile an AWQ-scaled ternary `.q42`
/// (`compile_gguf_to_q42_ternary_ffn_awq`), evaluate its perplexity + unique-word coherence, and return
/// `(reference_ppl, [(alpha, ppl, uniq)])`. α=0.0 is plain ternary (the baseline). `max_tok` caps
/// tokens/passage to bound the sweep. Honest: this measures whether AWQ rescues ternary — it does not
/// assume it does. Needs a GPU.
#[cfg(not(target_arch = "wasm32"))]
pub fn awq_sweep_blocking(
    gguf_path: &str,
    alphas: &[f32],
    max_tok: usize,
    quant: crate::p64_weight::FfnQuant,
) -> Result<(f64, Vec<(f32, f64, f64)>), String> {
    use crate::p64_weight::compile_gguf_to_q42_ffn_quant_awq;

    let bytes = std::fs::read(gguf_path).map_err(|e| format!("read gguf: {e}"))?;
    let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(&bytes);
    let n_layer = idx.hyperparams.n_layer;
    let n_embd = idx.hyperparams.n_embd;
    if n_layer == 0 || n_embd == 0 {
        return Err("gguf parse failed (n_layer/n_embd = 0)".into());
    }

    // 1. Capture per-channel salience + the Q8 reference PPL in one calibration forward.
    set_ternary_ffn(false);
    crate::llm_awq::enable(n_layer, n_embd)?;
    let (ref_ppl, _) = perplexity_eval_blocking(gguf_path, max_tok)?;
    let stats = crate::llm_awq::snapshot();
    crate::llm_awq::disable();
    if stats.is_empty() {
        return Err("AWQ: no activation stats captured".into());
    }

    // 2. Sweep: AWQ-scaled .q42 per α → eval PPL + coherence. Ternary needs the resident 2-bit path;
    //    Q4_0 runs through the standard quantized GEMM (no ternary toggle).
    set_ternary_ffn(matches!(quant, crate::p64_weight::FfnQuant::Ternary));
    let tmp = std::env::temp_dir();
    let mut results = Vec::with_capacity(alphas.len());
    for &alpha in alphas {
        let scales = if alpha == 0.0 {
            None
        } else {
            Some(stats.as_slice())
        };
        let q42 = compile_gguf_to_q42_ffn_quant_awq(&bytes, 14, scales, alpha, quant)
            .map_err(|e| format!("AWQ compile (alpha={alpha}): {e}"))?;
        let path = tmp.join(format!("awq_sweep_a{:.2}.q42", alpha));
        std::fs::write(&path, &q42).map_err(|e| format!("write q42: {e}"))?;
        let ps = path.to_string_lossy().to_string();
        let (ppl, _) = perplexity_eval_blocking(&ps, max_tok)?;
        let (text, _) = decode_with_metrics_blocking(&ps, "Once upon a time, there was a", 24)?;
        let uniq = crate::llm_eval::unique_word_ratio(&text);
        let _ = std::fs::remove_file(&path);
        results.push((alpha, ppl, uniq));
    }
    set_ternary_ffn(false);
    Ok((ref_ppl, results))
}

/// `decode_with_metrics` inside a fresh multi-thread runtime (residency mount needs `block_in_place`).
#[cfg(not(target_arch = "wasm32"))]
pub fn decode_with_metrics_blocking(
    model_path: &str,
    prompt: &str,
    decode_tokens: u32,
) -> Result<(String, f64), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| e.to_string())?;
    rt.block_on(async { decode_with_metrics(model_path, prompt, decode_tokens) })
}

/// W2: decode with the exact CPU sampler installed for the duration of the call. Returns
/// `(text, tok/s)`. Restores greedy (`None`) afterwards so it never leaks into other tests.
#[cfg(not(target_arch = "wasm32"))]
pub fn decode_sampled_blocking(
    model_path: &str,
    prompt: &str,
    decode_tokens: u32,
    cfg: crate::sampler::SamplerConfig,
) -> Result<(String, f64), String> {
    set_sampler_config(Some(cfg));
    let out = decode_with_metrics_blocking(model_path, prompt, decode_tokens);
    set_sampler_config(None);
    out
}

/// W6a — batched verify-primitive correctness probe. Prefills `prompt` (positions `[0, p)`,
/// `p = prompt_len-1`), snapshots the KV cache, then:
///   * **reference** — sequentially forwards `b` steps from the last prompt token via the exact
///     per-token path (`dispatch_transformer_forward` → output norm → full-logit argmax), collecting
///     the greedy continuation `r0..r_{b-1}` and the inputs `[cur, r0..r_{b-2}]`;
///   * restores the post-prefill KV, then runs the **batched** `verify_draft_batch(inputs, p)`.
/// Returns `(reference, verify)`; they must be equal — the batched forward writes byte-identical KV
/// and both sides take a full-logit CPU argmax (no top-k tie-break gap). Runs on a dedicated thread
/// with a current-thread runtime (mirrors the decode/perplexity paths so GPU readback works).
#[cfg(not(target_arch = "wasm32"))]
pub fn spec_verify_probe_blocking(
    model_path: &str,
    prompt: &str,
    b: usize,
) -> Result<(Vec<u32>, Vec<u32>, Vec<u32>), String> {
    use crate::gguf_bridge::QTensorEngine;
    use crate::gguf_sharder::{GgufTensorIndex, GgufTokenizer};
    if !std::path::Path::new(model_path).exists() {
        return Err(format!("model not found: {model_path}"));
    }
    let model_path = model_path.to_string();
    let prompt = prompt.to_string();

    std::thread::spawn(move || -> Result<(Vec<u32>, Vec<u32>, Vec<u32>), String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| e.to_string())?;
        let _g = rt.enter();

        let mut engine = QTensorEngine::new();
        engine.load_gguf(&model_path);
        let mmap = engine
            .gguf_mmap
            .clone()
            .ok_or_else(|| "model did not memory-map".to_string())?;
        let tok = GgufTokenizer::from_gguf(&mmap);
        let idx = GgufTensorIndex::from_gguf(&mmap);
        // The batched verify tail binds the RESIDENT logits projection (like resident_decode). Plain
        // `load_gguf` does not upload it (only the residency-mount path does), so do it here.
        if !engine.mc8_upload_resident_logits(&idx) {
            return Err("resident logits upload failed (verify tail needs it)".into());
        }
        let emb_dim = idx.emb_dim();
        if emb_dim == 0 {
            return Err("embedding dimension is 0".into());
        }
        let vocab = tok.vocab_len().max(1) as usize;
        let toks = tok.encode(&prompt);
        if toks.len() < b + 2 {
            return Err(format!(
                "prompt too short: {} tokens, need >= {}",
                toks.len(),
                b + 2
            ));
        }

        let mut emb = vec![0f32; emb_dim.max(8192)];
        let mut sa = vec![0f32; 16384];
        let mut sb = vec![0f32; 16384];
        let mut logits = vec![0f32; vocab];
        let mmap_b: &[u8] = &mmap;

        let argmax = |v: &[f32]| -> u32 {
            let mut best_i = 0u32;
            let mut best_v = f32::NEG_INFINITY;
            for (i, &x) in v.iter().enumerate() {
                if x > best_v {
                    best_v = x;
                    best_i = i as u32;
                }
            }
            best_i
        };

        // Prefill positions [0, p) so the prefix KV matches what decode would have.
        let p = toks.len() - 1;
        engine.reset_kv_cache();
        for i in 0..p {
            let n = idx.dequantize_token_embedding_into(mmap_b, toks[i], &mut emb[..emb_dim]);
            if n == 0 {
                return Err(format!("embedding lookup failed for token {}", toks[i]));
            }
            let _ = engine.dispatch_transformer_forward(
                &idx,
                &mut emb[..emb_dim],
                emb_dim,
                &mut sa,
                &mut sb,
                i as u32,
                0,
            );
        }
        // No KV snapshot needed: the reference decode below writes only positions [p, p+b), never the
        // prefix [0, p) — so the prefix KV stays valid — and `verify_draft_batch` overwrites [p, p+b)
        // with the same inputs. (Note: `get_kv_cache_cpu` returns the CPU mirror, which is NOT synced
        // from the GPU KV writes, so it cannot be used to snapshot the GPU cache here.)

        // Reference: exact sequential greedy decode of `b` steps from the last prompt token.
        let mut inputs: Vec<u32> = Vec::with_capacity(b);
        let mut reference: Vec<u32> = Vec::with_capacity(b);
        let mut input = toks[p];
        let mut pos = p as u32;
        for _ in 0..b {
            inputs.push(input);
            let n = idx.dequantize_token_embedding_into(mmap_b, input, &mut emb[..emb_dim]);
            if n == 0 {
                return Err("reference embedding lookup failed".into());
            }
            let _ = engine.dispatch_transformer_forward(
                &idx,
                &mut emb[..emb_dim],
                emb_dim,
                &mut sa,
                &mut sb,
                pos,
                0,
            );
            let _ = engine.apply_output_norm_inplace(&idx, &mut emb[..emb_dim], emb_dim);
            let nl =
                engine.dispatch_output_logits_into(&idx, &emb[..emb_dim], emb_dim, &mut logits);
            if nl == 0 {
                return Err("reference output projection produced no logits".into());
            }
            let out = argmax(&logits[..nl]);
            reference.push(out);
            input = out;
            pos += 1;
        }

        // Run the batched verify over the same inputs (prefix KV [0, p) is still valid).
        let mut verify_out: Vec<u32> = Vec::new();
        let mut verify_logit: Vec<f32> = Vec::new();
        engine
            .verify_draft_batch(&idx, &inputs, p as u32, &mut verify_out, &mut verify_logit)
            .ok_or_else(|| "verify_draft_batch ineligible/failed".to_string())?;

        // Resident-path reference: re-prefill the prefix, then forward each input through the DEFAULT
        // single-fence resident path (GPU top-1). Isolates verify(batched, CPU argmax) vs
        // resident(single, GPU top-1) — the crux of the transparency question. u32::MAX marks a
        // resident-ineligible position.
        engine.reset_kv_cache();
        for i in 0..p {
            let n = idx.dequantize_token_embedding_into(mmap_b, toks[i], &mut emb[..emb_dim]);
            if n == 0 {
                return Err("resident-ref prefill embedding failed".into());
            }
            let _ = engine.dispatch_transformer_forward(
                &idx,
                &mut emb[..emb_dim],
                emb_dim,
                &mut sa,
                &mut sb,
                i as u32,
                0,
            );
        }
        let mut resident_out: Vec<u32> = Vec::with_capacity(b);
        for (j, &t) in inputs.iter().enumerate() {
            let n = idx.dequantize_token_embedding_into(mmap_b, t, &mut emb[..emb_dim]);
            if n == 0 {
                return Err("resident-ref embedding failed".into());
            }
            match engine.dispatch_token_forward_resident(&idx, &emb[..emb_dim], (p + j) as u32) {
                Some(r) => resident_out.push(r.best_token_id),
                None => resident_out.push(u32::MAX),
            }
        }

        Ok((reference, verify_out, resident_out))
    })
    .join()
    .map_err(|_| "spec-verify probe thread panicked".to_string())?
}
