use super::*;

#[test]
fn probe_gguf_layer_names_if_exists() {
    use memmap2::MmapOptions;
    use std::fs::File;
    let path = "C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf";
    if !std::path::Path::new(path).exists() {
        return;
    }
    let f = File::open(path).unwrap();
    let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };
    if mmap.len() < 24 || &mmap[0..4] != b"GGUF" {
        return;
    }
    let tensor_count = u64::from_le_bytes(mmap[8..16].try_into().unwrap()) as usize;
    let kv_count = u64::from_le_bytes(mmap[16..24].try_into().unwrap()) as usize;
    let mut pos = 24usize;
    for _ in 0..kv_count {
        let klen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().unwrap()) as usize;
        pos += 8;
        let key = std::str::from_utf8(&mmap[pos..pos + klen]).unwrap_or("");
        pos += klen;
        let vtype = u32::from_le_bytes(mmap[pos..pos + 4].try_into().unwrap());
        pos += 4;
        if key.contains("block")
            || key.contains("layer")
            || key.contains("embedding")
            || key.contains("head")
        {
            if vtype == 4 && pos + 4 <= mmap.len() {
                let v = u32::from_le_bytes(mmap[pos..pos + 4].try_into().unwrap());
                println!("KV {key} = {v}");
            }
        }
        gguf_skip_value(&mmap, &mut pos, vtype).unwrap();
    }
    let _blk_samples = 0usize;
    for _ in 0..tensor_count {
        let nlen = u64::from_le_bytes(mmap[pos..pos + 8].try_into().unwrap()) as usize;
        pos += 8;
        let name = std::str::from_utf8(&mmap[pos..pos + nlen]).unwrap_or("");
        pos += nlen;
        let n_dims = u32::from_le_bytes(mmap[pos..pos + 4].try_into().unwrap()) as usize;
        pos += 4;
        let mut dims = [0u64; 4];
        for d in 0..n_dims {
            dims[d] = u64::from_le_bytes(mmap[pos..pos + 8].try_into().unwrap());
            pos += 8;
        }
        let ggml_type = u32::from_le_bytes(mmap[pos..pos + 4].try_into().unwrap());
        let byte_off = u64::from_le_bytes(mmap[pos + 4..pos + 12].try_into().unwrap());
        pos += 12;
        if name.starts_with("blk.0.") && (name.contains("attn_q") || name.contains("ffn_down")) {
            println!("tensor: {name} type={ggml_type} dims={dims:?} off={byte_off:#x}");
        }
    }
}

#[test]
fn test_gguf_ontology_extraction() {
    let sharder = GGufSharder::new(
        "C:/Projects/qualiaDB/gemma-4-E4B-it-GGUF/gemma-4-E4B-it-Q4_K_M.gguf".to_string(),
    );

    let superblock = sharder.extract_ontology_to_superblock();
    // Just verify it yields a superblock structural scaffold
    assert_eq!(
        superblock.active_quin_count, 0,
        "SuperBlock should be freshly initialized"
    );
}

#[test]
fn test_gguf_bidx_pointer_generation() {
    use crate::QuinPointerExt;

    let sharder = GGufSharder::new("mock_model.gguf".to_string());
    let pointers = sharder.generate_bidx_pointer_map();

    assert_eq!(pointers.len(), 1, "Failed to generate pointer map");

    let quin = pointers[0];
    assert_eq!(
        quin.extract_modality_flag(),
        crate::MODALITY_FLAG_LLM_TENSOR,
        "Pointer Modality Flag was not LLM"
    );
    assert_eq!(
        quin.extract_byte_offset(),
        0x00000ABC,
        "Pointer byte offset extracted incorrectly"
    );
}

#[test]
fn encode_prompt_prepends_bos_when_enabled() {
    let mut tok = GgufTokenizer::default();
    tok.add_bos_token = true;
    tok.bos_token_id = 42;
    let ids = tok.encode_prompt("hi");
    assert_eq!(ids.first(), Some(&42));
    assert!(ids.len() >= 2);
}

#[test]
fn stop_tokens_include_chat_end_specials() {
    let mut tok = GgufTokenizer::default();
    tok.eos_token_id = 2;
    tok.token_to_id_map.insert("<|eot_id|>".into(), 128009);
    tok.token_to_id_map.insert("<|im_end|>".into(), 151645);
    tok.rebuild_stop_token_ids();
    assert!(tok.is_stop_token(2), "eos must stop");
    assert!(tok.is_stop_token(128009), "Llama-3 eot_id must stop");
    assert!(tok.is_stop_token(151645), "ChatML im_end must stop");
    assert!(!tok.is_stop_token(42), "ordinary id must not stop");
    assert!(tok.stop_tokens().len() >= 3);
}

