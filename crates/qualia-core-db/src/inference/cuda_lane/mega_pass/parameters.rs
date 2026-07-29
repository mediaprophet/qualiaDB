//! Stable and per-token parameter packs for the captured decode graph.

use super::super::device::MultiWeightDevice;
use super::MegaPassLayerDims;
use crate::ggml_quants::ggml_row_bytes;
use crate::wgsl_forge::execute::memory::BufferView;

#[derive(Clone, Copy)]
pub(super) struct ParameterViews {
    pub step: BufferView,
    pub qkv: BufferView,
    pub rope_q: BufferView,
    pub rope_k: BufferView,
    pub kv_write: BufferView,
    pub sdpa: BufferView,
    pub sdpa_scale: BufferView,
    pub output_projection: BufferView,
    pub ffn: BufferView,
    pub down: BufferView,
    pub rms: BufferView,
    pub argmax: BufferView,
    pub lm_head: BufferView,
}

#[derive(Clone, Copy)]
pub(super) struct ParameterSpec {
    pub token_idx: u32,
    pub ring_slot: u32,
    pub input_token_id: u32,
    pub first_weight_key: u64,
    pub n_layer: u32,
    pub n_embd: usize,
    pub q_dim: usize,
    pub kv_dim: usize,
    pub n_head: usize,
    pub n_kv: usize,
    pub head_dim: usize,
    pub max_context: u32,
    pub slot_kv_elems: u32,
    pub rope_base_bits: u32,
    pub rope_scale_bits: u32,
    pub eps_bits: u32,
    pub sdpa_scale_bits: u32,
    pub attention_segments: u32,
    pub weight_type: u32,
    pub first_dims: MegaPassLayerDims,
    pub lm_head_in: usize,
    pub lm_head_out: usize,
}

pub(super) fn prepare_parameter_packs(
    dev: &mut MultiWeightDevice,
    views: ParameterViews,
    spec: ParameterSpec,
) -> Option<u64> {
    if let Err(error) = dev.ctx.write_view(
        &views.step,
        bytemuck::cast_slice(&[spec.token_idx, spec.ring_slot, spec.input_token_id]),
    ) {
        log::warn!("mega_pass|step_params|{error:?}");
        return None;
    }

    let params_key = spec.first_weight_key
        ^ (spec.n_layer as u64).rotate_left(7)
        ^ (spec.n_embd as u64).rotate_left(13)
        ^ (spec.first_dims.gate_out as u64).rotate_left(19)
        ^ (spec.rope_base_bits as u64).rotate_left(29)
        ^ (spec.rope_scale_bits as u64).rotate_left(37)
        ^ (spec.max_context as u64).rotate_left(43)
        ^ (spec.attention_segments as u64).rotate_left(47)
        ^ (dev.kv_block_size as u64).rotate_left(51);
    if dev.mega_params_key == params_key {
        return Some(params_key);
    }

    let q_per_kv = (spec.n_head / spec.n_kv.max(1)).max(1) as u32;
    let common_qkv = [
        spec.n_embd as u32,
        spec.q_dim as u32,
        spec.kv_dim as u32,
        ggml_row_bytes(spec.weight_type, spec.n_embd)? as u32,
        spec.eps_bits,
    ];
    let common_rope_q = [
        spec.n_head as u32,
        spec.head_dim as u32,
        0,
        spec.rope_base_bits,
        spec.rope_scale_bits,
    ];
    let common_rope_k = [
        spec.n_kv as u32,
        spec.head_dim as u32,
        0,
        spec.rope_base_bits,
        spec.rope_scale_bits,
    ];
    let common_kv = [
        spec.n_kv as u32,
        spec.head_dim as u32,
        dev.kv_block_size,
        dev.kv_blocks_per_layer,
        spec.slot_kv_elems,
    ];
    let common_sdpa = [
        spec.n_head as u32,
        spec.n_kv as u32,
        spec.head_dim as u32,
        spec.max_context,
        dev.kv_block_size,
        dev.kv_blocks_per_layer,
        spec.slot_kv_elems,
        q_per_kv,
        spec.attention_segments,
    ];
    let common_o = [
        spec.first_dims.o_in as u32,
        spec.first_dims.o_out as u32,
        ggml_row_bytes(spec.weight_type, spec.first_dims.o_in)? as u32,
    ];
    let common_ffn = [
        spec.first_dims.gate_in as u32,
        spec.first_dims.gate_out as u32,
        ggml_row_bytes(spec.weight_type, spec.first_dims.gate_in)? as u32,
        spec.eps_bits,
    ];
    let common_down = [
        spec.first_dims.down_in as u32,
        spec.first_dims.down_out as u32,
        ggml_row_bytes(spec.weight_type, spec.first_dims.down_in)? as u32,
    ];
    let common_sdpa_scale = [spec.sdpa_scale_bits];
    let common_output_norm = [spec.n_embd as u32, spec.eps_bits];
    let common_argmax = [spec.lm_head_out as u32];
    let static_writes = [
        (&views.qkv, bytemuck::cast_slice(&common_qkv)),
        (&views.rope_q, bytemuck::cast_slice(&common_rope_q)),
        (&views.rope_k, bytemuck::cast_slice(&common_rope_k)),
        (&views.kv_write, bytemuck::cast_slice(&common_kv)),
        (&views.sdpa, bytemuck::cast_slice(&common_sdpa)),
        (&views.sdpa_scale, bytemuck::cast_slice(&common_sdpa_scale)),
        (&views.output_projection, bytemuck::cast_slice(&common_o)),
        (&views.ffn, bytemuck::cast_slice(&common_ffn)),
        (&views.down, bytemuck::cast_slice(&common_down)),
        (&views.rms, bytemuck::cast_slice(&common_output_norm)),
        (&views.argmax, bytemuck::cast_slice(&common_argmax)),
    ];
    for (view, bytes) in static_writes {
        if let Err(error) = dev.ctx.write_view(view, bytes) {
            log::warn!("mega_pass|static_params|{error:?}");
            return None;
        }
    }
    if spec.lm_head_in > 0 {
        let lm_dims = [
            spec.lm_head_in as u32,
            spec.lm_head_out as u32,
            ggml_row_bytes(spec.weight_type, spec.lm_head_in)? as u32,
        ];
        if let Err(error) = dev
            .ctx
            .write_view(&views.lm_head, bytemuck::cast_slice(&lm_dims))
        {
            log::warn!("mega_pass|lm_static_params|{error:?}");
            return None;
        }
    }

    dev.mega_params_key = params_key;
    dev.decode_graph = None;
    dev.decode_graph_key = 0;
    dev.decode_graph_node_count = 0;
    dev.decode_graph_h2d_bytes_per_token = 0;
    Some(params_key)
}
