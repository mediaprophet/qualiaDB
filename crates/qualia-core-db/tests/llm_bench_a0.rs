//! A0 (STELLAR §A) — native LLM benchmark baseline.
//!
//! Produces the missing trustworthy native SmolLM2-360M number (cold/warm TTFT,
//! prefill/decode tok/s) through the **real** inference path via the shared
//! `llm_bench` harness. Local-only: skips cleanly when the GGUF models are absent
//! (they are gitignored under `docs/models/`).
//!
//! Run:
//!   cargo test -p qualia-core-db --release --test llm_bench_a0 -- --nocapture
//!
//! Integration tests compile the library WITHOUT `cfg(test)`, so this runs at
//! FULL model depth (all layers, full vocab sweep) — a unit test would only run
//! the 2-layer `TEST_TRANSFORMER_LAYER_CAP` and report a meaningless number.

#![cfg(not(target_arch = "wasm32"))]

use qualia_core_db::llm_bench::{self, BenchConfig};
use std::path::{Path, PathBuf};

/// First existing path among the standard relative roots, else None.
fn find_model(name: &str) -> Option<PathBuf> {
    let candidates = [
        format!("../../docs/models/{name}"),
        format!("docs/models/{name}"),
    ];
    candidates
        .iter()
        .map(PathBuf::from)
        .find(|p| Path::new(p).exists())
}

fn results_dir() -> PathBuf {
    for root in ["../../benchmarks/results", "benchmarks/results"] {
        let p = PathBuf::from(root);
        if p.parent().map(|d| d.exists()).unwrap_or(false) {
            return p;
        }
    }
    PathBuf::from("benchmarks/results")
}

#[test]
fn a0_native_llm_baseline() {
    let prompt = "The capital of France is";
    let candidates = [
        ("SmolLM2-360M Q8", "smollm2-360m-instruct-q8_0.gguf", "Q8_0"),
        ("SmolLM2-360M Q4_K_M", "SmolLM2-360M-Instruct-Q4_K_M.gguf", "Q4_K_M"),
    ];

    let cfgs: Vec<BenchConfig> = candidates
        .iter()
        .filter_map(|(label, file, quant)| {
            find_model(file).map(|p| {
                let mut c = BenchConfig::new(*label, p.to_string_lossy(), *quant, prompt);
                c.decode_tokens = 64; // fixed count → stable, comparable tok/s
                c.warm_repeats = 3;
                c
            })
        })
        .collect();

    if cfgs.is_empty() {
        eprintln!("[a0] no SmolLM2 GGUF models under docs/models/ — skipping baseline");
        return;
    }

    let results = llm_bench::run_suite_blocking(&cfgs);
    assert!(
        !results.is_empty(),
        "[a0] harness ran but produced no results — every case failed to load (see warnings)"
    );

    // Console table + per-row detail.
    println!("\n=== A0 native LLM baseline (SmolLM2-360M, A2000) ===");
    println!("{}", llm_bench::results_to_table(&results));
    for r in &results {
        println!(
            "[a0] {:<22} layers={} prompt_tok={} out_tok={} \
cold_ttft={:.0}ms warm_ttft={:.0}ms load={:.0}ms prefill={:.1}t/s decode={:.2}t/s directml={}",
            r.label,
            r.model.n_layer,
            r.prompt_tokens,
            r.output_tokens,
            r.cold_ttft_ms,
            r.warm_ttft_ms,
            r.load_ms,
            r.prefill_tok_s,
            r.decode_tok_s,
            r.model.directml_enabled,
        );
        if r.decode_tok_s <= 0.0 || r.output_tokens == 0 {
            eprintln!(
                "[a0][WARN] {} produced no decoded tokens — native path did not generate \
(this is a real finding, not a perf number).",
                r.label
            );
        }
    }

    // Persist artifacts for external review.
    let dir = results_dir();
    let _ = std::fs::create_dir_all(&dir);
    let json_path = dir.join("llm_a0_baseline.json");
    let csv_path = dir.join("llm_a0_baseline.csv");
    if std::fs::write(&json_path, llm_bench::results_to_json(&results)).is_ok() {
        println!("[a0] wrote {}", json_path.display());
    }
    if std::fs::write(&csv_path, llm_bench::results_to_csv(&results)).is_ok() {
        println!("[a0] wrote {}", csv_path.display());
    }
}

