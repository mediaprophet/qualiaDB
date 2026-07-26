
use super::*;

/// Build a minimal safetensor with the given `(name, dtype, shape, nbytes)` tensors (zeroed
/// data, but each tensor stamped with a distinct first byte so round-trips are checkable).
fn synth_safetensor(t: &[(&str, &str, Vec<usize>, usize)]) -> Vec<u8> {
    let mut entries = serde_json::Map::new();
    let mut cursor = 0usize;
    for (name, dtype, shape, nbytes) in t {
        let (begin, end) = (cursor, cursor + nbytes);
        cursor = end;
        entries.insert(
            (*name).to_string(),
            serde_json::json!({ "dtype": dtype, "shape": shape, "data_offsets": [begin, end] }),
        );
    }
    let header_bytes = serde_json::to_vec(&serde_json::Value::Object(entries)).unwrap();
    let mut out = Vec::new();
    out.extend_from_slice(&(header_bytes.len() as u64).to_le_bytes());
    out.extend_from_slice(&header_bytes);
    let data_start = out.len();
    out.resize(out.len() + cursor, 0u8);
    // stamp each tensor's first byte with an index so we can verify the right bytes landed.
    let plan = crate::safetensor::parse_safetensor_header(&out).unwrap();
    for (i, te) in plan.tensors.iter().enumerate() {
        out[data_start + te.begin] = (i as u8) + 1;
    }
    out
}

/// GATE B: streaming safetensor → P64 round-trips, and peak working memory ≈ the largest
/// single tensor (NOT the whole file).
#[test]
fn transcode_safetensor_streams_and_round_trips() {
    // three F16 tensors of 8 / 64 / 16 bytes (largest = 64; total = 88).
    let src = synth_safetensor(&[
        ("a", "F16", vec![4], 8),
        ("big", "F16", vec![32], 64),
        ("c", "F16", vec![8], 16),
    ]);
    let mut out = Vec::new();
    let report = transcode_safetensor_to_p64(&src, 12, &mut out).unwrap();

    // peak working memory == largest tensor, and strictly less than the sum (not the whole file).
    assert_eq!(report.n_tensors, 3);
    assert_eq!(report.largest_tensor_bytes, 64);
    assert_eq!(report.total_tensor_bytes, 88);
    assert_eq!(
        report.peak_working_bytes, 64,
        "one tensor in flight = largest, not the file"
    );
    assert!(report.peak_working_bytes < report.total_tensor_bytes);

    // the emitted container is a valid P64 and parses back.
    let idx = P64TensorIndex::from_p64(&out).expect("transcoded container must round-trip");
    assert_eq!(idx.header.n_tensors, 3);
    assert_eq!(
        idx.header.format_flags & FORMAT_FLAG_RAW_TRANSCODE,
        FORMAT_FLAG_RAW_TRANSCODE
    );
    assert_eq!(idx.entries.len(), 3);

    // tensor bytes survived verbatim: each blob's first byte is its stamp; sizes match.
    let plan = crate::safetensor::parse_safetensor_header(&src).unwrap();
    for (i, (e, st)) in idx.entries.iter().zip(plan.tensors.iter()).enumerate() {
        let blob = idx.blob(&out, e);
        assert_eq!(blob.len(), st.blob_size());
        assert_eq!(blob[0], (i as u8) + 1, "tensor {i} bytes mismatch");
        // identity preserved as the name hash.
        assert_eq!(e.scaffold_quin.subject, crate::q_hash(st.name.as_str()));
    }
}

/// GATE B: a low-precision (Q4-class) dtype is rejected — high-fidelity sources only.
#[test]
fn transcode_rejects_low_precision() {
    // "U8" is not a high-fidelity weight dtype → the dtype gate rejects it.
    let src = synth_safetensor(&[("w", "U8", vec![16], 16)]);
    let mut out = Vec::new();
    let err = transcode_safetensor_to_p64(&src, 12, &mut out).unwrap_err();
    assert!(
        err.contains("high-fidelity") || err.contains("rejected"),
        "got: {err}"
    );
    // and the underlying GGML gate rejects Q4_K directly.
    assert!(!crate::safetensor::is_high_fidelity_ggml(12));
}

