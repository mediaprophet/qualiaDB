use super::*;
use crate::gguf_sharder::GgufTokenizer;
use std::path::Path;
use std::time::Instant;

fn run_gguf_smoke_decode(path: &str, prompt: &str, gen_tokens: usize) {
    if !Path::new(path).exists() {
        eprintln!("Skipping {path} (not present)");
        return;
    }
    let t0 = Instant::now();
    let mut engine = QTensorEngine::new();
    let report = engine
        .load_gguf_checked(path)
        .unwrap_or_else(|e| panic!("load_gguf_checked({path}) failed: {e}"));
    let load_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let mmap = engine.clone_model_bytes().expect("retained model owner");
    let index = engine
        .tensor_index_cache
        .clone()
        .expect("tensor_index_cache");
    let tokenizer = GgufTokenizer::from_gguf(&mmap);
    let mut tokens = tokenizer.encode_prompt(prompt);
    if tokens.is_empty() {
        tokens.push(tokenizer.bos_token_id);
    }
    let emb_dim = index.emb_dim();
    assert!(emb_dim > 0 && emb_dim <= 4096);
    eprintln!(
        "[smoke] Loaded {} in {:.1} ms | layers={} emb_dim={} arch={} kv_mib={:.1} prompt_tokens={:?}",
        path,
        load_ms,
        report.n_layer,
        emb_dim,
        index.hyperparams.architecture,
        report.kv_cache_bytes as f64 / (1024.0 * 1024.0),
        tokens
    );

    let mut hidden = vec![0.0f32; emb_dim];
    let mut scratch_a = vec![0.0f32; MAX_STACK_GEMM_OUT];
    let mut scratch_b = vec![0.0f32; MAX_STACK_GEMM_OUT];
    let mut chunk_logits = vec![0.0f32; VOCAB_CHUNK_ROWS];

    let t_prefill = Instant::now();
    for (pos, &tok) in tokens.iter().enumerate() {
        let n = index.dequantize_token_embedding_into(&mmap, tok, &mut hidden);
        assert_eq!(n, emb_dim, "embedding dequant failed at pos={pos}");
        let ran = if index.hyperparams.architecture == crate::gguf_sharder::ARCH_GEMMA4 {
            engine.dispatch_gemma4_token(
                &index,
                tok,
                pos as u32,
                &mut hidden,
                &mut scratch_a,
                &mut scratch_b,
            )
        } else {
            engine.dispatch_transformer_forward(
                &index,
                &mut hidden,
                emb_dim,
                &mut scratch_a,
                &mut scratch_b,
                pos as u32,
                0,
            )
        };
        assert_eq!(
            ran, report.n_layer,
            "expected all {} layers to execute at pos={pos}, got {ran}",
            report.n_layer
        );
        assert!(
            hidden.iter().all(|v| v.is_finite()),
            "non-finite hidden after prefill token pos={pos}"
        );
    }
    let prefill_s = t_prefill.elapsed().as_secs_f64();

    assert!(engine.apply_output_norm_inplace(&index, &mut hidden, emb_dim));
    let first = engine
        .dispatch_output_argmax_chunked(&index, &hidden, emb_dim, &mut chunk_logits, 0, None)
        .expect("argmax token 0");
    assert!(first.max_logit.is_finite());

    let mut generated = vec![first.best_token_id];
    let t_decode = Instant::now();
    for step in 1..gen_tokens {
        let pos = (tokens.len() - 1 + step) as u32;
        let prev = *generated.last().unwrap();
        let n = index.dequantize_token_embedding_into(&mmap, prev, &mut hidden);
        assert_eq!(n, emb_dim);
        let ran = if index.hyperparams.architecture == crate::gguf_sharder::ARCH_GEMMA4 {
            engine.dispatch_gemma4_token(
                &index,
                prev,
                pos,
                &mut hidden,
                &mut scratch_a,
                &mut scratch_b,
            )
        } else {
            engine.dispatch_transformer_forward(
                &index,
                &mut hidden,
                emb_dim,
                &mut scratch_a,
                &mut scratch_b,
                pos,
                0,
            )
        };
        assert_eq!(ran, report.n_layer);
        assert!(engine.apply_output_norm_inplace(&index, &mut hidden, emb_dim));
        let next = engine
            .dispatch_output_argmax_chunked(&index, &hidden, emb_dim, &mut chunk_logits, 0, None)
            .expect("argmax decode step");
        assert!(next.max_logit.is_finite());
        generated.push(next.best_token_id);
    }
    let decode_s = t_decode.elapsed().as_secs_f64();
    let decoded_text = tokenizer.decode(&generated);
    eprintln!(
        "[smoke] {} | prefill {:.2} tok/s ({} toks in {:.2}s) | decode {:.2} tok/s ({} toks in {:.2}s) | ids={:?} | text={:?}",
        path,
        tokens.len() as f64 / prefill_s.max(1e-9),
        tokens.len(),
        prefill_s,
        (gen_tokens.saturating_sub(1)) as f64 / decode_s.max(1e-9),
        gen_tokens.saturating_sub(1),
        decode_s,
        generated,
        decoded_text
    );
}

#[test]
fn real_qwen35_0_8b_gguf_end_to_end_decode() {
    let builder = std::thread::Builder::new()
        .name("qwen35-0.8b-test".into())
        .stack_size(32 * 1024 * 1024);
    let handle = builder
        .spawn(|| {
            run_gguf_smoke_decode(
                r"C:\LLM_files\qwen\Qwen3.5-0.8B-GGUF\Qwen3.5-0.8B-Q4_K_M.gguf",
                "The capital of France is",
                8,
            );
        })
        .unwrap();
    handle.join().unwrap();
}

#[test]
fn real_qwen35_4b_gguf_end_to_end_decode() {
    let builder = std::thread::Builder::new()
        .name("qwen35-4b-test".into())
        .stack_size(32 * 1024 * 1024);
    let handle = builder
        .spawn(|| {
            run_gguf_smoke_decode(
                r"C:\LLM_files\qwen\Qwen3.5-4B-GGUF\Qwen3.5-4B-Q4_K_M.gguf",
                "The capital of France is",
                6,
            );
        })
        .unwrap();
    handle.join().unwrap();
}

#[test]
fn real_gemma4_e4b_gguf_end_to_end_decode() {
    let builder = std::thread::Builder::new()
        .name("gemma4-e4b-test".into())
        .stack_size(32 * 1024 * 1024);
    let handle = builder
        .spawn(|| {
            run_gguf_smoke_decode(
                r"C:\LLM_files\gemma\gemma-4-E4B-it-GGUF\gemma-4-E4B-it-Q4_K_M.gguf",
                "The capital of France is",
                6,
            );
        })
        .unwrap();
    handle.join().unwrap();
}

#[test]
fn real_gemma4_26b_a4b_gguf_end_to_end_decode() {
    let builder = std::thread::Builder::new()
        .name("gemma4-26b-a4b-test".into())
        .stack_size(32 * 1024 * 1024);
    let handle = builder
        .spawn(|| {
            run_gguf_smoke_decode(
                r"C:\LLM_files\gemma\gemma-4-26B-A4B-it-GGUF\gemma-4-26B-A4B-it-MXFP4_MOE.gguf",
                "The capital of France is",
                6,
            );
        })
        .unwrap();
    handle.join().unwrap();
}
