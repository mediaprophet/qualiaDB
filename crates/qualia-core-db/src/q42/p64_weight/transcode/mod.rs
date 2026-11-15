//! Safetensors and vision compatibility adapters for P64 conversion.
use super::*;
mod quantized;
mod verbatim;
mod vision;
use quantized::{transcode_safetensor_with_policy, SafetensorQuantization};
pub use verbatim::*;
pub use vision::*;

pub use etalon_model::p64::convert::TranscodeReport;

/// Transcodes a SafeTensor to the P64 weight stream.
pub fn transcode_safetensor_to_p64<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_with_policy(src, page_log2, out, SafetensorQuantization::Verbatim)
}

/// Task #12 / STELLAR §A — **streaming transcode with BitNet 1.58b ternary compression**:
/// safetensor (high-fidelity) → P64, each tensor quantized to `{-1,0,+1}` with a per-tensor
/// absmean scale and packed at ≈ 1.6 bits/weight (`ternary` module) *during* transcode.
///
/// Same streaming discipline as [`transcode_safetensor_to_p64`] (layout from the header; one tensor
/// in flight). Each blob is `[scale: f32][packed trits]` with `ggml_type =
/// ternary::GGML_TYPE_TERNARY_158`; the container carries `FORMAT_FLAG_TERNARY`. Decode with
/// `ternary::dequantize_blob`. Round-trips through [`P64TensorIndex::from_p64`].
pub fn transcode_safetensor_to_p64_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_with_policy(src, page_log2, out, SafetensorQuantization::AllTernary)
}

/// Task #12 / STELLAR §A — **policy transcode**: ternary the FFN projections, keep everything else
/// (attention, norms, embeddings) verbatim high-fidelity, in ONE P64. This is the real §A policy
/// (`tensor_roles::ternary_eligible`): ternarising attention/norms wrecks coherence, so only
/// `ffn_gate`/`ffn_up`/`ffn_down` are packed to 1.6 bits; the rest pass through unchanged.
///
/// Per tensor the manifest records the engine role (from the name) and `ggml_type =
/// ternary::GGML_TYPE_TERNARY_158` for ternary blobs (decode via `ternary::dequantize_blob`) or the
/// source GGML type for verbatim blobs. Round-trips through [`P64TensorIndex::from_p64`].
pub fn transcode_safetensor_to_p64_policy<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_with_policy(src, page_log2, out, SafetensorQuantization::FfnTernary)
}

/// Transcodes a SafeTensor FFN block to ternary P64.
pub fn transcode_safetensor_to_p64_ffn_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_to_p64_policy(src, page_log2, out)
}

/// Transcodes a SafeTensor FFN block to ternary in a Q42 volume.
pub fn transcode_safetensor_to_q42_ffn_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_to_p64_policy(src, page_log2, out)
}