/// TASK #12 (§A): ternary transcode compresses an F16 tensor to ≈1.6 bits/weight and the
/// container round-trips + dequantizes correctly.
#[test]
fn transcode_ternary_compresses_and_round_trips() {
    // one F16 tensor, 100 weights of alternating ±2.0 (absmean scale = 2.0, exact reconstruction)
    let count = 100usize;
    let weights: Vec<f32> = (0..count)
        .map(|i| if i % 2 == 0 { 2.0 } else { -2.0 })
        .collect();
    let mut data = Vec::new();
    for &w in &weights {
        data.extend_from_slice(&half::f16::from_f32(w).to_le_bytes());
    }
    let header = serde_json::json!({ "w": { "dtype": "F16", "shape": [count], "data_offsets": [0, data.len()] } });
    let hb = serde_json::to_vec(&header).unwrap();
    let mut src = Vec::new();
    src.extend_from_slice(&(hb.len() as u64).to_le_bytes());
    src.extend_from_slice(&hb);
    src.extend_from_slice(&data);

    let mut out = Vec::new();
    let report = transcode_safetensor_to_p64_ternary(&src, 12, &mut out).unwrap();
    assert_eq!(report.n_tensors, 1);

    // source F16 tensor = 200 bytes; ternary blob = 4 + ceil(100/5) = 24 bytes (>8x smaller).
    assert_eq!(
        report.total_tensor_bytes,
        crate::ternary::ternary_blob_len(count)
    );
    assert!(
        report.total_tensor_bytes * 5 < data.len(),
        "ternary must be >5x smaller than F16"
    );

    // container round-trips and is flagged ternary.
    let idx = P64TensorIndex::from_p64(&out).expect("ternary container must round-trip");
    assert_eq!(
        idx.header.format_flags & FORMAT_FLAG_TERNARY,
        FORMAT_FLAG_TERNARY
    );
    assert_eq!(
        idx.entries[0].ggml_type,
        crate::ternary::GGML_TYPE_TERNARY_158
    );

    // dequantize: uniform ±2.0 → scale (absmean) = 2.0, so reconstruction is exact ±2.0.
    let blob = idx.blob(&out, &idx.entries[0]);
    let mut deq = vec![0.0f32; count];
    crate::ternary::dequantize_blob(blob, &mut deq);
    assert!((deq[0] - 2.0).abs() < 1e-3, "deq[0] {}", deq[0]);
    assert!((deq[1] + 2.0).abs() < 1e-3, "deq[1] {}", deq[1]);
}

