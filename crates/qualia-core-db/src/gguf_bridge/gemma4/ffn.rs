//! Mechanical extraction of the existing Gemma GEGLU and parallel expert graph.
//! Exact role admission and session state are supplied by the architecture owner.
use super::primitives::{gelu_pytorch_tanh, named_layer_tensor, unit_rms_norm_inplace};
use crate::gguf_bridge::*;

pub(super) fn execute(
    engine: &mut QTensorEngine,
    index: &crate::gguf_sharder::GgufTensorIndex,
    layer: u32,
    tensors: &crate::gguf_sharder::LayerTensors,
    hidden: &mut [f32],
    scratch_a: &mut [f32],
    scratch_b: &mut [f32],
    eps: f32,
) -> bool {
    let n_embd = index.hyperparams.n_embd as usize;
    let mut h_norm = [0.0; MAX_HIDDEN_DIM];
    // === 2. FFN Sub-Block (GEGLU dense + optional parallel MoE) ===
    h_norm[..n_embd].copy_from_slice(&hidden[..n_embd]);
    if let Some(fn_info) = tensors.ffn_norm.as_ref() {
        let Some(mmap) = engine.model_bytes() else {
            return false;
        };
        let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
        if dequant_norm_row_into(
            mmap,
            index.tensor_data_start,
            fn_info,
            &mut norm_w[..n_embd],
        ) < n_embd
        {
            return false;
        }
        rms_norm_inplace(&mut h_norm[..n_embd], &norm_w[..n_embd], eps);
    }

    let (Some(gate_info), Some(up_info), Some(down_info)) = (
        tensors.ffn_gate.as_ref(),
        tensors.ffn_up.as_ref(),
        tensors.ffn_down.as_ref(),
    ) else {
        return false;
    };
    let (gate_in, n_ffn) = QTensorEngine::matmul_dims(gate_info);
    let (up_in, up_out) = QTensorEngine::matmul_dims(up_info);
    let (dn_in, dn_out) = QTensorEngine::matmul_dims(down_info);
    if gate_in != n_embd
        || up_in != n_embd
        || up_out != n_ffn
        || dn_in != n_ffn
        || dn_out != n_embd
        || n_ffn > MAX_STACK_GEMM_DIM
    {
        return false;
    }
    let mut gate_buf = [0.0f32; MAX_STACK_GEMM_DIM];
    let mut up_buf = [0.0f32; MAX_STACK_GEMM_DIM];
    if !engine.dispatch_gemm_into(
        index,
        gate_info,
        &h_norm[..n_embd],
        &mut gate_buf[..n_ffn],
        n_embd,
        n_ffn,
    ) || !engine.dispatch_gemm_into(
        index,
        up_info,
        &h_norm[..n_embd],
        &mut up_buf[..n_ffn],
        n_embd,
        n_ffn,
    ) {
        return false;
    }
    for i in 0..n_ffn {
        gate_buf[i] = gelu_pytorch_tanh(gate_buf[i]) * up_buf[i];
    }
    if !engine.dispatch_gemm_into(
        index,
        down_info,
        &gate_buf[..n_ffn],
        &mut scratch_b[..n_embd],
        n_ffn,
        n_embd,
    ) {
        return false;
    }

    if let Some(router_info) = tensors.moe_router.as_ref() {
        let Some(mmap) = engine.model_bytes() else {
            return false;
        };
        if let Some(pfn1) = named_layer_tensor(index, layer, b"post_ffw_norm_1.weight") {
            let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(mmap, index.tensor_data_start, &pfn1, &mut norm_w[..n_embd])
                < n_embd
            {
                return false;
            }
            rms_norm_inplace(&mut scratch_b[..n_embd], &norm_w[..n_embd], eps);
        }
        let mut h_router = [0.0f32; MAX_HIDDEN_DIM];
        h_router[..n_embd].copy_from_slice(&hidden[..n_embd]);
        unit_rms_norm_inplace(&mut h_router[..n_embd], eps);
        if let Some(shexp_info) = tensors.moe_shared_gate_input.as_ref() {
            let mut shexp_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(
                mmap,
                index.tensor_data_start,
                shexp_info,
                &mut shexp_w[..n_embd],
            ) < n_embd
            {
                return false;
            }
            let inv_sqrt = 1.0f32 / (n_embd as f32).sqrt();
            for i in 0..n_embd {
                h_router[i] *= shexp_w[i] * inv_sqrt;
            }
        }
        let mut h_moe = [0.0f32; MAX_HIDDEN_DIM];
        h_moe[..n_embd].copy_from_slice(&hidden[..n_embd]);
        if let Some(pre2) = named_layer_tensor(index, layer, b"pre_ffw_norm_2.weight") {
            let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(mmap, index.tensor_data_start, &pre2, &mut norm_w[..n_embd])
                < n_embd
            {
                return false;
            }
            rms_norm_inplace(&mut h_moe[..n_embd], &norm_w[..n_embd], eps);
        }
        let actual_experts = router_info.dims[1] as usize;
        let Ok(routing) = index.moe_routing.admit(actual_experts) else {
            return false;
        };
        let num_experts = routing.expert_count().min(256);
        let topk = routing.experts_per_token().clamp(1, 16);
        if num_experts == 0 {
            return false;
        }
        let Ok(router_raw) =
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, router_info)
        else {
            return false;
        };
        let mut gate_logits = [0.0f32; 256];
        if !etalon_inference::ordinary::experts::compute_router_logits_from_gguf(
            router_raw,
            router_info,
            &h_router[..n_embd],
            num_experts,
            &mut gate_logits[..num_experts],
        ) {
            return false;
        }
        let mut expert_indices = [0u16; 16];
        let mut expert_weights = [0.0f32; 16];
        let Ok(selected) = crate::inference::moe::routing::route_topk(
            &gate_logits[..num_experts],
            topk,
            true,
            &mut expert_indices,
            &mut expert_weights,
        ) else {
            return false;
        };
        let (Some(gate_up), Some(down_exps)) = (
            tensors.moe_gate_up_exps.as_ref(),
            tensors.moe_down_exps.as_ref(),
        ) else {
            return false;
        };
        let fused_rows = gate_up.dims[1] as usize;
        let inter_dim = fused_rows / 2;
        if fused_rows == 0
            || fused_rows % 2 != 0
            || inter_dim > 4096
            || gate_up.dims[0] as usize != n_embd
            || down_exps.dims[0] as usize != inter_dim
            || down_exps.dims[1] as usize != n_embd
        {
            return false;
        }
        let (Ok(gu_raw), Ok(dn_raw)) = (
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, gate_up),
            crate::ggml_quants::fetch_tensor_bytes(mmap, index.tensor_data_start, down_exps),
        ) else {
            return false;
        };
        let mut down_exps_scale = [1.0f32; 256];
        if let Some(scale_info) = named_layer_tensor(index, layer, b"ffn_down_exps.scale") {
            let scale_len = (scale_info.dims[0] as usize).min(num_experts);
            if scale_len > 0 {
                let _ = dequant_norm_row_into(
                    mmap,
                    index.tensor_data_start,
                    &scale_info,
                    &mut down_exps_scale[..scale_len],
                );
            }
        }
        scratch_a[..n_embd].fill(0.0);
        let mut exp_outs = [[0.0f32; MAX_HIDDEN_DIM]; 16];
        let mut exp_ok = [true; 16];
        for sel in 0..selected {
            let out_buf = &mut exp_outs[sel];
            let ok_flag = &mut exp_ok[sel];
            let exp = expert_indices[sel] as usize;
            let w = expert_weights[sel] * down_exps_scale[exp.min(255)];
            let exp_gu_base = exp * fused_rows;
            let mut row = [0.0f32; MAX_HIDDEN_DIM];
            let mut geglu = [0.0f32; 4096];
            let mut expert_ok = true;
            for r in 0..inter_dim {
                if crate::ggml_quants::dequant_matrix_row_into(
                    gu_raw,
                    gate_up,
                    exp_gu_base + r,
                    &mut row[..n_embd],
                )
                .is_err()
                {
                    expert_ok = false;
                    break;
                }
                let g = etalon_inference::ordinary::experts::dot(&h_moe[..n_embd], &row[..n_embd]);
                if crate::ggml_quants::dequant_matrix_row_into(
                    gu_raw,
                    gate_up,
                    exp_gu_base + inter_dim + r,
                    &mut row[..n_embd],
                )
                .is_err()
                {
                    expert_ok = false;
                    break;
                }
                let u = etalon_inference::ordinary::experts::dot(&h_moe[..n_embd], &row[..n_embd]);
                geglu[r] = gelu_pytorch_tanh(g) * u;
            }
            if expert_ok {
                let exp_dn_base = exp * n_embd;
                for r in 0..n_embd {
                    if crate::ggml_quants::dequant_matrix_row_into(
                        dn_raw,
                        down_exps,
                        exp_dn_base + r,
                        &mut row[..inter_dim],
                    )
                    .is_err()
                    {
                        expert_ok = false;
                        break;
                    }
                    out_buf[r] = w * etalon_inference::ordinary::experts::dot(
                        &geglu[..inter_dim],
                        &row[..inter_dim],
                    );
                }
            }
            *ok_flag = expert_ok;
        }
        if exp_ok[..selected].iter().any(|&ok| !ok) {
            return false;
        }
        for sel in 0..selected {
            add_residual_inplace(&mut scratch_a[..n_embd], &exp_outs[sel][..n_embd], n_embd);
        }
        if let Some(pfn2) = named_layer_tensor(index, layer, b"post_ffw_norm_2.weight") {
            let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
            if dequant_norm_row_into(mmap, index.tensor_data_start, &pfn2, &mut norm_w[..n_embd])
                < n_embd
            {
                return false;
            }
            rms_norm_inplace(&mut scratch_a[..n_embd], &norm_w[..n_embd], eps);
        }
        add_residual_inplace(&mut scratch_b[..n_embd], &scratch_a[..n_embd], n_embd);
    }

    if let Some(pfn_info) = named_layer_tensor(index, layer, b"post_ffw_norm.weight") {
        let Some(mmap) = engine.model_bytes() else {
            return false;
        };
        let mut norm_w = [0.0f32; MAX_HIDDEN_DIM];
        if dequant_norm_row_into(
            mmap,
            index.tensor_data_start,
            &pfn_info,
            &mut norm_w[..n_embd],
        ) < n_embd
        {
            return false;
        }
        rms_norm_inplace(&mut scratch_b[..n_embd], &norm_w[..n_embd], eps);
    }
    add_residual_inplace(&mut hidden[..n_embd], &scratch_b[..n_embd], n_embd);

    hidden.iter().all(|v| v.is_finite())
}
