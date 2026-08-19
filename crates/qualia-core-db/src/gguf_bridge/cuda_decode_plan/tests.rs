use std::path::Path;

use crate::ggml_quants::fetch_tensor_bytes;
use crate::gguf_bridge::cpu_ops::{dequant_norm_row_into, rms_norm_inplace};
use crate::gguf_bridge::{stack_gemm_quant, QTensorEngine, RMS_NORM_EPS};
use crate::gguf_sharder::GgufTensorIndex;
use crate::inference_modes::{set_inference_mode, InferenceMode};

fn cpu_layer_position_zero(
    index: &GgufTensorIndex,
    mmap: &[u8],
    layer: u32,
    hidden: &mut [f32],
) -> Option<()> {
    let tensors = index.get_layer_tensors(layer);
    let n_embd = index.hyperparams.n_embd as usize;
    let n_head = index.hyperparams.n_head as usize;
    let n_kv = index.hyperparams.effective_n_kv_head() as usize;
    let head_dim = index.hyperparams.head_dim() as usize;
    let q_dim = n_head * head_dim;
    let kv_dim = n_kv * head_dim;
    if hidden.len() < n_embd || n_kv == 0 {
        return None;
    }
    let q_per_kv = n_head / n_kv;
    if q_per_kv == 0 {
        return None;
    }

    let mut norm_weight = vec![0.0f32; n_embd];
    let mut normed = hidden[..n_embd].to_vec();
    let attn_norm = tensors.attn_norm.as_ref()?;
    if dequant_norm_row_into(mmap, index.tensor_data_start, attn_norm, &mut norm_weight) < n_embd {
        return None;
    }
    rms_norm_inplace(&mut normed, &norm_weight, RMS_NORM_EPS);

    // With one KV position, softmax has one element. Every query head therefore receives
    // the value vector of its GQA KV head; Q/K/RoPE cannot affect the position-zero result.
    let v_info = tensors.attn_v.as_ref()?;
    let v_raw = fetch_tensor_bytes(mmap, index.tensor_data_start, v_info).ok()?;
    let (v_in, v_out) = QTensorEngine::matmul_dims(v_info);
    if v_in != n_embd || v_out != kv_dim {
        return None;
    }
    let mut value = vec![0.0f32; kv_dim];
    if !stack_gemm_quant(v_raw, v_info, &normed, &mut value, v_in, v_out) {
        return None;
    }
    let mut attention = vec![0.0f32; q_dim];
    for q_head in 0..n_head {
        let kv_head = q_head / q_per_kv;
        let q_base = q_head * head_dim;
        let kv_base = kv_head * head_dim;
        attention[q_base..q_base + head_dim].copy_from_slice(&value[kv_base..kv_base + head_dim]);
    }

    let o_info = tensors.attn_output.as_ref()?;
    let o_raw = fetch_tensor_bytes(mmap, index.tensor_data_start, o_info).ok()?;
    let (o_in, o_out) = QTensorEngine::matmul_dims(o_info);
    let mut projected = vec![0.0f32; o_out];
    if o_in != q_dim
        || o_out != n_embd
        || !stack_gemm_quant(o_raw, o_info, &attention, &mut projected, o_in, o_out)
    {
        return None;
    }
    for (dst, residual) in hidden[..n_embd].iter_mut().zip(projected) {
        *dst += residual;
    }

    normed.copy_from_slice(&hidden[..n_embd]);
    let ffn_norm = tensors.ffn_norm.as_ref()?;
    if dequant_norm_row_into(mmap, index.tensor_data_start, ffn_norm, &mut norm_weight) < n_embd {
        return None;
    }
    rms_norm_inplace(&mut normed, &norm_weight, RMS_NORM_EPS);

    let gate_info = tensors.ffn_gate.as_ref()?;
    let up_info = tensors.ffn_up.as_ref()?;
    let down_info = tensors.ffn_down.as_ref()?;
    let gate_raw = fetch_tensor_bytes(mmap, index.tensor_data_start, gate_info).ok()?;
    let up_raw = fetch_tensor_bytes(mmap, index.tensor_data_start, up_info).ok()?;
    let down_raw = fetch_tensor_bytes(mmap, index.tensor_data_start, down_info).ok()?;
    let (gate_in, n_ffn) = QTensorEngine::matmul_dims(gate_info);
    let (up_in, up_out) = QTensorEngine::matmul_dims(up_info);
    let (down_in, down_out) = QTensorEngine::matmul_dims(down_info);
    if gate_in != n_embd
        || up_in != n_embd
        || up_out != n_ffn
        || down_in != n_ffn
        || down_out != n_embd
    {
        return None;
    }
    let mut gate = vec![0.0f32; n_ffn];
    let mut up = vec![0.0f32; n_ffn];
    if !stack_gemm_quant(gate_raw, gate_info, &normed, &mut gate, gate_in, n_ffn)
        || !stack_gemm_quant(up_raw, up_info, &normed, &mut up, up_in, n_ffn)
    {
        return None;
    }
    for (gate, up) in gate.iter_mut().zip(up) {
        *gate = (*gate / (1.0 + (-*gate).exp())) * up;
    }
    let mut down = vec![0.0f32; n_embd];
    if !stack_gemm_quant(down_raw, down_info, &gate, &mut down, down_in, down_out) {
        return None;
    }
    for (dst, residual) in hidden[..n_embd].iter_mut().zip(down) {
        *dst += residual;
    }
    Some(())
}