/// A1a correctness: GPU top-k (k=1) must decode byte-identical text to the argmax path. Skips if the
/// model is absent. Run: `cargo test -p qualia-core-db --release --test llm_bench_a0
/// a1a_gpu_topk_matches_argmax_text -- --nocapture`.
#[test]
fn a1a_gpu_topk_matches_argmax_text() {
    // Q4_K_M: attention weights are Q4_K, which the native GPU path supports (Q8_0 is not — separate gap).
    let Some(path) = find_model("SmolLM2-360M-Instruct-Q4_K_M.gguf")
        .or_else(|| find_model("smollm2-360m-instruct-q8_0.gguf"))
    else {
        eprintln!("[a1a] model absent — skipping token-identity check");
        return;
    };
    let model = path.to_string_lossy().to_string();
    // Unambiguous continuation prompt — a working forward must produce real words, not EOS/garbage.
    let prompt = "Once upon a time, there was a";
    let (off, on) = llm_bench::compare_topk_decode_blocking(&model, prompt, 24)
        .expect("compare_topk_decode");
    println!("[a1a] argmax: {off:?}");
    println!("[a1a] topk  : {on:?}");
    assert_eq!(
        off, on,
        "GPU top-k (k=1) must emit identical text to the argmax path"
    );
    // #48 regression guard: native decode must produce coherent text, not EOS/garbage spam.
    assert!(
        !off.trim_start().starts_with("<|endoftext|>") && off.contains(' ') && off.len() > 8,
        "native decode must produce coherent text (regression of #48), got: {off:?}"
    );
    println!("[a1a] token-identity verified + coherent generation: top-k == argmax");
}

/// A1c GEMM Q8 enablement: Q8_0 weights now route to the GPU GEMM shader (`fused_transformer.wgsl`)
/// instead of the CPU `stack_gemm_quant` fallback (the FFN bottleneck for Q8_0 models). Native decode
/// must STILL produce coherent text — i.e. the shader's GPU Q8_0 dequant matches the CPU reference.
/// Forces the Q8_0 model specifically (the a1a test above uses Q4_K_M and would not exercise Q8).
/// Skips if absent. Run: `cargo test -p qualia-core-db --release --test llm_bench_a0
/// a1c_q8_gemm_decode_coherent -- --nocapture`.
#[test]
fn a1c_q8_gemm_decode_coherent() {
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[a1c] Q8_0 model absent — skipping GPU Q8 GEMM coherence check");
        return;
    };
    let model = path.to_string_lossy().to_string();
    let prompt = "Once upon a time, there was a";
    let (off, _on) =
        llm_bench::compare_topk_decode_blocking(&model, prompt, 24).expect("compare_topk_decode (q8)");
    println!("[a1c] q8 argmax: {off:?}");
    // If the GPU Q8_0 dequant diverged from the CPU path, the FFN/output projection would corrupt the
    // residual stream → EOS/garbage (regression of #48). Coherent text == GPU Q8 GEMM matches CPU.
    assert!(
        !off.trim_start().starts_with("<|endoftext|>") && off.contains(' ') && off.len() > 8,
        "GPU Q8_0 GEMM must yield coherent decode (else dequant mismatch vs CPU), got: {off:?}"
    );
    println!("[a1c] GPU Q8_0 GEMM verified coherent.");
}

