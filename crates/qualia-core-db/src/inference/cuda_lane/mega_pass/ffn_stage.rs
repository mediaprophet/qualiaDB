//! Per-layer FFN/SwiGLU and down-projection stage.

use super::super::device::MultiWeightDevice;
use super::super::q8::{
    q8_dp4a_swiglu_source, q8_gemv_resid_source, q8_swiglu_source, Q8_0_DP4A_GEMV_RESID_ENTRY,
    Q8_0_DP4A_GEMV_RESID_SRC, Q8_0_DP4A_SWIGLU_ENTRY, Q8_0_GEMV_RESID_ENTRY, Q8_0_GEMV_ROWS,
    Q8_0_SWIGLU_ENTRY, Q8_ACTIVATION_QUANT_ENTRY, Q8_ACTIVATION_QUANT_SRC,
};
use super::super::tuning::CudaQ8Tuning;
use super::plan::LayerWeightKeys;
use super::{MegaPassLayerDims, MegaPassWeightLayout};
use crate::wgsl_forge::emit::cuda_c::{
    Q4K_SOA_GEMV_RESID_ENTRY, Q4K_SOA_GEMV_RESID_SRC, Q4K_SOA_WMMA_GEMV_RESID_ENTRY,
    Q4K_SOA_WMMA_GEMV_RESID_SRC, RMSNORM_F32_ENTRY, RMSNORM_F32_SRC,
};
use crate::wgsl_forge::emit::cuda_c_fused::{
    Q4K_SOA_RMSNORM_SWIGLU_ENTRY, Q4K_SOA_RMSNORM_SWIGLU_SRC,
};
use crate::wgsl_forge::execute::memory::BufferView;
use crate::wgsl_forge::execute::CudaPipeline;
use crate::wgsl_forge::Schedule;

#[derive(Clone, Copy)]
pub(super) struct FfnViews {
    pub residual: BufferView,
    pub norm: BufferView,
    pub norm_weight: BufferView,
    pub intermediate: BufferView,
    pub q8_activation: BufferView,
    pub q8_activation_scales: BufferView,
    pub rms_params: BufferView,
    pub ffn_params: BufferView,
    pub down_params: BufferView,
}

#[derive(Clone, Copy)]
pub(super) struct FfnSpec {
    pub layer: usize,
    pub n_embd: usize,
    pub rows: usize,
    pub wmma_rows: usize,
    pub weight_layout: MegaPassWeightLayout,
    pub dims: MegaPassLayerDims,
    pub keys: LayerWeightKeys,
}