/// TASK #12 (§A): policy transcode ternaries the FFN, keeps attention/norm verbatim, populates
/// engine roles, and round-trips — all in one container.
#[test]
fn transcode_ffn_ternary_policy_mixed_container() {
    // P64_ROLE_* and P64_LAYER_GLOBAL are in scope via `use super::*`.
    // three HF-named F16 tensors: an FFN gate (ternary), an attention q_proj + a norm (verbatim).
    let count = 50usize;
    let f16 = |v: f32| half::f16::from_f32(v).to_le_bytes();
    let mut gate = Vec::new();
    let mut q = Vec::new();
    let mut norm = Vec::new();
    for i in 0..count {
        gate.extend_from_slice(&f16(if i % 2 == 0 { 1.0 } else { -1.0 }));
        q.extend_from_slice(&f16(0.25));
        norm.extend_from_slice(&f16(0.5));
    }
    let (gl, ql) = (gate.len(), q.len());
    let header = serde_json::json!({
        "model.layers.0.mlp.gate_proj.weight": { "dtype": "F16", "shape": [count], "data_offsets": [0, gl] },
        "model.layers.0.self_attn.q_proj.weight": { "dtype": "F16", "shape": [count], "data_offsets": [gl, gl + ql] },
        "model.norm.weight": { "dtype": "F16", "shape": [count], "data_offsets": [gl + ql, gl + ql + norm.len()] },
    });
    let hb = serde_json::to_vec(&header).unwrap();
    let mut src = Vec::new();
    src.extend_from_slice(&(hb.len() as u64).to_le_bytes());
    src.extend_from_slice(&hb);
    src.extend_from_slice(&gate);
    src.extend_from_slice(&q);
    src.extend_from_slice(&norm);

    let mut out = Vec::new();
    let report = transcode_safetensor_to_p64_ffn_ternary(&src, 12, &mut out).unwrap();
    assert_eq!(report.n_tensors, 3);

    let idx = P64TensorIndex::from_p64(&out).expect("mixed container must round-trip");
    // entries are ordered by source offset: gate, q_proj, norm.
    let by_role = |role_id: u16| {
        idx.entries
            .iter()
            .find(|e| e.role == role)
            .expect("role present")
    };

    // FFN gate → ternary, FFN_GATE role, much smaller than its F16 source (100 bytes).
    let g = by_role(P64_ROLE_FFN_GATE);
    assert_eq!(g.ggml_type, crate::ternary::GGML_TYPE_TERNARY_158);
    assert_eq!(g.layer, 0);
    assert_eq!(g.byte_len as usize, crate::ternary::ternary_blob_len(count)); // 4 + ceil(50/5) = 14
    assert!((g.byte_len as usize) * 5 < gate.len());

    // attention q_proj → verbatim F16, ATTN_Q role.
    let a = by_role(P64_ROLE_ATTN_Q);
    assert_eq!(a.ggml_type, crate::safetensor::GGML_F16);
    assert_eq!(a.byte_len as usize, ql); // verbatim, unchanged
    assert_eq!(idx.blob(&out, a), &q[..]); // bytes preserved exactly

    // norm → verbatim, OUTPUT_NORM (global).
    let nrm = by_role(P64_ROLE_OUTPUT_NORM);
    assert_eq!(nrm.ggml_type, crate::safetensor::GGML_F16);
    assert_eq!(nrm.layer, P64_LAYER_GLOBAL);

    // the FFN blob dequantizes (±1.0 uniform → scale 1.0 → ±1.0).
    let mut deq = vec![0.0f32; count];
    crate::ternary::dequantize_blob(idx.blob(&out, g), &mut deq);
    assert!((deq[0] - 1.0).abs() < 1e-3 && (deq[1] + 1.0).abs() < 1e-3);
}

fn le_u16(b: &[u8], o: usize) -> u16 {
    u16::from_le_bytes(b[o..o + 2].try_into().unwrap())
}
fn le_u32(b: &[u8], o: usize) -> u32 {
    u32::from_le_bytes(b[o..o + 4].try_into().unwrap())
}
fn le_u64(b: &[u8], o: usize) -> u64 {
    u64::from_le_bytes(b[o..o + 8].try_into().unwrap())
}

