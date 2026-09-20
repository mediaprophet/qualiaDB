//! Fail-closed tensor contract for the Qwen4Exp execution graph.
//!
//! A Qwen4Exp checkpoint is not executable merely because it has a PLE table.
//! Every layer must expose both Gated Residual paths, full MoE tensors, and
//! exactly one valid token mixer: GatedDeltaNet or Qwen Sparse Attention.

use crate::gguf_sharder::{GgufTensorIndex, ARCH_QWEN4EXP};

pub const QWEN4EXP_LAYER_HC_ATTN: u16 = 1 << 0;
pub const QWEN4EXP_LAYER_HC_FFN: u16 = 1 << 1;
pub const QWEN4EXP_LAYER_MOE: u16 = 1 << 2;
pub const QWEN4EXP_LAYER_GDN: u16 = 1 << 3;
pub const QWEN4EXP_LAYER_QSA: u16 = 1 << 4;
pub const QWEN4EXP_LAYER_PLE: u16 = 1 << 5;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Qwen4ExpLayerContract {
    pub layer: u32,
    pub flags: u16,
    pub reserved: u16,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Qwen4ExpGraphReport {
    pub layers: u32,
    pub gated_deltanet_layers: u32,
    pub qsa_layers: u32,
    pub ple_layers: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Qwen4ExpGraphError {
    WrongArchitecture,
    MissingPleTable,
    OutputTooSmall,
    MissingGatedResidual { layer: u32, ffn: bool },
    MissingMoe { layer: u32 },
    MissingTokenMixer { layer: u32 },
    AmbiguousTokenMixer { layer: u32 },
}

impl core::fmt::Display for Qwen4ExpGraphError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match *self {
            Self::WrongArchitecture => write!(f, "GGUF is not qwen4exp"),
            Self::MissingPleTable => write!(f, "qwen4exp GGUF is missing per-layer PLE table"),
            Self::OutputTooSmall => write!(f, "Qwen4Exp layer contract output is too small"),
            Self::MissingGatedResidual { layer, ffn } => write!(
                f,
                "Qwen4Exp layer {layer} is missing {} Gated Residual tensors",
                if ffn { "FFN" } else { "attention" }
            ),
            Self::MissingMoe { layer } => {
                write!(f, "Qwen4Exp layer {layer} is missing MoE tensors")
            }
            Self::MissingTokenMixer { layer } => write!(
                f,
                "Qwen4Exp layer {layer} has neither complete GatedDeltaNet nor QSA tensors"
            ),
            Self::AmbiguousTokenMixer { layer } => write!(
                f,
                "Qwen4Exp layer {layer} exposes both GatedDeltaNet and QSA token mixers"
            ),
        }
    }
}

impl std::error::Error for Qwen4ExpGraphError {}

/// Validate the exact tensor graph into caller-owned layer receipts.
///
/// This is cold activation work. It neither reads weight payloads nor allocates
/// per layer, and it makes unsupported/checkpoint-converted layouts fail before
/// the decode path can mistake them for a Llama transformer.
pub fn validate_qwen4exp_graph(
    index: &GgufTensorIndex,
    out: &mut [Qwen4ExpLayerContract],
) -> Result<Qwen4ExpGraphReport, Qwen4ExpGraphError> {
    if index.hyperparams.architecture != ARCH_QWEN4EXP {
        return Err(Qwen4ExpGraphError::WrongArchitecture);
    }
    if index.ple_ngram_embedding_info().is_none() {
        return Err(Qwen4ExpGraphError::MissingPleTable);
    }
    let layers = index.hyperparams.n_layer as usize;
    if out.len() < layers {
        return Err(Qwen4ExpGraphError::OutputTooSmall);
    }
    let mut report = Qwen4ExpGraphReport {
        layers: layers as u32,
        ..Qwen4ExpGraphReport::default()
    };
    for layer in 0..layers {
        let layer_index = layer as u32;
        let tensors = index.get_layer_tensors(layer_index);
        let attn_hc = tensors.has_qwen_hyper_connection(false);
        if !attn_hc {
            return Err(Qwen4ExpGraphError::MissingGatedResidual {
                layer: layer_index,
                ffn: false,
            });
        }
        let ffn_hc = tensors.has_qwen_hyper_connection(true);
        if !ffn_hc {
            return Err(Qwen4ExpGraphError::MissingGatedResidual {
                layer: layer_index,
                ffn: true,
            });
        }
        let moe = tensors.moe_router.is_some()
            && tensors.moe_gate_exps.is_some()
            && tensors.moe_up_exps.is_some()
            && tensors.moe_down_exps.is_some()
            && tensors.moe_shared_gate.is_some()
            && tensors.moe_shared_up.is_some()
            && tensors.moe_shared_down.is_some()
            && tensors.moe_shared_gate_input.is_some();
        if !moe {
            return Err(Qwen4ExpGraphError::MissingMoe { layer: layer_index });
        }
        let gdn = tensors.is_hybrid_ssm_layer();
        let qsa = tensors.has_qwen_sparse_attention();
        if gdn && qsa {
            return Err(Qwen4ExpGraphError::AmbiguousTokenMixer { layer: layer_index });
        }
        if !gdn && !qsa {
            return Err(Qwen4ExpGraphError::MissingTokenMixer { layer: layer_index });
        }
        let ple = tensors.has_qwen_ple();
        let mut flags = QWEN4EXP_LAYER_HC_ATTN | QWEN4EXP_LAYER_HC_FFN | QWEN4EXP_LAYER_MOE;
        if gdn {
            flags |= QWEN4EXP_LAYER_GDN;
            report.gated_deltanet_layers += 1;
        }
        if qsa {
            flags |= QWEN4EXP_LAYER_QSA;
            report.qsa_layers += 1;
        }
        if ple {
            flags |= QWEN4EXP_LAYER_PLE;
            report.ple_layers += 1;
        }
        out[layer] = Qwen4ExpLayerContract {
            layer: layer_index,
            flags,
            reserved: 0,
        };
    }
    Ok(report)
}
