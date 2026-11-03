//! Preserved final FFN conversion and AWQ behavior.
use super::*;
/// Task #12 / STELLAR §A — like [`compile_gguf_to_p64`] but **ternary-packs the FFN projections**
/// (gate/up/down) during the compile, producing a **complete, runnable** P64: hyperparameters +
/// tokenizer are preserved (so the live loader boots it and builds the KV cache), while the FFN
/// tensors are BitNet-1.58b ternary blobs (`ternary::dequantize_blob` / the 2-bit GPU kernel).
/// Attention / norms / embeddings stay verbatim at their source precision. This is the loadable
/// container the live FFN-ternary dispatch path will run + measure against.
pub fn compile_gguf_to_p64_ternary_ffn(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, None, 0.0, FfnQuant::Ternary)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_ternary_ffn(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ternary_ffn(input, page_log2)
}

/// Target quantization for the FFN tensors in an AWQ P64 compile.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FfnQuant {
    /// BitNet 1.58b ternary (`GGML_TYPE_TERNARY_158`, resident 2-bit GPU path).
    Ternary,
    /// ggml Q4_0 4-bit (`GGML_TYPE_Q4_0`, the standard quantized GPU GEMM path) — AWQ's design regime.
    Q4_0,
}

/// AWQ-aware ternary FFN compile.
pub fn compile_gguf_to_p64_ternary_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, awq_scales, alpha, FfnQuant::Ternary)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_ternary_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ternary_ffn_awq(input, page_log2, awq_scales, alpha)
}

/// AWQ-aware **Q4_0** FFN compile (Path A) — FFN packed to 4-bit Q4_0 (AWQ's design regime); all else
/// verbatim from the source GGUF.
pub fn compile_gguf_to_p64_q4_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, awq_scales, alpha, FfnQuant::Q4_0)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_q4_ffn_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_q4_ffn_awq(input, page_log2, awq_scales, alpha)
}