#[test]
fn compile_smollm2_to_p64_layout() {
    let path = "C:/Projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
    if !std::path::Path::new(path).exists() {
        eprintln!("[p64] model not present — skipping");
        return;
    }
    let gguf = std::fs::read(path).expect("read gguf");
    let p64 = compile_gguf_to_p64(&gguf, 0).expect("compile");

    // Magic + version + default page size.
    assert_eq!(&p64[0..4], &P64_MAGIC, "magic");
    assert_eq!(le_u16(&p64, 4), P64_VERSION, "version");
    assert_eq!(le_u16(&p64, 6), 14, "default page_log2 = 16KB");
    let page = 1usize << 14;

    // Tensor count: SmolLM2-360M has 32 layers × 9 per-layer tensors + globals.
    let n_tensors = le_u32(&p64, 8) as usize;
    let n_layers = le_u32(&p64, 12);
    assert_eq!(n_layers, 32, "n_layers");
    assert!(
        n_tensors >= 32 * 9,
        "expected ≥288 tensors, got {n_tensors}"
    );

    // Hyperparameter block (v2 header) round-trips SmolLM2-360M geometry.
    assert_eq!(le_u32(&p64, 16), 960, "n_embd");
    assert_eq!(le_u32(&p64, 20), 15, "n_head");
    assert_eq!(le_u32(&p64, 24), 5, "n_kv_head");

    // Blob region + the first tensor blob both sit on a 16KB boundary.
    let manifest_offset = le_u64(&p64, 40) as usize;
    let blob_offset = le_u64(&p64, 48) as usize;
    assert_eq!(blob_offset % page, 0, "blob region 16KB-aligned");
    let first_entry = manifest_offset; // entry[0]
    let first_blob = le_u64(&p64, first_entry + 16) as usize; // blob_offset field @ entry+16
    let first_len = le_u64(&p64, first_entry + 24) as usize;
    assert_eq!(first_blob % page, 0, "first tensor blob 16KB-aligned");
    assert_eq!(first_blob, blob_offset, "first blob == blob region start");
    assert!(first_blob + first_len <= p64.len(), "first blob in-bounds");

    // Every tensor blob is 16KB-aligned and in-bounds.
    for k in 0..n_tensors {
        let e = manifest_offset + k * P64_TENSOR_ENTRY_BYTES;
        let bo = le_u64(&p64, e + 16) as usize;
        let bl = le_u64(&p64, e + 24) as usize;
        assert_eq!(bo % page, 0, "tensor {k} blob 16KB-aligned");
        assert!(bo + bl <= p64.len(), "tensor {k} in-bounds");
    }

    // Round-trip through the runtime reader.
    let idx = P64TensorIndex::from_p64(&p64).expect("from_p64");
    assert_eq!(idx.entries.len(), n_tensors, "reader entry count");
    assert_eq!(
        idx.header.blob_offset as usize, blob_offset,
        "reader blob_offset"
    );
    let hp = idx.hyperparams();
    assert_eq!(hp.n_layer, 32);
    assert_eq!(hp.n_embd, 960);
    assert_eq!(hp.n_head, 15);
    assert_eq!(hp.effective_n_kv_head(), 5);
    for (k, e) in idx.entries.iter().enumerate() {
        assert_eq!(e.blob_offset as usize % page, 0, "reader entry {k} aligned");
        assert_eq!(
            idx.blob(&p64, e).len(),
            e.byte_len as usize,
            "reader blob len {k}"
        );
    }
    // Bad magic is rejected.
    let mut bad = p64.clone();
    bad[0] = b'X';
    assert!(
        P64TensorIndex::from_p64(&bad).is_err(),
        "bad magic rejected"
    );

    // Integrity: header CRC populated; a flipped manifest byte (corrupted offset) is rejected.
    assert_ne!(le_u32(&p64, 72), 0, "header_crc populated");
    let mut tampered = p64.clone();
    tampered[manifest_offset + 16] ^= 0xFF; // first entry's blob_offset
    assert!(
        P64TensorIndex::from_p64(&tampered).is_err(),
        "manifest tamper must be caught by CRC before any bind"
    );

    eprintln!(
            "[p64] OK: {n_tensors} tensors, {n_layers} layers, blob@{blob_offset}, total {} MB; reader round-trip + hyperparams verified",
            p64.len() / (1024 * 1024)
        );
}

