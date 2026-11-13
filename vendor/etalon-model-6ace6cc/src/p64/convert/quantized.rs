use super::codecs::{ternary_blob, ternary_blob_len, GGML_TYPE_TERNARY_158};
use super::*;
#[derive(Clone, Copy, Debug)]
pub enum SafetensorQuantization {
    Verbatim,
    AllTernary,
    FfnTernary,
}
#[derive(Debug)]
pub struct QuantizedTensor<'a> {
    pub name: &'a str,
    pub shape: &'a [usize],
    pub begin: usize,
    pub end: usize,
    pub ggml_type: u32,
    pub role: Option<crate::tensor_roles::TensorRole>,
}
impl QuantizedTensor<'_> {
    fn byte_len(&self) -> usize {
        self.end - self.begin
    }
}
/// Decode a high-fidelity source tensor's bytes (`F32`/`F16`/`BF16`) to `f32` into `out` (cleared
/// and refilled). Cold-path (ingest) helper for the ternary transcode.
fn decode_safetensor_to_f32(raw: &[u8], ggml: u32, count: usize, out: &mut Vec<f32>) {
    use crate::ggml::{
        GGML_TYPE_BF16 as GGML_BF16, GGML_TYPE_F16 as GGML_F16, GGML_TYPE_F32 as GGML_F32,
    };
    out.clear();
    out.reserve(count);
    match ggml {
        GGML_F32 => {
            for k in 0..count {
                let o = k * 4;
                if o + 4 > raw.len() {
                    break;
                }
                out.push(f32::from_le_bytes([
                    raw[o],
                    raw[o + 1],
                    raw[o + 2],
                    raw[o + 3],
                ]));
            }
        }
        GGML_F16 => {
            for k in 0..count {
                let o = k * 2;
                if o + 2 > raw.len() {
                    break;
                }
                out.push(half::f16::from_le_bytes([raw[o], raw[o + 1]]).to_f32());
            }
        }
        GGML_BF16 => {
            for k in 0..count {
                let o = k * 2;
                if o + 2 > raw.len() {
                    break;
                }
                out.push(half::bf16::from_le_bytes([raw[o], raw[o + 1]]).to_f32());
            }
        }
        _ => {}
    }
}

