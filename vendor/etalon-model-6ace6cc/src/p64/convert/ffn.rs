use super::*;
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum FfnQuant {
    /// BitNet 1.58b ternary (`GGML_TYPE_TERNARY_158`, resident 2-bit GPU path).
    Ternary,
    /// ggml Q4_0 4-bit (`GGML_TYPE_Q4_0`, the standard quantized GPU GEMM path) — AWQ's design regime.
    Q4_0,
}

pub fn requantize_ffn(
    input: &[u8],
    base: Vec<u8>,
    source_data_start: usize,
    source_entries: &[(u64, GgufTensorInfo)],
    budget: ConversionBudget,
    awq_scales: Option<&[Vec<f32>]>,
    alpha: f32,
    quant: FfnQuant,
) -> Result<Vec<u8>, String> {
    use super::codecs::{q4_0_bytes, quantize_q4_0_from_f32};
    use super::codecs::{ternary_blob, ternary_blob_len, GGML_TYPE_TERNARY_158};
    use crate::ggml::GGML_TYPE_Q4_0;

    let view = super::super::P64View::parse(&base, super::super::IntegrityMode::Full)
        .map_err(|e| e.to_string())?;
    let mut header = *view.header();
    budget.scratch(
        view.tensor_count()
            .checked_mul(64)
            .ok_or("p64: workspace overflow")?,
    )?;
    let mut entries = (0..view.tensor_count())
        .map(|i| view.tensor(i).map(|v| v.entry()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    let fixed_scratch = workspace_sum(&[
        header.checksum_offset as usize,
        entries
            .len()
            .checked_mul(64)
            .ok_or("p64: workspace overflow")?,
    ])?;
    budget.scratch(fixed_scratch)?;
    let metadata = base[..header.checksum_offset as usize].to_vec();
    drop(base);
    let string_region =
        &metadata[header.string_table_offset as usize..header.manifold_table_offset as usize];
    let manifold_region =
        &metadata[header.manifold_table_offset as usize..header.tokenizer_offset as usize];
    let tokenizer_region =
        &metadata[header.tokenizer_offset as usize..header.checksum_offset as usize];

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

    budget.output(total_size)?;
    let mut output = vec![0u8; total_size];
    header.write_le(&mut output[..P64_WEIGHT_HEADER_BYTES]);
    let hparams_start = header.hparams_offset as usize;
    output[hparams_start..hparams_start + 64]
        .copy_from_slice(&metadata[hparams_start..hparams_start + 64]);
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
    let max_count = entries
        .iter()
        .filter(|e| is_ffn(e.role_id))
        .map(element_count)
        .try_fold(0usize, |n, c| c.map(|c| n.max(c)))?;
    budget.scratch(workspace_sum(&[
        fixed_scratch,
        max_count.checked_mul(9).ok_or("p64: workspace overflow")?,
    ])?)?;
    let mut scratch = Vec::<f32>::with_capacity(max_count);
    for (position, entry) in entries.iter().enumerate() {
        let source_entry = source_entries
            .iter()
            .find(|(_, info)| info.byte_offset == entry.source_offset)
            .map(|(_, info)| *info)
            .ok_or_else(|| format!("p64: source tensor {position} disappeared"))?;
        let source_size = crate::ggml::tensor_byte_len(&source_entry)
            .ok_or_else(|| format!("p64: source tensor {position} type is unsupported"))?;
        let source_start = source_data_start
            .checked_add(
                usize::try_from(source_entry.byte_offset)
                    .map_err(|_| "p64: source tensor offset overflow")?,
            )
            .ok_or("p64: source tensor offset overflow")?;
        let source_end = source_start
            .checked_add(source_size)
            .ok_or("p64: source tensor length overflow")?;
        if source_end > input.len() {
            return Err(format!("p64: source tensor {position} is out of bounds"));
        }
        let target_start = entry.blob_offset as usize;
        let target_end = target_start + entry.blob_size as usize;

        if is_ffn(entry.role_id) {
            let count = element_count(entry)?;
            budget.scratch(workspace_sum(&[
                fixed_scratch,
                count.checked_mul(9).ok_or("p64: workspace overflow")?,
            ])?)?;
            scratch.resize(count, 0.0);
            crate::ggml::dequantize_row_into(
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
                crate::ggml::GGML_TYPE_F32 if source_size >= count * 4 => {
                    for channel in 0..count {
                        let source = source_start + channel * 4;
                        let value =
                            f32::from_le_bytes(input[source..source + 4].try_into().unwrap())
                                / awq_scale(entry.manifold_idx, channel);
                        let target = target_start + channel * 4;
                        output[target..target + 4].copy_from_slice(&value.to_le_bytes());
                    }
                }
                crate::ggml::GGML_TYPE_F16 if source_size >= count * 2 => {
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
    super::super::P64View::parse(&output, super::super::IntegrityMode::Full)
        .map_err(|e| e.to_string())?;
    Ok(output)
}
