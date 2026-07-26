//! P4 device attention: sticky-x QKV → RoPE → device KV write → GQA SDPA →
//! O-proj GEMV → single residual-delta readback.

use super::device::{ensure_device, ensure_sticky_x, ensure_weight_resident, multi_weight_device};
use super::paged_attention::{PAGED_GQA_TILED_ENTRY, PAGED_GQA_TILED_SRC};
use super::weight_cache::weight_fingerprint;

#[inline]
pub(crate) fn f32_bits_u32(v: f32) -> u32 {
    v.to_bits()
}

/// **P4 device attention:** sticky-x QKV (no intermediate D2H) → RoPE → device KV write
/// → GQA SDPA → O-proj GEMV → **one** residual-delta readback.
///
/// Dual-writes the current token's K/V into `host_kv` (same indices as
/// `KvCacheLayout`) so a later host fallback still sees a consistent cache.
/// Requires [`ensure_device_kv_cache`] to have succeeded for this layout.
pub fn try_q4k_soa_attention_device(
    n_embd: usize,
    n_head: usize,
    n_kv: usize,
    head_dim: usize,
    layer: u32,
    token_idx: u32,
    max_context: u32,
    layer_stride: u32,
    slot_kv_elems: u32,
    rope_base: f32,
    rope_scale: f32,
    x_normed: &[f32],
    q_raw: &[u8],
    k_raw: &[u8],
    v_raw: &[u8],
    o_raw: &[u8],
    host_kv: Option<&mut [f32]>,
    out_delta: &mut [f32],
) -> bool {
    use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA};
    use crate::wgsl_forge::dispatch::{caps, ensure_cuda_runtime_path};
    use crate::wgsl_forge::emit::cuda_c::{
        KV_SLOT_WRITE_ENTRY, KV_SLOT_WRITE_SRC, Q4K_SOA_GEMV_ENTRY, Q4K_SOA_GEMV_ROWS,
        Q4K_SOA_GEMV_SRC, Q4K_SOA_QKV_ENTRY, Q4K_SOA_QKV_SRC, ROPE_INTERLEAVED_ENTRY,
        ROPE_INTERLEAVED_SRC,
    };
    use crate::wgsl_forge::execute::{CudaPipeline, QualiaCompute};

    if !crate::inference_modes::prefer_tensor_core_gemm() {
        return false;
    }
    ensure_cuda_runtime_path();
    if !caps().cuda {
        return false;
    }
    let q_dim = n_head.saturating_mul(head_dim);
    let kv_dim = n_kv.saturating_mul(head_dim);
    if n_embd == 0
        || n_head == 0
        || n_kv == 0
        || head_dim == 0
        || head_dim > 256
        || n_kv > n_head
        || x_normed.len() < n_embd
        || out_delta.len() < n_embd
        || q_dim > 32_768
        || token_idx as u64 >= max_context as u64
        || max_context == 0
        || max_context > 4096
    {
        return false;
    }
    let Some(row_in) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, n_embd) else {
        return false;
    };
    let Some(row_o) = ggml_row_bytes(GGML_TYPE_Q4_K_SOA, q_dim) else {
        return false;
    };
    let need_q = row_in.saturating_mul(q_dim);
    let need_kv = row_in.saturating_mul(kv_dim);
    let need_o = row_o.saturating_mul(n_embd);
    if q_raw.len() < need_q
        || k_raw.len() < need_kv
        || v_raw.len() < need_kv
        || o_raw.len() < need_o
        || need_q > 48 * 1024 * 1024
        || need_o > 48 * 1024 * 1024
    {
        return false;
    }

    let kq = weight_fingerprint(q_raw, n_embd, q_dim);
    let kk = weight_fingerprint(k_raw, n_embd, kv_dim);
    let kvw = weight_fingerprint(v_raw, n_embd, kv_dim);
    let ko = weight_fingerprint(o_raw, q_dim, n_embd);

    let Ok(mut guard) = multi_weight_device().lock() else {
        return false;
    };
    if !ensure_device(&mut guard) {
        return false;
    }
    let dev = guard.as_mut().unwrap();
    let Some(mut view_kv) = dev.kv else {
        return false;
    };
    let Some(mut view_block_table) = dev.kv_block_table else {
        return false;
    };
    if dev.kv_total_f32 == 0
        || dev.kv_max_context != max_context
        || dev.kv_n_layer == 0
        || layer >= dev.kv_n_layer
        || dev.kv_n_kv_head != n_kv as u32
        || dev.kv_head_dim != head_dim as u32
        || dev.kv_layer_stride != layer_stride
        || dev.kv_slot_kv_elems != slot_kv_elems
        || dev.kv_block_size == 0
        || dev.kv_blocks_per_layer != max_context.div_ceil(dev.kv_block_size)
    {
        return false;
    }
    if !ensure_weight_resident(dev, kq, &q_raw[..need_q])
        || !ensure_weight_resident(dev, kk, &k_raw[..need_kv])
        || !ensure_weight_resident(dev, kvw, &v_raw[..need_kv])
        || !ensure_weight_resident(dev, ko, &o_raw[..need_o])
    {
        return false;
    }

    let mut view_x = match ensure_sticky_x(dev, &x_normed[..n_embd]) {
        Some(v) => v,
        None => return false,
    };
    view_x.binding = 0;
    let mut view_wq = *dev.weights.get(&kq).unwrap();
    let mut view_wk = *dev.weights.get(&kk).unwrap();
    let mut view_wv = *dev.weights.get(&kvw).unwrap();
    let mut view_wo = *dev.weights.get(&ko).unwrap();
    view_wq.binding = 1;
    view_wk.binding = 2;
    view_wv.binding = 3;

    // Transient: yq, yk, yv, attn_out, o_out, dims packs
    dev.ctx.restore_checkpoint(dev.permanent_end);
    let zq = vec![0.0f32; q_dim];
    let zk = vec![0.0f32; kv_dim];
    let zv = vec![0.0f32; kv_dim];
    let za = vec![0.0f32; q_dim];
    let zo = vec![0.0f32; n_embd];
    let mut view_yq = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zq), 4, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_yk = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zk), 5, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_yv = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zv), 6, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_attn = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&za), 2, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    let mut view_o = match dev.ctx.allocate_and_write(bytemuck::cast_slice(&zo), 2, 0) {
        Ok(v) => v,
        Err(_) => return false,
    };
    view_yq.binding = 4;
    view_yk.binding = 5;
    view_yv.binding = 6;

    let dims_qkv: [u32; 4] = [n_embd as u32, q_dim as u32, kv_dim as u32, row_in as u32];
    let view_dims_qkv = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims_qkv), 7, 0)
    {
        Ok(mut v) => {
            v.binding = 7;
            v
        }
        Err(_) => return false,
    };

    let schedule256 = crate::wgsl_forge::Schedule {
        workgroup_size: 256,
        ..Default::default()
    };

    // RoPE / KV / SDPA / O param packs allocated up front so each pipeline
    // (which borrows ctx) can be scoped and dropped before the next allocate.
    let rope_params_k: [u32; 5] = [
        n_kv as u32,
        head_dim as u32,
        token_idx,
        f32_bits_u32(rope_base),
        f32_bits_u32(if rope_scale > 0.0 && rope_scale.is_finite() {
            rope_scale
        } else {
            1.0
        }),
    ];
    let rope_params_q: [u32; 5] = [
        n_head as u32,
        head_dim as u32,
        token_idx,
        rope_params_k[3],
        rope_params_k[4],
    ];
    let view_rp_k = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&rope_params_k), 1, 0)
    {
        Ok(mut v) => {
            v.binding = 1;
            v
        }
        Err(_) => return false,
    };
    let view_rp_q = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&rope_params_q), 1, 0)
    {
        Ok(mut v) => {
            v.binding = 1;
            v
        }
        Err(_) => return false,
    };

    // --- 1) Fused QKV ---
    {
        let pipe_qkv = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            Q4K_SOA_QKV_SRC,
            Q4K_SOA_QKV_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|qkv_compile|{e:?}");
                return false;
            }
        };
        let n_q_blocks = q_dim.div_ceil(Q4K_SOA_GEMV_ROWS as usize).max(1);
        if let Err(e) = pipe_qkv.dispatch_async(
            &[
                view_x,
                view_wq,
                view_wk,
                view_wv,
                view_yq,
                view_yk,
                view_yv,
                view_dims_qkv,
            ],
            &schedule256,
            n_q_blocks.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|attn_dev|qkv_dispatch|{e:?}");
            return false;
        }
    }

    // --- 2) RoPE K + RoPE Q (in-place on device) ---
    {
        let pipe_rope = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            ROPE_INTERLEAVED_SRC,
            ROPE_INTERLEAVED_ENTRY,
            &[0, 1],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|rope_compile|{e:?}");
                return false;
            }
        };
        view_yk.binding = 0;
        let k_pairs = n_kv.saturating_mul(head_dim / 2).max(1);
        if let Err(e) = pipe_rope.dispatch_async(
            &[view_yk, view_rp_k],
            &schedule256,
            k_pairs.div_ceil(256).saturating_mul(256).max(256),
        ) {
            log::warn!("cuda_lane|attn_dev|rope_k|{e:?}");
            return false;
        }
        view_yq.binding = 0;
        let q_pairs = n_head.saturating_mul(head_dim / 2).max(1);
        if let Err(e) = pipe_rope.dispatch_async(
            &[view_yq, view_rp_q],
            &schedule256,
            q_pairs.div_ceil(256).saturating_mul(256).max(256),
        ) {
            log::warn!("cuda_lane|attn_dev|rope_q|{e:?}");
            return false;
        }
    }

    // --- 3) Device KV write (K then V) ---
    // Allocate all param packs before compiling (CudaPipeline borrows ctx).
    let slot = token_idx % max_context;
    let kvw_params_k: [u32; 7] = [
        n_kv as u32,
        head_dim as u32,
        layer,
        slot,
        layer_stride,
        slot_kv_elems,
        0,
    ];
    let kvw_params_v: [u32; 7] = [
        n_kv as u32,
        head_dim as u32,
        layer,
        slot,
        layer_stride,
        slot_kv_elems,
        1,
    ];
    let q_per_kv = (n_head / n_kv.max(1)).max(1) as u32;
    let scale = 1.0f32 / (head_dim as f32).sqrt();
    let sdpa_params: [u32; 8] = [
        n_head as u32,
        n_kv as u32,
        head_dim as u32,
        max_context,
        dev.kv_block_size,
        dev.kv_blocks_per_layer,
        slot_kv_elems,
        q_per_kv,
    ];
    let layer_id = [layer];
    let step = [token_idx, slot];
    let scale_bits = [f32_bits_u32(scale)];
    let dims_o: [u32; 3] = [q_dim as u32, n_embd as u32, row_o as u32];
    let view_kp = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&kvw_params_k), 2, 0)
    {
        Ok(mut v) => {
            v.binding = 2;
            v
        }
        Err(_) => return false,
    };
    let view_vp = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&kvw_params_v), 2, 0)
    {
        Ok(mut v) => {
            v.binding = 2;
            v
        }
        Err(_) => return false,
    };
    let view_sp = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&sdpa_params), 3, 0)
    {
        Ok(mut v) => {
            v.binding = 3;
            v
        }
        Err(_) => return false,
    };
    let view_sb = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&scale_bits), 4, 0)
    {
        Ok(mut v) => {
            v.binding = 4;
            v
        }
        Err(_) => return false,
    };
    let view_layer = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&layer_id), 5, 0)
    {
        Ok(mut v) => {
            v.binding = 5;
            v
        }
        Err(_) => return false,
    };
    let view_step = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&step), 6, 0)
    {
        Ok(mut v) => {
            v.binding = 6;
            v
        }
        Err(_) => return false,
    };
    let view_dims_o = match dev
        .ctx
        .allocate_and_write(bytemuck::cast_slice(&dims_o), 3, 0)
    {
        Ok(mut v) => {
            v.binding = 3;
            v
        }
        Err(_) => return false,
    };

    {
        let pipe_kvw = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            KV_SLOT_WRITE_SRC,
            KV_SLOT_WRITE_ENTRY,
            &[0, 1, 2],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|kvw_compile|{e:?}");
                return false;
            }
        };
        view_yk.binding = 0;
        view_kv.binding = 1;
        let kv_elems = kv_dim.max(1);
        if let Err(e) = pipe_kvw.dispatch_async(
            &[view_yk, view_kv, view_kp],
            &schedule256,
            kv_elems.div_ceil(256).saturating_mul(256).max(256),
        ) {
            log::warn!("cuda_lane|attn_dev|kv_write_k|{e:?}");
            return false;
        }
        view_yv.binding = 0;
        if let Err(e) = pipe_kvw.dispatch_async(
            &[view_yv, view_kv, view_vp],
            &schedule256,
            kv_elems.div_ceil(256).saturating_mul(256).max(256),
        ) {
            log::warn!("cuda_lane|attn_dev|kv_write_v|{e:?}");
            return false;
        }
    }

    // --- 4) Device SDPA ---
    {
        view_yq.binding = 0;
        view_kv.binding = 1;
        view_attn.binding = 2;
        view_block_table.binding = 7;
        let pipe_sdpa = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            PAGED_GQA_TILED_SRC,
            PAGED_GQA_TILED_ENTRY,
            &[0, 1, 2, 3, 4, 5, 6, 7],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|sdpa_compile|{e:?}");
                return false;
            }
        };
        // grid = n_head blocks × 256 threads
        if let Err(e) = pipe_sdpa.dispatch_async(
            &[
                view_yq,
                view_kv,
                view_attn,
                view_sp,
                view_sb,
                view_layer,
                view_step,
                view_block_table,
            ],
            &schedule256,
            n_head.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|attn_dev|sdpa_dispatch|{e:?}");
            return false;
        }
    }

    // --- 5) O-proj multi-row GEMV (attn stays on device) ---
    {
        view_attn.binding = 0;
        view_wo.binding = 1;
        view_o.binding = 2;
        let pipe_o = match CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            Q4K_SOA_GEMV_SRC,
            Q4K_SOA_GEMV_ENTRY,
            &[0, 1, 2, 3],
        ) {
            Ok(p) => p,
            Err(e) => {
                log::warn!("cuda_lane|attn_dev|o_compile|{e:?}");
                return false;
            }
        };
        let n_o_blocks = n_embd.div_ceil(Q4K_SOA_GEMV_ROWS as usize).max(1);
        // Last kernel before D2H: fenced QualiaCompute::dispatch so trait path is live.
        if let Err(e) = QualiaCompute::dispatch(
            &pipe_o,
            &[view_attn, view_wo, view_o, view_dims_o],
            &schedule256,
            n_o_blocks.saturating_mul(256),
        ) {
            log::warn!("cuda_lane|attn_dev|o_dispatch|{e:?}");
            return false;
        }
    }

    // --- 6) Single residual-delta readback ---
    let y = match dev.ctx.read_buffer_f32(&view_o) {
        Ok(v) => v,
        Err(e) => {
            log::warn!("cuda_lane|attn_dev|readback|{e:?}");
            return false;
        }
    };
    out_delta[..n_embd].copy_from_slice(&y[..n_embd]);

    // Dual-write host KV (small K/V after RoPE) for fallback consistency.
    if let Some(hkv) = host_kv {
        if let (Ok(k_host), Ok(v_host)) = (
            dev.ctx.read_buffer_f32(&view_yk),
            dev.ctx.read_buffer_f32(&view_yv),
        ) {
            let layer_base =
                layer as usize * layer_stride as usize + slot as usize * slot_kv_elems as usize * 2;
            let v_off = n_kv * head_dim;
            for i in 0..kv_dim {
                let ki = layer_base + i;
                let vi = layer_base + v_off + i;
                if ki < hkv.len() {
                    hkv[ki] = k_host[i];
                }
                if vi < hkv.len() {
                    hkv[vi] = v_host[i];
                }
            }
        }
    }
    true
}
