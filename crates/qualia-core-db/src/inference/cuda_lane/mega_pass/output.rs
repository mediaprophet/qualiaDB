//! Output normalization, logits projection, graph finalization, and token readback.

use super::super::device::MultiWeightDevice;
use super::super::q8::{
    Q8_0_DP4A_GEMV_ENTRY, Q8_0_DP4A_GEMV_SRC, Q8_0_GEMV_ENTRY, Q8_0_GEMV_ROWS, Q8_0_GEMV_SRC,
    Q8_ACTIVATION_QUANT_ENTRY, Q8_ACTIVATION_QUANT_SRC,
};
use super::super::tuning::CudaQ8Tuning;
use super::MegaPassWeightLayout;
use crate::ggml_quants::{ggml_row_bytes, GGML_TYPE_Q4_K_SOA, GGML_TYPE_Q8_0};
use crate::wgsl_forge::emit::cuda_c::{
    ARGMAX_F32_ENTRY, ARGMAX_F32_SRC, Q4K_SOA_GEMV_ENTRY, Q4K_SOA_GEMV_ROWS, Q4K_SOA_GEMV_SRC,
    Q4K_SOA_WMMA_GEMV_ENTRY, Q4K_SOA_WMMA_GEMV_ROWS, Q4K_SOA_WMMA_GEMV_SRC, RMSNORM_F32_ENTRY,
    RMSNORM_F32_SRC,
};
use crate::wgsl_forge::execute::memory::BufferView;
use crate::wgsl_forge::execute::CudaPipeline;
use crate::wgsl_forge::Schedule;

#[derive(Clone, Copy)]
pub(super) struct OutputViews {
    pub residual: BufferView,
    pub norm: BufferView,
    pub norm_weight: BufferView,
    pub q8_activation: BufferView,
    pub q8_activation_scales: BufferView,
    pub rms_params: BufferView,
    pub lm_dims: BufferView,
    pub logits: BufferView,
    pub token: BufferView,
    pub argmax_params: BufferView,
}

pub(super) struct OutputSpec<'a> {
    pub n_embd: usize,
    pub weight_layout: MegaPassWeightLayout,
    pub output_norm_key: Option<u64>,
    pub lm_head_raw: Option<&'a [u8]>,
    pub lm_head_key: Option<u64>,
    pub lm_head_in: usize,
    pub lm_head_out: usize,
    pub capturing_graph: bool,
    pub graph_key: u64,
    pub device_embedding: bool,
}