#[test]
#[serial_test::serial]
fn q8_whole_model_hidden_matches_cpu_at_position_zero() {
    let model_path = std::env::var("QUALIA_TEST_Q8_GGUF")
        .unwrap_or_else(|_| r"C:\LLM_Models\GGUF\smollm2-360m-instruct-q8_0.gguf".into());
    if !Path::new(&model_path).is_file() {
        eprintln!("whole-model Q8 parity skipped: model not found");
        return;
    }

    let mut cuda = QTensorEngine::new();
    cuda.load_model_checked(&model_path).unwrap();
    let mmap = cuda.gguf_mmap.clone().unwrap();
    let index = GgufTensorIndex::from_gguf(&mmap);
    let emb_dim = index.emb_dim();
    let mut reference = vec![0.0f32; emb_dim];
    assert_eq!(
        index.dequantize_token_embedding_into(&mmap, 64, &mut reference),
        emb_dim
    );
    assert!(cpu_layer_position_zero(&index, &mmap, 0, &mut reference).is_some());

    set_inference_mode(InferenceMode::CudaTc);
    std::env::set_var("QUALIA_CUDA_STAGE_DEBUG", "1");
    let mut actual = vec![0.0f32; emb_dim];
    assert_eq!(
        index.dequantize_token_embedding_into(&mmap, 64, &mut actual),
        emb_dim
    );
    assert!(cuda.try_prepared_cuda_hidden_for_test(&index, &mut actual, emb_dim, 0, 1,));
    set_inference_mode(InferenceMode::Portable);
    std::env::remove_var("QUALIA_CUDA_STAGE_DEBUG");
    std::env::remove_var("QUALIA_LLM_DEBUG_DECODE");

    let dot = reference
        .iter()
        .zip(&actual)
        .map(|(a, b)| a * b)
        .sum::<f32>();
    let reference_norm = reference.iter().map(|v| v * v).sum::<f32>().sqrt();
    let actual_norm = actual.iter().map(|v| v * v).sum::<f32>().sqrt();
    let cosine = dot / (reference_norm * actual_norm).max(f32::MIN_POSITIVE);
    let max_abs = reference
        .iter()
        .zip(&actual)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    assert!(
        cosine > 0.995,
        "first-layer hidden mismatch: cosine={cosine} max_abs={max_abs}"
    );

    for layer in 1..index.hyperparams.n_layer {
        assert!(
            cpu_layer_position_zero(&index, &mmap, layer, &mut reference).is_some(),
            "CPU oracle rejected layer {layer}"
        );
    }
    assert_eq!(
        index.dequantize_token_embedding_into(&mmap, 64, &mut actual),
        emb_dim
    );
    set_inference_mode(InferenceMode::CudaTc);
    assert!(cuda.try_prepared_cuda_hidden_for_test(
        &index,
        &mut actual,
        emb_dim,
        0,
        index.hyperparams.n_layer,
    ));
    set_inference_mode(InferenceMode::Portable);

    let dot = reference
        .iter()
        .zip(&actual)
        .map(|(a, b)| a * b)
        .sum::<f32>();
    let reference_norm = reference.iter().map(|v| v * v).sum::<f32>().sqrt();
    let actual_norm = actual.iter().map(|v| v * v).sum::<f32>().sqrt();
    let cosine = dot / (reference_norm * actual_norm).max(f32::MIN_POSITIVE);
    let max_abs = reference
        .iter()
        .zip(&actual)
        .map(|(a, b)| (a - b).abs())
        .fold(0.0f32, f32::max);
    let dp4a_swiglu_candidate = std::env::var("QUALIA_CUDA_Q8_DP4A_SWIGLU")
        .ok()
        .is_some_and(|value| matches!(value.as_str(), "1" | "true" | "on"))
        || std::env::var("QUALIA_CUDA_Q8_PROFILE").ok().as_deref() == Some("a2000-smollm2-q8-v1");
    let minimum_cosine = if dp4a_swiglu_candidate { 0.990 } else { 0.995 };
    eprintln!(
        "q8_whole_model_quality cosine={cosine:.8} minimum={minimum_cosine:.3} \
         max_abs={max_abs:.6}"
    );
    assert!(
        cosine > minimum_cosine,
        "whole-model hidden mismatch: cosine={cosine} minimum={minimum_cosine} max_abs={max_abs}"
    );

    let output_norm_info = index.output_norm_info().expect("output norm");
    let mut output_norm = vec![0.0f32; emb_dim];
    assert!(
        dequant_norm_row_into(
            &mmap,
            index.tensor_data_start,
            output_norm_info,
            &mut output_norm,
        ) >= emb_dim
    );
    let mut normalized = reference.clone();
    rms_norm_inplace(&mut normalized, &output_norm, RMS_NORM_EPS);
    let lm_info = index.logits_projection_info().expect("LM head");
    let lm_raw =
        fetch_tensor_bytes(&mmap, index.tensor_data_start, lm_info).expect("LM head bytes");
    let (lm_in, lm_out) = QTensorEngine::matmul_dims(lm_info);
    let mut logits = vec![0.0f32; lm_out];
    assert_eq!(lm_in, emb_dim);
    assert!(stack_gemm_quant(
        lm_raw,
        lm_info,
        &normalized,
        &mut logits,
        lm_in,
        lm_out,
    ));
    let expected_token = logits
        .iter()
        .enumerate()
        .max_by(|(left_index, left), (right_index, right)| {
            left.total_cmp(right)
                .then_with(|| right_index.cmp(left_index))
        })
        .map(|(index, _)| index as u32)
        .unwrap();

    set_inference_mode(InferenceMode::CudaTc);
    let actual_token = cuda
        .try_cuda_mega_pass_decode_token(&index, 64, &mut actual, emb_dim, 0)
        .expect("prepared CUDA token");
    set_inference_mode(InferenceMode::Portable);
    assert_eq!(
        actual_token, expected_token,
        "output RMSNorm/LM-head/argmax mismatch"
    );
}

