//! GGUF import compatibility over the canonical inference tensor directory.
pub use etalon_model::inference::{
    write_blk_tensor_name, write_hf_layer_tensor_name, GgufHeaderError, MAX_GGUF_HEADER_BYTES,
};
pub type GgufTensorIndex = etalon_model::inference::TensorIndex<super::GgufHyperparams>;
