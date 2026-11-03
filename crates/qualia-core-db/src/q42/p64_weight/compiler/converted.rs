//! Preserved non-verbatim numerical conversion and serialization.
use super::*;
pub(super) fn compile_converted(
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

    let planned = planning::ordered_tensors(&index)?;

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
        Q4kSoaRequant,
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
            && info.n_dims >= 2;
        // Q4_K source → direct SoA expand (lossless layout transform).
        // Non-Q4_K source → dequant to f32 then re-quantize to Q4_K_SOA.
        let do_soa_requant = do_soa && info.ggml_type != crate::ggml_quants::GGML_TYPE_Q4_K;
        let do_soa = do_soa && !do_soa_requant;
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
        } else if do_soa || do_soa_requant {
            let n0 = info.dims[0] as usize;
            let n1 = info.dims[1].max(1) as usize;
            let bytes =
                crate::ggml_quants::ggml_row_bytes(crate::ggml_quants::GGML_TYPE_Q4_K_SOA, n0)
                    .and_then(|r| r.checked_mul(n1))
                    .ok_or("p64: Q4_K SoA size overflow")?;
            let kind = if do_soa_requant {
                BlobKind::Q4kSoaRequant
            } else {
                BlobKind::Q4kSoa
            };
            (crate::ggml_quants::GGML_TYPE_Q4_K_SOA as u16, bytes, kind)
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
        && blob_kind
            .iter()
            .any(|k| matches!(k, BlobKind::Q4kSoa | BlobKind::Q4kSoaRequant))
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
            BlobKind::Q4kSoaRequant => {
                let info = &planned[position].3;
                let source_blob_size = crate::ggml_quants::tensor_byte_len(info)
                    .ok_or("p64: Q4_K SoA requant missing source size")?;
                let source_start = tensor_data_start + entry.source_offset as usize;
                let source_end = source_start + source_blob_size;
                let raw = &input[source_start..source_end];
                let n0 = info.dims[0] as usize;
                let n1 = info.dims[1].max(1) as usize;
                let total_elems = n0
                    .checked_mul(n1)
                    .ok_or("p64: Q4_K SoA requant element count overflow")?;
                // Dequantize source → f32, then re-quantize f32 → Q4_K_SOA.
                let mut f32_buf: Vec<f32> = vec![0.0f32; total_elems];
                crate::ggml_quants::dequantize_row_into(
                    raw,
                    info.ggml_type,
                    total_elems,
                    &mut f32_buf,
                )
                .map_err(|e| format!("p64: Q4_K SoA requant dequant: {e:?}"))?;
                crate::ggml_quants::quantize_f32_to_q4_k_soa_tensor(
                    &f32_buf,
                    n0,
                    n1,
                    &mut output[target_start..target_end],
                )
                .map_err(|e| format!("p64: Q4_K SoA requant: {e:?}"))?;
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