/// Proves inference-from-P64 equivalence WITHOUT a browser: the synthetic GGUF index built
/// from the P64 manifest returns byte-identical weights + matching metadata vs the original
/// GGUF index for every tensor. Identical weights → identical logits → identical output. The
/// P64 carries the tokenizer in its embedded Q42T section.
#[test]
fn p64_synthetic_index_matches_gguf() {
    let path = "C:/Projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
    if !std::path::Path::new(path).exists() {
        eprintln!("[p64] model not present — skipping");
        return;
    }
    let gguf = std::fs::read(path).expect("read gguf");
    let p64 = compile_gguf_to_p64(&gguf, 0).expect("compile");
    let orig = GgufTensorIndex::from_gguf(&gguf);
    let q = P64TensorIndex::from_p64(&p64).expect("from_p64");
    let synth = q.to_gguf_index();

    let mut checked = 0usize;
    let mut cmp = |label: &str, s: Option<GgufTensorInfo>, o: Option<GgufTensorInfo>| match (s, o) {
        (Some(s), Some(o)) => {
            assert_eq!(s.ggml_type, o.ggml_type, "{label} ggml_type");
            assert_eq!(s.dims[0], o.dims[0], "{label} dim0");
            assert_eq!(s.dims[1], o.dims[1], "{label} dim1");
            let sb = crate::ggml_quants::fetch_tensor_bytes(&p64, synth.tensor_data_start, &s)
                .expect("P64 tensor bytes");
            let ob = crate::ggml_quants::fetch_tensor_bytes(&gguf, orig.tensor_data_start, &o)
                .expect("gguf tensor bytes");
            assert_eq!(sb, ob, "{label} weight bytes differ");
            checked += 1;
        }
        (None, None) => {}
        _ => panic!("{label}: tensor presence mismatch (synthetic vs gguf)"),
    };
    for l in 0..orig.hyperparams.n_layer {
        let st = synth.get_layer_tensors(l);
        let ot = orig.get_layer_tensors(l);
        cmp(&format!("L{l}.attn_q"), st.attn_q, ot.attn_q);
        cmp(&format!("L{l}.attn_k"), st.attn_k, ot.attn_k);
        cmp(&format!("L{l}.attn_v"), st.attn_v, ot.attn_v);
        cmp(&format!("L{l}.attn_output"), st.attn_output, ot.attn_output);
        cmp(&format!("L{l}.attn_norm"), st.attn_norm, ot.attn_norm);
        cmp(&format!("L{l}.ffn_gate"), st.ffn_gate, ot.ffn_gate);
        cmp(&format!("L{l}.ffn_up"), st.ffn_up, ot.ffn_up);
        cmp(&format!("L{l}.ffn_down"), st.ffn_down, ot.ffn_down);
        cmp(&format!("L{l}.ffn_norm"), st.ffn_norm, ot.ffn_norm);
    }
    cmp(
        "token_embd",
        synth.token_embd_info().copied(),
        orig.token_embd_info().copied(),
    );
    cmp(
        "output",
        synth.output_weight_info().copied(),
        orig.output_weight_info().copied(),
    );
    cmp(
        "output_norm",
        synth.output_norm_info().copied(),
        orig.output_norm_info().copied(),
    );

    assert!(
        checked >= 32 * 9,
        "expected ≥288 tensors byte-checked, got {checked}"
    );
    eprintln!("[p64] synthetic index == GGUF: {checked} tensors byte-identical + metadata match");
}

/// Proves the v3 tokenizer section round-trips: a tokenizer rebuilt from the P64 section
/// encodes/decodes identically to the GGUF tokenizer. With weight byte-parity (above), this
/// guarantees P64-only inference produces the same tokens as the GGUF path.
#[test]
fn p64_tokenizer_roundtrip() {
    use crate::gguf_sharder::GgufTokenizer;
    let path = "C:/Projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
    if !std::path::Path::new(path).exists() {
        eprintln!("[p64] model not present — skipping");
        return;
    }
    let gguf = std::fs::read(path).expect("read gguf");
    let p64 = compile_gguf_to_p64(&gguf, 0).expect("compile");
    let q = P64TensorIndex::from_p64(&p64).expect("from_p64");

    let tok_bytes = q.tokenizer_bytes(&p64);
    assert!(!tok_bytes.is_empty(), "tokenizer section present");
    let tok_p64 = GgufTokenizer::from_p64_section(tok_bytes).expect("from_p64_section");
    let tok_gguf = GgufTokenizer::from_gguf(&gguf);

    assert_eq!(tok_p64.bos_token_id, tok_gguf.bos_token_id, "bos");
    assert_eq!(tok_p64.eos_token_id, tok_gguf.eos_token_id, "eos");
    assert_eq!(tok_p64.add_bos_token, tok_gguf.add_bos_token, "add_bos");
    assert_eq!(tok_p64.vocab.len(), tok_gguf.vocab.len(), "vocab len");
    for prompt in [
        "The capital of France is",
        "<|im_start|>user\nWhat is the capital of France?<|im_end|>\n<|im_start|>assistant\n",
    ] {
        assert_eq!(
            tok_p64.encode_prompt(prompt),
            tok_gguf.encode_prompt(prompt),
            "encode mismatch for {prompt:?}"
        );
    }
    let ids = tok_gguf.encode_prompt("The capital of France is");
    assert_eq!(
        tok_p64.decode(&ids),
        tok_gguf.decode(&ids),
        "decode mismatch"
    );
    eprintln!(
        "[p64] tokenizer round-trip: encode/decode identical to GGUF ({} vocab, section {} KB)",
        tok_p64.vocab.len(),
        tok_bytes.len() / 1024
    );
}