/// ChatML tokenizer probe: does the GGUF tokenizer map SmolLM2's chat special tokens
/// `<|im_start|>`/`<|im_end|>` to single vocabulary IDs (atomic), or shatter them into literal
/// byte/BPE pieces? If shattered, the instruct chat template can't be expressed and the instruct
/// behaviour LM Studio shows is unreachable from our path. Diagnostic — prints the IDs + a verdict
/// (asserts only that tokenisation ran). Forces the Q8 model. Skips if absent. Run:
/// `cargo test -p qualia-core-db --release --test llm_bench_a0 chatml_tokenizer_check -- --nocapture`.
#[test]
fn chatml_tokenizer_check() {
    use qualia_core_db::gguf_sharder::GgufTokenizer;
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[chatml] model absent — skipping tokenizer check");
        return;
    };
    let bytes = std::fs::read(&path).expect("read gguf");
    let tok = GgufTokenizer::from_gguf(&bytes);
    println!(
        "[chatml] bos={} eos={} add_bos={}",
        tok.bos_token_id, tok.eos_token_id, tok.add_bos_token
    );
    for tag in ["<|im_start|>", "<|im_end|>", "<|endoftext|>"] {
        let ids = tok.encode(tag);
        println!("[chatml] {tag:?} -> {ids:?} ({} token(s))", ids.len());
    }
    let chatml = "<|im_start|>user\nOnce upon a time, there was a<|im_end|>\n<|im_start|>assistant\n";
    let ids = tok.encode(chatml);
    println!("[chatml] full ChatML prompt -> {} tokens: {ids:?}", ids.len());
    assert!(!ids.is_empty(), "tokenizer produced no tokens");

    let start = tok.encode("<|im_start|>");
    let end = tok.encode("<|im_end|>");
    if start.len() == 1 && end.len() == 1 {
        println!(
            "[chatml] PASS — special tokens are ATOMIC (im_start={}, im_end={}) → chat template is expressible",
            start[0], end[0]
        );
    } else {
        println!(
            "[chatml] FAIL — special tokens SHATTERED (im_start={} pieces, im_end={} pieces) → chat template not expressible as-is",
            start.len(),
            end.len()
        );
    }
}

/// Tokenizer parity dump (bug-hunt stage 1): print our token IDs + counts for several prompts so they
/// can be compared against llama.cpp's `prompt_eval_count` / IDs. A first-token generation divergence
/// from the reference most often originates here. Forces Q8. Skips if absent.
#[test]
fn tokenizer_parity_dump() {
    use qualia_core_db::gguf_sharder::GgufTokenizer;
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[tokparity] model absent — skipping");
        return;
    };
    let bytes = std::fs::read(&path).expect("read gguf");
    let tok = GgufTokenizer::from_gguf(&bytes);
    for p in [
        "Once upon a time, there was a",
        "The capital of France is",
        "Hello world",
    ] {
        let raw = tok.encode(p);
        let withp = tok.encode_prompt(p);
        println!("[tokparity] {p:?}");
        println!("[tokparity]   encode()        = {} toks {raw:?}", raw.len());
        println!("[tokparity]   encode_prompt() = {} toks {withp:?}", withp.len());
    }
}

/// Forward-divergence probe (bug-hunt): run OUR engine greedy on prompts whose llama.cpp greedy
/// answers are known, to isolate a forward-pass defect from tokenization (counts already match).
/// Reference (llama.cpp, same weights): "Once upon a time, there was a"→" young…", "The capital of
/// France is"→" Paris", "Hello world"→"!". Forces Q8. Skips if absent.
#[test]
fn forward_divergence_probe() {
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[fwddiv] model absent — skipping");
        return;
    };
    let model = path.to_string_lossy().to_string();
    for p in [
        "Once upon a time, there was a",
        "The capital of France is",
        "Hello world",
    ] {
        let (out, _) = llm_bench::compare_topk_decode_blocking(&model, p, 8).expect("decode");
        println!("[fwddiv] {p:?} -> {out:?}");
    }
}

