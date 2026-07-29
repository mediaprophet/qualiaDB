//! Per-layer attention stage for the captured CUDA decode pass.

use super::super::device::MultiWeightDevice;
use super::super::q8::{
    q8_dp4a_qkv_rope_warp8_source, q8_gemv_resid_source, q8_qkv_rope_source,
    Q8_0_DP4A_GEMV_RESID_ENTRY, Q8_0_DP4A_GEMV_RESID_SRC, Q8_0_DP4A_QKV_ROPE_WARP8_ENTRY,
    Q8_0_GEMV_RESID_ENTRY, Q8_0_QKV_ROPE_ENTRY, Q8_ACTIVATION_QUANT_ENTRY, Q8_ACTIVATION_QUANT_SRC,
};
use super::super::tuning::CudaQ8Tuning;
use super::plan::LayerWeightKeys;
use super::{MegaPassLayerDims, MegaPassWeightLayout};
use crate::wgsl_forge::emit::cuda_c::{
    Q4K_SOA_GEMV_RESID_ENTRY, Q4K_SOA_GEMV_RESID_SRC, Q4K_SOA_WMMA_GEMV_RESID_ENTRY,
    Q4K_SOA_WMMA_GEMV_RESID_SRC, RMSNORM_F32_ENTRY, RMSNORM_F32_SRC,
};
use crate::wgsl_forge::emit::cuda_c_fused::{
    KV_SLOT_WRITE_BOTH_ENTRY, KV_SLOT_WRITE_BOTH_SRC, Q4K_SOA_RMSNORM_QKV_ROPE_ENTRY,
    Q4K_SOA_RMSNORM_QKV_ROPE_SRC,
};
use crate::wgsl_forge::execute::memory::BufferView;
use crate::wgsl_forge::execute::CudaPipeline;
use crate::wgsl_forge::Schedule;

#[derive(Clone, Copy)]
pub(super) struct AttentionViews {
    pub residual: BufferView,
    pub norm: BufferView,
    pub norm_weight: BufferView,
    pub yq: BufferView,
    pub yk: BufferView,
    pub yv: BufferView,
    pub attention: BufferView,
    pub attention_partials: BufferView,
    pub q8_activation: BufferView,
    pub q8_activation_scales: BufferView,
    pub rms_params: BufferView,
    pub qkv_params: BufferView,
    pub rope_q_params: BufferView,
    pub rope_k_params: BufferView,
    pub kv_write_params: BufferView,
    pub sdpa_params: BufferView,
    pub sdpa_scale: BufferView,
    pub output_projection_params: BufferView,
    pub layer_id: BufferView,
    pub step: BufferView,
    pub kv: BufferView,
    pub block_table: BufferView,
}

#[derive(Clone, Copy)]
pub(super) struct AttentionSpec {
    pub layer: usize,
    pub n_embd: usize,
    pub n_head: usize,
    pub q_dim: usize,
    pub kv_dim: usize,
    pub rows: usize,
    pub wmma_rows: usize,
    pub weight_layout: MegaPassWeightLayout,
    pub dims: MegaPassLayerDims,
    pub keys: LayerWeightKeys,
    pub attention_segments: usize,
}