pub(super) fn dispatch_ffn(
    dev: &mut MultiWeightDevice,
    mut views: FfnViews,
    spec: FfnSpec,
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
    let mut gate_weight = *dev.weights.get(&spec.keys.kg)?;
    let mut up_weight = *dev.weights.get(&spec.keys.ku)?;

    if spec.weight_layout == MegaPassWeightLayout::Q8_0 {
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
            log::warn!("mega_pass|ffn_rmsnorm|layer={}|{error:?}", spec.layer);
            return None;
        }

        if tuning.dp4a_swiglu_layer(spec.layer) {
            views.norm.binding = 0;
            views.q8_activation.binding = 1;
            views.q8_activation_scales.binding = 2;
            views.ffn_params.binding = 3;
            let quantizer = compile(
                dev,
                Q8_ACTIVATION_QUANT_SRC,
                Q8_ACTIVATION_QUANT_ENTRY,
                &[0, 1, 2, 3],
            )?;
            let quant_blocks = spec.dims.gate_in.div_ceil(32);
            if let Err(error) = quantizer.dispatch_async_sorted(
                &[
                    views.norm,
                    views.q8_activation,
                    views.q8_activation_scales,
                    views.ffn_params,
                ],
                &schedule256,
                quant_blocks.div_ceil(8).saturating_mul(256),
            ) {
                log::warn!("mega_pass|swiglu_quantize|layer={}|{error:?}", spec.layer);
                return None;
            }
            views.q8_activation.binding = 0;
            views.q8_activation_scales.binding = 1;
            gate_weight.binding = 2;
            up_weight.binding = 3;
            views.intermediate.binding = 4;
            views.ffn_params.binding = 5;
            let swiglu = compile(
                dev,
                q8_dp4a_swiglu_source(),
                Q8_0_DP4A_SWIGLU_ENTRY,
                &[0, 1, 2, 3, 4, 5],
            )?;
            if let Err(error) = swiglu.dispatch_async_sorted(
                &[
                    views.q8_activation,
                    views.q8_activation_scales,
                    gate_weight,
                    up_weight,
                    views.intermediate,
                    views.ffn_params,
                ],
                &schedule128,
                spec.dims.gate_out.saturating_mul(128),
            ) {
                log::warn!("mega_pass|dp4a_swiglu|layer={}|{error:?}", spec.layer);
                return None;
            }
        } else {
            views.norm.binding = 0;
            gate_weight.binding = 1;
            up_weight.binding = 2;
            views.intermediate.binding = 3;
            views.ffn_params.binding = 4;
            let swiglu = compile(dev, q8_swiglu_source(), Q8_0_SWIGLU_ENTRY, &[0, 1, 2, 3, 4])?;
            if let Err(error) = swiglu.dispatch_async_sorted(
                &[
                    views.norm,
                    gate_weight,
                    up_weight,
                    views.intermediate,
                    views.ffn_params,
                ],
                &schedule256,
                spec.dims
                    .gate_out
                    .div_ceil(Q8_0_GEMV_ROWS)
                    .saturating_mul(256),
            ) {
                log::warn!("mega_pass|swiglu|layer={}|{error:?}", spec.layer);
                return None;
            }
        }
    } else {
        views.residual.binding = 0;
        views.norm_weight.binding = 1;
        gate_weight.binding = 2;
        up_weight.binding = 3;
        views.intermediate.binding = 4;
        views.ffn_params.binding = 5;
        let swiglu = compile(
            dev,
            Q4K_SOA_RMSNORM_SWIGLU_SRC,
            Q4K_SOA_RMSNORM_SWIGLU_ENTRY,
            &[0, 1, 2, 3, 4, 5],
        )?;
        if let Err(error) = swiglu.dispatch_async_sorted(
            &[
                views.residual,
                views.norm_weight,
                gate_weight,
                up_weight,
                views.intermediate,
                views.ffn_params,
            ],
            &schedule256,
            spec.dims.gate_out.saturating_mul(256),
        ) {
            log::warn!("mega_pass|swiglu|layer={}|{error:?}", spec.layer);
            return None;
        }
    }

    let mut down_weight = *dev.weights.get(&spec.keys.kd)?;
    views.intermediate.binding = 0;
    down_weight.binding = 1;
    views.norm.binding = 2;
    views.down_params.binding = 3;
    views.residual.binding = 4;
    if spec.weight_layout == MegaPassWeightLayout::Q8_0 && tuning.dp4a_down_projection {
        views.q8_activation.binding = 1;
        views.q8_activation_scales.binding = 2;
        let quantizer = compile(
            dev,
            Q8_ACTIVATION_QUANT_SRC,
            Q8_ACTIVATION_QUANT_ENTRY,
            &[0, 1, 2, 3],
        )?;
        let quant_blocks = spec.dims.down_in.div_ceil(32);
        if let Err(error) = quantizer.dispatch_async_sorted(
            &[
                views.intermediate,
                views.q8_activation,
                views.q8_activation_scales,
                views.down_params,
            ],
            &schedule256,
            quant_blocks.div_ceil(8).saturating_mul(256),
        ) {
            log::warn!("mega_pass|down_quantize|layer={}|{error:?}", spec.layer);
            return None;
        }
        views.q8_activation.binding = 0;
        views.q8_activation_scales.binding = 1;
        down_weight.binding = 2;
        views.norm.binding = 3;
        views.down_params.binding = 4;
        views.residual.binding = 5;
        let down = compile(
            dev,
            Q8_0_DP4A_GEMV_RESID_SRC,
            Q8_0_DP4A_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4, 5],
        )?;
        if let Err(error) = down.dispatch_async_sorted(
            &[
                views.q8_activation,
                views.q8_activation_scales,
                down_weight,
                views.norm,
                views.down_params,
                views.residual,
            ],
            &schedule128,
            spec.dims.down_out.saturating_mul(128),
        ) {
            log::warn!("mega_pass|dp4a_down_resid|layer={}|{error:?}", spec.layer);
            return None;
        }
    } else if spec.weight_layout == MegaPassWeightLayout::Q4KSoa
        && wmma_usable(spec.dims.down_in, spec.dims.down_out)
    {
        let down = compile(
            dev,
            Q4K_SOA_WMMA_GEMV_RESID_SRC,
            Q4K_SOA_WMMA_GEMV_RESID_ENTRY,
            &[0, 1, 2, 3, 4],
        )?;
        let blocks = spec.dims.down_out.div_ceil(spec.wmma_rows).max(1);
        if let Err(error) = down.dispatch_async_sorted(
            &[
                views.intermediate,
                down_weight,
                views.norm,
                views.down_params,
                views.residual,
            ],
            &schedule128,
            blocks.saturating_mul(128),
        ) {
            log::warn!("mega_pass|wmma_down_resid|layer={}|{error:?}", spec.layer);
            return None;
        }
    } else {
        let (source, entry) = match spec.weight_layout {
            MegaPassWeightLayout::Q4KSoa => (Q4K_SOA_GEMV_RESID_SRC, Q4K_SOA_GEMV_RESID_ENTRY),
            MegaPassWeightLayout::Q8_0 => (q8_gemv_resid_source(), Q8_0_GEMV_RESID_ENTRY),
        };
        let down = compile(dev, source, entry, &[0, 1, 2, 3, 4])?;
        let blocks = spec.dims.down_out.div_ceil(spec.rows).max(1);
        if let Err(error) = down.dispatch_async_sorted(
            &[
                views.intermediate,
                down_weight,
                views.norm,
                views.down_params,
                views.residual,
            ],
            &schedule256,
            blocks.saturating_mul(256),
        ) {
            log::warn!("mega_pass|down_resid|layer={}|{error:?}", spec.layer);
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
            "[cuda-stage] layer={} post_ffn_max_abs={max_abs}",
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