/// AWQ-aware FFN-quantized P64 compile. `quant` selects the FFN target (ternary or Q4_0). When
/// `awq_scales` is `Some` (per-layer per-input-channel salience from [`crate::llm_awq::snapshot`]) the
/// gate/up input channel `i` is scaled by `s_i^alpha` before packing and `ffn_norm` is divided by
/// `s_i^alpha` — mathematically exact in f32 (`(X·norm/s^a)·(W·s^a)=(X·norm)·W`) — moving salient
/// channels into a range the quant grid represents better. `awq_scales = None` / `alpha == 0.0`
/// reproduces the plain (un-calibrated) compile. The down projection is left un-scaled (no clean fold
/// site — a v2 item). Everything outside the FFN passes through verbatim from the source GGUF.
pub fn compile_gguf_to_p64_ffn_quant_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
    quant: FfnQuant,
) -> Result<Vec<u8>, String> {
    use crate::ggml_quants::GGML_TYPE_Q4_0;
    use crate::llm_kernel_parity::{q4_0_bytes, quantize_q4_0_from_f32};
    use crate::ternary::{ternary_blob, ternary_blob_len, GGML_TYPE_TERNARY_158};

    let base = compile_gguf_to_p64(input, page_log2)?;
    let base_index = P64TensorIndex::from_p64(&base)?;
    let mut header = base_index.header;
    let hparams = base_index.hparams;
    let mut entries = base_index.entries;
    let string_region =
        base[header.string_table_offset as usize..header.manifold_table_offset as usize].to_vec();
    let manifold_region =
        base[header.manifold_table_offset as usize..header.tokenizer_offset as usize].to_vec();
    let tokenizer_region =
        base[header.tokenizer_offset as usize..header.checksum_offset as usize].to_vec();
    drop(base);

    let page = header.page_size as usize;
    let checksum_start = header.checksum_offset as usize;
    let checksum_bytes = (entries.len() + 1)
        .checked_mul(4)
        .ok_or("p64: checksum table overflow")?;
    let mut cursor = align_up(checksum_start + checksum_bytes, page);
    let is_ffn = |role: u16| {
        matches!(
            role,
            P64_ROLE_FFN_GATE | P64_ROLE_FFN_UP | P64_ROLE_FFN_DOWN
        )
    };
    let element_count = |entry: &P64TensorEntry| -> Result<usize, String> {
        entry.dimensions[..entry.rank as usize]
            .iter()
            .try_fold(1usize, |count, dimension| {
                count.checked_mul((*dimension).max(1) as usize)
            })
            .ok_or_else(|| "p64: tensor element count overflow".to_string())
    };

    for entry in &mut entries {
        let output_size = if is_ffn(entry.role_id) {
            let count = element_count(entry)?;
            match quant {
                FfnQuant::Ternary => {
                    entry.dtype = u16::try_from(GGML_TYPE_TERNARY_158)
                        .map_err(|_| "p64: ternary type exceeds u16")?;
                    ternary_blob_len(count)
                }
                FfnQuant::Q4_0 => {
                    entry.dtype =
                        u16::try_from(GGML_TYPE_Q4_0).map_err(|_| "p64: Q4_0 type exceeds u16")?;
                    q4_0_bytes(count)
                }
            }
        } else {
            entry.blob_size as usize
        };
        cursor = align_up(cursor, page);
        entry.blob_offset = u32::try_from(cursor).map_err(|_| "p64: container exceeds 4 GiB")?;
        entry.blob_size = u32::try_from(output_size).map_err(|_| "p64: tensor exceeds 4 GiB")?;
        cursor = cursor
            .checked_add(output_size)
            .ok_or("p64: output size overflow")?;
    }
    let total_size = align_up(cursor, 64);
    if total_size > u32::MAX as usize {
        return Err("p64: 32-bit relative-offset container exceeds 4 GiB".to_string());
    }
    if matches!(quant, FfnQuant::Ternary) {
        header.flags |= FORMAT_FLAG_TERNARY;
    } else {
        header.flags &= !FORMAT_FLAG_TERNARY;
    }

    let mut output = vec![0u8; total_size];
    header.write_le(&mut output[..P64_WEIGHT_HEADER_BYTES]);
    let hparams_start = header.hparams_offset as usize;
    hparams.write_le(&mut output[hparams_start..hparams_start + 64]);
    let tensor_table_start = header.tensor_table_offset as usize;
    for (position, entry) in entries.iter().enumerate() {
        let start = tensor_table_start + position * P64_TENSOR_ENTRY_BYTES;
        write_tensor_entry(entry, &mut output[start..start + P64_TENSOR_ENTRY_BYTES]);
    }
    let string_start = header.string_table_offset as usize;
    output[string_start..string_start + string_region.len()].copy_from_slice(&string_region);
    let manifold_start = header.manifold_table_offset as usize;
    output[manifold_start..manifold_start + manifold_region.len()]
        .copy_from_slice(&manifold_region);
    let tokenizer_start = header.tokenizer_offset as usize;
    output[tokenizer_start..tokenizer_start + tokenizer_region.len()]
        .copy_from_slice(&tokenizer_region);

    let source_index = GgufTensorIndex::from_gguf(input);
    let source_data_start = source_index.tensor_data_start as usize;
    let awq_enabled = awq_scales.is_some() && alpha != 0.0;
    let awq_scale = |layer: u32, channel: usize| -> f32 {
        awq_scales
            .and_then(|layers| layers.get(layer as usize))
            .and_then(|channels| channels.get(channel))
            .copied()
            .unwrap_or(1.0)
            .max(1e-6)
            .powf(alpha)
    };
    let mut scratch = Vec::<f32>::new();
    for (position, entry) in entries.iter().enumerate() {
        let source_entry = source_index
            .entries
            .iter()
            .find(|(_, info)| info.byte_offset == entry.source_offset)
            .map(|(_, info)| *info)
            .ok_or_else(|| format!("p64: source tensor {position} disappeared"))?;
        let source_size = crate::ggml_quants::tensor_byte_len(&source_entry)
            .ok_or_else(|| format!("p64: source tensor {position} type is unsupported"))?;
        let source_start = source_data_start + source_entry.byte_offset as usize;
        let source_end = source_start + source_size;
        if source_end > input.len() {
            return Err(format!("p64: source tensor {position} is out of bounds"));
        }
        let target_start = entry.blob_offset as usize;
        let target_end = target_start + entry.blob_size as usize;

        if is_ffn(entry.role_id) {
            let count = element_count(entry)?;
            scratch.resize(count, 0.0);
            crate::ggml_quants::dequantize_row_into(
                &input[source_start..source_end],
                source_entry.ggml_type,
                count,
                &mut scratch,
            )
            .map_err(|error| format!("p64: FFN dequantization failed: {error:?}"))?;
            if awq_enabled && matches!(entry.role_id, P64_ROLE_FFN_GATE | P64_ROLE_FFN_UP) {
                let n_in = entry.dimensions[0] as usize;
                let n_out = entry.dimensions[1] as usize;
                if n_in > 0 && n_in.saturating_mul(n_out) == count {
                    for output_channel in 0..n_out {
                        let row = output_channel * n_in;
                        for input_channel in 0..n_in {
                            scratch[row + input_channel] *=
                                awq_scale(entry.manifold_idx, input_channel);
                        }
                    }
                }
            }
            match quant {
                FfnQuant::Ternary => {
                    let blob = ternary_blob(&scratch);
                    if blob.len() != entry.blob_size as usize {
                        return Err("p64: ternary output length mismatch".to_string());
                    }
                    output[target_start..target_end].copy_from_slice(&blob);
                }
                FfnQuant::Q4_0 => {
                    if !quantize_q4_0_from_f32(&scratch, &mut output[target_start..target_end]) {
                        return Err(format!(
                            "p64: Q4_0 quantization failed for tensor {position}"
                        ));
                    }
                }
            }
        } else if awq_enabled && entry.role_id == P64_ROLE_FFN_NORM {
            let count = element_count(entry)?;
            match source_entry.ggml_type {
                crate::ggml_quants::GGML_TYPE_F32 if source_size >= count * 4 => {
                    for channel in 0..count {
                        let source = source_start + channel * 4;
                        let value =
                            f32::from_le_bytes(input[source..source + 4].try_into().unwrap())
                                / awq_scale(entry.manifold_idx, channel);
                        let target = target_start + channel * 4;
                        output[target..target + 4].copy_from_slice(&value.to_le_bytes());
                    }
                }
                crate::ggml_quants::GGML_TYPE_F16 if source_size >= count * 2 => {
                    for channel in 0..count {
                        let source = source_start + channel * 2;
                        let value =
                            half::f16::from_le_bytes(input[source..source + 2].try_into().unwrap())
                                .to_f32()
                                / awq_scale(entry.manifold_idx, channel);
                        let target = target_start + channel * 2;
                        output[target..target + 2]
                            .copy_from_slice(&half::f16::from_f32(value).to_le_bytes());
                    }
                }
                _ => {
                    return Err(format!(
                        "p64: AWQ cannot fold FFN norm type {} at manifold {}",
                        source_entry.ggml_type, entry.manifold_idx
                    ));
                }
            }
        } else {
            if source_size != entry.blob_size as usize {
                return Err(format!(
                    "p64: verbatim tensor {position} changed byte length"
                ));
            }
            output[target_start..target_end].copy_from_slice(&input[source_start..source_end]);
        }
        let crc = crc32c(&output[target_start..target_end]);
        let crc_start = checksum_start + 4 + position * 4;
        output[crc_start..crc_start + 4].copy_from_slice(&crc.to_le_bytes());
    }
    let metadata_crc = crc32c(&output[..checksum_start]);
    output[checksum_start..checksum_start + 4].copy_from_slice(&metadata_crc.to_le_bytes());
    P64TensorIndex::from_p64(&output)?;
    Ok(output)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42_ffn_quant_awq(
    input: &[u8],
    page_log2: u16,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
    quant: FfnQuant,
) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_ffn_quant_awq(input, page_log2, awq_scales, alpha, quant)
}
