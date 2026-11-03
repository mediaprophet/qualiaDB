use super::*;

pub const P64_ROLE_VISION_CONV2D: u16 = 0x80;
pub const P64_ROLE_VISION_BN: u16 = 0x81;
pub const P64_ROLE_VISION_FC: u16 = 0x82;

/// A raw vision tensor description for P64 weight container packing.
#[derive(Debug, Clone)]
pub struct RawVisionTensor {
    pub name: String,
    pub shape: Vec<usize>,
    pub data: Vec<f32>,
    pub role_id: u16,
}

/// Transcode raw vision model tensors into a zero-copy P64 container format.
pub fn transcode_vision_tensors_to_p64<W: std::io::Write>(
    tensors: &[RawVisionTensor],
    page_log2: u16,
    out: &mut W,
) -> Result<TranscodeReport, String> {
    if tensors.is_empty() {
        return Err("p64: Vision tensor list is empty".to_string());
    }
    let page_log2 = if page_log2 == 0 {
        P64_DEFAULT_PAGE_LOG2
    } else {
        page_log2
    };
    let page = 1usize << page_log2;

    let mut entries = Vec::with_capacity(tensors.len());
    let mut names_blob = Vec::new();
    let mut total_bytes = 0usize;
    let mut max_working = 0usize;

    let mut name_offsets = Vec::with_capacity(tensors.len());
    for tensor in tensors {
        let name_off = names_blob.len() as u32;
        name_offsets.push(name_off);
        names_blob.extend_from_slice(tensor.name.as_bytes());
        names_blob.push(0);
    }

    let header_bytes = P64_WEIGHT_HEADER_BYTES; // 64
    let hparams_offset = 64u32;
    let hparams_bytes = 64usize;
    let tensor_table_offset = (header_bytes + hparams_bytes) as u32; // 128
    let entry_bytes = tensors.len() * P64_TENSOR_ENTRY_BYTES;
    let string_table_offset = (tensor_table_offset as usize + entry_bytes) as u32;
    let names_bytes = names_blob.len();
    let manifold_table_offset = align_up((string_table_offset as usize) + names_bytes, 64) as u32;
    let manifold_bytes = 64usize; // 1 layer default
    let tokenizer_offset = (manifold_table_offset as usize + manifold_bytes) as u32;
    let checksum_offset = tokenizer_offset;
    let checksum_count = tensors.len() + 1;
    let checksum_bytes = checksum_count * 4;
    let meta_region_bytes = (checksum_offset as usize) + checksum_bytes;
    let blob_region_start = align_up(meta_region_bytes, page);

    let mut current_blob_offset = blob_region_start;
    let mut tensor_crcs = Vec::with_capacity(tensors.len());

    for (i, tensor) in tensors.iter().enumerate() {
        let name_off = name_offsets[i];
        let byte_len = tensor.data.len() * std::mem::size_of::<f32>();
        max_working = max_working.max(byte_len);

        let mut dimensions = [1u32; 4];
        for (i, &dim) in tensor.shape.iter().take(4).enumerate() {
            dimensions[i] = dim as u32;
        }

        let blob_off = current_blob_offset;
        entries.push(P64TensorEntry {
            name_offset: name_off,
            role_id: tensor.role_id,
            dtype: crate::safetensor::GGML_F32 as u16,
            manifold_idx: 0,
            rank: tensor.shape.len() as u32,
            dimensions,
            blob_offset: blob_off as u32,
            blob_size: byte_len as u32,
            source_offset: 0,
            source_name_hash: crate::q_hash(&tensor.name),
            alt_dtype: 0,
            precision_views_mask: 0,
            alt_blob_offset: 0,
        });

        // Compute tensor CRC32C
        let mut raw_bytes = Vec::with_capacity(byte_len);
        for &val in &tensor.data {
            raw_bytes.extend_from_slice(&val.to_le_bytes());
        }
        let crc = crate::container_10d::crc32c::crc32c(&raw_bytes);
        tensor_crcs.push(crc);

        current_blob_offset = align_up(current_blob_offset + byte_len, page);
        total_bytes += byte_len;
    }

    let mut meta_buf = vec![0u8; checksum_offset as usize];

    let hdr = P64WeightHeader {
        magic: P64_MAGIC,
        version: P64_VERSION,
        flags: P64_FLAG_LITTLE_ENDIAN,
        role_table_offset: tensor_table_offset,
        tensor_table_offset,
        tokenizer_offset,
        hparams_offset,
        string_table_offset,
        checksum_offset,
        manifold_table_offset,
        tensor_count: entries.len() as u32,
        page_size: page as u32,
        reserved: [0u8; 20],
    };
    hdr.write_le(&mut meta_buf[0..64]);

    // HParams
    let hparams = P64HParams {
        n_layer: 0,
        n_embd: 0,
        n_head: 0,
        n_kv_head: 0,
        vocab_size: 0,
        rope_freq_base: 10000.0,
        rope_scale: 1.0,
        head_dim: 0,
        head_dim_swa: 0,
        sliding_window: 0,
        shared_kv_layers: 0,
        logit_softcap: 0.0,
        architecture: 0,
        arch_flags: 0,
        reserved: [0u8; 8],
    };
    hparams.write_le(&mut meta_buf[64..128]);

    // Tensor Entries
    for (i, entry) in entries.iter().enumerate() {
        let off = (tensor_table_offset as usize) + i * P64_TENSOR_ENTRY_BYTES;
        write_tensor_entry(entry, &mut meta_buf[off..off + P64_TENSOR_ENTRY_BYTES]);
    }

    // String Table
    meta_buf[string_table_offset as usize..(string_table_offset as usize) + names_bytes]
        .copy_from_slice(&names_blob);

    // Compute Metadata CRC32C over meta_buf
    let meta_crc = crate::container_10d::crc32c::crc32c(&meta_buf);

    out.write_all(&meta_buf).map_err(|e| e.to_string())?;

    // Write Checksum Table (metadata CRC first, followed by tensor CRCs)
    let mut checksum_buf = vec![0u8; checksum_bytes];
    checksum_buf[0..4].copy_from_slice(&meta_crc.to_le_bytes());
    for (i, &crc) in tensor_crcs.iter().enumerate() {
        let off = 4 + i * 4;
        checksum_buf[off..off + 4].copy_from_slice(&crc.to_le_bytes());
    }
    out.write_all(&checksum_buf).map_err(|e| e.to_string())?;

    // Pad to blob_region_start
    let pad_to_blobs = blob_region_start - meta_region_bytes;
    if pad_to_blobs > 0 {
        out.write_all(&vec![0u8; pad_to_blobs])
            .map_err(|e| e.to_string())?;
    }

    let mut written_bytes = blob_region_start;
    for tensor in tensors {
        let mut raw_bytes = Vec::with_capacity(tensor.data.len() * 4);
        for &val in &tensor.data {
            raw_bytes.extend_from_slice(&val.to_le_bytes());
        }
        out.write_all(&raw_bytes).map_err(|e| e.to_string())?;
        written_bytes += raw_bytes.len();

        let pad = align_up(raw_bytes.len(), page) - raw_bytes.len();
        if pad > 0 {
            out.write_all(&vec![0u8; pad]).map_err(|e| e.to_string())?;
            written_bytes += pad;
        }
    }

    Ok(TranscodeReport {
        n_tensors: tensors.len(),
        bytes_written: written_bytes,
        largest_tensor_bytes: max_working,
        total_tensor_bytes: total_bytes,
        peak_working_bytes: max_working,
    })
}
