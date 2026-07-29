//! CUDA mega-pass: chain all transformer layers into one CUDA stream with a
//! single fence at the end. No per-layer D2H readback.

use super::attention::f32_bits_u32;
use super::device::{ensure_device, ensure_mega_pass_arena, multi_weight_device};
use super::q8::{Q8_0_EMBEDDING_LOOKUP_ENTRY, Q8_0_EMBEDDING_LOOKUP_SRC, Q8_0_GEMV_ROWS};
use super::tuning::cuda_q8_tuning_for_model;

mod attention_sdpa;
mod attention_stage;
mod ffn_stage;
mod output;
mod parameters;
mod plan;
mod types;

pub use types::{MegaPassLayerDims, MegaPassLayerWeights, MegaPassPlanView, MegaPassWeightLayout};
mod prepare;

pub use prepare::prepare_mega_pass_kernels;

/// Run from a caller-provided f32 embedding. Kept as the portable compatibility entry point.
pub fn try_cuda_mega_pass(
    n_embd: usize,
    n_head: usize,
    n_kv: usize,
    head_dim: usize,
    n_layer: u32,
    token_idx: u32,
    max_context: u32,
    layer_stride: u32,
    slot_kv_elems: u32,
    rope_base: f32,
    rope_scale: f32,
    rms_eps: f32,
    hidden: &mut [f32],
    plan: &impl MegaPassPlanView,
    output_norm_key: Option<u64>,
    lm_head_raw: Option<&[u8]>,
    lm_head_key: Option<u64>,
    lm_head_in: usize,
    lm_head_out: usize,
) -> Option<u32> {
    try_cuda_mega_pass_with_token(
        n_embd,
        n_head,
        n_kv,
        head_dim,
        n_layer,
        token_idx,
        max_context,
        layer_stride,
        slot_kv_elems,
        rope_base,
        rope_scale,
        rms_eps,
        hidden,
        plan,
        output_norm_key,
        lm_head_raw,
        lm_head_key,
        lm_head_in,
        lm_head_out,
        None,
    )
}