/// Teacher-forcing probe (bug-hunt, decisive): feed our engine llama.cpp's OWN coherent prefix and
/// see if we continue coherently (→ forward correct; the earlier divergence was a near-tie first-token
/// flip) or degrade (→ real forward defect that compounds). llama.cpp ref continuation:
/// ". She spent years saving up her savings, working multiple jobs, and traveling to". Forces Q8.
#[test]
fn teacher_forcing_probe() {
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[teacher] model absent — skipping");
        return;
    };
    let model = path.to_string_lossy().to_string();
    let prefix = "Once upon a time, there was a young woman named Sarah who had always dreamed of traveling the world";
    let (out, _) = llm_bench::compare_topk_decode_blocking(&model, prefix, 16).expect("decode");
    println!("[teacher] OURS  -> {out:?}");
    println!("[teacher] LLAMA -> \". She spent years saving up her savings, working multiple jobs, and traveling to\"");
}

/// Route-mask state probe (bug-hunt): is the neuro-symbolic SPARSE attention route mask active during
/// a plain (no-graph) decode? If `active_bits`/`seq` stay 0, attention was DENSE → the sparse mask is
/// NOT the degeneration cause and the hunt continues (KV/prefill numerics). If >0, something published
/// a sparse mask into base LM decode → that's the bug. Forces Q8. Skips if absent.
#[test]
fn route_mask_state_after_decode() {
    use qualia_core_db::compute_universe::{attention_mask_seq, attention_route_mask};
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[mask] model absent — skipping");
        return;
    };
    let model = path.to_string_lossy().to_string();
    println!(
        "[mask] before decode: route.active_bits={} mask_seq={}",
        attention_route_mask().active_bits,
        attention_mask_seq()
    );
    let (out, _) = llm_bench::compare_topk_decode_blocking(&model, "Once upon a time, there was a", 16)
        .expect("decode");
    println!(
        "[mask] after  decode: route.active_bits={} mask_seq={}",
        attention_route_mask().active_bits,
        attention_mask_seq()
    );
    println!("[mask] out={out:?}");
}

/// ChatML decode demo: feed the engine the SAME content under (a) raw completion vs (b) the ChatML
/// instruct template, both pure greedy (no repetition penalty yet), and print both. Isolates the
/// TEMPLATE's effect on quality from the still-missing sampler/penalty. Forces Q8. Skips if absent.
/// Run: `cargo test -p qualia-core-db --release --test llm_bench_a0 chatml_decode_demo -- --nocapture`.
#[test]
fn chatml_decode_demo() {
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf") else {
        eprintln!("[chatml-decode] model absent — skipping");
        return;
    };
    let model = path.to_string_lossy().to_string();
    let raw = "Once upon a time, there was a";
    let chatml = "<|im_start|>user\nOnce upon a time, there was a<|im_end|>\n<|im_start|>assistant\n";

    let (raw_out, _) = llm_bench::compare_topk_decode_blocking(&model, raw, 48).expect("raw decode");
    println!("[chatml-decode] RAW greedy:\n  {raw_out:?}");

    let (tmpl_out, _) =
        llm_bench::compare_topk_decode_blocking(&model, chatml, 48).expect("templated decode");
    println!("[chatml-decode] CHATML greedy:\n  {tmpl_out:?}");
}

