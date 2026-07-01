//! Task #12 / STELLAR §A — transcode the *real* downloaded model through the FFN-ternary policy.
//!
//! Local-only: skips cleanly unless `docs/models/SmolLM2-360M-Instruct/model.safetensors` is present
//! (it is gitignored). Run with `cargo test -p qualia-core-db --test transcode_real_model -- --nocapture`.

use std::path::Path;

/// Task #12 (§A): compile the real Q8 SmolLM2-360M GGUF into a **runnable** ternary-FFN P64
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

    let out = qualia_core_db::p64_weight::compile_gguf_to_p64_ternary_ffn(&mmap, 14)
        .expect("ternary-FFN GGUF compile");
    let idx = qualia_core_db::p64_weight::P64TensorIndex::from_p64(&out)
        .expect("round-trip from_p64");

    // The container is COMPLETE (runnable): hyperparams + tokenizer carried through.
    assert!(idx.hparams.n_layer > 0, "n_layers must survive");
    assert!(idx.hparams.n_embd > 0, "n_embd must survive");
    assert!(idx.hparams.vocab_size > 0, "vocab must survive");
    assert!(
        !idx.tokenizer_bytes(&out).is_empty(),
        "tokenizer section must survive (runnable)"
    );

    let tern = qualia_core_db::ternary::GGML_TYPE_TERNARY_158;
    let n_tern = idx
        .entries
        .iter()
        .filter(|e| e.dtype as u32 == tern)
        .count();
    let n_verb = idx.entries.len() - n_tern;
    assert_eq!(
        n_tern,
        3 * idx.hparams.n_layer as usize,
        "all 3 FFN projections per layer ternarised"
    );
    assert!(out.len() < src_bytes, "ternary FFN must shrink the model");

    eprintln!("── SmolLM2-360M Q8 GGUF → runnable ternary-FFN P64 ──");
    eprintln!("source GGUF (Q8) : {:.1} MB", src_bytes as f64 / 1e6);
    eprintln!(
        "output P64        : {:.1} MB  ({:.2}x smaller)",
        out.len() as f64 / 1e6,
        src_bytes as f64 / out.len() as f64
    );
    eprintln!(
        "hyperparams       : {} layers, n_embd {}, n_head {}, vocab {}",
        idx.hparams.n_layer, idx.hparams.n_embd, idx.hparams.n_head, idx.hparams.vocab_size
    );
    eprintln!(
        "tensors           : {} ternary (FFN), {} verbatim; tokenizer {} bytes",
        n_tern,
        n_verb,
        idx.tokenizer_bytes(&out).len()
    );

    // a sample FFN tensor dequantizes.
    if let Some(e) = idx.entries.iter().find(|e| e.dtype as u32 == tern) {
        let count = (e.dimensions[0] as usize).max(1) * (e.dimensions[1] as usize).max(1);
        let mut deq = vec![0.0f32; count];
        qualia_core_db::ternary::dequantize_blob(idx.blob(&out, e), &mut deq);
        let nz = deq.iter().filter(|v| **v != 0.0).count();
        eprintln!("sample FFN tensor : {} weights, {} non-zero", count, nz);
    }
}

