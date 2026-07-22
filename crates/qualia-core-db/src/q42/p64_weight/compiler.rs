//! GGUF -> P64 **compiler / builder**: `compile_gguf_to_p64*` (verbatim / f16-expand / Q4_K SoA),
//! the FFN-quantized AWQ compilers (ternary / Q4_0), the legacy compiler, and their historical
//! `_q42` aliases, plus the compile-time helpers (`p64_tensor_name`, f16 blob expansion).

use super::*;
use crate::gguf_sharder::{GgufTensorIndex, GgufTensorInfo};
use crate::container_10d::crc32c::crc32c;

fn p64_tensor_name(role: u16, layer: u16, source_name_hash: u64) -> String {
    if layer == P64_LAYER_GLOBAL {
        return match role {
            P64_ROLE_TOKEN_EMBD => "token_embd.weight".to_string(),
            P64_ROLE_OUTPUT => "output.weight".to_string(),
            P64_ROLE_OUTPUT_NORM => "output_norm.weight".to_string(),
            _ => format!("tensor.{source_name_hash:016x}"),
        };
    }
    match p64_role_suffix(role) {
        Some(suffix) => format!("blk.{layer}.{}", String::from_utf8_lossy(suffix)),
        None => format!("tensor.{source_name_hash:016x}"),
    }
}

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

/// Compile a GGUF image into the cache-line-native P64 container (verbatim layout).
///
/// Every GGUF tensor is retained. Known inference tensors receive a semantic
/// role/layer; unknown tensors receive [`P64_ROLE_UNKNOWN`] but retain their
/// source name hash and source byte offset. Each tensor starts on a hardware
/// page boundary and has an in-band CRC-32C. The metadata, tokenizer and 10D
/// manifold table share a separate CRC, validated before any blob is exposed.
pub fn compile_gguf_to_p64(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64_with_layout(input, page_log2, P64ConvertLayout::Verbatim)
}

