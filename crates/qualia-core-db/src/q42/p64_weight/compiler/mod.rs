//! GGUF compiler policies and compatibility entrypoints.
use super::*;
use crate::gguf_sharder::{GgufTensorIndex, GgufTensorInfo};
mod converted;
mod ffn;
mod legacy;
mod names;
mod native_metadata;
pub use native_metadata::gguf_native_model_metadata;
mod planning;
mod verbatim;
pub use ffn::*;
use names::p64_tensor_name;
pub use planning::GgufCompileTensor;
pub use verbatim::*;
/// Conversion-time weight layout policy for [`compile_gguf_to_p64_with_layout`].
///
/// The designed product path is: import GGUF once → store GPU-friendly bytes in `.p64`.
/// [`P64ConvertLayout::Verbatim`] is the historical byte-preserving container swap
/// (same kernels, same speed). [`P64ConvertLayout::F16Expand`] dequantizes 2-D weight
/// matrices to IEEE f16 so decode can use the fast `unpack2x16float` path.
/// [`P64ConvertLayout::Q4kSoa`] rewrites Q4_K matrices to a 160 B/superblock SoA with
/// pre-expanded f16 sub-scales (decode GEMV skips scale unpack + header barriers).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum P64ConvertLayout {
    /// Copy GGML quant blocks byte-for-byte (no speed change vs running the GGUF).
    #[default]
    Verbatim,
    /// Expand 2-D matrices (attn/FFN/embd/output) to f16; leave 1-D norms as source.
    /// Rejected if the result would exceed the 4 GiB u32-offset container limit.
    F16Expand,
    /// Convert 2-D Q4_K weight matrices to [`crate::ggml_quants::GGML_TYPE_Q4_K_SOA`].
    /// Other tensors stay verbatim. ~11% larger than Q4_K; aimed at 3B-class decode.
    Q4kSoa,
}

/// Compile all source tensors with the original semantic order and metadata.
/// Verbatim payloads use Etalon's checked layer-packed P64 writer. A zero page
/// selector retains the 16 KiB default. Parsing retains its existing cold policy.
pub fn compile_gguf_to_p64(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_with_layout(input, page_log2, P64ConvertLayout::Verbatim)
}
/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64(input, page_log2)
}

/// Select the original numerical conversion policy. Verbatim uses Etalon serialization.
pub fn compile_gguf_to_p64_with_layout(
    input: &[u8],
    page_log2: u16,
    layout: P64ConvertLayout,
) -> Result<Vec<u8>, String> {
    if layout == P64ConvertLayout::Verbatim {
        verbatim::compile_compat(input, page_log2)
    } else {
        converted::compile_converted(input, page_log2, layout)
    }
}