#[test]
#[serial_test::serial]
fn q8_graph_replay_256_steps_are_zero_alloc_after_warmup() {
    let model_path = std::env::var("QUALIA_TEST_Q8_GGUF")
        .unwrap_or_else(|_| r"C:\LLM_Models\GGUF\smollm2-360m-instruct-q8_0.gguf".into());
    if !Path::new(&model_path).is_file() {
        eprintln!("Q8 graph zero-allocation test skipped: model not found");
        return;
    }
    std::env::remove_var("QUALIA_CUDA_STAGE_DEBUG");
    let mut engine = QTensorEngine::new();
    engine.load_model_checked(&model_path).unwrap();
    let mmap = engine.gguf_mmap.clone().unwrap();
    let index = GgufTensorIndex::from_gguf(&mmap);
    let emb_dim = index.emb_dim();
    let mut hidden = vec![0.0f32; emb_dim];
    // The global inference mode atomic can be raced by parallel non-serial
    // tests that call active_inference_mode() (which re-reads the env var
    // and overwrites the atomic). Re-set CudaTc immediately before the
    // plan-building call and retry a few times to win the race.
    let mut next_token = None;
    for _attempt in 0..8 {
        set_inference_mode(InferenceMode::CudaTc);
        next_token = engine.try_cuda_mega_pass_decode_token(&index, 64, &mut hidden, emb_dim, 0);
        if next_token.is_some() {
            break;
        }
        std::thread::yield_now();
    }
    let mut next_token = next_token.expect("warm graph capture");
    let mut generated = [0u32; 256];
    crate::specialized_libs::computational_geometry::allocation_counter::assert_zero_alloc(
        "q8_cuda_graph_replay_256_steps",
        || {
            for position in 1..=256u32 {
                next_token = engine
                    .try_cuda_mega_pass_decode_token(
                        &index,
                        next_token,
                        &mut hidden,
                        emb_dim,
                        position,
                    )
                    .expect("prepared graph replay");
                generated[position as usize - 1] = next_token;
            }
        },
    );
    set_inference_mode(InferenceMode::Portable);
    assert!(generated.iter().any(|token| *token != 0));
}
