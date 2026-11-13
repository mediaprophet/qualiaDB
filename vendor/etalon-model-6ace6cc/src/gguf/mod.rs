//! Borrowed GGUF metadata traversal with explicit byte, nesting and work limits.
//!
//! A strict view validates the whole metadata directory; a KV cursor retains an
//! explicit error after a partial prefix. Neither interprets model semantics,
//! normalizes names, truncates shapes, nor requires tensor payload or padding.

mod cursor;
mod types;
mod view;

pub use cursor::KvCursor;
pub use types::{
    fnv1a64, Header, KvView, ParseError, ParseErrorKind, ParseLimits, TensorView, VersionPolicy,
    MAX_ARRAY_DEPTH,
};
pub use view::{MetadataView, TensorIter};

#[cfg(test)]
mod tests;

#[cfg(feature = "inference-model")]
pub use crate::inference::{
    parse_architecture_id, write_blk_tensor_name, write_hf_layer_tensor_name, GgufHeaderError,
    HyperparamsSource as GgufHyperparamsSource, LayerTensors, ModelHyperparams as GgufHyperparams,
    Qwen4ExpPleConfig, TensorIndex as GgufTensorIndex, TensorInfo as GgufTensorInfo,
    ARCH_DEEPSEEK_MOE, ARCH_FLAG_HAS_PLE, ARCH_FLAG_HAS_QK_NORM, ARCH_FLAG_HAS_SHARED_KV,
    ARCH_FLAG_HAS_SOFTCAP, ARCH_FLAG_HAS_SWA, ARCH_GEMMA, ARCH_GEMMA2, ARCH_GEMMA3, ARCH_GEMMA4,
    ARCH_GLM4, ARCH_GRANITE_HYBRID, ARCH_LLAMA, ARCH_OTHER, ARCH_PHI4, ARCH_QWEN2, ARCH_QWEN35_MOE,
    ARCH_QWEN4EXP, ARCH_UNKNOWN, DEFAULT_ROPE_FREQ_BASE, MAX_GGUF_HEADER_BYTES,
};