/// Like [`compile_gguf_to_p64`] but selects a conversion-time layout policy.
pub fn compile_gguf_to_p64_with_layout(
    input: &[u8],
    page_log2: u16,
    layout: P64ConvertLayout,
) -> Result<Vec<u8>, String> {
    let index = GgufTensorIndex::from_gguf(input);
    if index.tensor_data_start == 0 || index.entries.is_empty() {
        return Err("p64: GGUF parse yielded no tensors".to_string());
    }
    let page_log2 = if page_log2 == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        page_log2
    };
    if !(8..=30).contains(&page_log2) {
        return Err(format!("p64: page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    let tensor_data_start = index.tensor_data_start as usize;

    // (role, layer, source-name hash, source tensor info)
    let mut planned: Vec<(u16, u16, u64, GgufTensorInfo)> = Vec::with_capacity(index.entries.len());
    let mut push_known = |role: u16, layer: u16, candidate: Option<GgufTensorInfo>| {
        if let Some(info) = candidate {
            if planned
                .iter()
                .any(|(_, _, _, existing)| existing.byte_offset == info.byte_offset)
            {
                return;
            }
            let name_hash = index
                .entries
                .iter()
                .find(|(_, source)| source.byte_offset == info.byte_offset)
                .map(|(hash, _)| *hash)
                .unwrap_or(0);
            planned.push((role, layer, name_hash, info));
        }
    };
    for layer in 0..index.hyperparams.n_layer {
        let tensors = index.get_layer_tensors(layer);
        let layer = u16::try_from(layer).map_err(|_| "p64: layer index exceeds u16")?;
        push_known(P64_ROLE_ATTN_NORM, layer, tensors.attn_norm);
        push_known(P64_ROLE_ATTN_Q, layer, tensors.attn_q);
        push_known(P64_ROLE_ATTN_K, layer, tensors.attn_k);
        push_known(P64_ROLE_ATTN_V, layer, tensors.attn_v);
        push_known(P64_ROLE_ATTN_OUTPUT, layer, tensors.attn_output);
        push_known(P64_ROLE_FFN_NORM, layer, tensors.ffn_norm);
        push_known(P64_ROLE_FFN_GATE, layer, tensors.ffn_gate);
        push_known(P64_ROLE_FFN_UP, layer, tensors.ffn_up);
        push_known(P64_ROLE_FFN_DOWN, layer, tensors.ffn_down);
    }
    push_known(
        P64_ROLE_TOKEN_EMBD,
        P64_LAYER_GLOBAL,
        index.token_embd_info().copied(),
    );
    push_known(
        P64_ROLE_OUTPUT,
        P64_LAYER_GLOBAL,
        index.output_weight_info().copied(),
    );
    push_known(
        P64_ROLE_OUTPUT_NORM,
        P64_LAYER_GLOBAL,
        index.output_norm_info().copied(),
    );
    for (name_hash, info) in &index.entries {
        if !planned
            .iter()
            .any(|(_, _, _, existing)| existing.byte_offset == info.byte_offset)
        {
            planned.push((P64_ROLE_UNKNOWN, P64_LAYER_GLOBAL, *name_hash, *info));
        }
    }
    // Keep **layer-major** order for known roles (built above). Sorting by GGUF
    // source offset used to destroy sequential layer packing and hurt residency.
    // Unknown tensors trail, ordered by source offset for stable CRC layout only.
    let mut unknowns: Vec<_> = planned
        .iter()
        .copied()
        .filter(|(role, _, _, _)| *role == P64_ROLE_UNKNOWN)
        .collect();
    unknowns.sort_by_key(|(_, _, _, info)| info.byte_offset);
    planned.retain(|(role, _, _, _)| *role != P64_ROLE_UNKNOWN);
    planned.extend(unknowns);

    let mut string_table = vec![0u8];
    let mut name_offsets = Vec::with_capacity(planned.len());
    for (role, layer, name_hash, _) in &planned {
        let name_offset =
            u32::try_from(string_table.len()).map_err(|_| "p64: string table exceeds 4 GiB")?;
        name_offsets.push(name_offset);
        let name = p64_tensor_name(*role, *layer, *name_hash);
        string_table.extend_from_slice(name.as_bytes());
        string_table.push(0);
    }
    let tokenizer = crate::gguf_sharder::GgufTokenizer::from_gguf(input).to_p64_section();
    let manifold_count = index
        .hyperparams
        .n_layer
        .checked_add(1)
        .ok_or("p64: manifold count overflow")? as usize;

    let hparams_offset = P64_WEIGHT_HEADER_BYTES;
    let n_layer_u = index.hyperparams.n_layer as usize;
    // Layer schedule sits immediately after hparams (pipeline residency map).
    let schedule_offset = align_up(hparams_offset + 64, 64);
    let schedule_bytes = n_layer_u
        .checked_mul(core::mem::size_of::<P64LayerScheduleEntry>())
        .ok_or("p64: schedule overflow")?;
    let tensor_table_offset = align_up(schedule_offset + schedule_bytes, 64);
    let tensor_table_bytes = planned
        .len()
        .checked_mul(P64_TENSOR_ENTRY_BYTES)
        .ok_or("p64: tensor table overflow")?;
    let string_table_offset = tensor_table_offset + tensor_table_bytes;
    let manifold_table_offset = align_up(string_table_offset + string_table.len(), 64);
    let manifold_bytes = manifold_count
        .checked_mul(P64_MANIFOLD_ENTRY_BYTES)
        .ok_or("p64: manifold table overflow")?;
    let tokenizer_offset = align_up(manifold_table_offset + manifold_bytes, 64);
    let checksum_offset = align_up(tokenizer_offset + tokenizer.len(), 64);
    let checksum_bytes = planned
        .len()
        .checked_add(1)
        .and_then(|count| count.checked_mul(4))
        .ok_or("p64: checksum table overflow")?;
    let blob_region_offset = align_up(checksum_offset + checksum_bytes, page);

    let mut entries = Vec::with_capacity(planned.len());
    // Parallel to `entries`: conversion kind for the blob write pass.
    #[derive(Clone, Copy)]
    enum BlobKind {
        Copy,
        F16Expand,
        Q4kSoa,
    }
    let mut blob_kind: Vec<BlobKind> = Vec::with_capacity(planned.len());
    let mut cursor = blob_region_offset;
    for (position, (role, layer, name_hash, info)) in planned.iter().enumerate() {
        let source_blob_size = crate::ggml_quants::tensor_byte_len(info)
            .ok_or_else(|| format!("p64: unsupported GGML type {}", info.ggml_type))?;
        let source_start = tensor_data_start
            .checked_add(info.byte_offset as usize)
            .ok_or("p64: source tensor offset overflow")?;
        let source_end = source_start
            .checked_add(source_blob_size)
            .ok_or("p64: source tensor length overflow")?;
        if source_end > input.len() {
            return Err(format!("p64: source tensor {position} is out of bounds"));
        }

        let do_f16 = matches!(layout, P64ConvertLayout::F16Expand)
            && p64_role_is_weight_matrix(*role)
            && info.n_dims >= 2
            && info.ggml_type != crate::ggml_quants::GGML_TYPE_F16
            && info.ggml_type != crate::ggml_quants::GGML_TYPE_F32;
        let do_soa = matches!(layout, P64ConvertLayout::Q4kSoa)
            && p64_role_is_weight_matrix(*role)
            && info.n_dims >= 2
            && info.ggml_type == crate::ggml_quants::GGML_TYPE_Q4_K;
        let (out_dtype, blob_size, kind) = if do_f16 {
            let n0 = info.dims[0] as usize;
            let n1 = info.dims[1] as usize;
            let elems = n0
                .checked_mul(n1)
                .ok_or("p64: f16 expand element count overflow")?;
            let bytes = elems
                .checked_mul(2)
                .ok_or("p64: f16 expand byte count overflow")?;
            (
                crate::ggml_quants::GGML_TYPE_F16 as u16,
                bytes,
                BlobKind::F16Expand,
            )
        } else if do_soa {
            let n0 = info.dims[0] as usize;
            let n1 = info.dims[1].max(1) as usize;
            let bytes = crate::ggml_quants::ggml_row_bytes(
                crate::ggml_quants::GGML_TYPE_Q4_K_SOA,
                n0,
            )
            .and_then(|r| r.checked_mul(n1))
            .ok_or("p64: Q4_K SoA size overflow")?;
            (
                crate::ggml_quants::GGML_TYPE_Q4_K_SOA as u16,
                bytes,
                BlobKind::Q4kSoa,
            )
        } else {
            (
                u16::try_from(info.ggml_type).map_err(|_| "p64: GGML type exceeds u16")?,
                source_blob_size,
                BlobKind::Copy,
            )
        };

        // Pipeline packing: page-align only at **layer boundaries** (and first blob).
        // Within a layer, 256-byte align — cuts ~page waste × tensors/layer (decode residency
        // and CUDA multi-weight fill walk contiguous layer ranges).
        let pack_align = if position == 0 {
            page
        } else {
            let prev_layer = planned[position - 1].1;
            if *layer != prev_layer {
                page
            } else {
                256
            }
        };
        cursor = align_up(cursor, pack_align);
        let mut dimensions = [0u32; 4];
        for (target, source) in dimensions.iter_mut().zip(info.dims) {
            *target = u32::try_from(source).map_err(|_| "p64: tensor dimension exceeds u32")?;
        }
        let manifold_idx = if *layer == P64_LAYER_GLOBAL {
            index.hyperparams.n_layer
        } else {
            *layer as u32
        };
        entries.push(P64TensorEntry {
            name_offset: name_offsets[position],
            role_id: *role,
            dtype: out_dtype,
            manifold_idx,
            rank: info.n_dims,
            dimensions,
            blob_offset: u32::try_from(cursor).map_err(|_| "p64: container exceeds 4 GiB")?,
            blob_size: u32::try_from(blob_size).map_err(|_| "p64: tensor exceeds 4 GiB")?,
            source_offset: info.byte_offset,
            source_name_hash: *name_hash,
            alt_dtype: 0,
            precision_views_mask: 0,
            alt_blob_offset: 0,
        });
        blob_kind.push(kind);
        cursor = cursor
            .checked_add(blob_size)
            .ok_or("p64: container size overflow")?;
    }
    let total_size = align_up(cursor, 64);
    if total_size > u32::MAX as usize {
        return Err("p64: 32-bit relative-offset container exceeds 4 GiB".to_string());
    }

    let mut flags = P64_FLAG_LITTLE_ENDIAN
        | P64_FLAG_LAYER_MAJOR
        | P64_FLAG_LAYER_PACK
        | P64_FLAG_LAYER_SCHEDULE;
    if matches!(layout, P64ConvertLayout::Q4kSoa)
        && blob_kind.iter().any(|k| matches!(k, BlobKind::Q4kSoa))
    {
        flags |= P64_FLAG_Q4K_SOA;
    }
    // Build per-layer blob ranges for the schedule table (decode/CUDA residency).
    let mut schedule = vec![P64LayerScheduleEntry::default(); n_layer_u];
    for (i, s) in schedule.iter_mut().enumerate() {
        s.layer = i as u32;
        s.blob_begin = u32::MAX;
        s.blob_end = 0;
    }
    for e in &entries {
        let li = e.manifold_idx as usize;
        if li >= n_layer_u {
            continue; // globals
        }
        let s = &mut schedule[li];
        s.blob_begin = s.blob_begin.min(e.blob_offset);
        s.blob_end = s.blob_end.max(e.blob_offset.saturating_add(e.blob_size));
        s.tensor_count = s.tensor_count.saturating_add(1);
        if e.role_id < 16 {
            s.roles_mask |= 1u16 << e.role_id;
        }
    }
    for s in &mut schedule {
        if s.blob_begin == u32::MAX {
            s.blob_begin = 0;
            s.blob_end = 0;
        }
    }
    let header = P64WeightHeader {
        magic: P64_MAGIC,
        version: P64_VERSION,
        flags,
        role_table_offset: schedule_offset as u32, // layer schedule (not a role string table)
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
    let hp = P64HParams {
        n_layer: index.hyperparams.n_layer,
        n_embd: index.hyperparams.n_embd,
        n_head: index.hyperparams.n_head,
        n_kv_head: index.hyperparams.effective_n_kv_head(),
        vocab_size: index.vocab_dim() as u32,
        rope_freq_base: index.hyperparams.effective_rope_freq_base(),
        rope_scale: index.hyperparams.effective_rope_scale(),
        head_dim: index.hyperparams.head_dim,
        head_dim_swa: index.hyperparams.head_dim_swa,
        sliding_window: index.hyperparams.sliding_window,
        shared_kv_layers: index.hyperparams.shared_kv_layers,
        logit_softcap: index.hyperparams.logit_softcap,
        architecture: index.hyperparams.architecture,
        arch_flags: index.hyperparams.arch_flags,
        reserved: [0; 8],
    };

    let mut output = vec![0u8; total_size];
    header.write_le(&mut output[..P64_WEIGHT_HEADER_BYTES]);
    hp.write_le(&mut output[hparams_offset..hparams_offset + 64]);
    for (i, s) in schedule.iter().enumerate() {
        let start = schedule_offset + i * core::mem::size_of::<P64LayerScheduleEntry>();
        let dest = &mut output[start..start + 64];
        dest.fill(0);
        dest[0..4].copy_from_slice(&s.layer.to_le_bytes());
        dest[4..8].copy_from_slice(&s.blob_begin.to_le_bytes());
        dest[8..12].copy_from_slice(&s.blob_end.to_le_bytes());
        dest[12..14].copy_from_slice(&s.tensor_count.to_le_bytes());
        dest[14..16].copy_from_slice(&s.roles_mask.to_le_bytes());
    }
    for (position, entry) in entries.iter().enumerate() {
        let start = tensor_table_offset + position * P64_TENSOR_ENTRY_BYTES;
        write_tensor_entry(entry, &mut output[start..start + P64_TENSOR_ENTRY_BYTES]);
    }
    output[string_table_offset..string_table_offset + string_table.len()]
        .copy_from_slice(&string_table);
    for layer in 0..manifold_count {
        let coordinate = crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
            layer.min(index.hyperparams.n_layer as usize) as u32,
            index.hyperparams.n_layer.max(1),
        );
        let start = manifold_table_offset + layer * P64_MANIFOLD_ENTRY_BYTES;
        write_manifold_coordinate(
            &coordinate,
            &mut output[start..start + P64_MANIFOLD_ENTRY_BYTES],
        );
    }
    output[tokenizer_offset..tokenizer_offset + tokenizer.len()].copy_from_slice(&tokenizer);

    for (position, entry) in entries.iter().enumerate() {
        let target_start = entry.blob_offset as usize;
        let target_end = target_start + entry.blob_size as usize;
        match blob_kind[position] {
            BlobKind::F16Expand => {
                let info = &planned[position].3;
                let source_blob_size = crate::ggml_quants::tensor_byte_len(info)
                    .ok_or("p64: f16 expand missing source size")?;
                let source_start = tensor_data_start + entry.source_offset as usize;
                let source_end = source_start + source_blob_size;
                let raw = &input[source_start..source_end];
                expand_tensor_to_f16_blob(raw, info, &mut output[target_start..target_end])?;
            }
            BlobKind::Q4kSoa => {
                let info = &planned[position].3;
                let source_blob_size = crate::ggml_quants::tensor_byte_len(info)
                    .ok_or("p64: Q4_K SoA missing source size")?;
                let source_start = tensor_data_start + entry.source_offset as usize;
                let source_end = source_start + source_blob_size;
                let raw = &input[source_start..source_end];
                let n0 = info.dims[0] as usize;
                let n1 = info.dims[1].max(1) as usize;
                crate::ggml_quants::expand_q4k_tensor_to_soa(
                    raw,
                    n0,
                    n1,
                    &mut output[target_start..target_end],
                )
                .map_err(|e| format!("p64: Q4_K SoA expand: {e:?}"))?;
            }
            BlobKind::Copy => {
                let source_start = tensor_data_start + entry.source_offset as usize;
                let source_end = source_start + entry.blob_size as usize;
                output[target_start..target_end].copy_from_slice(&input[source_start..source_end]);
            }
        }
        let crc = crc32c(&output[target_start..target_end]);
        let crc_start = checksum_offset + 4 + position * 4;
        output[crc_start..crc_start + 4].copy_from_slice(&crc.to_le_bytes());
    }
    let metadata_crc = crc32c(&output[..checksum_offset]);
    output[checksum_offset..checksum_offset + 4].copy_from_slice(&metadata_crc.to_le_bytes());
    Ok(output)
}

/// Roles that are 2-D weight matrices (eligible for f16 expand). Norms stay source dtype.
#[inline]
fn p64_role_is_weight_matrix(role: u16) -> bool {
    matches!(
        role,
        P64_ROLE_ATTN_K
            | P64_ROLE_ATTN_V
            | P64_ROLE_ATTN_Q
            | P64_ROLE_ATTN_OUTPUT
            | P64_ROLE_FFN_GATE
            | P64_ROLE_FFN_UP
            | P64_ROLE_FFN_DOWN
            | P64_ROLE_TOKEN_EMBD
            | P64_ROLE_OUTPUT
    )
}

/// Dequantize a full 2-D GGUF tensor into a row-major f16 blob (`out` length = n0*n1*2).
fn expand_tensor_to_f16_blob(
    raw: &[u8],
    info: &GgufTensorInfo,
    out: &mut [u8],
) -> Result<(), String> {
    let n0 = info.dims[0] as usize; // cols (in)
    let n1 = info.dims[1] as usize; // rows (out)
    let need = n0
        .checked_mul(n1)
        .and_then(|e| e.checked_mul(2))
        .ok_or("p64: f16 expand size overflow")?;
    if out.len() < need {
        return Err("p64: f16 expand output buffer too small".into());
    }
    let mut row_f32 = vec![0f32; n0];
    for r in 0..n1 {
        crate::ggml_quants::dequant_matrix_row_into(raw, info, r, &mut row_f32)
            .map_err(|e| format!("p64: f16 expand dequant row {r}: {e:?}"))?;
        let row_off = r * n0 * 2;
        for (c, &v) in row_f32.iter().enumerate() {
            let bits = half::f16::from_f32(v).to_le_bytes();
            let o = row_off + c * 2;
            out[o] = bits[0];
            out[o + 1] = bits[1];
        }
    }
    Ok(())
}

/// Compile a flat GGUF byte image into a P64 LLM-weight container.
/// `page_log2 == 0` selects the default (16 KB). Returns the little-endian container bytes.
#[allow(dead_code)]
fn compile_gguf_to_p64_legacy(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    let idx = crate::gguf_sharder::GgufTensorIndex::from_gguf(input);
    if idx.tensor_data_start == 0 && idx.hyperparams.n_layer == 0 {
        return Err("GGUF parse failed or yielded no tensor metadata".to_string());
    }
    let page_log2 = if page_log2 == 0 {
        12 // 4096 default
    } else {
        page_log2
    };
    if page_log2 < 8 || page_log2 > 30 {
        return Err(format!("page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    let n_layer = idx.hyperparams.n_layer;
    let tds = idx.tensor_data_start as usize;

    let mut planned: Vec<(u16, u16, crate::gguf_sharder::GgufTensorInfo)> = Vec::new();
    let mut push = |role_id: u16, layer: u16, t: Option<crate::gguf_sharder::GgufTensorInfo>| {
        if let Some(info) = t {
            planned.push((role_id, layer, info));
        }
    };
    for layer in 0..n_layer {
        let t = idx.get_layer_tensors(layer);
        let l = layer as u16;
        push(P64_ROLE_ATTN_NORM, l, t.attn_norm);
        push(P64_ROLE_ATTN_Q, l, t.attn_q);
        push(P64_ROLE_ATTN_K, l, t.attn_k);
        push(P64_ROLE_ATTN_V, l, t.attn_v);
        push(P64_ROLE_ATTN_OUTPUT, l, t.attn_output);
        push(P64_ROLE_FFN_NORM, l, t.ffn_norm);
        push(P64_ROLE_FFN_GATE, l, t.ffn_gate);
        push(P64_ROLE_FFN_UP, l, t.ffn_up);
        push(P64_ROLE_FFN_DOWN, l, t.ffn_down);
    }
    push(
        P64_ROLE_TOKEN_EMBD,
        P64_LAYER_GLOBAL,
        idx.token_embd_info().copied(),
    );
    push(
        P64_ROLE_OUTPUT_NORM,
        P64_LAYER_GLOBAL,
        idx.output_norm_info().copied(),
    );
    push(
        P64_ROLE_OUTPUT,
        P64_LAYER_GLOBAL,
        idx.output_weight_info().copied(),
    );

    // Filter missing/invalid tensors
    planned.retain(|(_, _, info)| info.dims[0] > 0);

    // Build the string table
    let mut string_table: Vec<u8> = Vec::new();
    let mut name_offsets = std::collections::HashMap::new();

    // Push a dummy null at start
    string_table.push(0u8);

    for (role, layer, info) in &planned {
        let name = if let Some(suffix) = p64_role_suffix(*role) {
            if *layer == P64_LAYER_GLOBAL {
                String::from_utf8_lossy(suffix).to_string()
            } else {
                format!("blk.{}.{}", layer, String::from_utf8_lossy(suffix))
            }
        } else {
            format!("tensor_{}", info.byte_offset)
        };

        if !name_offsets.contains_key(&name) {
            let offset = string_table.len() as u32;
            name_offsets.insert(name.clone(), offset);
            string_table.extend_from_slice(name.as_bytes());
            string_table.push(0u8);
        }
    }

    // Now extract vocabulary from GGUF and append to string table.
    let tokenizer_offset = string_table.len() as u32;
    let tok = crate::gguf_sharder::GgufTokenizer::from_gguf(input);

    let mut tok_bytes: Vec<u8> = Vec::new();
    // Serialize vocabulary sizes and strings
    tok_bytes.extend_from_slice(&(tok.vocab.len() as u32).to_le_bytes());
    for v in &tok.vocab {
        let v_bytes = v.as_bytes();
        tok_bytes.extend_from_slice(&(v_bytes.len() as u32).to_le_bytes());
        tok_bytes.extend_from_slice(v_bytes);
    }
    let tokenizer_size = tok_bytes.len() as u32;

    string_table.extend_from_slice(&tok_bytes);

    // Padding string table to 64 bytes
    while string_table.len() % 64 != 0 {
        string_table.push(0u8);
    }

    let string_table_size = string_table.len() as u32;
    let tensor_count = planned.len() as u32;

    // Build Manifold Table
    let mut manifold_table: Vec<u8> = Vec::new();
    let total_layers = idx.hyperparams.n_layer;
    for l in 0..total_layers {
        let coord = crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(
            l,
            total_layers,
        );
        // We do unsafe cast because bytemuck might not be derived for ManifoldCoordinate10D yet. Wait, we can just write the f32s.
        manifold_table.extend_from_slice(&coord.scale.to_le_bytes());
        manifold_table.extend_from_slice(&coord.attention_depth.to_le_bytes());
        manifold_table.extend_from_slice(&coord.epistemic_weight.to_le_bytes());
        manifold_table.extend_from_slice(&coord.topological_spin.to_le_bytes());
        manifold_table.extend_from_slice(&coord.temporal_decay.to_le_bytes());
        manifold_table.extend_from_slice(&coord.entropy_bias.to_le_bytes());
        manifold_table.extend_from_slice(&coord.spatial_phase.to_le_bytes());
        manifold_table.extend_from_slice(&coord.recurrence_frequency.to_le_bytes());
        manifold_table.extend_from_slice(&coord.density_threshold.to_le_bytes());
        manifold_table.extend_from_slice(&coord.manifold_curvature.to_le_bytes());
    }
    // Global layer
    let global_coord =
        crate::modalities::manifold::ManifoldCoordinate10D::from_sequential_layer(0, 1);
    manifold_table.extend_from_slice(&global_coord.scale.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.attention_depth.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.epistemic_weight.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.topological_spin.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.temporal_decay.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.entropy_bias.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.spatial_phase.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.recurrence_frequency.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.density_threshold.to_le_bytes());
    manifold_table.extend_from_slice(&global_coord.manifold_curvature.to_le_bytes());
    let manifold_table_size = manifold_table.len() as u32;

    // Layout
    let hparams_offset = 64;
    let entries_offset = 128;
    let string_table_offset = entries_offset + (tensor_count * 64) as u32;
    let manifold_table_offset = string_table_offset + string_table_size;
    let end_of_manifold_table = manifold_table_offset + manifold_table_size;
    let page_aligned_tensor_start =
        (end_of_manifold_table + (page as u32) - 1) & !((page as u32) - 1);

    let mut out = vec![0u8; page_aligned_tensor_start as usize];

    // 1. Header
    out[0..4].copy_from_slice(&P64_MAGIC);
    out[4..6].copy_from_slice(&P64_VERSION.to_le_bytes());
    out[6..8].copy_from_slice(&0u16.to_le_bytes()); // format flags

    out[8..12].copy_from_slice(&0u32.to_le_bytes()); // role_table_offset
    out[12..16].copy_from_slice(&(entries_offset as u32).to_le_bytes()); // tensor_table_offset
    out[16..20].copy_from_slice(&(tokenizer_offset as u32).to_le_bytes()); // tokenizer_offset
    out[20..24].copy_from_slice(&(hparams_offset as u32).to_le_bytes()); // hparams_offset
    out[24..28].copy_from_slice(&string_table_offset.to_le_bytes()); // string_table_offset
    out[28..32].copy_from_slice(&0u32.to_le_bytes()); // checksum_offset
    out[32..36].copy_from_slice(&manifold_table_offset.to_le_bytes()); // manifold_table_offset

    out[36..40].copy_from_slice(&tensor_count.to_le_bytes());
    out[40..44].copy_from_slice(&(page as u32).to_le_bytes());
    // reserved[0..4]: embedded tokenizer blob byte length
    out[44..48].copy_from_slice(&tokenizer_size.to_le_bytes());

    // 2. HParams
    // Just mock for now or use the fields if they are pub. The DOD rewrite doesn't require a strict HParams struct unless defined.
    // Actually we need to serialize idx.hyperparams into the 64 byte slot.
    let hparams = &idx.hyperparams;
    let h_off = hparams_offset as usize;
    out[h_off..h_off + 4].copy_from_slice(&hparams.n_layer.to_le_bytes());
    out[h_off + 4..h_off + 8].copy_from_slice(&hparams.n_embd.to_le_bytes());
    out[h_off + 8..h_off + 12].copy_from_slice(&hparams.n_head.to_le_bytes());
    out[h_off + 12..h_off + 16].copy_from_slice(&hparams.n_kv_head.to_le_bytes());
    out[h_off + 16..h_off + 20].copy_from_slice(&0u32.to_le_bytes()); // vocab_size (filled later if known)
    out[h_off + 20..h_off + 24].copy_from_slice(&hparams.rope_freq_base.to_le_bytes());
    out[h_off + 24..h_off + 28].copy_from_slice(&hparams.rope_scale.to_le_bytes());
    out[h_off + 28..h_off + 32].copy_from_slice(&hparams.head_dim.to_le_bytes());
    out[h_off + 32..h_off + 36].copy_from_slice(&hparams.head_dim_swa.to_le_bytes());
    out[h_off + 36..h_off + 40].copy_from_slice(&hparams.sliding_window.to_le_bytes());
    out[h_off + 40..h_off + 44].copy_from_slice(&hparams.shared_kv_layers.to_le_bytes());
    out[h_off + 44..h_off + 48].copy_from_slice(&hparams.logit_softcap.to_le_bytes());
    out[h_off + 48..h_off + 52].copy_from_slice(&hparams.architecture.to_le_bytes());
    out[h_off + 52..h_off + 56].copy_from_slice(&hparams.arch_flags.to_le_bytes());

    // 2b. Manifold Table
    let mt_off = manifold_table_offset as usize;
    out[mt_off..mt_off + manifold_table.len()].copy_from_slice(&manifold_table);

    // 3. Entries and Tensor Blobs
    let mut cursor_blob = page_aligned_tensor_start as usize;
    for (i, (role, layer, info)) in planned.iter().enumerate() {
        let e_off = entries_offset as usize + i * 64;

        let name = if let Some(suffix) = p64_role_suffix(*role) {
            if *layer == P64_LAYER_GLOBAL {
                String::from_utf8_lossy(suffix).to_string()
            } else {
                format!("blk.{}.{}", layer, String::from_utf8_lossy(suffix))
            }
        } else {
            format!("tensor_{}", info.byte_offset)
        };
        let n_offset = *name_offsets.get(&name).unwrap();

        // Align blob to its internal alignment requirement if needed, but P64 natively page aligns at start
        // and blobs follow one another. Wait, ggml requires 32-byte alignment usually.
        // Let's pad cursor_blob to 32 bytes
        cursor_blob = (cursor_blob + 31) & !31;

        let n_elements =
            info.dims[0] * info.dims[1].max(1) * info.dims[2].max(1) * info.dims[3].max(1);
        let byte_len = crate::ggml_quants::tensor_byte_len(info).unwrap_or(0);

        // Copy the tensor blob
        out.resize(cursor_blob + byte_len, 0);
        let src_start = tds + info.byte_offset as usize;
        if src_start + byte_len <= input.len() {
            out[cursor_blob..cursor_blob + byte_len]
                .copy_from_slice(&input[src_start..src_start + byte_len]);
        }

        // Write Entry
        out[e_off..e_off + 4].copy_from_slice(&n_offset.to_le_bytes());
        out[e_off + 4..e_off + 6].copy_from_slice(&role.to_le_bytes());
        out[e_off + 6..e_off + 8].copy_from_slice(&(info.ggml_type as u16).to_le_bytes());

        let m_idx = if *layer == P64_LAYER_GLOBAL {
            total_layers
        } else {
            *layer as u32
        };
        out[e_off + 8..e_off + 12].copy_from_slice(&m_idx.to_le_bytes()); // manifold_idx

        out[e_off + 12..e_off + 16].copy_from_slice(&info.n_dims.to_le_bytes());
        out[e_off + 16..e_off + 20].copy_from_slice(&(info.dims[0] as u32).to_le_bytes());
        out[e_off + 20..e_off + 24].copy_from_slice(&(info.dims[1] as u32).to_le_bytes());
        out[e_off + 24..e_off + 28].copy_from_slice(&(info.dims[2] as u32).to_le_bytes());
        out[e_off + 28..e_off + 32].copy_from_slice(&(info.dims[3] as u32).to_le_bytes());
        out[e_off + 32..e_off + 36].copy_from_slice(&(cursor_blob as u32).to_le_bytes());
        out[e_off + 36..e_off + 40].copy_from_slice(&(byte_len as u32).to_le_bytes());
        out[e_off + 40..e_off + 44].copy_from_slice(&(n_elements as u32).to_le_bytes());

        cursor_blob += byte_len;
    }

    // String table write
    out[string_table_offset as usize..string_table_offset as usize + string_table.len()]
        .copy_from_slice(&string_table);

    Ok(out)
}

/// Compatibility alias for the historical pre-P64 API name.
pub fn compile_gguf_to_q42(input: &[u8], page_log2: u16) -> Result<Vec<u8>, String> {
    compile_gguf_to_p64(input, page_log2)
}

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
