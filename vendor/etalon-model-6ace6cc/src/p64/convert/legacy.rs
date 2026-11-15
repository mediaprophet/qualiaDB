//! Retained private historical compiler.
use super::*;
pub type LegacyTensor = (u16, u16, GgufTensorInfo);
#[derive(Debug)]
pub struct LegacyInput<'a> {
    pub tensor_data_start: usize,
    pub hparams: &'a crate::inference::ModelHyperparams,
    pub tensors: &'a [LegacyTensor],
    pub names: &'a [String],
    pub vocab: &'a [String],
    pub manifold_bytes: &'a [u8],
}

pub fn compile_legacy(
    input: &[u8],
    page_log2: u16,
    source: LegacyInput<'_>,
    budget: ConversionBudget,
) -> Result<Vec<u8>, String> {
    let page_log2 = if page_log2 == 0 {
        12 // 4096 default
    } else {
        page_log2
    };
    if page_log2 < 8 || page_log2 > 30 {
        return Err(format!("page_log2 {page_log2} out of range"));
    }
    let page = 1usize << page_log2;
    let tds = source.tensor_data_start;
    let planned = source.tensors;
    if source.names.len() != planned.len() {
        return Err("p64: tensor name count mismatch".into());
    }
    let names_bytes = names_len(source.names.iter().map(String::as_str))?;
    let vocab_bytes = source
        .vocab
        .iter()
        .try_fold(4usize, |n, s| n.checked_add(4)?.checked_add(s.len()))
        .ok_or("p64: tokenizer overflow")?;
    budget.scratch(workspace_sum(&[
        planned
            .len()
            .checked_mul(256)
            .ok_or("p64: workspace overflow")?,
        names_bytes
            .checked_mul(3)
            .ok_or("p64: workspace overflow")?,
        vocab_bytes
            .checked_mul(3)
            .ok_or("p64: workspace overflow")?,
        128,
    ])?)?;
    // Build the string table
    let mut string_table: Vec<u8> = Vec::with_capacity(
        names_bytes
            .checked_add(vocab_bytes)
            .and_then(|n| n.checked_add(64))
            .ok_or("p64: workspace overflow")?,
    );
    let mut name_offsets = std::collections::HashMap::new();

    // Push a dummy null at start
    string_table.push(0u8);

    for name in source.names {
        if !name_offsets.contains_key(name) {
            let offset = string_table.len() as u32;
            name_offsets.insert(name.clone(), offset);
            string_table.extend_from_slice(name.as_bytes());
            string_table.push(0u8);
        }
    }

    // Now extract vocabulary from GGUF and append to string table.
    let tokenizer_offset = string_table.len() as u32;
    let vocab = source.vocab;

    let mut tok_bytes: Vec<u8> = Vec::with_capacity(vocab_bytes);
    // Serialize vocabulary sizes and strings
    tok_bytes.extend_from_slice(&(vocab.len() as u32).to_le_bytes());
    for v in vocab {
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
    let manifold_table = source.manifold_bytes;
    let total_layers = source.hparams.n_layer;
    let manifold_table_size = manifold_table.len() as u32;

    // Layout
    let hparams_offset = 64;
    let entries_offset = 128;
    let string_table_offset = entries_offset + (tensor_count * 64) as u32;
    let manifold_table_offset = string_table_offset + string_table_size;
    let end_of_manifold_table = manifold_table_offset + manifold_table_size;
    let page_aligned_tensor_start =
        (end_of_manifold_table + (page as u32) - 1) & !((page as u32) - 1);

    budget.output(page_aligned_tensor_start as usize)?;
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
    let hparams = source.hparams;
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

        let name = &source.names[i];
        let n_offset = *name_offsets.get(name).unwrap();

        // Align blob to its internal alignment requirement if needed, but P64 natively page aligns at start
        // and blobs follow one another. Wait, ggml requires 32-byte alignment usually.
        // Let's pad cursor_blob to 32 bytes
        cursor_blob = (cursor_blob + 31) & !31;

        let n_elements =
            info.dims[0] * info.dims[1].max(1) * info.dims[2].max(1) * info.dims[3].max(1);
        let byte_len = crate::ggml::tensor_byte_len(info).unwrap_or(0);

        // Copy the tensor blob
        let end = cursor_blob
            .checked_add(byte_len)
            .ok_or("p64: output size overflow")?;
        budget.output(end)?;
        out.resize(end, 0);
        let src_start = tds
            .checked_add(
                usize::try_from(info.byte_offset)
                    .map_err(|_| "p64: source tensor offset overflow")?,
            )
            .ok_or("p64: source tensor offset overflow")?;
        if src_start
            .checked_add(byte_len)
            .ok_or("p64: source tensor length overflow")?
            <= input.len()
        {
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