#[test]
fn merge_stop_token_ids_from_helper() {
    let mut tok = GgufTokenizer::default();
    tok.eos_token_id = 2;
    tok.rebuild_stop_token_ids();
    tok.merge_stop_token_ids(&[128009, 2, 151645]);
    assert!(tok.is_stop_token(128009));
    assert!(tok.is_stop_token(151645));
    assert!(tok.is_stop_token(2));
}

#[test]
fn chat_template_family_detection_and_rendering() {
    // ChatML (SmolLM2 / Qwen2): <|im_start|> present in the vocab.
    let mut chatml = GgufTokenizer::default();
    chatml.token_to_id_map.insert("<|im_start|>".into(), 100);
    assert_eq!(chatml.chat_family(), ChatFamily::ChatMl);
    assert_eq!(
        chatml.apply_chat_template(None, "hi"),
        "<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n"
    );
    assert_eq!(
        chatml.apply_chat_template(Some("be brief"), "hi"),
        "<|im_start|>system\nbe brief<|im_end|>\n<|im_start|>user\nhi<|im_end|>\n<|im_start|>assistant\n"
    );

    // Llama-3.x: <|start_header_id|> present.
    let mut l3 = GgufTokenizer::default();
    l3.token_to_id_map.insert("<|start_header_id|>".into(), 100);
    assert_eq!(l3.chat_family(), ChatFamily::Llama3);
    assert_eq!(
        l3.apply_chat_template(None, "hi"),
        "<|start_header_id|>user<|end_header_id|>\n\nhi<|eot_id|><|start_header_id|>assistant<|end_header_id|>\n\n"
    );

    // Gemma: <start_of_turn> present (no system role — folded into the user turn).
    let mut g = GgufTokenizer::default();
    g.token_to_id_map.insert("<start_of_turn>".into(), 100);
    assert_eq!(g.chat_family(), ChatFamily::Gemma);
    assert_eq!(
        g.apply_chat_template(None, "hi"),
        "<start_of_turn>user\nhi<end_of_turn>\n<start_of_turn>model\n"
    );

    // Gemma 4: <|turn> / <turn|> paired markers.
    let mut g4 = GgufTokenizer::default();
    g4.token_to_id_map.insert("<|turn>".into(), 105);
    g4.token_to_id_map.insert("<turn|>".into(), 106);
    assert_eq!(g4.chat_family(), ChatFamily::Gemma4);
    assert_eq!(
        g4.apply_chat_template(None, "hi"),
        "<|turn>user\nhi<turn|><|turn>model\n"
    );

    // No recognised chat specials → raw prompt unchanged.
    let none = GgufTokenizer::default();
    assert_eq!(none.chat_family(), ChatFamily::None);
    assert_eq!(none.apply_chat_template(None, "hi"), "hi");
}

#[test]
fn gemma4_decode_supported_fails_closed() {
    let mut h = GgufHyperparams::default();
    h.architecture = ARCH_GEMMA4;
    h.arch_flags = ARCH_FLAG_HAS_PLE | ARCH_FLAG_HAS_SWA | ARCH_FLAG_HAS_SHARED_KV;
    // Ensure force env is not set for this test.
    std::env::remove_var("QUALIA_LLM_FORCE_UNSUPPORTED_ARCH");
    let err = h.decode_supported().unwrap_err();
    assert!(err.contains("gemma4"), "{err}");
    assert!(err.contains("PLE") || err.contains("per-layer"), "{err}");
}

#[test]
fn encode_prompt_skips_duplicate_bos() {
    let mut tok = GgufTokenizer::default();
    tok.add_bos_token = true;
    tok.bos_token_id = 0;
    tok.token_to_id = vec![("<|endoftext|>".into(), 0), ("a".into(), 10)];
    tok.vocab = vec!["<|endoftext|>".into(), "a".into()];
    let ids = tok.encode_prompt("<|endoftext|>a");
    assert_eq!(ids, vec![0, 10]);
}

#[test]
fn decode_maps_bpe_space_marker_to_ascii_space() {
    let mut tok = GgufTokenizer::default();
    tok.vocab = vec!["The".into(), "\u{0120}capital".into(), "\u{2581}of".into()];
    assert_eq!(tok.decode(&[0, 1, 2]), "The capital of");
}

#[test]
fn decode_recovers_exact_gpt2_utf8_bytes() {
    let mut tok = GgufTokenizer::default();
    tok.pre_type = "smollm".into();
    tok.vocab = vec!["caf\u{00c3}\u{00a9}".into(), "\u{0120}ok".into()];
    assert_eq!(tok.decode(&[0, 1]), "café ok");
    assert_eq!(tok.decode_token_bytes_cold(0), b"caf\xc3\xa9");
    assert_eq!(tok.decode_token_bytes_cold(1), b" ok");
}