/// A1b inc 3 ON-DEVICE GATE: native P64 boot + the FFN dispatch branch on the REAL SmolLM2 FFN
/// weights. Compiles the q8 GGUF → ternary P64, mmaps it, boots it natively
/// (`adopt_resident_p64_mmap` → builds the resident 2-bit set), then for FFN gate/up/down on the
/// first + last layer asserts the GPU 2-bit path (toggle ON) == the CPU base-3 oracle (toggle OFF).
/// This isolates ternary-FFN correctness on real weights from the full decode loop. Skips if the q8
/// GGUF is absent. Run: `cargo test -p qualia-core-db --test transcode_real_model
/// ternary_ffn_native_boot_and_dispatch_matches_cpu -- --nocapture`.
#[cfg(not(target_arch = "wasm32"))]
#[test]
fn ternary_ffn_native_boot_and_dispatch_matches_cpu() {
    use qualia_core_db::gguf_bridge::QTensorEngine;
    use qualia_core_db::llm_bench::set_ternary_ffn;
    use qualia_core_db::p64_weight::P64TensorIndex;

    let candidates = [
        "../../docs/models/smollm2-360m-instruct-q8_0.gguf",
        "docs/models/smollm2-360m-instruct-q8_0.gguf",
    ];
    let Some(path) = candidates.iter().map(Path::new).find(|p| p.exists()) else {
        eprintln!("[a1b dispatch] q8 GGUF absent — skipping");
        return;
    };
    let src = std::fs::File::open(path).expect("open gguf");
    let src_mmap = unsafe { memmap2::Mmap::map(&src) }.expect("mmap gguf");
    let p64 = qualia_core_db::p64_weight::compile_gguf_to_p64_ternary_ffn(&src_mmap, 14)
        .expect("ternary-FFN compile");
    let tmp = std::env::temp_dir().join("a1b_smollm2_ternary_ffn.p64");
    std::fs::write(&tmp, &p64).expect("write temp P64");
    let f = std::fs::File::open(&tmp).expect("open temp P64");
    let mmap = std::sync::Arc::new(unsafe { memmap2::Mmap::map(&f) }.expect("mmap P64"));

    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    let mut engine = rt
        .block_on(async { QTensorEngine::try_new().await })
        .expect("native engine");
    let report = engine
        .adopt_resident_p64_mmap(mmap.clone())
        .expect("native P64 boot");
    eprintln!(
        "[a1b] booted ternary P64 natively: {} layers, {:.1} MB mapped, {} resident ternary FFN tensors",
        report.n_layer,
        report.mapped_bytes as f64 / 1e6,
        engine.ternary_ffn_resident_len()
    );
    // the resident GPU path must actually be populated (3 FFN projections × n_layer).
    assert_eq!(
        engine.ternary_ffn_resident_len(),
        3 * report.n_layer as usize,
        "all FFN projections must be resident (else the GPU path silently fell back to CPU)"
    );

    let q = P64TensorIndex::from_p64(&mmap[..]).expect("from_p64");
    let index = q.to_gguf_index();

    let mut max_diff = 0f32;
    let mut checked = 0usize;
    for layer in [0u32, report.n_layer.saturating_sub(1)] {
        let t = index.get_layer_tensors(layer);
        for info in [t.ffn_gate, t.ffn_up, t.ffn_down].into_iter().flatten() {
            let n_in = info.dims[0] as usize;
            let n_out = info.dims[1] as usize;
            let act: Vec<f32> = (0..n_in).map(|j| ((j % 23) as f32) * 0.07 - 0.8).collect();
            let mut out_on = vec![0f32; n_out];
            let mut out_off = vec![0f32; n_out];
            set_ternary_ffn(true);
            assert!(
                engine.dispatch_gemm_into(&index, &info, &act, &mut out_on, n_in, n_out),
                "GPU ternary dispatch (layer {layer})"
            );
            set_ternary_ffn(false);
            assert!(
                engine.dispatch_gemm_into(&index, &info, &act, &mut out_off, n_in, n_out),
                "CPU ternary dispatch (layer {layer})"
            );
            for i in 0..n_out {
                max_diff = max_diff.max((out_on[i] - out_off[i]).abs());
            }
            checked += 1;
        }
    }
    set_ternary_ffn(false);
    eprintln!(
        "[a1b] FFN dispatch parity: {checked} real tensors, max |GPU 2-bit − CPU base-3| = {max_diff:.3e}"
    );
    // GPU 2-bit and CPU base-3 compute scale·Σ trit·act in the same order (multiply by ±1.0 is
    // exact), so they agree to float noise — a generous bound still catches any real divergence.
    assert!(
        max_diff < 1e-2,
        "GPU 2-bit ternary FFN must match the CPU base-3 oracle on real weights (max_diff {max_diff})"
    );
    let _ = std::fs::remove_file(&tmp);
    eprintln!("[a1b] native P64 boot + ternary FFN dispatch verified on the real model.");
}

