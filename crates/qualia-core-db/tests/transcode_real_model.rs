//! Task #12 / STELLAR §A — transcode the *real* downloaded model through the FFN-ternary policy.
//!
//! Local-only: skips cleanly unless `docs/models/SmolLM2-360M-Instruct/model.safetensors` is present
//! (it is gitignored). Run with `cargo test -p qualia-core-db --test transcode_real_model -- --nocapture`.

use std::path::Path;

/// Task #12 (§A): compile the real Q8 SmolLM2-360M GGUF into a **runnable** ternary-FFN Q42W
/// (hyperparams + tokenizer preserved; FFN tensors ternarised) and verify it round-trips + loads.
#[test]
fn compile_smollm2_q8_gguf_to_runnable_ternary_ffn() {
    let candidates = [
        "../../docs/models/smollm2-360m-instruct-q8_0.gguf",
        "docs/models/smollm2-360m-instruct-q8_0.gguf",
    ];
    let Some(path) = candidates.iter().map(Path::new).find(|p| p.exists()) else {
        eprintln!("compile_real_gguf: q8 GGUF not present — skipping");
        return;
    };
    let file = std::fs::File::open(path).expect("open gguf");
    let mmap = unsafe { memmap2::Mmap::map(&file) }.expect("mmap gguf");
    let src_bytes = mmap.len();

    let out = qualia_core_db::q42_weight::compile_gguf_to_q42_ternary_ffn(&mmap, 14)
        .expect("ternary-FFN GGUF compile");
    let idx = qualia_core_db::q42_weight::Q42TensorIndex::from_q42(&out).expect("round-trip from_q42");

    // The container is COMPLETE (runnable): hyperparams + tokenizer carried through.
    assert!(idx.header.n_layers > 0, "n_layers must survive");
    assert!(idx.header.n_embd > 0, "n_embd must survive");
    assert!(idx.header.vocab_size > 0, "vocab must survive");
    assert!(idx.header.tokenizer_len > 0, "tokenizer section must survive (runnable)");

    let tern = qualia_core_db::ternary::GGML_TYPE_TERNARY_158;
    let n_tern = idx.entries.iter().filter(|e| e.ggml_type == tern).count();
    let n_verb = idx.entries.len() - n_tern;
    assert_eq!(n_tern, 3 * idx.header.n_layers as usize, "all 3 FFN projections per layer ternarised");
    assert!(out.len() < src_bytes, "ternary FFN must shrink the model");

    eprintln!("── SmolLM2-360M Q8 GGUF → runnable ternary-FFN .q42 ──");
    eprintln!("source GGUF (Q8) : {:.1} MB", src_bytes as f64 / 1e6);
    eprintln!("output .q42       : {:.1} MB  ({:.2}x smaller)", out.len() as f64 / 1e6, src_bytes as f64 / out.len() as f64);
    eprintln!("hyperparams       : {} layers, n_embd {}, n_head {}, vocab {}",
        idx.header.n_layers, idx.header.n_embd, idx.header.n_head, idx.header.vocab_size);
    eprintln!("tensors           : {} ternary (FFN), {} verbatim; tokenizer {} bytes",
        n_tern, n_verb, idx.header.tokenizer_len);

    // a sample FFN tensor dequantizes.
    if let Some(e) = idx.entries.iter().find(|e| e.ggml_type == tern) {
        let count = (e.dim0 as usize).max(1) * (e.dim1 as usize).max(1);
        let mut deq = vec![0.0f32; count];
        qualia_core_db::ternary::dequantize_blob(idx.blob(&out, e), &mut deq);
        let nz = deq.iter().filter(|v| **v != 0.0).count();
        eprintln!("sample FFN tensor : {} weights, {} non-zero", count, nz);
    }
}

#[test]
fn transcode_smollm2_360m_ffn_ternary() {
    // resolve the model path relative to the workspace (tests run with CWD = crate dir).
    let candidates = [
        "../../docs/models/SmolLM2-360M-Instruct/model.safetensors",
        "docs/models/SmolLM2-360M-Instruct/model.safetensors",
    ];
    let Some(path) = candidates.iter().map(Path::new).find(|p| p.exists()) else {
        eprintln!("transcode_real_model: SmolLM2-360M not present — skipping (download to docs/models/)");
        return;
    };

    let file = std::fs::File::open(path).expect("open model");
    let mmap = unsafe { memmap2::Mmap::map(&file) }.expect("mmap model");
    let src_bytes = mmap.len();

    let mut out: Vec<u8> = Vec::new();
    let report = qualia_core_db::q42_weight::transcode_safetensor_to_q42_ffn_ternary(&mmap, 14, &mut out)
        .expect("ternary-policy transcode");

    // Parse the emitted container back (validates CRC + manifest).
    let idx = qualia_core_db::q42_weight::Q42TensorIndex::from_q42(&out).expect("round-trip from_q42");

    // Classify entries: ternary (FFN) vs verbatim, and tally the tensor-data bytes.
    let tern_code = qualia_core_db::ternary::GGML_TYPE_TERNARY_158;
    let (mut n_tern, mut n_verb, mut tern_bytes, mut verb_bytes) = (0u32, 0u32, 0u64, 0u64);
    for e in &idx.entries {
        if e.ggml_type == tern_code {
            n_tern += 1;
            tern_bytes += e.byte_len;
        } else {
            n_verb += 1;
            verb_bytes += e.byte_len;
        }
    }

    eprintln!("── SmolLM2-360M FFN-ternary transcode ─────────────────────────────");
    eprintln!("source safetensor : {:.1} MB", src_bytes as f64 / 1e6);
    eprintln!("output .q42 (Q42W): {:.1} MB  ({:.2}x smaller)", report.bytes_written as f64 / 1e6,
        src_bytes as f64 / report.bytes_written as f64);
    eprintln!("tensors           : {} total — {} ternary (FFN), {} verbatim (attn/norm/embed)",
        report.n_tensors, n_tern, n_verb);
    eprintln!("ternary tensor-data: {:.1} MB   verbatim tensor-data: {:.1} MB",
        tern_bytes as f64 / 1e6, verb_bytes as f64 / 1e6);
    eprintln!("peak working mem  : {:.1} MB (one tensor in flight)", report.peak_working_bytes as f64 / 1e6);

    // Sanity: this model HAS FFN layers, so some tensors must have been ternarised, and the
    // container must round-trip with the right tensor count.
    assert!(n_tern > 0, "expected FFN tensors to be ternarised");
    assert_eq!(idx.entries.len(), report.n_tensors);
    assert!(report.bytes_written < src_bytes, "ternary FFN should shrink the model");

    // Dequantize one ternary (FFN) tensor to confirm it decodes.
    if let Some(e) = idx.entries.iter().find(|e| e.ggml_type == tern_code) {
        let count = (e.dim0 as usize).max(1) * (e.dim1 as usize).max(1);
        let mut deq = vec![0.0f32; count];
        qualia_core_db::ternary::dequantize_blob(idx.blob(&out, e), &mut deq);
        let nonzero = deq.iter().filter(|v| **v != 0.0).count();
        eprintln!("sample FFN tensor : {} weights, {} non-zero after dequant", count, nonzero);
    }
}