pub fn transcode_quantized<W: std::io::Write>(
    src: &[u8],
    data_start: usize,
    tensors: &[QuantizedTensor<'_>],
    n_layer: u32,
    manifold_records: &[u8],
    budget: ConversionBudget,
    page_log2: u16,
    out: &mut W,
    policy: SafetensorQuantization,
) -> Result<TranscodeReport, String> {
    let page_log2 = if page_log2 == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        page_log2
    };
    if !(8..=30).contains(&page_log2) {
        return Err(format!("p64: page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    if tensors.is_empty() {
        return Err("p64: Safetensors source contains no tensors".into());
    }
    let names_bytes = names_len(tensors.iter().map(|t| t.name))?;
    let fixed_scratch = workspace_sum(&[
        names_bytes,
        tensors
            .len()
            .checked_mul(128)
            .ok_or("p64: workspace overflow")?,
    ])?;
    budget.scratch(fixed_scratch)?;
    let mut max_count = 0usize;
    for tensor in tensors {
        let count = tensor
            .shape
            .iter()
            .try_fold(1usize, |n, d| n.checked_mul(*d))
            .ok_or_else(|| format!("p64: tensor '{}' shape overflow", tensor.name))?;
        let end = data_start
            .checked_add(tensor.end)
            .ok_or("p64: source tensor length overflow")?;
        if tensor.begin > tensor.end || end > src.len() {
            return Err(format!(
                "p64: tensor '{}' source is out of bounds",
                tensor.name
            ));
        }
        let width = match tensor.ggml_type {
            0 => 4,
            1 | 30 => 2,
            _ => {
                return Err(format!(
                    "p64: tensor '{}' type is not a supported high-fidelity source",
                    tensor.name
                ))
            }
        };
        let bytes = count
            .checked_mul(width)
            .ok_or("p64: tensor byte-size overflow")?;
        if bytes != tensor.byte_len() {
            return Err(format!(
                "p64: tensor '{}' shape/dtype byte-size mismatch",
                tensor.name
            ));
        }
        max_count = max_count.max(count);
    }
    let source_types = tensors.iter().map(|t| t.ggml_type).collect::<Vec<_>>();
    let roles = tensors.iter().map(|t| t.role).collect::<Vec<_>>();
    let ternary_for = |position: usize| match policy {
        SafetensorQuantization::Verbatim => false,
        SafetensorQuantization::AllTernary => true,
        SafetensorQuantization::FfnTernary => roles[position]
            .map(|role| crate::tensor_roles::ternary_eligible(role.role))
            .unwrap_or(false),
    };

    let mut string_table = Vec::with_capacity(names_bytes);
    string_table.push(0);
    let mut name_offsets = Vec::with_capacity(tensors.len());
    for tensor in tensors {
        name_offsets.push(
            u32::try_from(string_table.len()).map_err(|_| "p64: string table exceeds 4 GiB")?,
        );
        string_table.extend_from_slice(tensor.name.as_bytes());
        string_table.push(0);
    }

    let hparams_offset = P64_WEIGHT_HEADER_BYTES;
    let tensor_table_offset = align_up(hparams_offset + 64, 64);
    let tensor_table_bytes = tensors
        .len()
        .checked_mul(P64_TENSOR_ENTRY_BYTES)
        .ok_or("p64: tensor table overflow")?;
    let string_table_offset = tensor_table_offset + tensor_table_bytes;
    let manifold_table_offset = align_up(string_table_offset + string_table.len(), 64);
    let manifold_count = n_layer as usize + 1;
    let manifold_bytes = manifold_count
        .checked_mul(P64_MANIFOLD_ENTRY_BYTES)
        .ok_or("p64: manifold table overflow")?;
    let tokenizer_offset = manifold_table_offset + manifold_bytes;
    let checksum_offset = align_up(tokenizer_offset, 64);
    let checksum_bytes = (tensors.len() + 1)
        .checked_mul(4)
        .ok_or("p64: checksum table overflow")?;
    let blob_region_offset = align_up(checksum_offset + checksum_bytes, page);

    let mut entries = Vec::with_capacity(tensors.len());
    let mut cursor = blob_region_offset;
    let mut largest_tensor_bytes = 0usize;
    let mut total_tensor_bytes = 0usize;
    let mut peak_working_bytes = 0usize;
    for (position, tensor) in tensors.iter().enumerate() {
        let element_count = tensor.shape.iter().copied().product::<usize>();
        let blob_size = if ternary_for(position) {
            ternary_blob_len(element_count)
        } else {
            tensor.byte_len()
        };
        cursor = align_up(cursor, page);
        let mapped = roles[position];
        let role_id = mapped.map(|role| role.role).unwrap_or(P64_ROLE_UNKNOWN);
        let manifold_idx = mapped
            .filter(|role| role.layer != P64_LAYER_GLOBAL)
            .map(|role| role.layer as u32)
            .unwrap_or(n_layer);
        let mut dimensions = [0u32; 4];
        for (target, source) in dimensions.iter_mut().zip(tensor.shape.iter().take(4)) {
            *target = u32::try_from(*source).map_err(|_| "p64: tensor dimension exceeds u32")?;
        }
        entries.push(P64TensorEntry {
            name_offset: name_offsets[position],
            role_id,
            dtype: if ternary_for(position) {
                GGML_TYPE_TERNARY_158 as u16
            } else {
                source_types[position] as u16
            },
            manifold_idx,
            rank: tensor.shape.len().clamp(1, 4) as u32,
            dimensions,
            blob_offset: u32::try_from(cursor).map_err(|_| "p64: container exceeds 4 GiB")?,
            blob_size: u32::try_from(blob_size).map_err(|_| "p64: tensor exceeds 4 GiB")?,
            source_offset: tensor.begin as u64,
            source_name_hash: q_hash(&tensor.name),
            alt_dtype: 0,
            precision_views_mask: 0,
            alt_blob_offset: 0,
        });
        cursor = cursor
            .checked_add(blob_size)
            .ok_or("p64: container size overflow")?;
        largest_tensor_bytes = largest_tensor_bytes.max(blob_size);
        total_tensor_bytes = total_tensor_bytes
            .checked_add(blob_size)
            .ok_or("p64: tensor byte total overflow")?;
        peak_working_bytes = peak_working_bytes.max(tensor.byte_len());
    }
    if cursor > u32::MAX as usize {
        return Err("p64: 32-bit relative-offset container exceeds 4 GiB".to_string());
    }

    let mut flags = P64_FLAG_LITTLE_ENDIAN | FORMAT_FLAG_RAW_TRANSCODE;
    if !matches!(policy, SafetensorQuantization::Verbatim) {
        flags |= FORMAT_FLAG_TERNARY;
    }
    let header = P64WeightHeader {
        magic: P64_MAGIC,
        version: P64_VERSION,
        flags,
        role_table_offset: 0,
        tensor_table_offset: tensor_table_offset as u32,
        tokenizer_offset: tokenizer_offset as u32,
        hparams_offset: hparams_offset as u32,
        string_table_offset: string_table_offset as u32,
        checksum_offset: checksum_offset as u32,
        manifold_table_offset: manifold_table_offset as u32,
        tensor_count: entries.len() as u32,
        page_size: page as u32,
        reserved: [0; 20],
    };
    let hparams = P64HParams {
        n_layer,
        n_embd: 0,
        n_head: 0,
        n_kv_head: 0,
        vocab_size: 0,
        rope_freq_base: 0.0,
        rope_scale: 0.0,
        head_dim: 0,
        head_dim_swa: 0,
        sliding_window: 0,
        shared_kv_layers: 0,
        logit_softcap: 0.0,
        architecture: 0,
        arch_flags: 0,
        reserved: [0; 8],
    };

    budget.output(cursor)?;
    let metadata_bytes = checksum_offset
        .checked_add(checksum_bytes)
        .ok_or("p64: workspace overflow")?;
    let fixed_scratch = workspace_sum(&[fixed_scratch, metadata_bytes])?;
    budget.scratch(workspace_sum(&[
        fixed_scratch,
        max_count.checked_mul(9).ok_or("p64: workspace overflow")?,
    ])?)?;
    let mut metadata = vec![0u8; checksum_offset + checksum_bytes];
    header.write_le(&mut metadata[..P64_WEIGHT_HEADER_BYTES]);
    hparams.write_le(&mut metadata[hparams_offset..hparams_offset + 64]);
    for (position, entry) in entries.iter().enumerate() {
        let start = tensor_table_offset + position * P64_TENSOR_ENTRY_BYTES;
        write_tensor_entry(entry, &mut metadata[start..start + P64_TENSOR_ENTRY_BYTES]);
    }
    metadata[string_table_offset..string_table_offset + string_table.len()]
        .copy_from_slice(&string_table);
    if manifold_records.len() != manifold_count * 64 {
        return Err("p64: manifold byte count mismatch".into());
    }
    metadata[manifold_table_offset..manifold_table_offset + manifold_records.len()]
        .copy_from_slice(manifold_records);

    let mut float_scratch = Vec::with_capacity(max_count);
    for (position, tensor) in tensors.iter().enumerate() {
        let source_start = data_start + tensor.begin;
        let source_end = data_start + tensor.end;
        let crc = if ternary_for(position) {
            let count = tensor.shape.iter().copied().product::<usize>();
            budget.scratch(workspace_sum(&[
                fixed_scratch,
                count.checked_mul(9).ok_or("p64: workspace overflow")?,
            ])?)?;
            decode_safetensor_to_f32(
                &src[source_start..source_end],
                source_types[position],
                count,
                &mut float_scratch,
            );
            if float_scratch.len() != count {
                return Err(format!(
                    "p64: tensor '{}' decode was incomplete",
                    tensor.name
                ));
            }
            let blob = ternary_blob(&float_scratch);
            peak_working_bytes = peak_working_bytes.max(blob.len());
            crc32c(&blob)
        } else {
            crc32c(&src[source_start..source_end])
        };
        let start = checksum_offset + 4 + position * 4;
        metadata[start..start + 4].copy_from_slice(&crc.to_le_bytes());
    }
    let metadata_crc = crc32c(&metadata[..checksum_offset]);
    metadata[checksum_offset..checksum_offset + 4].copy_from_slice(&metadata_crc.to_le_bytes());

    out.write_all(&metadata)
        .map_err(|error| error.to_string())?;
    let zeros = [0u8; 4096];
    let mut bytes_written = metadata.len();
    for (position, tensor) in tensors.iter().enumerate() {
        let target = entries[position].blob_offset as usize;
        while bytes_written < target {
            let count = (target - bytes_written).min(zeros.len());
            out.write_all(&zeros[..count])
                .map_err(|error| error.to_string())?;
            bytes_written += count;
        }
        let source_start = data_start + tensor.begin;
        let source_end = data_start + tensor.end;
        if ternary_for(position) {
            let count = tensor.shape.iter().copied().product::<usize>();
            budget.scratch(workspace_sum(&[
                fixed_scratch,
                count.checked_mul(9).ok_or("p64: workspace overflow")?,
            ])?)?;
            decode_safetensor_to_f32(
                &src[source_start..source_end],
                source_types[position],
                count,
                &mut float_scratch,
            );
            let blob = ternary_blob(&float_scratch);
            out.write_all(&blob).map_err(|error| error.to_string())?;
            bytes_written += blob.len();
        } else {
            out.write_all(&src[source_start..source_end])
                .map_err(|error| error.to_string())?;
            bytes_written += source_end - source_start;
        }
    }

    Ok(TranscodeReport {
        n_tensors: entries.len(),
        bytes_written,
        largest_tensor_bytes,
        total_tensor_bytes,
        peak_working_bytes,
    })
}