/// CUDA mega-pass: chain all transformer layers into one CUDA stream with a
/// **single fence** at the end. No per-layer D2H readback. The hidden state
/// stays on-device for the entire forward pass; only the final logits/token
/// come back to host.
///
/// Returns `Some(token_id)` on success, `None` to fall back to per-layer path.
///
/// Requirements:
/// - All weights must be Q4_K_SOA and pre-resident (caller preloads via
///   `preload_q4k_soa_weights`).
/// - Device KV cache must be initialized via `ensure_device_kv_cache`.
/// - `QUALIA_LLM_CUDA_DECODE` must be enabled.
pub(crate) fn try_cuda_mega_pass_with_token(
    n_embd: usize,
    n_head: usize,
    n_kv: usize,
    head_dim: usize,
    n_layer: u32,
    token_idx: u32,
    max_context: u32,
    layer_stride: u32,
    slot_kv_elems: u32,
    rope_base: f32,
    rope_scale: f32,
    rms_eps: f32,
    hidden: &mut [f32],
    plan: &impl MegaPassPlanView,
    output_norm_key: Option<u64>,
    lm_head_raw: Option<&[u8]>,
    lm_head_key: Option<u64>,
    lm_head_in: usize,
    lm_head_out: usize,
    input_token: Option<(u32, u64)>,
) -> Option<u32> {
    use crate::ggml_quants::{GGML_TYPE_Q4_K_SOA, GGML_TYPE_Q8_0};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{Q4K_SOA_GEMV_ROWS, Q4K_SOA_WMMA_GEMV_ROWS};
    use crate::wgsl_forge::execute::CudaPipeline;

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return None;
    }
    let tuning = cuda_q8_tuning_for_model(n_embd, n_head, n_kv, head_dim, n_layer, lm_head_out);
    let weight_layout = plan.weight_layout();
    let weight_type = match weight_layout {
        MegaPassWeightLayout::Q4KSoa => GGML_TYPE_Q4_K_SOA,
        MegaPassWeightLayout::Q8_0 => GGML_TYPE_Q8_0,
    };
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return None;
    }
    let q_dim = n_head.saturating_mul(head_dim);
    let kv_dim = n_kv.saturating_mul(head_dim);
    if n_embd == 0
        || n_head == 0
        || n_kv == 0
        || head_dim == 0
        || head_dim > 256
        || n_kv > n_head
        || hidden.len() < n_embd
        || plan.layer_count() < n_layer as usize
        || n_layer as usize > plan::MAX_MEGA_PASS_LAYERS
        || token_idx as u64 >= max_context as u64
        || max_context == 0
        || max_context > crate::gguf_bridge::MAX_CUDA_CONTEXT_WINDOW
    {
        return None;
    }
    let Ok(mut guard) = multi_weight_device().lock() else {
        return None;
    };
    if !ensure_device(&mut guard) {
        return None;
    }
    let dev = guard.as_mut().unwrap();
    let view_kv = dev.kv?;
    let view_block_table = dev.kv_block_table?;
    if dev.kv_total_f32 == 0
        || dev.kv_max_context != max_context
        || dev.kv_n_layer < n_layer
        || dev.kv_n_kv_head != n_kv as u32
        || dev.kv_head_dim != head_dim as u32
        || dev.kv_layer_stride != layer_stride
        || dev.kv_slot_kv_elems != slot_kv_elems
        || dev.kv_block_size == 0
        || dev.kv_blocks_per_layer != max_context.div_ceil(dev.kv_block_size)
    {
        return None;
    }

    // Plan construction has already fingerprinted and uploaded every immutable layer weight.
    let mut layer_keys = [plan::LayerWeightKeys::default(); plan::MAX_MEGA_PASS_LAYERS];
    plan::collect_layer_keys(
        dev,
        plan,
        weight_type,
        n_embd,
        n_layer as usize,
        &mut layer_keys,
    )?;

    // Sticky arena: allocate once (or validate existing), then overwrite in-place.
    let n_ffn = plan.layer_dims(0).map(|d| d.gate_out).unwrap_or(0);
    if n_ffn == 0 || n_ffn > 32_768 {
        return None;
    }
    let max_vocab = lm_head_out.max(1);
    if !ensure_mega_pass_arena(
        dev,
        n_embd,
        n_head,
        head_dim,
        q_dim,
        kv_dim,
        n_ffn,
        max_vocab,
        plan.layer_count(),
    ) {
        return None;
    }

    // Borrow the sticky arena fields.
    let arena = dev.mega_pass_arena.as_ref().unwrap();
    let mut view_residual = arena.hidden_a;
    let mut view_norm = arena.hidden_b;
    let view_yq = arena.yq;
    let view_yk = arena.yk;
    let view_yv = arena.yv;
    let view_attn = arena.attn_out;
    let view_attn_partials = arena.attn_partials;
    let view_ffn_mid = arena.ffn_mid;
    let view_q8_activation = arena.q8_activation;
    let view_q8_activation_scales = arena.q8_activation_scales;
    let mut view_norm_w = *dev.weights.get(&plan.layer_norm_keys(0)?[0])?;
    let view_p_rms = arena.p_rms;
    let mut view_p_qkv = arena.p_qkv;
    let view_p_rope = arena.p_rope;
    let view_p_rope_k = arena.p_rope_k;
    let view_p_kvw = arena.p_kvw;
    let view_p_sdpa = arena.p_sdpa;
    let view_p_sdpa_scale = arena.p_sdpa_scale;
    let view_p_gemv_dims = arena.p_gemv_dims;
    let view_p_ffn_dims = arena.p_ffn_dims;
    let view_p_down_dims = arena.p_down_dims;
    let view_logits = arena.logits;
    let view_token = arena.token;
    let view_p_argmax = arena.p_argmax;
    let view_p_lm_dims = arena.p_lm_dims;
    let mut view_p_step = arena.p_step;

    if let Some((_, embedding_key)) = input_token {
        if weight_layout != MegaPassWeightLayout::Q8_0 || !dev.weights.contains_key(&embedding_key)
        {
            log::warn!("mega_pass|device_embedding|ineligible");
            return None;
        }
    } else {
        // Compatibility path for callers that mutate the embedding on the host (for example,
        // CPU LoRA). Native token decode uses the resident lookup below and skips this upload.
        if let Err(e) = dev
            .ctx
            .write_view(&view_residual, bytemuck::cast_slice(&hidden[..n_embd]))
        {
            log::warn!("mega_pass|hidden_upload|{e:?}");
            return None;
        }
    }

    let schedule256 = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let wmma_rows = Q4K_SOA_WMMA_GEMV_ROWS as usize;

    let slot = token_idx % max_context;
    let attention_segments = super::paged_attention::segments_for_position(token_idx) as u32;
    let rope_base_bits = f32_bits_u32(rope_base);
    let rope_scale_bits = f32_bits_u32(if rope_scale > 0.0 && rope_scale.is_finite() {
        rope_scale
    } else {
        1.0
    });
    let eps_bits = f32_bits_u32(rms_eps);
    let sdpa_scale_bits = f32_bits_u32(1.0f32 / (head_dim as f32).sqrt());

    let rows = match weight_layout {
        MegaPassWeightLayout::Q4KSoa => Q4K_SOA_GEMV_ROWS as usize,
        MegaPassWeightLayout::Q8_0 => Q8_0_GEMV_ROWS,
    };
    let first_dims = *plan.layer_dims(0)?;
    if (1..n_layer as usize).any(|layer| plan.layer_dims(layer) != Some(&first_dims)) {
        log::debug!("mega_pass|skip|heterogeneous_layer_shapes");
        return None;
    }

    let params_key = parameters::prepare_parameter_packs(
        dev,
        parameters::ParameterViews {
            step: view_p_step,
            qkv: view_p_qkv,
            rope_q: view_p_rope,
            rope_k: view_p_rope_k,
            kv_write: view_p_kvw,
            sdpa: view_p_sdpa,
            sdpa_scale: view_p_sdpa_scale,
            output_projection: view_p_gemv_dims,
            ffn: view_p_ffn_dims,
            down: view_p_down_dims,
            rms: view_p_rms,
            argmax: view_p_argmax,
            lm_head: view_p_lm_dims,
        },
        parameters::ParameterSpec {
            token_idx,
            ring_slot: slot,
            input_token_id: input_token.map_or(0, |input| input.0),
            first_weight_key: layer_keys[0].kq,
            n_layer,
            n_embd,
            q_dim,
            kv_dim,
            n_head,
            n_kv,
            head_dim,
            max_context,
            slot_kv_elems,
            rope_base_bits,
            rope_scale_bits,
            eps_bits,
            sdpa_scale_bits,
            attention_segments,
            weight_type,
            first_dims,
            lm_head_in,
            lm_head_out,
        },
    )?;

    let graph_key = params_key
        ^ output_norm_key.unwrap_or(0).rotate_left(11)
        ^ lm_head_key.unwrap_or(0).rotate_left(23)
        ^ input_token
            .map(|(_, key)| key.rotate_left(31) ^ 0x454D_4245_4444_494E)
            .unwrap_or(0)
        ^ tuning.graph_fingerprint()
        ^ if weight_layout == MegaPassWeightLayout::Q8_0 {
            0x5138_5041_4745_4431
        } else {
            0
        };
    let graph_eligible = weight_layout == MegaPassWeightLayout::Q8_0
        && n_layer as usize == plan.layer_count()
        && output_norm_key.is_some()
        && lm_head_raw.is_some()
        && lm_head_key.is_some()
        && !tuning.stage_debug;
    if graph_eligible && dev.decode_graph_key == graph_key {
        if let Some(graph) = dev.decode_graph.as_ref() {
            if let Err(error) = dev.ctx.launch_graph(graph) {
                log::warn!("mega_pass|graph_replay|{error:?}");
                dev.decode_graph = None;
                dev.decode_graph_key = 0;
                dev.decode_graph_node_count = 0;
                dev.decode_graph_h2d_bytes_per_token = 0;
                return None;
            }
            let mut token = [0u32; 1];
            if let Err(error) = dev.ctx.read_buffer_u32_into(&view_token, &mut token) {
                log::warn!("mega_pass|graph_token_readback|{error:?}");
                return None;
            }
            return Some(token[0]);
        }
    }

    macro_rules! compile_pipe {
        ($src:expr, $entry:expr, $bindings:expr) => {
            CudaPipeline::compile_cuda_c_source_cached(&dev.ctx, $src, $entry, $bindings)
        };
    }

    let capturing_graph = graph_eligible && dev.decode_graph.is_none();
    if capturing_graph {
        if let Err(error) = dev.ctx.begin_graph_capture() {
            log::warn!("mega_pass|graph_begin|{error:?}");
            return None;
        }
    }

    if let Some((_, embedding_key)) = input_token {
        let mut view_embedding = *dev.weights.get(&embedding_key)?;
        view_embedding.binding = 0;
        view_residual.binding = 1;
        view_p_qkv.binding = 2;
        view_p_step.binding = 3;
        let embedding_pipe = compile_pipe!(
            Q8_0_EMBEDDING_LOOKUP_SRC,
            Q8_0_EMBEDDING_LOOKUP_ENTRY,
            &[0, 1, 2, 3]
        )
        .ok()?;
        if let Err(error) = embedding_pipe.dispatch_async_sorted(
            &[view_embedding, view_residual, view_p_qkv, view_p_step],
            &schedule256,
            n_embd.div_ceil(256).saturating_mul(256),
        ) {
            log::warn!("mega_pass|device_embedding|{error:?}");
            return None;
        }
    }

    for l in 0..n_layer as usize {
        let d = plan.layer_dims(l)?;
        let keys = &layer_keys[l];
        let view_layer_id = dev.mega_pass_arena.as_ref()?.p_layer_ids[l];

        if l != 0 {
            view_norm_w = *dev.weights.get(&plan.layer_norm_keys(l)?[0])?;
        }
        let (next_residual, next_norm) = attention_stage::dispatch_attention(
            dev,
            attention_stage::AttentionViews {
                residual: view_residual,
                norm: view_norm,
                norm_weight: view_norm_w,
                yq: view_yq,
                yk: view_yk,
                yv: view_yv,
                attention: view_attn,
                attention_partials: view_attn_partials,
                q8_activation: view_q8_activation,
                q8_activation_scales: view_q8_activation_scales,
                rms_params: view_p_rms,
                qkv_params: view_p_qkv,
                rope_q_params: view_p_rope,
                rope_k_params: view_p_rope_k,
                kv_write_params: view_p_kvw,
                sdpa_params: view_p_sdpa,
                sdpa_scale: view_p_sdpa_scale,
                output_projection_params: view_p_gemv_dims,
                layer_id: view_layer_id,
                step: view_p_step,
                kv: view_kv,
                block_table: view_block_table,
            },
            attention_stage::AttentionSpec {
                layer: l,
                n_embd,
                n_head,
                q_dim,
                kv_dim,
                rows,
                wmma_rows,
                weight_layout,
                dims: *d,
                keys: *keys,
                attention_segments: attention_segments as usize,
            },
            tuning,
        )?;
        view_residual = next_residual;
        view_norm = next_norm;

        let (next_residual, next_norm) = ffn_stage::dispatch_ffn(
            dev,
            ffn_stage::FfnViews {
                residual: view_residual,
                norm: view_norm,
                norm_weight: *dev.weights.get(&plan.layer_norm_keys(l)?[1])?,
                intermediate: view_ffn_mid,
                q8_activation: view_q8_activation,
                q8_activation_scales: view_q8_activation_scales,
                rms_params: view_p_rms,
                ffn_params: view_p_ffn_dims,
                down_params: view_p_down_dims,
            },
            ffn_stage::FfnSpec {
                layer: l,
                n_embd,
                rows,
                wmma_rows,
                weight_layout,
                dims: *d,
                keys: *keys,
            },
            tuning,
        )?;
        view_residual = next_residual;
        view_norm = next_norm;
    }

    return output::dispatch_output(
        dev,
        output::OutputViews {
            residual: view_residual,
            norm: view_norm,
            norm_weight: view_norm_w,
            q8_activation: view_q8_activation,
            q8_activation_scales: view_q8_activation_scales,
            rms_params: view_p_rms,
            lm_dims: view_p_lm_dims,
            logits: view_logits,
            token: view_token,
            argmax_params: view_p_argmax,
        },
        output::OutputSpec {
            n_embd,
            weight_layout,
            output_norm_key,
            lm_head_raw,
            lm_head_key,
            lm_head_in,
            lm_head_out,
            capturing_graph,
            graph_key,
            device_embedding: input_token.is_some(),
        },
        tuning,
        hidden,
    );
}

#[cfg(test)]
mod tests;
