//! Streaming **safetensor -> P64 transcode**: one tensor in flight (peak working memory ≈ the
//! largest single tensor), verbatim / all-ternary / FFN-ternary policies, the `TranscodeReport`,
//! and the historical `_q42` alias.

use super::*;
use crate::container_10d::crc32c::crc32c;

/// Decode a high-fidelity source tensor's bytes (`F32`/`F16`/`BF16`) to `f32` into `out` (cleared
/// and refilled). Cold-path (ingest) helper for the ternary transcode.
fn decode_safetensor_to_f32(raw: &[u8], ggml: u32, count: usize, out: &mut Vec<f32>) {
    use crate::safetensor::{GGML_BF16, GGML_F16, GGML_F32};
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

/// Outcome of a streaming transcode — the numbers that make the memory claim falsifiable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TranscodeReport {
    pub n_tensors: usize,
    pub bytes_written: usize,
    pub largest_tensor_bytes: usize,
    pub total_tensor_bytes: usize,
    /// High-water mark of the transcoder's working buffer — one tensor in flight, so ≈ the largest
    /// single tensor, **never** the whole file.
    pub peak_working_bytes: usize,
}

/// Phase 6 / task #12 — **streaming, versioned transcode: safetensor (high-fidelity) → P64**.
///
/// Writes a valid P64 container to `out` forward-only (round-trips through [`P64TensorIndex::from_p64`]).
/// The full layout is computed from the safetensor *header* alone (no tensor reads), so each tensor's
/// bytes pass through **one reused scratch buffer** — the transcoder's peak working memory is ≈ the
/// largest single tensor, not the whole file. On the real path `src` is an `mmap` (demand-paged by
/// the OS), so the file is never resident in full.
///
/// Rejects low-precision (`Q4`-class) inputs — high-fidelity (`F32/F16/BF16/Q8`) only. The legacy
/// `compile_gguf_to_q42` path is untouched (GGUF support unchanged).
#[derive(Clone, Copy)]
enum SafetensorQuantization {
    Verbatim,
    AllTernary,
    FfnTernary,
}