pub(super) fn dispatch_attention(
    dev: &mut MultiWeightDevice,
    mut views: AttentionViews,
    spec: AttentionSpec,
    tuning: CudaQ8Tuning,
) -> Option<(BufferView, BufferView)> {
    let schedule256 = Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let schedule128 = Schedule {
        workgroup_size: 128,
        ..Default::default()
    };
    let mut q_weight = *dev.weights.get(&spec.keys.kq)?;
    let mut k_weight = *dev.weights.get(&spec.keys.kk)?;
    let mut v_weight = *dev.weights.get(&spec.keys.kvw)?;
    let q_blocks = spec.q_dim.div_ceil(spec.rows).max(1);

    match spec.weight_layout {
        MegaPassWeightLayout::Q4KSoa => {
            views.residual.binding = 0;
            views.norm_weight.binding = 1;
            q_weight.binding = 2;
            k_weight.binding = 3;
            v_weight.binding = 4;
            views.yq.binding = 5;
            views.yk.binding = 6;
            views.yv.binding = 7;
            views.qkv_params.binding = 8;
            views.rope_q_params.binding = 9;
            views.rope_k_params.binding = 10;
            views.step.binding = 11;
            let pipe = compile(
                dev,
                Q4K_SOA_RMSNORM_QKV_ROPE_SRC,
                Q4K_SOA_RMSNORM_QKV_ROPE_ENTRY,
                &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
            )?;
            if let Err(error) = pipe.dispatch_async_sorted(
                &[
                    views.residual,
                    views.norm_weight,
                    q_weight,
                    k_weight,
                    v_weight,
                    views.yq,
                    views.yk,
                    views.yv,
                    views.qkv_params,
                    views.rope_q_params,
                    views.rope_k_params,
                    views.step,
                ],
                &schedule256,
                q_blocks.saturating_mul(256),
            ) {
                log::warn!("mega_pass|rmsnorm_qkv_rope|layer={}|{error:?}", spec.layer);
                return None;
            }
        }
        MegaPassWeightLayout::Q8_0 => {
            views.residual.binding = 0;
            views.norm_weight.binding = 1;
            views.norm.binding = 2;
            views.rms_params.binding = 3;
            let norm = compile(dev, RMSNORM_F32_SRC, RMSNORM_F32_ENTRY, &[0, 1, 2, 3])?;
            if let Err(error) = norm.dispatch_async_sorted(
                &[
                    views.residual,
                    views.norm_weight,
                    views.norm,
                    views.rms_params,
                ],
                &schedule256,
                256,
            ) {
                log::warn!("mega_pass|attn_rmsnorm|layer={}|{error:?}", spec.layer);
                return None;
            }

            if tuning.dp4a_qkv {
                views.norm.binding = 0;
                views.q8_activation.binding = 1;
                views.q8_activation_scales.binding = 2;
                views.qkv_params.binding = 3;
                let quantizer = compile(
                    dev,
                    Q8_ACTIVATION_QUANT_SRC,
                    Q8_ACTIVATION_QUANT_ENTRY,
                    &[0, 1, 2, 3],
                )?;
                let quant_blocks = spec.dims.q_in.div_ceil(32);
                if let Err(error) = quantizer.dispatch_async_sorted(
                    &[
                        views.norm,
                        views.q8_activation,
                        views.q8_activation_scales,
                        views.qkv_params,
                    ],
                    &schedule256,
                    quant_blocks.div_ceil(8).saturating_mul(256),
                ) {
                    log::warn!("mega_pass|qkv_quantize|layer={}|{error:?}", spec.layer);
                    return None;
                }
                views.q8_activation.binding = 0;
                views.q8_activation_scales.binding = 1;
                q_weight.binding = 2;
                k_weight.binding = 3;
                v_weight.binding = 4;
                views.yq.binding = 5;
                views.yk.binding = 6;
                views.yv.binding = 7;
                views.qkv_params.binding = 8;
                views.rope_q_params.binding = 9;
                views.rope_k_params.binding = 10;
                views.step.binding = 11;
                let pipe = compile(
                    dev,
                    q8_dp4a_qkv_rope_warp8_source(),
                    Q8_0_DP4A_QKV_ROPE_WARP8_ENTRY,
                    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
                )?;
                if let Err(error) = pipe.dispatch_async_sorted(
                    &[
                        views.q8_activation,
                        views.q8_activation_scales,
                        q_weight,
                        k_weight,
                        v_weight,
                        views.yq,
                        views.yk,
                        views.yv,
                        views.qkv_params,
                        views.rope_q_params,
                        views.rope_k_params,
                        views.step,
                    ],
                    &schedule256,
                    q_blocks.saturating_mul(256),
                ) {
                    log::warn!("mega_pass|dp4a_qkv_rope|layer={}|{error:?}", spec.layer);
                    return None;
                }
            } else {
                views.norm.binding = 0;
                q_weight.binding = 1;
                k_weight.binding = 2;
                v_weight.binding = 3;
                views.yq.binding = 4;
                views.yk.binding = 5;
                views.yv.binding = 6;
                views.qkv_params.binding = 7;
                views.rope_q_params.binding = 8;
                views.rope_k_params.binding = 9;
                views.step.binding = 10;
                let pipe = compile(
                    dev,
                    q8_qkv_rope_source(),
                    Q8_0_QKV_ROPE_ENTRY,
                    &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
                )?;
                if let Err(error) = pipe.dispatch_async_sorted(
                    &[
                        views.norm,
                        q_weight,
                        k_weight,
                        v_weight,
                        views.yq,
                        views.yk,
                        views.yv,
                        views.qkv_params,
                        views.rope_q_params,
                        views.rope_k_params,
                        views.step,
                    ],
                    &schedule256,
                    q_blocks.saturating_mul(256),
                ) {
                    log::warn!("mega_pass|qkv_rope|layer={}|{error:?}", spec.layer);
                    return None;
                }
            }
        }
    }

    views.yk.binding = 0;
    views.yv.binding = 1;
    views.kv.binding = 2;
    views.kv_write_params.binding = 3;
    views.layer_id.binding = 4;
    views.step.binding = 5;
    views.block_table.binding = 6;
    let kv_write = compile(
        dev,
        KV_SLOT_WRITE_BOTH_SRC,
        KV_SLOT_WRITE_BOTH_ENTRY,
        &[0, 1, 2, 3, 4, 5, 6],
    )?;
    let kv_work = spec
        .kv_dim
        .max(1)
        .saturating_mul(2)
        .div_ceil(256)
        .saturating_mul(256)
        .max(256);
    if let Err(error) = kv_write.dispatch_async_sorted(
        &[
            views.yk,
            views.yv,
            views.kv,
            views.kv_write_params,
            views.layer_id,
            views.step,
            views.block_table,
        ],
        &schedule256,
        kv_work,
    ) {
        log::warn!("mega_pass|kv_write|layer={}|{error:?}", spec.layer);
        return None;
    }

    super::attention_sdpa::dispatch_sdpa(dev, views, spec, &schedule256)?;

    let mut output_weight = *dev.weights.get(&spec.keys.ko)?;
    views.attention.binding = 0;
    output_weight.binding = 1;
    views.norm.binding = 2;
    views.output_projection_params.binding = 3;
    views.residual.binding = 4;
    if spec.weight_layout == MegaPassWeightLayout::Q4KSoa
        && wmma_usable(spec.dims.o_in, spec.dims.o_out)
    {
        let pipe = compile(
            dev,
            Q4K_SOA_WMMA_GEMV_RESID_SRC,
            Q4K_SOA_WMMA_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4],
        )?;
        let blocks = spec.dims.o_out.div_ceil(spec.wmma_rows).max(1);
        if let Err(error) = pipe.dispatch_async_sorted(
            &[
                views.attention,
                output_weight,
                views.norm,
                views.output_projection_params,
                views.residual,
            ],
            &schedule128,
            blocks.saturating_mul(128),
        ) {
            log::warn!("mega_pass|wmma_o_proj|layer={}|{error:?}", spec.layer);
            return None;
        }
    } else if spec.weight_layout == MegaPassWeightLayout::Q8_0 && tuning.dp4a_o_projection {
        views.q8_activation.binding = 1;
        views.q8_activation_scales.binding = 2;
        let quantizer = compile(
            dev,
            Q8_ACTIVATION_QUANT_SRC,
            Q8_ACTIVATION_QUANT_ENTRY,
            &[0, 1, 2, 3],
        )?;
        let quant_blocks = spec.dims.o_in.div_ceil(32);
        if let Err(error) = quantizer.dispatch_async_sorted(
            &[
                views.attention,
                views.q8_activation,
                views.q8_activation_scales,
                views.output_projection_params,
            ],
            &schedule256,
            quant_blocks.div_ceil(8).saturating_mul(256),
        ) {
            log::warn!("mega_pass|o_proj_quantize|layer={}|{error:?}", spec.layer);
            return None;
        }
        views.q8_activation.binding = 0;
        views.q8_activation_scales.binding = 1;
        output_weight.binding = 2;
        views.norm.binding = 3;
        views.output_projection_params.binding = 4;
        views.residual.binding = 5;
        let pipe = compile(
            dev,
            Q8_0_DP4A_GEMV_RESID_SRC,
            Q8_0_DP4A_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4, 5],
        )?;
        if let Err(error) = pipe.dispatch_async_sorted(
            &[
                views.q8_activation,
                views.q8_activation_scales,
                output_weight,
                views.norm,
                views.output_projection_params,
                views.residual,
            ],
            &schedule128,
            spec.dims.o_out.saturating_mul(128),
        ) {
            log::warn!("mega_pass|dp4a_o_proj_resid|layer={}|{error:?}", spec.layer);
            return None;
        }
    } else {
        let (source, entry) = match spec.weight_layout {
            MegaPassWeightLayout::Q4KSoa => (Q4K_SOA_GEMV_RESID_SRC, Q4K_SOA_GEMV_RESID_ENTRY),
            MegaPassWeightLayout::Q8_0 => (q8_gemv_resid_source(), Q8_0_GEMV_RESID_ENTRY),
        };
        let pipe = compile(dev, source, entry, &[0, 1, 2, 3, 4])?;
        let blocks = spec.dims.o_out.div_ceil(spec.rows).max(1);
        if let Err(error) = pipe.dispatch_async_sorted(
            &[
                views.attention,
                output_weight,
                views.norm,
                views.output_projection_params,
                views.residual,
            ],
            &schedule256,
            blocks.saturating_mul(256),
        ) {
            log::warn!("mega_pass|o_proj_resid|layer={}|{error:?}", spec.layer);
            return None;
        }
    }

    std::mem::swap(&mut views.residual, &mut views.norm);
    if tuning.stage_debug && spec.layer < 3 {
        let stage = dev.ctx.read_buffer_f32(&views.residual).ok()?;
        let max_abs = stage[..spec.n_embd]
            .iter()
            .fold(0.0f32, |current, value| current.max(value.abs()));
        eprintln!(
            "[cuda-stage] layer={} post_attention_max_abs={max_abs}",
            spec.layer
        );
    }
    Some((views.residual, views.norm))
}

fn compile<'a>(
    dev: &'a MultiWeightDevice,
    source: &str,
    entry: &str,
    bindings: &[u32],
) -> Option<CudaPipeline<'a>> {
    CudaPipeline::compile_cuda_c_source_cached(&dev.ctx, source, entry, bindings).ok()
}

const fn wmma_usable(n_in: usize, n_out: usize) -> bool {
    n_in > 0 && n_out > 0 && n_in % 256 == 0 && n_out % 16 == 0
}