#[test]
fn transcode_smollm2_360m_ffn_ternary() {
    // resolve the model path relative to the workspace (tests run with CWD = crate dir).
    let candidates = [
        "../../docs/models/SmolLM2-360M-Instruct/model.safetensors",
        "docs/models/SmolLM2-360M-Instruct/model.safetensors",
    ];
    let Some(path) = candidates.iter().map(Path::new).find(|p| p.exists()) else {
        eprintln!(
            "transcode_real_model: SmolLM2-360M not present — skipping (download to docs/models/)"
        );
        return;
    };

    let file = std::fs::File::open(path).expect("open model");
    let mmap = unsafe { memmap2::Mmap::map(&file) }.expect("mmap model");
    let src_bytes = mmap.len();

    let mut out: Vec<u8> = Vec::new();
    let report =
        qualia_core_db::p64_weight::transcode_safetensor_to_p64_ffn_ternary(&mmap, 14, &mut out)
            .expect("ternary-policy transcode");

    // Parse the emitted container back (validates CRC + manifest).
    let idx = qualia_core_db::p64_weight::P64TensorIndex::from_p64(&out)
        .expect("round-trip from_p64");

    // Classify entries: ternary (FFN) vs verbatim, and tally the tensor-data bytes.
    let tern_code = qualia_core_db::ternary::GGML_TYPE_TERNARY_158;
    let (mut n_tern, mut n_verb, mut tern_bytes, mut verb_bytes) = (0u32, 0u32, 0u64, 0u64);
    for e in &idx.entries {
        if e.dtype as u32 == tern_code {
            n_tern += 1;
            tern_bytes += e.blob_size as u64;
        } else {
            n_verb += 1;
            verb_bytes += e.blob_size as u64;
        }
    }

    eprintln!("── SmolLM2-360M FFN-ternary transcode ─────────────────────────────");
    eprintln!("source safetensor : {:.1} MB", src_bytes as f64 / 1e6);
    eprintln!(
        "output P64        : {:.1} MB  ({:.2}x smaller)",
        report.bytes_written as f64 / 1e6,
        src_bytes as f64 / report.bytes_written as f64
    );
    eprintln!(
        "tensors           : {} total — {} ternary (FFN), {} verbatim (attn/norm/embed)",
        report.n_tensors, n_tern, n_verb
    );
    eprintln!(
        "ternary tensor-data: {:.1} MB   verbatim tensor-data: {:.1} MB",
        tern_bytes as f64 / 1e6,
        verb_bytes as f64 / 1e6
    );
    eprintln!(
        "peak working mem  : {:.1} MB (one tensor in flight)",
        report.peak_working_bytes as f64 / 1e6
    );

    // Sanity: this model HAS FFN layers, so some tensors must have been ternarised, and the
    // container must round-trip with the right tensor count.
    assert!(n_tern > 0, "expected FFN tensors to be ternarised");
    assert_eq!(idx.entries.len(), report.n_tensors);
    assert!(
        report.bytes_written < src_bytes,
        "ternary FFN should shrink the model"
    );

    // Dequantize one ternary (FFN) tensor to confirm it decodes.
    if let Some(e) = idx.entries.iter().find(|e| e.dtype as u32 == tern_code) {
        let count = (e.dimensions[0] as usize).max(1) * (e.dimensions[1] as usize).max(1);
        let mut deq = vec![0.0f32; count];
        qualia_core_db::ternary::dequantize_blob(idx.blob(&out, e), &mut deq);
        let nonzero = deq.iter().filter(|v| **v != 0.0).count();
        eprintln!(
            "sample FFN tensor : {} weights, {} non-zero after dequant",
            count, nonzero
        );
    }
}