fn transcode_safetensor_with_policy<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
    policy: SafetensorQuantization,
) -> Result<TranscodeReport, String> {
    use crate::safetensor::{
        ggml_elem_bytes, is_high_fidelity_ggml, parse_safetensor_header, safetensor_dtype_to_ggml,
    };
    use crate::ternary::{ternary_blob, ternary_blob_len, GGML_TYPE_TERNARY_158};
    let page_log2 = if page_log2 == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        page_log2
    };
    if !(8..=30).contains(&page_log2) {
        return Err(format!("p64: page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    let plan = parse_safetensor_header(src)?;
    if plan.tensors.is_empty() {
        return Err("p64: Safetensors source contains no tensors".to_string());
    }

    let mut source_types = Vec::with_capacity(plan.tensors.len());
    let mut roles = Vec::with_capacity(plan.tensors.len());
    let mut n_layer = 0u32;
    for tensor in &plan.tensors {
        let ggml_type = safetensor_dtype_to_ggml(&tensor.dtype).ok_or_else(|| {
            format!(
                "p64: tensor '{}' dtype {} is not a supported high-fidelity source",
                tensor.name, tensor.dtype
            )
        })?;
        if !is_high_fidelity_ggml(ggml_type) {
            return Err(format!(
                "p64: tensor '{}' is low precision and cannot be transcoded",
                tensor.name
            ));
        }
        let element_count = tensor
            .shape
            .iter()
            .try_fold(1usize, |count, dimension| count.checked_mul(*dimension))
            .ok_or_else(|| format!("p64: tensor '{}' shape overflow", tensor.name))?;
        let expected_bytes = element_count
            .checked_mul(ggml_elem_bytes(ggml_type).ok_or("p64: unsupported element width")?)
            .ok_or_else(|| format!("p64: tensor '{}' byte-size overflow", tensor.name))?;
        if expected_bytes != tensor.byte_len() {
            return Err(format!(
                "p64: tensor '{}' declares {} bytes but shape/dtype requires {}",
                tensor.name,
                tensor.byte_len(),
                expected_bytes
            ));
        }
        let role = crate::tensor_roles::name_to_role(&tensor.name);
        if let Some(mapped) = role {
            if mapped.layer != P64_LAYER_GLOBAL {
                n_layer = n_layer.max(mapped.layer as u32 + 1);
            }
        }
        source_types.push(ggml_type);
        roles.push(role);
    }

    let ternary_for = |position: usize| match policy {
        SafetensorQuantization::Verbatim => false,
        SafetensorQuantization::AllTernary => true,
        SafetensorQuantization::FfnTernary => roles[position]
            .map(|role| crate::tensor_roles::ternary_eligible(role.role))
            .unwrap_or(false),
    };

    let mut string_table = vec![0u8];
    let mut name_offsets = Vec::with_capacity(plan.tensors.len());
    for tensor in &plan.tensors {
        name_offsets.push(
            u32::try_from(string_table.len()).map_err(|_| "p64: string table exceeds 4 GiB")?,
        );
        string_table.extend_from_slice(tensor.name.as_bytes());
        string_table.push(0);
    }

    let hparams_offset = P64_WEIGHT_HEADER_BYTES;
    let tensor_table_offset = align_up(hparams_offset + 64, 64);
    let tensor_table_bytes = plan
        .tensors
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
    let checksum_bytes = (plan.tensors.len() + 1)
        .checked_mul(4)
        .ok_or("p64: checksum table overflow")?;
    let blob_region_offset = align_up(checksum_offset + checksum_bytes, page);

    let mut entries = Vec::with_capacity(plan.tensors.len());
    let mut cursor = blob_region_offset;
    let mut largest_tensor_bytes = 0usize;
    let mut total_tensor_bytes = 0usize;
    let mut peak_working_bytes = 0usize;
    for (position, tensor) in plan.tensors.iter().enumerate() {
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
            source_name_hash: crate::q_hash(&tensor.name),
            reserved: [0; 8],
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

    let mut metadata = vec![0u8; checksum_offset + checksum_bytes];
    header.write_le(&mut metadata[..P64_WEIGHT_HEADER_BYTES]);
    hparams.write_le(&mut metadata[hparams_offset..hparams_offset + 64]);
    for (position, entry) in entries.iter().enumerate() {
        let start = tensor_table_offset + position * P64_TENSOR_ENTRY_BYTES;
        write_tensor_entry(entry, &mut metadata[start..start + P64_TENSOR_ENTRY_BYTES]);
    }
    metadata[string_table_offset..string_table_offset + string_table.len()]
        .copy_from_slice(&string_table);
    for layer in 0..manifold_count {
        let coordinate = crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
            layer.min(n_layer as usize) as u32,
            n_layer.max(1),
        );
        let start = manifold_table_offset + layer * P64_MANIFOLD_ENTRY_BYTES;
        write_manifold_coordinate(
            &coordinate,
            &mut metadata[start..start + P64_MANIFOLD_ENTRY_BYTES],
        );
    }

    let mut float_scratch = Vec::new();
    for (position, tensor) in plan.tensors.iter().enumerate() {
        let source_start = plan.data_start + tensor.begin;
        let source_end = plan.data_start + tensor.end;
        let crc = if ternary_for(position) {
            let count = tensor.shape.iter().copied().product::<usize>();
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
    for (position, tensor) in plan.tensors.iter().enumerate() {
        let target = entries[position].blob_offset as usize;
        while bytes_written < target {
            let count = (target - bytes_written).min(zeros.len());
            out.write_all(&zeros[..count])
                .map_err(|error| error.to_string())?;
            bytes_written += count;
        }
        let source_start = plan.data_start + tensor.begin;
        let source_end = plan.data_start + tensor.end;
        if ternary_for(position) {
            let count = tensor.shape.iter().copied().product::<usize>();
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

pub fn transcode_safetensor_to_p64_ffn_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_to_p64_policy(src, page_log2, out)
}

pub fn transcode_safetensor_to_q42_ffn_ternary<W: std::io::Write>(
    src: &[u8],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    transcode_safetensor_to_p64_policy(src, page_log2, out)
}
