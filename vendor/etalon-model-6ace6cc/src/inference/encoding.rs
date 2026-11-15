use super::TensorInfo;

/// Numerical encoding of the primary P64 payload. Unknown IDs remain opaque.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeightEncoding {
    IeeeF32,
    IeeeF16,
    BrainF16,
    Float8E4M3,
    BlockQuantized(u32),
    Ternary158,
    Opaque(u32),
}
impl TensorInfo {
    pub fn encoding(&self) -> WeightEncoding {
        use crate::ggml::*;
        match self.ggml_type {
            GGML_TYPE_F32 => WeightEncoding::IeeeF32,
            GGML_TYPE_F16 => WeightEncoding::IeeeF16,
            GGML_TYPE_BF16 => WeightEncoding::BrainF16,
            GGML_TYPE_FP8_E4M3 => WeightEncoding::Float8E4M3,
            GGML_TYPE_TERNARY_158 => WeightEncoding::Ternary158,
            id if ggml_block_layout(id).is_some() => WeightEncoding::BlockQuantized(id),
            id => WeightEncoding::Opaque(id),
        }
    }
}