pub(super) fn dispatch_output(
    dev: &mut MultiWeightDevice,
    mut views: OutputViews,
    spec: OutputSpec<'_>,
    tuning: CudaQ8Tuning,
    hidden: &mut [f32],
) -> Option<u32> {
    let schedule256 = Schedule {
        workgroup_size: 256,
        ..Default::default()
    };
    let schedule128 = Schedule {
        workgroup_size: 128,
        ..Default::default()
    };
    let weight_type = match spec.weight_layout {
        MegaPassWeightLayout::Q4KSoa => GGML_TYPE_Q4_K_SOA,
        MegaPassWeightLayout::Q8_0 => GGML_TYPE_Q8_0,
    };
    let rows = match spec.weight_layout {
        MegaPassWeightLayout::Q4KSoa => Q4K_SOA_GEMV_ROWS as usize,
        MegaPassWeightLayout::Q8_0 => Q8_0_GEMV_ROWS,
    };

    if let Some(norm_key) = spec.output_norm_key {
        views.residual.binding = 0;
        views.norm_weight = *dev.weights.get(&norm_key)?;
        views.norm_weight.binding = 1;
        views.norm.binding = 2;
        views.rms_params.binding = 3;
        let pipe = CudaPipeline::compile_cuda_c_source_cached(
            &dev.ctx,
            RMSNORM_F32_SRC,
            RMSNORM_F32_ENTRY,
            &[0, 1, 2, 3],
        )
        .ok()?;
        if let Err(error) = pipe.dispatch_async_sorted(
            &[
                views.residual,
                views.norm_weight,
                views.norm,
                views.rms_params,
            ],
            &schedule256,
            256,
        ) {
            log::warn!("mega_pass|output_norm|{error:?}");
            return None;
        }
        views.residual = views.norm;
    }

    if let Some(lm_raw) = spec.lm_head_raw {
        if spec.lm_head_in > 0
            && spec.lm_head_out > 0
            && lm_raw.len() >= ggml_row_bytes(weight_type, spec.lm_head_in)? * spec.lm_head_out
        {
            let lm_key = spec.lm_head_key?;
            let mut lm_weight = match dev.weights.get(&lm_key) {
                Some(view) => *view,
                None => {
                    log::warn!("mega_pass|prepared_lm_head_missing");
                    return None;
                }
            };
            lm_weight.binding = 1;
            views.residual.binding = 0;
            views.lm_dims.binding = 3;
            views.logits.binding = 2;

            if spec.weight_layout == MegaPassWeightLayout::Q8_0 && tuning.dp4a_lm_head {
                views.q8_activation.binding = 1;
                views.q8_activation_scales.binding = 2;
                let quantizer = CudaPipeline::compile_cuda_c_source_cached(
                    &dev.ctx,
                    Q8_ACTIVATION_QUANT_SRC,
                    Q8_ACTIVATION_QUANT_ENTRY,
                    &[0, 1, 2, 3],
                )
                .ok()?;
                let quant_blocks = spec.lm_head_in.div_ceil(32);
                if let Err(error) = quantizer.dispatch_async_sorted(
                    &[
                        views.residual,
                        views.q8_activation,
                        views.q8_activation_scales,
                        views.lm_dims,
                    ],
                    &schedule256,
                    quant_blocks.div_ceil(8).saturating_mul(256),
                ) {
                    log::warn!("mega_pass|lm_head_quantize|{error:?}");
                    return None;
                }

                views.q8_activation.binding = 0;
                views.q8_activation_scales.binding = 1;
                lm_weight.binding = 2;
                views.logits.binding = 3;
                views.lm_dims.binding = 4;
                let pipe = CudaPipeline::compile_cuda_c_source_cached(
                    &dev.ctx,
                    Q8_0_DP4A_GEMV_SRC,
                    Q8_0_DP4A_GEMV_ENTRY,
                    &[0, 1, 2, 3, 4],
                )
                .ok()?;
                if let Err(error) = pipe.dispatch_async_sorted(
                    &[
                        views.q8_activation,
                        views.q8_activation_scales,
                        lm_weight,
                        views.logits,
                        views.lm_dims,
                    ],
                    &schedule128,
                    spec.lm_head_out.saturating_mul(128),
                ) {
                    log::warn!("mega_pass|dp4a_lm_head|{error:?}");
                    return None;
                }
            } else if spec.weight_layout == MegaPassWeightLayout::Q4KSoa
                && wmma_usable(spec.lm_head_in, spec.lm_head_out)
            {
                let blocks = spec
                    .lm_head_out
                    .div_ceil(Q4K_SOA_WMMA_GEMV_ROWS as usize)
                    .max(1);
                let pipe = CudaPipeline::compile_cuda_c_source_cached(
                    &dev.ctx,
                    Q4K_SOA_WMMA_GEMV_SRC,
                    Q4K_SOA_WMMA_GEMV_ENTRY,
                    &[0, 1, 2, 3],
                )
                .ok()?;
                if let Err(error) = pipe.dispatch_async_sorted(
                    &[views.residual, lm_weight, views.logits, views.lm_dims],
                    &schedule128,
                    blocks.saturating_mul(128),
                ) {
                    log::warn!("mega_pass|wmma_logits|{error:?}");
                    return None;
                }
            } else {
                let blocks = spec.lm_head_out.div_ceil(rows).max(1);
                let (source, entry) = match spec.weight_layout {
                    MegaPassWeightLayout::Q4KSoa => (Q4K_SOA_GEMV_SRC, Q4K_SOA_GEMV_ENTRY),
                    MegaPassWeightLayout::Q8_0 => (Q8_0_GEMV_SRC, Q8_0_GEMV_ENTRY),
                };
                let pipe = CudaPipeline::compile_cuda_c_source_cached(
                    &dev.ctx,
                    source,
                    entry,
                    &[0, 1, 2, 3],
                )
                .ok()?;
                if let Err(error) = pipe.dispatch_async_sorted(
                    &[views.residual, lm_weight, views.logits, views.lm_dims],
                    &schedule256,
                    blocks.saturating_mul(256),
                ) {
                    log::warn!("mega_pass|logits_gemv|{error:?}");
                    return None;
                }
            }

            views.logits.binding = 0;
            views.token.binding = 1;
            views.argmax_params.binding = 2;
            let argmax = CudaPipeline::compile_cuda_c_source_cached(
                &dev.ctx,
                ARGMAX_F32_SRC,
                ARGMAX_F32_ENTRY,
                &[0, 1, 2],
            )
            .ok()?;
            if let Err(error) = argmax.dispatch_async_sorted(
                &[views.logits, views.token, views.argmax_params],
                &schedule256,
                256,
            ) {
                log::warn!("mega_pass|argmax|{error:?}");
                return None;
            }

            if spec.capturing_graph {
                let graph = match dev.ctx.end_graph_capture() {
                    Ok(graph) => graph,
                    Err(error) => {
                        log::warn!("mega_pass|graph_end|{error:?}");
                        return None;
                    }
                };
                if let Err(error) = dev.ctx.launch_graph(&graph) {
                    log::warn!("mega_pass|graph_first_launch|{error:?}");
                    return None;
                }
                let nodes_per_token = match graph.node_count() {
                    Ok(count) => count as u64,
                    Err(error) => {
                        log::warn!("mega_pass|graph_node_count|{error:?}");
                        return None;
                    }
                };
                dev.decode_graph = Some(graph);
                dev.decode_graph_key = spec.graph_key;
                dev.decode_graph_node_count = nodes_per_token;
                dev.decode_graph_h2d_bytes_per_token = 12
                    + if spec.device_embedding {
                        0
                    } else {
                        (spec.n_embd * std::mem::size_of::<f32>()) as u64
                    };
                log::info!(
                    "mega_pass|graph_captured|key={:016x}|nodes_per_token={nodes_per_token}",
                    spec.graph_key
                );
            }

            let mut token = [0u32; 1];
            if let Err(error) = dev.ctx.read_buffer_u32_into(&views.token, &mut token) {
                log::warn!("mega_pass|argmax_readback|{error:?}");
                return None;
            }
            return Some(token[0]);
        }
    }

    log::debug!(
        "mega_pass|sentinel|no_soa_lm_head|reading_hidden|n_embd={}",
        spec.n_embd
    );
    let hidden_out = dev.ctx.read_buffer_f32(&views.residual).ok()?;
    if hidden_out.len() < spec.n_embd {
        log::warn!(
            "mega_pass|hidden_readback|short {} < {}",
            hidden_out.len(),
            spec.n_embd
        );
        return None;
    }
    hidden[..spec.n_embd].copy_from_slice(&hidden_out[..spec.n_embd]);
    Some(u32::MAX)
}

const fn wmma_usable(n_in: usize, n_out: usize) -> bool {
    n_in > 0 && n_out > 0 && n_in % 256 == 0 && n_out % 16 == 0
}
