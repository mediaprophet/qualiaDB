//! Host tile adapter for the canonical Etalon streamed MoE engine.
use crate::gguf_sharder::LayerTensors;
use super::{TrunkNvmeReader, QwenExpertTileReader};
pub use etalon_inference::qwen::streamed_moe::{execute_streamed_moe, select_top_experts, normalize_top_experts, StreamedMoeError, QWEN4EXP_EXPERT_COUNT, QWEN4EXP_TOP_EXPERTS};
#[allow(clippy::too_many_arguments)]
pub fn execute_streamed_moe_with_tiles(
    reader: &mut TrunkNvmeReader,
    tensors: &LayerTensors,
    input: &[f32],
    out: &mut [f32],
    router_logits: &mut [f32],
    top_indices: &mut [usize; QWEN4EXP_TOP_EXPERTS],
    top_logits: &mut [f32; QWEN4EXP_TOP_EXPERTS],
    top_weights: &mut [f32; QWEN4EXP_TOP_EXPERTS],
    cached_tiles: &[Option<&QwenExpertTileReader>; QWEN4EXP_TOP_EXPERTS],
    gate: &mut [f32],
    up: &mut [f32],
    activation: &mut [f32],
    expert_out: &mut [f32],
    raw_row: &mut [u8],
    dequantized_row: &mut [f32],
) -> Result<(), StreamedMoeError> {
    let cached = cached_tiles.map(|tile| tile.map(|reader| &**reader));
    etalon_inference::qwen::execute_streamed_moe_with_tiles(reader, tensors, input, out, router_logits, top_indices, top_logits, top_weights, &cached, gate, up, activation, expert_out, raw_row, dequantized_row)
}