/// Decode profiler — localize the ~2 s/token native decode: forward (32 layers) vs output
/// projection, and split **synchronization** (submit→poll(Wait) round-trips) from **kernel compute**
/// via an empty-round-trip baseline on the same device. This decides whether the lever is dispatch
/// fusion (sync-bound) or shader optimization (compute-bound) — no guessing. Skips if model absent.
/// Run: `cargo test -p qualia-core-db --release --test llm_bench_a0 a0_decode_profile -- --nocapture`.
#[test]
fn a0_decode_profile() {
    let Some(path) = find_model("smollm2-360m-instruct-q8_0.gguf")
        .or_else(|| find_model("SmolLM2-360M-Instruct-Q4_K_M.gguf"))
    else {
        eprintln!("[prof] model absent — skipping decode profile");
        return;
    };
    std::env::set_var("QUALIA_LLM_PROFILE_DECODE", "1");

    let mut cfg = BenchConfig::new(
        "decode-profile",
        path.to_string_lossy(),
        "auto",
        "Once upon a time, there was a",
    );
    cfg.decode_tokens = 16; // bounded; the per-token averages are what matter
    cfg.warm_repeats = 1; // the single warm run leaves the accumulators populated

    let results = llm_bench::run_suite_blocking(&[cfg]);
    assert!(
        !results.is_empty(),
        "[prof] harness produced no result (model failed to load)"
    );

    let snap = llm_bench::phase_snapshot();
    let waits = llm_bench::gpu_wait_count();
    let (ert_total, ert_n) = llm_bench::empty_rt();
    let toks = snap.decode_tokens.max(1);

    let per_ms = |ns: u64| (ns as f64 / toks as f64) / 1e6; // ms/token
    let total_ms = per_ms(snap.decode_ns).max(0.001);
    let fwd_ms = per_ms(snap.decode_forward_ns);
    let out_ms = per_ms(snap.decode_output_ns);
    let other_ms = (total_ms - fwd_ms - out_ms).max(0.0);
    let waits_per_tok = waits as f64 / toks as f64;
    let ert_per_ms = if ert_n > 0 {
        (ert_total as f64 / ert_n as f64) / 1e6
    } else {
        0.0
    };
    let sync_ms = waits_per_tok * ert_per_ms; // est. fixed fence overhead/token
    let sync_pct = 100.0 * sync_ms / total_ms;

    println!("\n=== A0 decode profile (SmolLM2-360M, A2000) ===");
    println!(
        "[prof] tokens={toks}  total={total_ms:.1} ms/tok  ({:.2} tok/s)",
        1000.0 / total_ms
    );
    println!(
        "[prof]   forward (32 layers) = {fwd_ms:.1} ms/tok ({:.0}%)",
        100.0 * fwd_ms / total_ms
    );
    let (attn_ns, ffn_ns) = llm_bench::decode_attn_ffn();
    let attn_ms = per_ms(attn_ns);
    let ffn_ms = per_ms(ffn_ns);
    let fwd_d = fwd_ms.max(0.001);
    println!(
        "[prof]     ├─ attention (QKV+SDPA+O, fused_attention shader) = {attn_ms:.1} ms/tok ({:.0}% of fwd)",
        100.0 * attn_ms / fwd_d
    );
    println!(
        "[prof]     └─ FFN (SwiGLU, GEMM shader)                      = {ffn_ms:.1} ms/tok ({:.0}% of fwd)",
        100.0 * ffn_ms / fwd_d
    );
    println!(
        "[prof]   output projection   = {out_ms:.1} ms/tok ({:.0}%)",
        100.0 * out_ms / total_ms
    );
    println!("[prof]   host / other        = {other_ms:.1} ms/tok");
    println!("[prof] GPU submit→wait round-trips = {waits_per_tok:.0}/tok");
    println!("[prof] empty round-trip baseline   = {ert_per_ms:.3} ms each (n={ert_n})");
    println!(
        "[prof] est. fence overhead = {sync_ms:.1} ms/tok ({sync_pct:.0}% of token) → {}",
        if sync_pct >= 60.0 {
            "SYNC-BOUND — dispatch fusion is the lever"
        } else if sync_pct <= 25.0 {
            "COMPUTE-BOUND — the kernels are slow; optimize shaders"
        } else {
            "MIXED — both sync and compute matter"
        }
    );

    std::env::remove_var("QUALIA_LLM_PROFILE_DECODE");
}