#[test]
fn hyperparams_default_rope_freq_base_is_100k() {
    let h = GgufHyperparams::default();
    assert_eq!(h.effective_rope_freq_base(), DEFAULT_ROPE_FREQ_BASE);
}

#[test]
fn smollm_gguf_output_weight_tie_probe() {
    use memmap2::MmapOptions;
    use std::fs::File;
    for (label, path) in [
        (
            "Q4_K_M",
            "C:/projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf",
        ),
        (
            "Q8_0",
            "C:/projects/qualiaDB/docs/models/smollm2-360m-instruct-q8_0.gguf",
        ),
    ] {
        if !std::path::Path::new(path).exists() {
            println!("[skip] {label} not at {path}");
            continue;
        }
        let f = File::open(path).unwrap();
        let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };
        let idx = GgufTensorIndex::from_gguf(&mmap);
        let (tied, emb_off, out_off, emb_dims, out_dims) = idx.weight_tie_probe();
        assert!(
            idx.token_embd_info().is_some(),
            "{label}: missing token_embd"
        );
        assert!(
            idx.logits_projection_info().is_some(),
            "{label}: no logits projection"
        );
        println!(
            "[{label}] tied={tied} emb_off={emb_off:#x} dims={emb_dims:?} out_off={out_off:#x} out_dims={out_dims:?}"
        );
        if tied {
            assert_eq!(emb_off, out_off, "{label}: tied offsets must match");
            assert_eq!(emb_dims, out_dims, "{label}: tied dims must match");
        }
    }
}

#[test]
fn smollm_tokenizer_audit_vs_hf_reference() {
    use memmap2::MmapOptions;
    use std::fs::File;
    let path = "C:/projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
    if !std::path::Path::new(path).exists() {
        return;
    }
    let f = File::open(path).unwrap();
    let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };
    let tok = GgufTokenizer::from_gguf(&mmap);
    let chatml = "<|im_start|>user\nWhat is the capital of France? Answer in one short sentence.<|im_end|>\n<|im_start|>assistant\n";
    let naked = "The capital of France is";
    let chat_ids = tok.encode(chatml);
    let naked_ids = tok.encode(naked);
    println!(
        "[audit] bos={} eos={} add_bos={} pre={:?}",
        tok.bos_token_id, tok.eos_token_id, tok.add_bos_token, tok.pre_type
    );
    println!("[audit] chatml len={} ids={:?}", chat_ids.len(), chat_ids);
    println!("[audit] naked len={} ids={:?}", naked_ids.len(), naked_ids);
    const HF_CHATML: &[u32] = &[
        1, 4093, 198, 1780, 314, 260, 3575, 282, 4649, 47, 19842, 281, 582, 1890, 6330, 30, 2, 198,
        1, 520, 9531, 198,
    ];
    const HF_NAKED: &[u32] = &[504, 3575, 282, 4649, 314];
    assert_eq!(
        chat_ids, HF_CHATML,
        "ChatML must not shred <|im_start|> specials"
    );
    assert_eq!(
        naked_ids, HF_NAKED,
        "naked English prompt must match HF BPE"
    );
}

#[test]
fn smollm_gguf_parses_rope_freq_base_when_present() {
    use memmap2::MmapOptions;
    use std::fs::File;
    let path = "C:/projects/qualiaDB/docs/models/SmolLM2-360M-Instruct-Q4_K_M.gguf";
    if !std::path::Path::new(path).exists() {
        return;
    }
    let f = File::open(path).unwrap();
    let mmap = unsafe { MmapOptions::new().map(&f).unwrap() };
    let idx = GgufTensorIndex::from_gguf(&mmap);
    assert!(
        (idx.hyperparams.rope_freq_base - 100_000.0).abs() < 1.0,
        "expected llama.rope.freq_base=100000, got {}",
        idx.hyperparams.rope_freq_base
    );
}

#[test]
fn test_wordnet_lexicon_mapping() {
    use crate::QuinPointerExt;
    let sharder = GGufSharder::new("mock.gguf".to_string());

    // Mock WordNet Synset ID for "Dog"
    let synset_dog = 0x8a2a1072b;
    let quin = sharder.map_wordnet_synset(synset_dog, 0x1000);

    assert_eq!(quin.subject, synset_dog);
    assert_eq!(
        quin.extract_modality_flag(),
        crate::MODALITY_FLAG_DENSE_PHYSICS,
        "Modality Flag should be Dense Physics"
    );
    assert_eq!(quin.extract_byte_offset(), 0x1000);
}
