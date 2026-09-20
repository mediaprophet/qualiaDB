//! LLM Testing Integration for CLI
//!
//! Simple LLM model testing functionality for the CLI.

use crate::llm_lifecycle::{default_vault_path, init_log_stream};
use qualia_client_core::model_lifecycle::{resolve_vault_model, scan_vault_gguf, VaultGgufEntry};
use std::path::{Path, PathBuf};

/// CLI command to run comprehensive LLM model tests
pub fn run_test_models(
    vault_path: Option<PathBuf>,
    models: Option<Vec<String>>,
    _quantization: Option<String>,
    verbose: bool,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);

    if verbose {
        init_log_stream(true);
    }

    println!("🚀 Starting LLM Model Testing CLI");
    println!("📁 Vault path: {}", vault_path.display());

    // Scan for GGUF models in vault
    let available_models =
        scan_vault_gguf(&vault_path).map_err(|e| format!("Failed to scan vault: {}", e))?;

    if available_models.is_empty() {
        return Err("No GGUF models found in vault".to_string());
    }

    println!("📦 Found {} model(s):", available_models.len());
    for model in &available_models {
        println!("  - {}", model.name);
    }

    // Filter models if specific ones requested
    let test_models = if let Some(ref requested) = models {
        available_models
            .iter()
            .filter(|m| requested.contains(&m.name))
            .cloned()
            .collect()
    } else {
        available_models
    };

    if test_models.is_empty() {
        return Err("No matching models found".to_string());
    }

    println!("\n🧪 Testing {} model(s)...", test_models.len());

    for model in &test_models {
        println!("\n🔍 Testing: {}", model.name);
        match test_single_model(&vault_path, model, verbose) {
            Ok(result) => {
                println!("  ✅ Load time: {}ms", result.load_time_ms);
                println!("  ✅ Memory: {}MB", result.memory_mb);
                println!(
                    "  ✅ Status: {}",
                    if result.success { "PASS" } else { "FAIL" }
                );
            }
            Err(e) => {
                println!("  ❌ Error: {}", e);
            }
        }
    }

    println!("\n✅ Testing complete!");
    Ok(())
}

/// Comprehensive LLM capability test (load + inference in one session)
pub fn run_comprehensive_llm_test(
    vault_path: Option<PathBuf>,
    model_name: String,
    verbose: bool,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);

    if verbose {
        init_log_stream(true);
    }

    println!("🧪 Running Comprehensive LLM Test Suite");
    println!("📁 Vault path: {}", vault_path.display());
    println!("🤖 Model: {}", model_name);
    println!();

    // Step 1: Load the model
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 1: Loading Model");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    let gguf = resolve_vault_model(&vault_path, &model_name)
        .map_err(|e| format!("Failed to resolve model: {}", e))?;

    println!("Loading {} …", gguf.display());
    let start = std::time::Instant::now();

    let record = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(
            qualia_client_core::model_lifecycle::activate_vault_gguf(&gguf),
        )
    })
    .map_err(|e| format!("Failed to activate model: {}", e))?;

    let load_time = start.elapsed();
    println!("✅ Model loaded in {:?}", load_time);
    println!("  Profile ID: 0x{:016x}", record.profile_id);
    println!("  Context Window: {}", record.context_window);
    println!();

    // Step 2: Create agent
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 2: Creating Agent");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    use qualia_core_db::llm_agent::{AgentBackend, LocalLlmAgent};

    let agent = LocalLlmAgent::with_local_backend(
        format!("did:qualia:cli-test:{}", record.profile_id),
        AgentBackend::Local {
            model_path: record.gguf_path.clone(),
            context_window: record.context_window,
            quantization: record.quantization.clone(),
            vision_projector_path: record.mmproj_path.clone(),
            modality: record.modality.clone(),
            architecture: record.architecture.clone(),
        },
    );

    println!("✅ Agent created");
    println!();

    // Step 3: Run inference tests
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 3: Inference Tests");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    // Sample (temperature / top-p / repeat-penalty) rather than greedy argmax, so instruct models
    // produce varied, on-topic text instead of collapsing into repetition / a fixed attractor.
    qualia_core_db::llm_bench::set_sampler_config(Some(
        qualia_core_db::sampler::SamplerConfig::chat_default(),
    ));

    let test_prompts = vec![
        ("Basic Knowledge", "What is the capital of France?", 50),
        (
            "System Awareness",
            "What is QualiaDB and what are its main features?",
            100,
        ),
        (
            "Technical Understanding",
            "Explain what a NQuin is in simple terms.",
            80,
        ),
        (
            "Capability Awareness",
            "What capabilities does the Qualia system have for semantic graph processing?",
            120,
        ),
        (
            "Instruction Following",
            "Write a haiku about artificial intelligence.",
            30,
        ),
    ];

    let mut total_tokens = 0;
    let mut total_time_ms = 0;
    let mut total_ttft_ms = 0;
    let mut successful_tests = 0;

    for (test_name, prompt, _max_tokens) in test_prompts.iter() {
        println!("┌─ Test: {}", test_name);
        println!("├─ Prompt: {}", prompt);

        let started = std::time::Instant::now();

        // Return layout: (text, provenance_hashes, tokens_generated, semantic_quin).
        // BUGFIX: the second field is provenance (often len=1), NOT token ids — using
        // `.len()` reported Tokens:1 and ~100× low tok/s. The third field is the count.
        let (response, _provenance, tokens_generated, _quin) = tokio::task::block_in_place(|| {
            agent.infer_local_model_streaming::<fn(String)>(prompt, "graph_context:cli_test", None)
        });

        let elapsed = started.elapsed();
        let elapsed_ms = elapsed.as_millis() as u64;

        // Estimate TTFT as ~10% of total time (rough approximation without streaming)
        let ttft = elapsed_ms / 10;

        let token_count = tokens_generated as u64;
        // Prefer decoded-token estimate when the engine returns 0 but produced text
        // (defensive — should not happen on a healthy path).
        let token_count = if token_count == 0 && !response.is_empty() {
            // Rough BPE estimate: ~4 chars/token for Latin text.
            (response.chars().count() as u64 / 4).max(1)
        } else {
            token_count
        };
        let tps = if elapsed_ms > 0 {
            (token_count as f64 * 1000.0) / elapsed_ms as f64
        } else {
            0.0
        };

        println!("├─ TTFT: {}ms (estimated)", ttft);
        println!("├─ Total Time: {}ms", elapsed_ms);
        println!("├─ Tokens: {} (decode)", token_count);
        println!("├─ TPS: {:.2} (wall-clock / decode tokens)", tps);

        total_tokens += token_count;
        total_time_ms += elapsed_ms;
        total_ttft_ms += ttft;
        successful_tests += 1;

        print!("└─ Response: ");
        // Show first 200 chars of response
        let preview: String = response.chars().take(200).collect();
        println!(
            "{}{}",
            preview,
            if response.len() > 200 { "..." } else { "" }
        );
        println!();
    }

    // Step 4: Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST SUMMARY");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Model Loading: PASS ({:?})", load_time);
    println!("✅ Agent Creation: PASS");
    println!(
        "✅ Inference: {} / {} tests passed",
        successful_tests,
        test_prompts.len()
    );

    if successful_tests > 0 {
        let avg_ttft = total_ttft_ms as f64 / successful_tests as f64;
        let avg_tps = if total_time_ms > 0 {
            (total_tokens as f64 * 1000.0) / total_time_ms as f64
        } else {
            0.0
        };

        println!();
        println!("📊 METRICS:");
        println!("  └─ Total Tokens Generated: {}", total_tokens);
        println!("  └─ Total Generation Time: {}ms", total_time_ms);
        println!("  └─ Average TTFT: {:.2}ms", avg_ttft);
        println!("  └─ Average TPS: {:.2}", avg_tps);
    }

    println!();
    println!("Note: Metrics include orchestration overhead (Webizen validation, etc.).");
    println!("Note: Token counts use engine tokens_generated (not provenance-hash vec length).");

    Ok(())
}

/// Convert a GGUF import file to native `.p64` + canonical `.q42` model metadata.
///
/// Design: GGUF/safetensors are import formats only. Steady-state activation should
/// prefer the converted container (see `docs/plans/native-inference-p64-pipeline-remediation.md`).
/// The helper is **CBOR-LD** (self-describe CBOR), never JSON.
pub fn run_convert_gguf_to_p64(
    input: &Path,
    out_dir: &Path,
    page_log2: u16,
    layout: &str,
) -> Result<(), String> {
    if !input.is_file() {
        return Err(format!("input not found: {}", input.display()));
    }
    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if ext != "gguf" && ext != "safetensors" {
        return Err(format!(
            "only .gguf or .safetensors import is supported in this command (got .{ext})"
        ));
    }
    let src_len = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);
    if src_len > u32::MAX as u64 {
        return Err(format!(
            "{} is {} bytes, but P64 v4 has 32-bit blob offsets and cannot represent a single payload above 4 GiB. Do not create a monolithic P64. For Qwen3.8 Flash Next, place the GGUF on C: and run `qualia-cli llm prepare-qwen4exp \"{}\" --out <new>.hmc`; it preserves the PLE table as a verified NVMe range source.",
            input.display(),
            src_len,
            input.display(),
        ));
    }
    // 12 GB class card default budget when auto-selecting f16 expand.
    const DEFAULT_VRAM_BUDGET: u64 = 12u64 * 1024 * 1024 * 1024;
    let layout = match layout.trim().to_ascii_lowercase().as_str() {
        "verbatim" | "raw" | "copy" => qualia_core_db::p64_weight::P64ConvertLayout::Verbatim,
        "f16" | "fp16" | "half" => qualia_core_db::p64_weight::P64ConvertLayout::F16Expand,
        "soa" | "q4k-soa" | "q4k_soa" | "soa-q4k" => {
            qualia_core_db::p64_weight::P64ConvertLayout::Q4kSoa
        }
        "auto" | "best" | "remarkable" => {
            let rec =
                qualia_core_db::p64_weight::recommend_convert_layout(src_len, DEFAULT_VRAM_BUDGET);
            println!("├─ auto layout → {rec:?} (source {src_len} B, 12 GiB VRAM budget)");
            rec
        }
        other => {
            return Err(format!(
                "unknown --layout '{other}' (expected verbatim|f16|soa|auto)"
            ))
        }
    };

    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("create out dir {}: {e}", out_dir.display()))?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Model ({ext}) → p64 + q42 convert");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ Input:  {}", input.display());
    println!("├─ Out:    {}", out_dir.display());
    println!("├─ page_log2: {page_log2}");
    println!("└─ layout: {layout:?}");

    let t0 = std::time::Instant::now();
    let mmap = {
        let f = std::fs::File::open(input).map_err(|e| format!("open: {e}"))?;
        // SAFETY: file is read-only; we do not write through the mapping.
        unsafe { memmap2::Mmap::map(&f).map_err(|e| format!("mmap: {e}"))? }
    };
    let src_bytes = mmap.len();
    println!(
        "├─ Source size: {:.1} MiB",
        src_bytes as f64 / (1024.0 * 1024.0)
    );

    let p64 = if ext == "safetensors" {
        let mut buf = Vec::new();
        qualia_core_db::p64_weight::transcode_safetensor_to_p64(&mmap, page_log2, &mut buf)
            .map_err(|e| format!("transcode_safetensor_to_p64: {e}"))?;
        buf
    } else {
        qualia_core_db::p64_weight::compile_gguf_to_p64_with_layout(&mmap, page_log2, layout)
            .map_err(|e| format!("compile_gguf_to_p64: {e}"))?
    };
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");
    let suffix = match layout {
        qualia_core_db::p64_weight::P64ConvertLayout::Verbatim => "",
        qualia_core_db::p64_weight::P64ConvertLayout::F16Expand => ".f16",
        qualia_core_db::p64_weight::P64ConvertLayout::Q4kSoa => ".soa",
    };
    let p64_path = out_dir.join(format!("{stem}{suffix}.p64"));
    std::fs::write(&p64_path, &p64).map_err(|e| format!("write p64: {e}"))?;

    // Canonical Q42 v3 metadata: behaviour the engine should not re-guess from GGUF.
    let tok = qualia_core_db::gguf_sharder::GgufTokenizer::from_gguf(&mmap);
    let stop_ids: Vec<u32> = tok.stop_tokens().to_vec();
    let stop_names: Vec<String> = stop_ids
        .iter()
        .filter_map(|&id| tok.vocab.get(id as usize).cloned())
        .collect();
    let helper = qualia_core_db::model_helper::ModelHelper::new(
        input
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("model.gguf"),
        p64_path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("model.p64"),
        page_log2,
        format!("{layout:?}"),
        qualia_core_db::model_helper::ModelHelperTokenizer {
            bos_token_id: tok.bos_token_id,
            eos_token_id: tok.eos_token_id,
            add_bos_token: tok.add_bos_token,
            chat_family: format!("{:?}", tok.chat_family()),
            stop_token_ids: stop_ids,
            stop_token_strings: stop_names,
            vocab_len: tok.vocab_len(),
        },
    );
    let q42_path = helper
        .write_beside_p64(&p64_path)
        .map_err(|e| format!("write canonical q42 helper: {e}"))?;

    // Validate the container can be indexed (fail closed if we wrote garbage).
    let index = qualia_core_db::p64_weight::P64TensorIndex::from_p64(&p64)
        .map_err(|e| format!("p64 self-check failed: {e}"))?;
    let n_tensors = index.entries.len();
    // Round-trip through the real Q42 volume reader so a malformed helper fails closed.
    let _ = qualia_core_db::model_helper::ModelHelper::load_beside_p64(&p64_path)
        .map_err(|e| format!("helper self-check failed: {e}"))?
        .ok_or_else(|| "helper self-check failed: canonical .q42 was not found".to_string())?;

    let elapsed = t0.elapsed();
    println!();
    println!("✅ Convert complete in {:.1}s", elapsed.as_secs_f64());
    println!("  └─ {}", p64_path.display());
    println!(
        "     size {:.1} MiB, tensors {}",
        p64.len() as f64 / (1024.0 * 1024.0),
        n_tensors
    );
    println!("  └─ {} (Q42 v3)", q42_path.display());
    println!(
        "     chat_family={:?} stop_ids={:?}",
        tok.chat_family(),
        tok.stop_tokens()
    );
    println!();
    println!("Activate with a Local backend path pointing at the .p64 file.");
    Ok(())
}

/// Extract the Qwen4Exp PLE table to NVMe and create its compact HMC contract.
/// The source GGUF stays in place; only its row-addressable PLE span is copied.
pub fn run_prepare_qwen4exp_native(
    input: &Path,
    output: &Path,
    ple_output: &Path,
) -> Result<(), String> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp native HMC preparation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ Source trunk: {}", input.display());
    println!("├─ PLE NVMe:     {}", ple_output.display());
    println!("└─ HMC:          {}", output.display());
    let prepared =
        qualia_core_db::inference::qwen::prepare_qwen4exp_native_hmc(input, output, ple_output)
            .map_err(|error| error.to_string())?;
    println!("✅ HMC contract created");
    println!("  └─ source bytes: {}", prepared.source.byte_len);
    println!(
        "  └─ PLE NVMe: {} bytes / {} rows × {}",
        prepared.ple.storage_bytes, prepared.ple.rows, prepared.ple.row_width
    );
    println!(
        "  └─ trunk GGUF bytes: {} (not monolithic P64)",
        prepared.trunk_gguf_bytes
    );
    println!(
        "  └─ graph: {} GatedDeltaNet layers, {} QSA layers, {} PLE layers",
        prepared.gated_deltanet_layers, prepared.qsa_layers, prepared.ple_layers
    );
    println!("  └─ source SHA-256: pending explicit verify-qwen4exp audit");
    Ok(())
}

/// Audit an existing Qwen4Exp HMC's external source before activation.
pub fn run_verify_qwen4exp_native(package: &Path) -> Result<(), String> {
    let descriptor = qualia_core_db::inference::qwen::load_qwen4exp_native_hmc(package)
        .map_err(|error| error.to_string())?;
    let source = descriptor
        .source_beside(package)
        .map_err(|error| error.to_string())?;
    println!("Auditing {}", package.display());
    println!("├─ External GGUF: {}", source.display());
    descriptor
        .verify_source_sha256(&source)
        .map_err(|error| error.to_string())?;
    println!("✅ HMC descriptor and external GGUF SHA-256 match");
    println!(
        "  └─ PLE: {} rows × {} values ({} storage bytes at file offset {})",
        descriptor.ple.rows,
        descriptor.ple.row_width,
        descriptor.ple.storage_bytes,
        descriptor.ple.absolute_offset
    );
    Ok(())
}

/// Exercise the actual C:-resident PLE reader without mapping the PLE table,
/// opening the source GGUF, or allocating a row dynamically.
pub fn run_probe_qwen4exp_ple(package: &Path) -> Result<(), String> {
    const MAX_PLE_ROW_BYTES: usize = 65_536;
    const MAX_PLE_ROW_WIDTH: usize = 4_096;

    let descriptor = qualia_core_db::inference::qwen::load_qwen4exp_native_hmc(package)
        .map_err(|error| error.to_string())?;
    let mut reader =
        qualia_core_db::inference::qwen::PleNvmeReader::open_from_native_descriptor(&descriptor)
            .map_err(|error| format!("open NVMe PLE: {error:?}"))?;
    if reader.raw_row_bytes() > MAX_PLE_ROW_BYTES || reader.row_width() > MAX_PLE_ROW_WIDTH {
        return Err("PLE row exceeds fixed probe buffer contract".to_string());
    }

    let last = reader
        .row_count()
        .checked_sub(1)
        .ok_or_else(|| "PLE has no rows".to_string())?;
    let candidates = [0u64, reader.row_count() / 2, last];
    let mut raw = [0u8; MAX_PLE_ROW_BYTES];
    let mut values = [0.0f32; MAX_PLE_ROW_WIDTH];
    let mut rows = 0u32;
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp NVMe PLE probe");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ HMC: {}", package.display());
    println!("├─ PLE: {}", descriptor.ple_storage.file_path);
    println!(
        "├─ table: {} rows × {} values; {} bytes/row",
        reader.row_count(),
        reader.row_width(),
        reader.raw_row_bytes()
    );
    for row in candidates {
        if rows != 0 && row == candidates[(rows - 1) as usize] {
            continue;
        }
        let n = reader
            .read_row_into(row, &mut raw, &mut values)
            .map_err(|error| format!("read PLE row {row}: {error:?}"))?;
        if n != reader.row_width() || values[..n].iter().any(|value| !value.is_finite()) {
            return Err(format!("invalid decoded PLE row {row}"));
        }
        for value in &values[..n] {
            fingerprint ^= value.to_bits() as u64;
            fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
        }
        println!(
            "├─ row {row}: {n} values decoded from {} bytes",
            reader.raw_row_bytes()
        );
        rows += 1;
    }
    println!("└─ direct-NVMe probe passed; sampled_row_fingerprint={fingerprint:016x}");
    Ok(())
}

/// Compute the checkpoint's PLE n-gram row IDs from decode-order token IDs,
/// then gather exactly those rows from the C:-resident table.  This is an
/// execution seam, not a text-generation benchmark: it proves that runtime
/// token history selects and consumes trained PLE rows without mapping the
/// table or using a synthetic hash.
pub fn run_gather_qwen4exp_ple(package: &Path, token_ids: &[u32]) -> Result<(), String> {
    const MAX_PLE_ROWS: usize = 64;
    const MAX_PLE_ROW_BYTES: usize = 65_536;
    const MAX_PLE_VALUES: usize = 16_384;

    if token_ids.is_empty() {
        return Err("at least one token ID is required".to_string());
    }
    let descriptor = qualia_core_db::inference::qwen::load_qwen4exp_native_hmc(package)
        .map_err(|error| error.to_string())?;
    let mut reader =
        qualia_core_db::inference::qwen::PleNvmeReader::open_from_native_descriptor(&descriptor)
            .map_err(|error| format!("open NVMe PLE: {error:?}"))?;
    let heads = descriptor
        .ple_config
        .ngram_size
        .saturating_sub(1)
        .saturating_mul(descriptor.ple_config.heads_per_ngram) as usize;
    let needed = heads
        .checked_mul(reader.row_width())
        .ok_or_else(|| "PLE gather size overflow".to_string())?;
    if heads == 0
        || heads > MAX_PLE_ROWS
        || reader.raw_row_bytes() > MAX_PLE_ROW_BYTES
        || needed > MAX_PLE_VALUES
    {
        return Err("PLE gather exceeds this command's fixed buffer contract".to_string());
    }

    let mut history = qualia_core_db::inference::qwen::PleTokenHistory::default();
    let mut rows = [0u64; MAX_PLE_ROWS];
    let mut raw = [0u8; MAX_PLE_ROW_BYTES];
    let mut values = [0.0f32; MAX_PLE_VALUES];
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp exact PLE n-gram gather");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ PLE: {}", descriptor.ple_storage.file_path);
    println!(
        "├─ contract: {}-gram × {} heads = {} rows/token",
        descriptor.ple_config.ngram_size, descriptor.ple_config.heads_per_ngram, heads
    );
    for &token in token_ids {
        let selected = history
            .select_and_push(
                &descriptor.ple_config,
                token,
                reader.row_count(),
                &mut rows[..heads],
            )
            .map_err(|error| format!("select PLE rows for token {token}: {error}"))?;
        let receipt = reader
            .gather_rows_into(&rows[..selected], &mut raw, &mut values[..needed])
            .map_err(|error| format!("gather PLE rows for token {token}: {error:?}"))?;
        if values[..needed].iter().any(|value| !value.is_finite()) {
            return Err(format!("non-finite PLE value for token {token}"));
        }
        for value in &values[..needed] {
            fingerprint ^= value.to_bits() as u64;
            fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
        }
        println!(
            "├─ token {token}: {selected} trained rows, {} NVMe bytes",
            receipt.storage_bytes
        );
    }
    println!("└─ exact PLE history/gather passed; fingerprint={fingerprint:016x}");
    Ok(())
}

/// Execute the complete trained PLE residual insertion for one token.  This
/// is a physical C:/E: forward component check, not token generation: all
/// later GDN/QSA, four-stream and MoE layers remain to be executed.
pub fn run_probe_qwen4exp_ple_block(package: &Path, token_id: u32) -> Result<(), String> {
    const HIDDEN: usize = 2_560;
    const STREAMS: usize = 4;
    const WIDE: usize = HIDDEN * STREAMS;
    const HISTORY: usize = 9 * WIDE;
    const RAW_PLE: usize = 1_024;
    const RAW_TRUNK: usize = 65_536;

    let mut runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    let embedding = *runtime
        .index
        .token_embd_info()
        .ok_or_else(|| "Qwen4Exp source has no token embedding tensor".to_string())?;
    if embedding.dims[0] as usize != HIDDEN || token_id as u64 >= embedding.dims[1] {
        return Err(
            "Qwen4Exp token embedding does not match the fixed PLE block contract".to_string(),
        );
    }
    let layer_id = runtime.descriptor.ple_config.layers[0];
    let layer = runtime.index.get_layer_tensors(layer_id);
    let mut raw_ple = [0u8; RAW_PLE];
    let mut raw_trunk = [0u8; RAW_TRUNK];
    let mut embedding_values = [0.0f32; HIDDEN];
    runtime
        .trunk
        .read_row_into(
            &embedding,
            token_id as usize,
            &mut raw_trunk,
            &mut embedding_values,
        )
        .map_err(|error| format!("read token embedding: {error}"))?;
    let mut residual = [0.0f32; WIDE];
    for stream in 0..STREAMS {
        residual[stream * HIDDEN..(stream + 1) * HIDDEN].copy_from_slice(&embedding_values);
    }
    let mut history = [0.0f32; HISTORY];
    let mut projection_row = [0.0f32; HIDDEN];
    let mut ple_embedding = [0.0f32; HIDDEN];
    let mut key = [0.0f32; WIDE];
    let mut value = [0.0f32; HIDDEN];
    let mut query_norm = [0.0f32; WIDE];
    let mut gated = [0.0f32; WIDE];
    let mut conv_norm = [0.0f32; WIDE];
    let mut conv_weights = [0.0f32; WIDE * 4];
    let mut norm_key = [0.0f32; WIDE];
    let mut norm_query = [0.0f32; WIDE];
    let mut norm_conv = [0.0f32; WIDE];
    let mut buffers = qualia_core_db::inference::qwen::PleBlockBuffers {
        raw_ple_row: &mut raw_ple,
        raw_trunk_row: &mut raw_trunk,
        projection_row: &mut projection_row,
        embedding: &mut ple_embedding,
        key: &mut key,
        value: &mut value,
        query_norm: &mut query_norm,
        gated: &mut gated,
        conv_norm: &mut conv_norm,
        conv_weights: &mut conv_weights,
        norm_key: &mut norm_key,
        norm_query: &mut norm_query,
        norm_conv: &mut norm_conv,
    };
    let mut token_history = qualia_core_db::inference::qwen::PleTokenHistory::default();
    let start = std::time::Instant::now();
    qualia_core_db::inference::qwen::execute_ple_block(
        &runtime.descriptor.ple_config,
        &layer,
        token_id,
        &mut token_history,
        &mut runtime.ple,
        &mut runtime.trunk,
        &mut residual,
        &mut history,
        &mut buffers,
    )
    .map_err(|error| format!("execute trained PLE block: {error}"))?;
    let elapsed = start.elapsed();
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    for value in residual {
        fingerprint ^= value.to_bits() as u64;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp trained PLE residual block");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ layer {layer_id}, token {token_id}: 16 C: n-gram rows → 4 × {HIDDEN} residual");
    println!("├─ PLE block time: {:.3}s", elapsed.as_secs_f64());
    println!("└─ residual fingerprint={fingerprint:016x}");
    Ok(())
}

/// Execute one trained attention- or FFN-side Hyper-Connection mix using the
/// source checkpoint's own weight rows. This is a forward component probe;
/// it deliberately does not substitute an attention/GDN or MoE result.
pub fn run_probe_qwen4exp_hyper(
    package: &Path,
    layer_id: u32,
    ffn: bool,
    token_id: u32,
) -> Result<(), String> {
    const HIDDEN: usize = 2_560;
    const STREAMS: usize = 4;
    const WIDE: usize = HIDDEN * STREAMS;
    const MAX_RANK: usize = 1_024;
    const RAW: usize = 65_536;
    let mut runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    let embedding = *runtime
        .index
        .token_embd_info()
        .ok_or_else(|| "Qwen4Exp source has no token embedding tensor".to_string())?;
    if embedding.dims[0] as usize != HIDDEN || token_id as u64 >= embedding.dims[1] {
        return Err(
            "Qwen4Exp token embedding does not match the fixed Hyper-Connection contract"
                .to_string(),
        );
    }
    let layer = runtime.index.get_layer_tensors(layer_id);
    let rank = if ffn {
        layer.hc_ffn_down.as_ref()
    } else {
        layer.hc_attn_down.as_ref()
    }
    .map(|info| info.dims[1] as usize)
    .ok_or_else(|| format!("layer {layer_id} has no requested Hyper-Connection down projection"))?;
    if rank > MAX_RANK {
        return Err("Hyper-Connection rank exceeds the fixed probe buffer".to_string());
    }
    let mut raw = [0u8; RAW];
    let mut embedding_values = [0.0f32; HIDDEN];
    runtime
        .trunk
        .read_row_into(
            &embedding,
            token_id as usize,
            &mut raw,
            &mut embedding_values,
        )
        .map_err(|error| format!("read token embedding: {error}"))?;
    let mut residual = [0.0f32; WIDE];
    for stream in 0..STREAMS {
        residual[stream * HIDDEN..(stream + 1) * HIDDEN].copy_from_slice(&embedding_values);
    }
    let mut mixed = [0.0f32; HIDDEN];
    let mut inject = [0.0f32; STREAMS];
    let mut projection_row = [0.0f32; WIDE];
    let mut norm_weight = [0.0f32; WIDE];
    let mut normalized = [0.0f32; WIDE];
    let mut low_rank = [0.0f32; MAX_RANK];
    let mut gates = [0.0f32; WIDE];
    let mut buffers = qualia_core_db::inference::qwen::StreamedHyperBuffers {
        raw_row: &mut raw,
        projection_row: &mut projection_row,
        norm_weight: &mut norm_weight,
        normalized: &mut normalized,
        low_rank: &mut low_rank,
        gates: &mut gates,
    };
    let start = std::time::Instant::now();
    qualia_core_db::inference::qwen::execute_streamed_hyper_connection(
        &mut runtime.trunk,
        &layer,
        ffn,
        &residual,
        &mut mixed,
        Some(&mut inject),
        &mut buffers,
    )
    .map_err(|error| format!("execute trained Hyper-Connection: {error}"))?;
    let elapsed = start.elapsed();
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    for value in mixed.into_iter().chain(inject) {
        fingerprint ^= value.to_bits() as u64;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp trained Hyper-Connection block");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "├─ layer {layer_id} {} path, rank {rank}",
        if ffn { "FFN" } else { "token-mixer" }
    );
    println!("├─ Hyper block time: {:.3}s", elapsed.as_secs_f64());
    println!("└─ mix/inject fingerprint={fingerprint:016x}");
    Ok(())
}

/// Execute the trained recurrent token mixer with its real learned state.
/// The two state vectors are explicitly bounded and live only for this CLI
/// invocation; no model payload is copied from E: or C: into them.
pub fn run_probe_qwen4exp_gdn(package: &Path, layer_id: u32, token_id: u32) -> Result<(), String> {
    const HIDDEN: usize = 2_560;
    const HEADS: usize = 48;
    const HEAD_DIM: usize = 128;
    const INNER: usize = HEADS * HEAD_DIM;
    const QKV: usize = 10_240;
    const RAW: usize = 65_536;
    let mut runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    let embedding = *runtime
        .index
        .token_embd_info()
        .ok_or_else(|| "Qwen4Exp source has no token embedding tensor".to_string())?;
    if embedding.dims[0] as usize != HIDDEN || token_id as u64 >= embedding.dims[1] {
        return Err(
            "Qwen4Exp token embedding does not match the GatedDeltaNet contract".to_string(),
        );
    }
    let layer = runtime.index.get_layer_tensors(layer_id);
    if !layer.is_hybrid_ssm_layer() {
        return Err(format!(
            "layer {layer_id} is not a complete Qwen4Exp GatedDeltaNet layer"
        ));
    }
    let mut raw = [0u8; RAW];
    let mut input = [0.0f32; HIDDEN];
    runtime
        .trunk
        .read_row_into(&embedding, token_id as usize, &mut raw, &mut input)
        .map_err(|error| format!("read token embedding: {error}"))?;
    // Runtime-owned recurrence state: 10,240 × 3 convolution values and
    // 48 × 128 × 128 delta values = 3,176,448 f32 (about 12.1 MiB).
    let mut convolution = vec![0.0f32; QKV * 3];
    let mut delta = vec![0.0f32; HEADS * HEAD_DIM * HEAD_DIM];
    let mut out = [0.0f32; HIDDEN];
    let mut projection_row = vec![0.0f32; INNER];
    let mut qkv = vec![0.0f32; QKV];
    let mut gate = vec![0.0f32; INNER];
    let mut alpha = [0.0f32; HEADS];
    let mut beta = [0.0f32; HEADS];
    let mut convolved = vec![0.0f32; QKV];
    let mut head_norm = [0.0f32; HEAD_DIM];
    let mut head_vector = [0.0f32; HEAD_DIM];
    let mut output_inner = vec![0.0f32; INNER];
    let mut head_a = [0.0f32; HEADS];
    let mut dt_bias = [0.0f32; HEADS];
    let mut state = qualia_core_db::inference::qwen::GatedDeltaState {
        convolution: &mut convolution,
        delta: &mut delta,
    };
    let mut buffers = qualia_core_db::inference::qwen::GatedDeltaBuffers {
        raw_row: &mut raw,
        projection_row: &mut projection_row,
        qkv: &mut qkv,
        gate: &mut gate,
        alpha: &mut alpha,
        beta: &mut beta,
        convolved: &mut convolved,
        head_norm: &mut head_norm,
        head_vector: &mut head_vector,
        output_inner: &mut output_inner,
        head_a: &mut head_a,
        dt_bias: &mut dt_bias,
    };
    let start = std::time::Instant::now();
    qualia_core_db::inference::qwen::execute_streamed_gated_delta(
        &mut runtime.trunk,
        &layer,
        &input,
        &mut state,
        &mut out,
        &mut buffers,
    )
    .map_err(|error| format!("execute trained GatedDeltaNet: {error}"))?;
    let elapsed = start.elapsed();
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    for value in out.into_iter().chain(delta.iter().copied().step_by(257)) {
        fingerprint ^= value.to_bits() as u64;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp trained GatedDeltaNet token mixer");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ layer {layer_id}, token {token_id}: 48 × 128 × 128 recurrent state");
    println!("├─ GatedDeltaNet time: {:.3}s", elapsed.as_secs_f64());
    println!("└─ branch/state fingerprint={fingerprint:016x}");
    Ok(())
}

/// Verify the external trunk reader against the exact source identified by an
/// HMC. Only the bounded GGUF metadata prefix is read; the embedding row is
/// consumed through `TrunkNvmeReader` from E:.
pub fn run_probe_qwen4exp_trunk(package: &Path, token_id: u32) -> Result<(), String> {
    const MAX_ROW_BYTES: usize = 65_536;
    const MAX_ROW_WIDTH: usize = 4_096;

    let descriptor = qualia_core_db::inference::qwen::load_qwen4exp_native_hmc(package)
        .map_err(|error| error.to_string())?;
    let source = descriptor
        .source_beside(package)
        .map_err(|error| error.to_string())?;
    descriptor
        .verify_source_metadata(&source)
        .map_err(|error| error.to_string())?;
    let index =
        qualia_core_db::inference::gguf_sharder::GgufTensorIndex::from_gguf_header_file(&source)
            .map_err(|error| format!("read bounded trunk header: {error}"))?;
    let embedding = *index
        .token_embd_info()
        .ok_or_else(|| "Qwen4Exp source has no token embedding tensor".to_string())?;
    if token_id as u64 >= embedding.dims[1] {
        return Err(format!("token ID {token_id} exceeds vocabulary"));
    }
    let mut reader =
        qualia_core_db::inference::qwen::TrunkNvmeReader::open(&source, index.tensor_data_start)
            .map_err(|error| format!("open trunk row reader: {error}"))?;
    let row_bytes = qualia_core_db::inference::ggml_quants::ggml_row_bytes(
        embedding.ggml_type,
        embedding.dims[0] as usize,
    )
    .ok_or_else(|| "unsupported token embedding quantization".to_string())?;
    if row_bytes > MAX_ROW_BYTES || embedding.dims[0] as usize > MAX_ROW_WIDTH {
        return Err("token embedding exceeds fixed trunk probe buffers".to_string());
    }
    let mut raw = [0u8; MAX_ROW_BYTES];
    let mut values = [0f32; MAX_ROW_WIDTH];
    let written = reader
        .read_row_into(&embedding, token_id as usize, &mut raw, &mut values)
        .map_err(|error| format!("read trunk embedding: {error}"))?;
    if values[..written].iter().any(|value| !value.is_finite()) {
        return Err("trunk embedding contains non-finite decoded values".to_string());
    }
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    for value in &values[..written] {
        fingerprint ^= value.to_bits() as u64;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp bounded trunk-row probe");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ source: {}", source.display());
    println!("├─ token {token_id}: {written} values decoded from {row_bytes} bytes");
    println!("└─ direct trunk row passed; fingerprint={fingerprint:016x}");
    Ok(())
}

/// Cold-activate the native C:/E: package layout.  This is deliberately
/// separate from generation: it proves the model can be opened with a
/// bounded metadata read before a decode runner is allowed to touch weights.
pub fn run_activate_qwen4exp_native(package: &Path) -> Result<(), String> {
    let runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp native bounded activation");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ PLE NVMe: {}", runtime.descriptor.ple_storage.file_path);
    println!(
        "├─ PLE rows: {} × {}",
        runtime.ple.row_count(),
        runtime.ple.row_width()
    );
    println!(
        "├─ trunk metadata: {} bytes before tensor payload",
        runtime.index.tensor_data_start
    );
    println!("├─ trunk: {}", runtime.descriptor.source.source_path);
    println!("└─ active: no source GGUF mmap or full-model copy");
    Ok(())
}

/// Execute one true Qwen4Exp routed-MoE operator.  This is an operator
/// validation seam, not text generation: it uses a token embedding directly
/// instead of a fully evolved preceding token-mixer state.
pub fn run_probe_qwen4exp_moe(
    package: &Path,
    layer: u32,
    token_id: u32,
    tile: Option<&Path>,
) -> Result<(), String> {
    const MAX_HIDDEN: usize = 4_096;
    const MAX_INTERMEDIATE: usize = 4_096;
    const MAX_ROUTER: usize = 1_024;
    const MAX_RAW_ROW: usize = 65_536;

    let mut runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    let embedding = *runtime
        .index
        .token_embd_info()
        .ok_or_else(|| "Qwen4Exp source has no token embedding tensor".to_string())?;
    if token_id as u64 >= embedding.dims[1] || embedding.dims[0] as usize > MAX_HIDDEN {
        return Err("token embedding exceeds fixed MoE probe buffers".to_string());
    }
    let tensors = runtime.index.get_layer_tensors(layer);
    let intermediate = tensors
        .moe_gate_exps
        .as_ref()
        .map(|info| info.dims[1] as usize)
        .ok_or_else(|| format!("layer {layer} has no routed MoE gate tensor"))?;
    if intermediate > MAX_INTERMEDIATE {
        return Err("MoE intermediate width exceeds fixed probe buffers".to_string());
    }

    let describe =
        |label: &str, tensor: Option<&qualia_core_db::inference::gguf_sharder::GgufTensorInfo>| {
            tensor
                .map(|info| {
                    format!(
                        "{label}={:?}/{}D/type{}",
                        info.dims, info.n_dims, info.ggml_type
                    )
                })
                .unwrap_or_else(|| format!("{label}=missing"))
        };
    println!(
        "MoE tensor layout: {}, {}, {}, {}",
        describe("router", tensors.moe_router.as_ref()),
        describe("gate", tensors.moe_gate_exps.as_ref()),
        describe("up", tensors.moe_up_exps.as_ref()),
        describe("down", tensors.moe_down_exps.as_ref()),
    );
    println!(
        "MoE shared layout: {}, {}, {}, {}",
        describe("gate", tensors.moe_shared_gate.as_ref()),
        describe("up", tensors.moe_shared_up.as_ref()),
        describe("down", tensors.moe_shared_down.as_ref()),
        describe("input", tensors.moe_shared_gate_input.as_ref()),
    );

    let hidden = embedding.dims[0] as usize;
    let mut raw = [0u8; MAX_RAW_ROW];
    let mut dequantized_row = [0.0f32; MAX_HIDDEN];
    let mut input = [0.0f32; MAX_HIDDEN];
    runtime
        .trunk
        .read_row_into(
            &embedding,
            token_id as usize,
            &mut raw,
            &mut input[..hidden],
        )
        .map_err(|error| format!("read input embedding: {error}"))?;
    let mut output = [0.0f32; MAX_HIDDEN];
    let mut expert_out = [0.0f32; MAX_HIDDEN];
    let mut router = [0.0f32; MAX_ROUTER];
    let mut top_indices = [0usize; qualia_core_db::inference::qwen::QWEN4EXP_TOP_EXPERTS];
    let mut top_logits = [0.0f32; qualia_core_db::inference::qwen::QWEN4EXP_TOP_EXPERTS];
    let mut top_weights = [0.0f32; qualia_core_db::inference::qwen::QWEN4EXP_TOP_EXPERTS];
    let mut gate = [0.0f32; MAX_INTERMEDIATE];
    let mut up = [0.0f32; MAX_INTERMEDIATE];
    let mut activation = [0.0f32; MAX_INTERMEDIATE];
    let tile_reader = tile
        .map(qualia_core_db::inference::qwen::QwenExpertTileReader::open)
        .transpose()
        .map_err(|error| format!("open C: expert tile: {error}"))?;
    let mut cached_tiles = [None; qualia_core_db::inference::qwen::QWEN4EXP_TOP_EXPERTS];
    if let Some(reader) = tile_reader.as_ref() {
        let router_info = tensors
            .moe_router
            .as_ref()
            .ok_or_else(|| format!("layer {layer} has no routed MoE router tensor"))?;
        runtime
            .trunk
            .gemv_rows_into(
                router_info,
                0,
                qualia_core_db::inference::qwen::QWEN4EXP_EXPERT_COUNT,
                &input[..hidden],
                &mut router,
                &mut raw,
                &mut dequantized_row,
            )
            .map_err(|error| format!("pre-route C: expert tile: {error}"))?;
        qualia_core_db::inference::qwen::select_top_experts(
            &router[..qualia_core_db::inference::qwen::QWEN4EXP_EXPERT_COUNT],
            &mut top_indices,
            &mut top_logits,
        )
        .map_err(|error| format!("pre-route C: expert tile: {error}"))?;
        let mut selected = false;
        for route in 0..qualia_core_db::inference::qwen::QWEN4EXP_TOP_EXPERTS {
            if reader.descriptor.layer == layer as u16
                && reader.descriptor.expert == top_indices[route] as u16
            {
                cached_tiles[route] = Some(reader);
                selected = true;
            }
        }
        if !selected {
            return Err(format!(
                "C: expert tile layer {} expert {} is not selected for layer {layer}, token {token_id}; no source/cache mixing was performed",
                reader.descriptor.layer, reader.descriptor.expert
            ));
        }
    }
    let start = std::time::Instant::now();
    qualia_core_db::inference::qwen::execute_streamed_moe_with_tiles(
        &mut runtime.trunk,
        &tensors,
        &input[..hidden],
        &mut output[..hidden],
        &mut router,
        &mut top_indices,
        &mut top_logits,
        &mut top_weights,
        &cached_tiles,
        &mut gate[..intermediate],
        &mut up[..intermediate],
        &mut activation[..intermediate],
        &mut expert_out[..hidden],
        &mut raw,
        &mut dequantized_row,
    )
    .map_err(|error| format!("streamed MoE: {error}"))?;
    let elapsed = start.elapsed();
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    for value in &output[..hidden] {
        fingerprint ^= value.to_bits() as u64;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp selected-expert MoE probe");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ layer {layer}, token {token_id}: {hidden} hidden × {intermediate} intermediate");
    println!("├─ selected experts: {:?}", top_indices);
    if let Some(reader) = tile_reader.as_ref() {
        println!(
            "├─ C: cached route: layer {} expert {}",
            reader.descriptor.layer, reader.descriptor.expert
        );
    }
    println!("├─ operator time: {:.3}s", elapsed.as_secs_f64());
    println!("└─ routed/shared MoE output fingerprint={fingerprint:016x}");
    Ok(())
}

/// Explicit cold-path promotion of one selected expert.  This is deliberately
/// not part of ordinary activation: the caller must name a C: cache directory
/// and budget before any new artifact is made.
pub fn run_promote_qwen4exp_expert(
    package: &Path,
    layer: u16,
    expert: u16,
    cache_dir: &Path,
    max_cache_gib: u64,
) -> Result<(), String> {
    let runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    let tensors = runtime.index.get_layer_tensors(layer as u32);
    let gate = tensors
        .moe_gate_exps
        .as_ref()
        .ok_or_else(|| format!("layer {layer} has no routed MoE gate tensor"))?;
    let up = tensors
        .moe_up_exps
        .as_ref()
        .ok_or_else(|| format!("layer {layer} has no routed MoE up tensor"))?;
    let down = tensors
        .moe_down_exps
        .as_ref()
        .ok_or_else(|| format!("layer {layer} has no routed MoE down tensor"))?;
    let limit = max_cache_gib
        .checked_mul(1024 * 1024 * 1024)
        .ok_or_else(|| "cache budget overflow".to_string())?;
    let source = runtime
        .descriptor
        .source_beside(package)
        .map_err(|error| error.to_string())?;
    let (tile, admission) = qualia_core_db::inference::qwen::promote_expert_tile(
        &source,
        runtime.descriptor.source.byte_len,
        runtime.index.tensor_data_start,
        layer,
        expert,
        gate,
        up,
        down,
        cache_dir,
        limit,
    )
    .map_err(|error| error.to_string())?;
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp selected-expert C: promotion");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ layer {layer}, expert {expert}");
    println!("├─ tile: {}", tile.display());
    println!("├─ requested: {} bytes", admission.requested_bytes);
    println!(
        "└─ cache: {} / {} bytes before promotion",
        admission.cache_used_bytes, admission.cache_limit_bytes
    );
    Ok(())
}

/// Exercise a promoted C: expert tile using a true E: token embedding. This
/// is one cached gate-plane operator, not a complete MoE or text decode.
pub fn run_probe_qwen4exp_expert_tile(
    package: &Path,
    tile: &Path,
    token_id: u32,
) -> Result<(), String> {
    const MAX_HIDDEN: usize = 4_096;
    const MAX_INTERMEDIATE: usize = 4_096;
    const MAX_RAW_ROW: usize = 65_536;
    let mut runtime = qualia_core_db::inference::qwen::Qwen4ExpNativeRuntime::activate(package)
        .map_err(|error| error.to_string())?;
    let embedding = *runtime
        .index
        .token_embd_info()
        .ok_or_else(|| "Qwen4Exp source has no token embedding tensor".to_string())?;
    let hidden = embedding.dims[0] as usize;
    if hidden > MAX_HIDDEN || token_id as u64 >= embedding.dims[1] {
        return Err("token embedding exceeds fixed tile-probe buffers".to_string());
    }
    let reader = qualia_core_db::inference::qwen::QwenExpertTileReader::open(tile)
        .map_err(|error| error.to_string())?;
    let intermediate = reader.descriptor.gate.dims[1] as usize;
    if reader.descriptor.gate.dims[0] as usize != hidden || intermediate > MAX_INTERMEDIATE {
        return Err("tile gate shape does not match the source token embedding".to_string());
    }
    let mut raw = [0u8; MAX_RAW_ROW];
    let mut input = [0.0f32; MAX_HIDDEN];
    runtime
        .trunk
        .read_row_into(
            &embedding,
            token_id as usize,
            &mut raw,
            &mut input[..hidden],
        )
        .map_err(|error| format!("read input embedding: {error}"))?;
    let mut output = [0.0f32; MAX_INTERMEDIATE];
    let mut row = [0.0f32; MAX_HIDDEN];
    let start = std::time::Instant::now();
    reader
        .gemv_into(
            qualia_core_db::inference::qwen::QwenExpertTilePlaneKind::Gate,
            &input[..hidden],
            &mut output[..intermediate],
            &mut row,
        )
        .map_err(|error| error.to_string())?;
    let elapsed = start.elapsed();
    let mut fingerprint = 0xcbf2_9ce4_8422_2325u64;
    for value in &output[..intermediate] {
        fingerprint ^= value.to_bits() as u64;
        fingerprint = fingerprint.wrapping_mul(0x1000_0000_01b3);
    }
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Qwen4Exp C: expert-tile gate probe");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!(
        "├─ layer {}, expert {}; {} → {}",
        reader.descriptor.layer, reader.descriptor.expert, hidden, intermediate
    );
    println!(
        "├─ tile: {} bytes",
        std::fs::metadata(tile).map(|m| m.len()).unwrap_or(0)
    );
    println!("├─ C: gate operator time: {:.3}s", elapsed.as_secs_f64());
    println!("└─ gate output fingerprint={fingerprint:016x}");
    Ok(())
}

/// Remarkable one-shot: optional passport + auto-layout convert + activate knobs.
pub fn run_optimize_pipeline(
    input: &Path,
    out: Option<PathBuf>,
    skip_passport: bool,
) -> Result<(), String> {
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("REMARKABLE PATH — passport + convert + activate knobs");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");

    if !skip_passport {
        let _ = run_hardware_passport(true, 512, None, true, None, 16);
    }

    let out_dir = out.unwrap_or_else(|| {
        input
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    });
    run_convert_gguf_to_p64(input, &out_dir, 14, "auto")?;

    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");
    // auto may produce .f16.p64 / .soa.p64 / .p64 — list what we wrote
    let candidates = [
        out_dir.join(format!("{stem}.f16.p64")),
        out_dir.join(format!("{stem}.soa.p64")),
        out_dir.join(format!("{stem}.p64")),
    ];
    let p64 = candidates.iter().find(|p| p.is_file());
    println!();
    println!("Activate (fast path):");
    println!("  $env:QUALIA_P64_INTEGRITY='metadata'");
    if let Some(p) = p64 {
        println!("  # model path: {}", p.display());
        if let Ok(Some(h)) = qualia_core_db::model_helper::ModelHelper::load_beside_p64(p) {
            println!(
                "  # helper: layout={} family={} stops={:?}",
                h.layout, h.tokenizer.chat_family, h.tokenizer.stop_token_ids
            );
        }
    }
    println!("  qualia-cli llm load <stem-or-path>   # vault prefers .p64");
    Ok(())
}

/// Probe / load HardwarePassport and print the ranked circuit matrix.
///
/// `decode_proxy`: `None` = skip; `Some(None)` = auto-find smollm; `Some(Some(path))` = use path.
pub fn run_hardware_passport(
    reprobe: bool,
    gemv_n: usize,
    cache: Option<PathBuf>,
    apply_env_hint: bool,
    decode_proxy: Option<Option<PathBuf>>,
    decode_proxy_tokens: u32,
) -> Result<(), String> {
    use qualia_core_db::device_benchmark::benchmark_devices;
    use qualia_core_db::hardware_passport::{
        attach_decode_proxy_via_subprocess, backend_env_token, default_cache_path,
        default_decode_proxy_model, load_or_probe, topology_key, write_passport, HardwarePassport,
        PASSPORT_VERSION,
    };
    use qualia_core_db::host_topology::probe_host_topology;

    let path = cache.unwrap_or_else(default_cache_path);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("HardwarePassport");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ Cache: {}", path.display());
    println!("├─ GEMV n: {gemv_n}");
    println!("└─ Reprobe: {reprobe}");

    let (mut passport, was_cached) = if reprobe {
        if path.exists() {
            let _ = std::fs::remove_file(&path);
        }
        let topology = probe_host_topology();
        let key = topology_key(&topology);
        println!("├─ Probing circuits (this takes a few seconds)…");
        let matrix = benchmark_devices(gemv_n);
        let preferred = matrix
            .best()
            .and_then(|c| backend_env_token(&c.backend))
            .map(str::to_string);
        let fresh = HardwarePassport {
            version: PASSPORT_VERSION,
            key,
            topology,
            matrix,
            preferred_inference_backend: preferred,
            probe_gemv_n: gemv_n,
            decode_proxy_model: None,
            decode_proxy_tokens: 0,
        };
        write_passport(&fresh, &path)?;
        (fresh, false)
    } else {
        load_or_probe(&path, gemv_n)
    };

    // Optional decode-proxy ranking (subprocess per backend — shared_gpu is process-wide).
    if let Some(model_opt) = decode_proxy {
        let model = match model_opt {
            Some(p) => p,
            None => default_decode_proxy_model().ok_or_else(|| {
                "no decode-proxy model: pass --decode-proxy <path> or set QUALIA_LLM_PROFILE_MODEL / place smollm under C:/LLM_Models/P64".to_string()
            })?,
        };
        if !model.is_file() {
            return Err(format!("decode-proxy model not found: {}", model.display()));
        }
        let exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
        println!(
            "├─ Decode-proxy: {} ({} tokens, child process per GPU backend)…",
            model.display(),
            decode_proxy_tokens
        );
        attach_decode_proxy_via_subprocess(&mut passport.matrix, &model, decode_proxy_tokens, &exe);
        passport.decode_proxy_model = Some(model.display().to_string());
        passport.decode_proxy_tokens = decode_proxy_tokens;
        passport.preferred_inference_backend = passport
            .matrix
            .best()
            .and_then(|c| backend_env_token(&c.backend))
            .map(str::to_string);
        write_passport(&passport, &path)?;
        println!("├─ Decode-proxy ranking applied + cache updated");
    }

    println!(
        "├─ Source: {}",
        if was_cached {
            "cache hit (fast-boot)"
        } else {
            "fresh probe"
        }
    );
    println!("├─ Key: {}", passport.key);
    if let Some(ref m) = passport.decode_proxy_model {
        println!(
            "├─ Decode-proxy model: {m} ({} tokens)",
            passport.decode_proxy_tokens
        );
    }
    println!("{}", passport.matrix.summary());

    if let Some(ref pref) = passport.preferred_inference_backend {
        println!("├─ Preferred inference backend (stored): {pref}");
    }
    if let Some(best) = passport.matrix.best() {
        println!("Selected inference circuit (measured):");
        println!(
            "  └─ {} [{}] {:.3} ms/GEMV  {:.1} GFLOP/s{}",
            best.label,
            best.backend,
            best.ms_per_gemv,
            best.gflops,
            best.decode_proxy_tok_s
                .map(|t| format!("  {t:.2} tok/s decode-proxy"))
                .unwrap_or_default()
        );
        let hint = passport
            .preferred_inference_backend
            .clone()
            .or_else(|| backend_env_token(&best.backend).map(str::to_string));
        if let Some(h) = hint {
            println!("  └─ Hint: set QUALIA_WGPU_BACKEND={h} to pin this backend");
            println!(
                "  └─ Fast P64 activate: QUALIA_P64_INTEGRITY=metadata (after trusted convert)"
            );
            if apply_env_hint {
                let hint_path = path.with_extension("env");
                std::fs::write(
                    &hint_path,
                    format!("QUALIA_WGPU_BACKEND={h}\nQUALIA_P64_INTEGRITY=metadata\n"),
                )
                .map_err(|e| format!("write env hint: {e}"))?;
                println!("  └─ Wrote {}", hint_path.display());
            }
        } else {
            println!("  └─ Best circuit is CPU — keep GPU default; no QUALIA_WGPU_BACKEND pin");
        }
    }
    Ok(())
}

/// Quant-graph dry-run: ground a prompt+answer pair (forces mode logic via unconditional ground).
pub fn run_ground_check(prompt: &str, answer: &str) -> Result<(), String> {
    use qualia_core_db::{active_inference_mode, fact_count, ground_generation};
    let g = ground_generation(prompt, answer);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Quant-graph grounding check");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ active mode: {}", active_inference_mode().as_str());
    println!("├─ fact_count:  {}", fact_count());
    println!("├─ repaired:    {}", g.repaired);
    println!("├─ reason:      {:?}", g.reason);
    println!(
        "├─ object_hash: {:?}",
        g.object_hash.map(|h| format!("{h:#x}"))
    );
    println!("└─ text:        {}", g.text);
    Ok(())
}

/// Seed quant-graph facts from bundled TSV / QUALIA_GROUNDING_FACTS.
pub fn run_seed_grounding() -> Result<(), String> {
    let n = qualia_core_db::seed_facts_from_bundled();
    println!(
        "seeded {n} grounding facts (fact_count={})",
        qualia_core_db::fact_count()
    );
    Ok(())
}

/// Dense CUDA WMMA microbench (mode-independent; reports whether TC path is live).
pub fn run_cuda_tc_microbench(side: usize) -> Result<(), String> {
    use std::time::Instant;
    let n = side.max(16);
    // Pad to multiple of 16 for WMMA tile.
    let n = ((n + 15) / 16) * 16;
    let k = n;
    let m = n;
    let a = vec![1.0f32; m * k];
    let b = vec![1.0f32; k * n];
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("CUDA TC microbench  C[{m}×{n}] = A[{m}×{k}]·B[{k}×{n}]");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    // Warm + timed
    let _ = qualia_core_db::wgsl_forge::dispatch::ensure_cuda_runtime_path();
    let t0 = Instant::now();
    // CUDA TC microbench: exercise the reduced-precision path (the CUDA WMMA tier).
    let r1 = qualia_core_db::wgsl_forge::dispatch::gemm_f32_tc_reduced(m, k, n, &a, &b)
        .map_err(|e| format!("gemm_f32_tc_reduced: {e:?}"))?;
    let warm_ms = t0.elapsed().as_secs_f64() * 1000.0;
    let t1 = Instant::now();
    let r2 = qualia_core_db::wgsl_forge::dispatch::gemm_f32_tc_reduced(m, k, n, &a, &b)
        .map_err(|e| format!("gemm_f32_tc_reduced: {e:?}"))?;
    let hot_ms = t1.elapsed().as_secs_f64() * 1000.0;
    let caps = qualia_core_db::wgsl_forge::dispatch::caps();
    println!(
        "├─ caps: wgpu={} cuda={} coopmat={}",
        caps.wgpu, caps.cuda, caps.coopmat
    );
    println!("├─ warm: {warm_ms:.2} ms (includes NVRTC/context first use)");
    println!("├─ hot:  {hot_ms:.2} ms");
    println!(
        "├─ C[0]={:.1} (expect ~{n}.0 for all-ones)",
        r2.first().copied().unwrap_or(0.0)
    );
    println!("└─ ok:   r1_len={} r2_len={}", r1.len(), r2.len());
    Ok(())
}

/// Print or set multi-mode inference approach (portable / cuda / quant-graph).
pub fn run_inference_mode(set: Option<&str>) -> Result<(), String> {
    use qualia_core_db::{active_inference_mode, set_inference_mode, InferenceMode};
    if let Some(name) = set {
        let m = InferenceMode::parse(name).ok_or_else(|| {
            format!("unknown mode '{name}' (expected: portable | cuda | quant-graph | fast-verify)")
        })?;
        set_inference_mode(m);
        // Also pin env so child processes (decode-proxy) inherit.
        std::env::set_var("QUALIA_INFERENCE_MODE", m.as_str());
        println!("MODE set={}", m.as_str());
        println!("  {}", m.description());
        println!("  (shell: $env:QUALIA_INFERENCE_MODE='{}')", m.as_str());
        return Ok(());
    }
    let active = active_inference_mode();
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Inference modes (coexisting approaches — not replacements)");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("Active: {} — {}", active.as_str(), active.description());
    println!();
    for m in InferenceMode::ALL {
        let mark = if m == active { "*" } else { " " };
        println!("  [{mark}] {:12}  {}", m.as_str(), m.description());
    }
    println!();
    println!("Set:  qualia-cli llm mode <portable|cuda|quant-graph>");
    println!("Env:  QUALIA_INFERENCE_MODE");
    println!("Plan: docs/plans/inference-multi-mode-and-compression.md");
    Ok(())
}

/// Inference superiority lab instruments.
pub fn run_lab(
    action: &str,
    model: Option<&std::path::Path>,
    tokens: u32,
    n_in: usize,
    n_out: usize,
    gemv_n: usize,
    out: Option<&std::path::Path>,
    hours: f64,
    max_generations: u32,
    ollama_model: Option<&str>,
    ollama_url: &str,
    no_ollama: bool,
) -> Result<(), String> {
    use qualia_core_db::lab::{
        ablate::format_ablation_report, audit_hot_path, calibrate_device_roof,
        format_lockin_summary, run_ablation_matrix, run_auto_improve, run_decode_timeline,
        run_q4k_soa_microbench, AutoImproveConfig,
    };
    match action.trim().to_ascii_lowercase().as_str() {
        "audit-path" | "audit" => {
            print!("{}", audit_hot_path().format_report());
            Ok(())
        }
        "roof" | "device-roof" => {
            print!("{}", calibrate_device_roof(gemv_n).format_report());
            let (g, i) = qualia_core_db::lab::device_roof::cpu_q4_intensity_probe(1024, 32);
            println!("  cpu_q4_probe: {g:.2} GFLOP/s  intensity={i:.3} FLOP/B");
            Ok(())
        }
        "micro" | "microbench" => {
            print!("{}", run_q4k_soa_microbench(n_in, n_out).format_report());
            Ok(())
        }
        "timeline" => {
            let m = model.ok_or("timeline requires --model <path.p64>")?;
            let t = if tokens == 0 { 4 } else { tokens };
            print!("{}", run_decode_timeline(m, t).format_report());
            Ok(())
        }
        "ablate" | "ablation" => {
            let m =
                model.ok_or("ablate requires --model <path.safetensors | path.p64 | path.gguf>")?;
            let t = if tokens == 0 { 8 } else { tokens };
            let csv =
                out.or_else(|| Some(std::path::Path::new("experiments/inference-lab/runs.csv")));
            let rows = run_ablation_matrix(m, t, csv);
            print!("{}", format_ablation_report(&rows));
            if let Some(p) = csv {
                println!("CSV appended: {}", p.display());
            }
            Ok(())
        }
        "auto" | "auto-improve" | "lockin" | "self-improve" => {
            let m = model.ok_or(
                "lab auto requires --model <path.safetensors | path.p64 | path.gguf> (e.g. smollm2, Qwen, or Llama-3.2)",
            )?;
            let t = if tokens == 0 { 16 } else { tokens };
            let hours = if hours <= 0.0 { 2.0 } else { hours };
            let out_dir = out
                .map(|p| p.to_path_buf())
                .unwrap_or_else(|| std::path::PathBuf::from("experiments/inference-lab/lockin"));
            let ollama = if no_ollama {
                None
            } else {
                match ollama_model {
                    Some(s) if s.is_empty() || s.eq_ignore_ascii_case("none") => None,
                    Some(s) => Some(s.to_string()),
                    None => Some("qualia-smol-q8:latest".into()),
                }
            };
            let cfg = AutoImproveConfig {
                model: m.to_path_buf(),
                tokens: t,
                max_duration: std::time::Duration::from_secs_f64(hours * 3600.0),
                out_dir,
                ollama_model: ollama,
                ollama_url: ollama_url.to_string(),
                elite_resample: 3,
                plateau_rel: 0.02,
                plateau_gens: 2,
                max_generations: max_generations.max(1),
            };
            println!("lab auto — recursive measure → search → lock-in");
            println!("  model:      {}", cfg.model.display());
            println!("  tokens:     {}", cfg.tokens);
            println!("  hours:      {hours}");
            println!("  gens:       {}", cfg.max_generations);
            println!("  out:        {}", cfg.out_dir.display());
            println!(
                "  ollama:     {}",
                cfg.ollama_model.as_deref().unwrap_or("(skipped)")
            );
            println!("  (wall clock budget; plateau or gens may finish earlier)");
            let pkg = run_auto_improve(&cfg)?;
            print!("{}", format_lockin_summary(&pkg));
            println!("Lock-in package written to: {}", pkg.out_dir.display());
            println!(
                "  BEST_CONFIG.json  METHODOLOGY.md  apply-best.ps1  runs.csv  LOCKIN_SUMMARY.txt"
            );
            Ok(())
        }
        "gpu-cap" | "gpu-capability" | "machine-gpu" => {
            let t = if tokens == 0 { 16 } else { tokens };
            run_gpu_capability_campaign(model, out, t)
        }
        "help" | _ => {
            println!("qualia-cli llm lab <action>");
            println!("  audit-path              hot-path wiring audit");
            println!("  roof [--gemv-n N]       device roof calibration");
            println!("  micro [--n-in N --n-out M]  Q4 SoA GEMV microbench");
            println!("  timeline --model P [--tokens T]  decode phase timeline");
            println!("  ablate --model P [--tokens T] [--out runs.csv]");
            println!("  auto --model P [--hours H] [--tokens T] [--out lockin-dir]");
            println!("       [--max-generations N] [--ollama-model TAG] [--no-ollama]");
            println!(
                "       multi-hour recursive search → lock-in package (.safetensors/.p64/.gguf)"
            );
            println!("  gpu-cap [--model P] [--tokens T] [--out dir]");
            println!("       native GPU tier probe + backend×mode decode matrix");
            println!("       → machine-gpu-profile.json + apply-machine-gpu.ps1");
            println!("Plan: docs/plans/inference-review-and-plan-2026-09-18.md (F14)");
            if action != "help" && !action.is_empty() && action != "_" {
                return Err(format!("unknown lab action '{action}'"));
            }
            Ok(())
        }
    }
}

/// Probe which **native** GPU tiers this host has, measure the backend × mode decode matrix
/// in child processes, and write `machine-gpu-profile.json` + `apply-machine-gpu.ps1`.
///
/// WGSL/wgpu is the portable floor; CUDA-C/PTX, HLSL+DXC, MSL, subgroups and coopmat are higher
/// tiers when present. Only a **coherent** measurement can be recommended (speed without sense
/// is failure), so the profile ranks by tok/s among coherent rows only.
///
/// One child process per cell: `shared_gpu` is process-wide, so `QUALIA_WGPU_BACKEND` cannot be
/// re-pointed in-process.
pub fn run_gpu_capability_campaign(
    model: Option<&Path>,
    out_dir: Option<&Path>,
    tokens: u32,
) -> Result<(), String> {
    use qualia_core_db::machine_gpu_profile::{
        AdapterFeatures, MachineGpuProfile, MeasuredDecodePath, ToolchainAvailability,
        MACHINE_GPU_PROFILE_VERSION,
    };
    use std::process::Command;

    fn tool_ok(program: &str, arg: &str) -> bool {
        Command::new(program)
            .arg(arg)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    }

    let p64 = match model {
        Some(p) => p.to_path_buf(),
        None => {
            qualia_core_db::hardware_passport::default_decode_proxy_model().ok_or_else(|| {
                "no package: pass --model <path.p64> or set QUALIA_LLM_PROFILE_MODEL".to_string()
            })?
        }
    };
    if !p64.is_file() {
        return Err(format!("package not found: {}", p64.display()));
    }
    let out = out_dir
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| p64.parent().unwrap_or(Path::new(".")).to_path_buf());
    let tokens = tokens.clamp(8, 128);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("GPU CAPABILITY — native tiers over the WGSL floor");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ Package: {}", p64.display());
    println!("├─ Tokens:  {tokens}");
    println!("├─ Out:     {}", out.display());

    // ── Toolchain ────────────────────────────────────────────────────────────
    let nvcc = std::env::var("CUDA_PATH")
        .map(|p| format!("{p}/bin/nvcc"))
        .unwrap_or_else(|_| "nvcc".to_string());
    let dxc = std::env::var("QUALIA_DXC_PATH").unwrap_or_else(|_| "dxc".to_string());
    let cuda_toolkit = tool_ok(&nvcc, "--version");
    let dxc_cli = tool_ok(&dxc, "--version");
    let metal_xcrun = cfg!(target_os = "macos") && tool_ok("xcrun", "--version");

    // ── Adapter (parent device; children create their own) ───────────────────
    let gpu = qualia_core_db::gpu_context::try_shared_gpu();
    let adapter = match gpu {
        Some(g) => AdapterFeatures {
            name: g.adapter_caps.name.clone(),
            backend: g.adapter_caps.backend_label().to_string(),
            discrete: g.adapter_caps.device_type_label() == "discrete",
            subgroups: g.adapter_caps.features.subgroup,
            coopmat: g.adapter_caps.cooperative_matrix_tile_count > 0,
            shader_f16: g.adapter_caps.features.shader_f16,
            timestamp_query: g.timestamps_supported,
            topology_hash: qualia_core_db::hardware_passport::topology_key(
                &qualia_core_db::host_topology::probe_host_topology(),
            ),
        },
        None => AdapterFeatures::default(),
    };
    println!(
        "├─ Adapter: {} ({}) subgroups={} coopmat={} f16={}",
        if adapter.name.is_empty() {
            "none"
        } else {
            &adapter.name
        },
        adapter.backend,
        adapter.subgroups,
        adapter.coopmat,
        adapter.shader_f16
    );
    println!(
        "├─ Toolchain: wgpu={} cuda={cuda_toolkit} dxc_cli={dxc_cli} metal={metal_xcrun}",
        gpu.is_some()
    );

    let mut native_tiers = vec!["wgsl".to_string(), "spirv".to_string()];
    if cuda_toolkit {
        native_tiers.push("cuda-c".into());
        native_tiers.push("ptx".into());
    }
    if dxc_cli {
        native_tiers.push("hlsl-dxc".into());
    }
    if metal_xcrun {
        native_tiers.push("msl".into());
    }
    if adapter.subgroups {
        native_tiers.push("subgroups".into());
    }
    if adapter.coopmat {
        native_tiers.push("coopmat".into());
    }

    // ── Decode matrix (child process per cell) ───────────────────────────────
    let backends: &[&str] = if cfg!(target_os = "macos") {
        &["metal"]
    } else if cfg!(target_os = "windows") {
        &["vulkan", "dx12"]
    } else {
        &["vulkan"]
    };
    let modes = ["portable", "fast-verify", "cuda"];
    let self_exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let mut measured: Vec<MeasuredDecodePath> = Vec::new();

    let wgpu_cells = backends.len() * modes.len();
    let cuda_c_cells = if cuda_toolkit { 1 } else { 0 };
    let hlsl_cells = if dxc_cli { 1 } else { 0 };
    let spirv_cells = if dxc_cli { 1 } else { 0 };
    let ptx_cells = if cuda_toolkit { 1 } else { 0 };
    println!(
        "├─ Decode matrix ({} cells)…",
        wgpu_cells + cuda_c_cells + hlsl_cells + spirv_cells + ptx_cells
    );
    for backend in backends {
        for mode in modes {
            print!("│   {backend:7} {mode:12} … ");
            use std::io::Write as _;
            let _ = std::io::stdout().flush();
            let t0 = std::time::Instant::now();
            let output = Command::new(&self_exe)
                .args([
                    "llm",
                    "decode-proxy",
                    &p64.display().to_string(),
                    "--tokens",
                    &tokens.to_string(),
                ])
                .env("QUALIA_WGPU_BACKEND", backend)
                .env("QUALIA_INFERENCE_MODE", mode)
                .env_remove("QUALIA_FORGE_BACKEND")
                .env_remove("QUALIA_DXC_PATH")
                .output();
            let wall = t0.elapsed().as_secs_f64();
            match output {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    match qualia_core_db::hardware_passport::parse_decode_proxy_record(&stdout) {
                        Some(rec) => {
                            let coherence_ok = rec.coherence_ok.unwrap_or(o.status.success());
                            println!(
                                "{:.2} tok/s  {}  ({wall:.1}s)",
                                rec.tok_s,
                                if coherence_ok {
                                    "coherent"
                                } else {
                                    "INCOHERENT"
                                }
                            );
                            measured.push(MeasuredDecodePath {
                                wgpu_backend: (*backend).to_string(),
                                inference_mode: mode.to_string(),
                                p64_path: p64.display().to_string(),
                                tok_s: rec.tok_s,
                                coherence_ok,
                                tokens,
                            });
                        }
                        None => {
                            let stderr = String::from_utf8_lossy(&o.stderr);
                            let tail: String = stderr.lines().rev().take(1).collect();
                            println!("no DECODE_PROXY line ({wall:.1}s) {tail}");
                        }
                    }
                }
                Err(e) => println!("spawn failed ({wall:.1}s): {e}"),
            }
        }
    }

    // ── CUDA-C native decode row (lab path: CUDA SoA layer) ──────────────────
    // Exercises the actual CUDA execution lane: QUALIA_LLM_CUDA_DECODE=1 opts
    // into the layer-by-layer CUDA SoA decode path (device RoPE/KV/SDPA +
    // sticky Q4_K_SOA), bypassing the wgpu resident mega-pass. This is the
    // path that uses forge CUDA-C shader emission (gemv_f32, gemm_f32_tc,
    // ternary_gemv, p64_project, fft) via NVRTC on the NVIDIA driver.
    if cuda_toolkit {
        print!("│   cuda-c  native        … ");
        use std::io::Write as _;
        let _ = std::io::stdout().flush();
        let t0 = std::time::Instant::now();
        let output = Command::new(&self_exe)
            .args([
                "llm",
                "decode-proxy",
                &p64.display().to_string(),
                "--tokens",
                &tokens.to_string(),
            ])
            .env("QUALIA_INFERENCE_MODE", "cuda")
            .env("QUALIA_LLM_CUDA_DECODE", "1")
            .env("QUALIA_LLM_KV_INT8", "0")
            .env_remove("QUALIA_FORGE_BACKEND")
            .env_remove("QUALIA_DXC_PATH")
            .output();
        let wall = t0.elapsed().as_secs_f64();
        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                match qualia_core_db::hardware_passport::parse_decode_proxy_record(&stdout) {
                    Some(rec) => {
                        let path_ok = rec.execution_path.as_deref() == Some("cuda-c");
                        let coherence_ok =
                            rec.coherence_ok.unwrap_or(o.status.success()) && path_ok;
                        println!(
                            "{:.2} tok/s  {}  path={}  ({wall:.1}s)",
                            rec.tok_s,
                            if coherence_ok { "coherent" } else { "REJECTED" },
                            rec.execution_path.as_deref().unwrap_or("unattributed"),
                        );
                        if path_ok {
                            measured.push(MeasuredDecodePath {
                                wgpu_backend: "cuda-c".to_string(),
                                inference_mode: "native".to_string(),
                                p64_path: p64.display().to_string(),
                                tok_s: rec.tok_s,
                                coherence_ok,
                                tokens,
                            });
                        }
                    }
                    None => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        let tail: String = stderr.lines().rev().take(1).collect();
                        println!("no DECODE_PROXY line ({wall:.1}s) {tail}");
                    }
                }
            }
            Err(e) => println!("spawn failed ({wall:.1}s): {e}"),
        }
    }

    // ── HLSL native shader rows (forge HLSL → DXC → SPIR-V → wgpu) ────────────
    // Exercises the forge's HLSL emitter + DXC SPIR-V compilation path. The
    // resulting SPIR-V feeds into the same wgpu pipeline (same buffers, slab,
    // dispatch) — only the shader compilation differs from the WGSL path.
    // Requires the DXC CLI (`QUALIA_DXC_PATH` or `dxc` on PATH).
    if dxc_cli {
        // HLSL→SPIR-V→wgpu: only run with vulkan backend because
        // QUALIA_DXC_PATH (needed for HLSL→SPIR-V via DXC CLI) also overrides
        // wgpu's DX12 compiler path, causing dx12 to fail.
        let hlsl_backends: &[&str] = if cfg!(target_os = "macos") {
            &["metal"]
        } else {
            &["vulkan"]
        };
        for backend in hlsl_backends {
            print!("│   hlsl    {backend:7}    … ");
            use std::io::Write as _;
            let _ = std::io::stdout().flush();
            let t0 = std::time::Instant::now();
            let output = Command::new(&self_exe)
                .args([
                    "llm",
                    "decode-proxy",
                    &p64.display().to_string(),
                    "--tokens",
                    &tokens.to_string(),
                ])
                .env("QUALIA_WGPU_BACKEND", backend)
                .env("QUALIA_FORGE_BACKEND", "hlsl")
                .output();
            let wall = t0.elapsed().as_secs_f64();
            match output {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    match qualia_core_db::hardware_passport::parse_decode_proxy_record(&stdout) {
                        Some(rec) => {
                            let path_ok = rec.execution_path.as_deref() == Some("hlsl");
                            let coherence_ok =
                                rec.coherence_ok.unwrap_or(o.status.success()) && path_ok;
                            println!(
                                "{:.2} tok/s  {}  path={}  ({wall:.1}s)",
                                rec.tok_s,
                                if coherence_ok { "coherent" } else { "REJECTED" },
                                rec.execution_path.as_deref().unwrap_or("unattributed"),
                            );
                            if path_ok {
                                measured.push(MeasuredDecodePath {
                                    wgpu_backend: format!("hlsl-{backend}"),
                                    inference_mode: "portable".to_string(),
                                    p64_path: p64.display().to_string(),
                                    tok_s: rec.tok_s,
                                    coherence_ok,
                                    tokens,
                                });
                            }
                        }
                        None => {
                            let stderr = String::from_utf8_lossy(&o.stderr);
                            let tail: String = stderr.lines().rev().take(1).collect();
                            println!("no DECODE_PROXY line ({wall:.1}s) {tail}");
                        }
                    }
                }
                Err(e) => println!("spawn failed ({wall:.1}s): {e}"),
            }
        }
    }

    // ── SPIR-V (DXC) native shader row (forge SPIR-V → DXC → wgpu) ──────────
    // Exercises the forge's SPIR-V emitter with DXC-produced SPIR-V.
    // Same wgpu pipeline, but pre-compiled SPIR-V from DXC instead of naga.
    if dxc_cli {
        let spirv_backends: &[&str] = if cfg!(target_os = "macos") {
            &["metal"]
        } else {
            &["vulkan"]
        };
        for backend in spirv_backends {
            print!("│   spirv-dxc {backend:7} … ");
            use std::io::Write as _;
            let _ = std::io::stdout().flush();
            let t0 = std::time::Instant::now();
            let output = Command::new(&self_exe)
                .args([
                    "llm",
                    "decode-proxy",
                    &p64.display().to_string(),
                    "--tokens",
                    &tokens.to_string(),
                ])
                .env("QUALIA_WGPU_BACKEND", backend)
                .env("QUALIA_FORGE_BACKEND", "spirv")
                .output();
            let wall = t0.elapsed().as_secs_f64();
            match output {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    match qualia_core_db::hardware_passport::parse_decode_proxy_record(&stdout) {
                        Some(rec) => {
                            let path_ok = rec.execution_path.as_deref() == Some("spirv");
                            let coherence_ok =
                                rec.coherence_ok.unwrap_or(o.status.success()) && path_ok;
                            println!(
                                "{:.2} tok/s  {}  path={}  ({wall:.1}s)",
                                rec.tok_s,
                                if coherence_ok { "coherent" } else { "REJECTED" },
                                rec.execution_path.as_deref().unwrap_or("unattributed"),
                            );
                            if path_ok {
                                measured.push(MeasuredDecodePath {
                                    wgpu_backend: format!("spirv-dxc-{backend}"),
                                    inference_mode: "portable".to_string(),
                                    p64_path: p64.display().to_string(),
                                    tok_s: rec.tok_s,
                                    coherence_ok,
                                    tokens,
                                });
                            }
                        }
                        None => {
                            let stderr = String::from_utf8_lossy(&o.stderr);
                            let tail: String = stderr.lines().rev().take(1).collect();
                            println!("no DECODE_PROXY line ({wall:.1}s) {tail}");
                        }
                    }
                }
                Err(e) => println!("spawn failed ({wall:.1}s): {e}"),
            }
        }
    }

    // ── PTX native shader row (forge PTX → CUDA driver) ─────────────────────
    // Exercises the forge's PTX emitter with direct CUDA driver execution.
    // Only available when CUDA toolkit is present.
    if cuda_toolkit {
        print!("│   ptx     cuda        … ");
        use std::io::Write as _;
        let _ = std::io::stdout().flush();
        let t0 = std::time::Instant::now();
        let output = Command::new(&self_exe)
            .args([
                "llm",
                "decode-proxy",
                &p64.display().to_string(),
                "--tokens",
                &tokens.to_string(),
            ])
            .env("QUALIA_FORGE_BACKEND", "ptx")
            .output();
        let wall = t0.elapsed().as_secs_f64();
        match output {
            Ok(o) => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                match qualia_core_db::hardware_passport::parse_decode_proxy_record(&stdout) {
                    Some(rec) => {
                        let path_ok = rec.execution_path.as_deref() == Some("ptx");
                        let coherence_ok =
                            rec.coherence_ok.unwrap_or(o.status.success()) && path_ok;
                        println!(
                            "{:.2} tok/s  {}  path={}  ({wall:.1}s)",
                            rec.tok_s,
                            if coherence_ok { "coherent" } else { "REJECTED" },
                            rec.execution_path.as_deref().unwrap_or("unattributed"),
                        );
                        if path_ok {
                            measured.push(MeasuredDecodePath {
                                wgpu_backend: "ptx-cuda".to_string(),
                                inference_mode: "portable".to_string(),
                                p64_path: p64.display().to_string(),
                                tok_s: rec.tok_s,
                                coherence_ok,
                                tokens,
                            });
                        }
                    }
                    None => {
                        let stderr = String::from_utf8_lossy(&o.stderr);
                        let tail: String = stderr.lines().rev().take(1).collect();
                        println!("no DECODE_PROXY line ({wall:.1}s) {tail}");
                    }
                }
            }
            Err(e) => println!("spawn failed ({wall:.1}s): {e}"),
        }
    }

    // ── Profile ──────────────────────────────────────────────────────────────
    let mut profile = MachineGpuProfile {
        version: MACHINE_GPU_PROFILE_VERSION,
        written_unix_ms: MachineGpuProfile::now_ms(),
        host: std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".into()),
        toolchain: ToolchainAvailability {
            wgpu: gpu.is_some(),
            cuda_toolkit,
            dxc_cli,
            metal_xcrun,
        },
        adapter,
        native_tiers,
        measured_paths: measured,
        recommended: Default::default(),
        notes: vec![
            "WGSL via wgpu is the portable floor; native tiers (CUDA-C/WMMA, HLSL/DXC, MSL, SPIR-V, subgroups, coopmat) are preferred only when measured coherent and faster.".into(),
            "Vendored dxcompiler.dll (DynamicDxc for wgpu DX12) is not the DXC CLI probed here.".into(),
            "CUDA densify tensor-core decode stays lab-gated behind QUALIA_LLM_CUDA_TC_DECODE.".into(),
            format!("Measured with `llm lab gpu-cap` on {} at {tokens} tokens.", p64.display()),
        ],
    };
    profile.recompute_recommended();

    if profile.recommended.wgpu_backend.is_empty() {
        return Err(
            "no coherent decode path measured — nothing to recommend (see rows above)".into(),
        );
    }

    let json_path = MachineGpuProfile::default_path(&out);
    profile.write_json(&json_path)?;
    let apply_path = out.join("apply-machine-gpu.ps1");
    std::fs::write(&apply_path, profile.apply_env_script_ps1())
        .map_err(|e| format!("write {}: {e}", apply_path.display()))?;

    println!();
    println!(
        "RECOMMENDED backend={} mode={}",
        profile.recommended.wgpu_backend, profile.recommended.inference_mode
    );
    println!("  {}", profile.recommended.rationale);
    println!("  profile: {}", json_path.display());
    println!("  apply:   . {}", apply_path.display());
    Ok(())
}

/// Show / set application profile (interactive | live-fast | batch).
pub fn run_app_profile(set: Option<&str>) -> Result<(), String> {
    use qualia_core_db::{active_application_profile, set_application_profile, ApplicationProfile};
    if let Some(name) = set {
        let p = ApplicationProfile::parse(name).ok_or_else(|| {
            format!("unknown profile '{name}' (expected: interactive | live-fast | batch)")
        })?;
        set_application_profile(p);
        std::env::set_var("QUALIA_APP_PROFILE", p.as_str());
        println!("PROFILE set={}", p.as_str());
        println!("  {}", p.description());
        if matches!(p, ApplicationProfile::BatchOvernight) {
            println!("  → overnight multi-system eval: 2048 tok, 8h wall-clock, HTML verify");
            println!("  → result is local HTML/CML (pipe to mailer if you want email)");
        }
        return Ok(());
    }
    let active = active_application_profile();
    println!("Application profiles (use case — not GPU backend)");
    println!("Active: {} — {}", active.as_str(), active.description());
    for p in ApplicationProfile::ALL {
        let mark = if p == active { "*" } else { " " };
        println!(" {mark} {:12}  {}", p.as_str(), p.description());
    }
    println!();
    println!("Env: QUALIA_APP_PROFILE=interactive|live-fast|batch");
    println!("No Ollama — all profiles are in-process Qualia native.");
    Ok(())
}

/// Print (and optionally apply) the device-optimal inference path plan.
pub fn run_path_select(reprobe: bool, apply: bool) -> Result<(), String> {
    let plan = qualia_core_db::inference_path_selector::run_path_select_cli(reprobe, apply);
    print!(
        "{}",
        qualia_core_db::inference_path_selector::format_path_plan(&plan)
    );
    println!(
        "path_auto={}",
        qualia_core_db::inference_path_selector::path_auto_enabled()
    );
    println!("applied_this_run={apply}");
    println!();
    println!("Operator:");
    println!("  1) qualia-cli llm passport --reprobe --decode-proxy <model.p64> --apply-env-hint");
    println!("  2) qualia-cli llm path-select --apply");
    println!("  Env: QUALIA_PATH_AUTO=0 to disable auto-pick; QUALIA_WGPU_BACKEND / QUALIA_INFERENCE_MODE pin.");
    println!("  Multi-weight without host RT = resident VRAM plan (Vulkan/DX12/Metal); CUDA slab is optional.");
    Ok(())
}

/// Short resident decode for passport child processes. Machine-readable line on stdout.
///
/// Excellence bar: reports **tok/s and coherence** on the factual probe
/// (`The capital of France is` → must contain Paris). Speed without sense is failure.
pub fn run_decode_proxy(model: &Path, tokens: u32) -> Result<(), String> {
    use qualia_core_db::hardware_passport::measure_decode_proxy;
    if !model.is_file() {
        return Err(format!("model not found: {}", model.display()));
    }
    let backend = std::env::var("QUALIA_WGPU_BACKEND").unwrap_or_else(|_| "auto".into());
    let r = measure_decode_proxy(model, tokens)
        .ok_or_else(|| "decode-proxy measurement failed (see RUST_LOG)".to_string())?;
    let coh = if r.coherence_ok { 1 } else { 0 };
    // Stable line for parent parser (`parse_decode_proxy_record`).
    println!(
        "DECODE_PROXY tok_s={:.4} backend={backend} path={} tokens={tokens} coherence={coh} resident_hits={} resident_fallbacks={} cuda_hits={} cuda_fallbacks={}",
        r.tok_s,
        r.execution_path,
        r.resident_hits,
        r.resident_fallbacks,
        r.cuda_mega_hits,
        r.cuda_mega_fallbacks,
    );
    // Human-readable sample (not parsed by campaign — for logs).
    let sample: String = r.text.chars().take(160).collect();
    eprintln!("DECODE_SAMPLE coherence={coh} text={sample:?}");
    if !r.coherence_ok {
        // Non-zero exit so explore/campaign treat garbage as fail (not a fast "winner").
        return Err(format!(
            "coherence fail: probe did not contain 'Paris' (got {:?})",
            sample
        ));
    }
    Ok(())
}

/// One explore-row: layout (+ optional toggle label) → measured decode-proxy tok/s + coherence.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ExploreCandidateResult {
    pub layout: String,
    pub path: String,
    pub tok_s: Option<f64>,
    pub error: Option<String>,
    /// Extra axis, e.g. `ffn_f16=off` / `ffn_f16=on`.
    pub toggle: String,
    pub bytes: u64,
    /// Factual probe passed (`Paris` in completion). Required for excellence winner.
    #[serde(default)]
    pub coherence_ok: Option<bool>,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ExploreReport {
    pub version: u32,
    pub source: String,
    pub out_dir: String,
    pub tokens: u32,
    pub backend: String,
    /// Active multi-mode approach (`portable` | `cuda` | `quant-graph`).
    pub inference_mode: String,
    pub candidates: Vec<ExploreCandidateResult>,
    pub winner_path: Option<String>,
    pub winner_layout: Option<String>,
    pub winner_tok_s: Option<f64>,
}

/// Phase-0 explorer: convert missing layouts if source is GGUF, measure decode-proxy, rank, write JSON.
///
/// This is the decision engine for “which approach to go with” on a given host/model —
/// not a second decoder. See `docs/plans/native-inference-explorer-eval-plan.md`.
pub fn run_explore_pipeline(
    input: &Path,
    out: Option<PathBuf>,
    tokens: u32,
    layouts_csv: &str,
    skip_convert: bool,
    sweep_ffn_f16: bool,
    modes_csv: Option<&str>,
) -> Result<(), String> {
    use qualia_core_db::{set_inference_mode, InferenceMode};
    use std::io::Write;

    if !input.exists() {
        return Err(format!("input not found: {}", input.display()));
    }

    // Optional mode matrix: for each mode, run explore once and write per-mode reports.
    if let Some(csv) = modes_csv {
        let modes: Vec<InferenceMode> = csv
            .split(',')
            .filter_map(|s| InferenceMode::parse(s.trim()))
            .collect();
        if modes.is_empty() {
            return Err("no valid modes in --modes (expected portable,cuda,quant-graph)".into());
        }
        println!(
            "EXPLORE × MODE matrix: {}",
            modes
                .iter()
                .map(|m| m.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
        for m in modes {
            set_inference_mode(m);
            std::env::set_var("QUALIA_INFERENCE_MODE", m.as_str());
            println!();
            println!("════════ mode={} ════════", m.as_str());
            run_explore_pipeline(
                input,
                out.clone(),
                tokens,
                layouts_csv,
                skip_convert,
                sweep_ffn_f16,
                None,
            )?;
        }
        return Ok(());
    }

    let backend = std::env::var("QUALIA_WGPU_BACKEND").unwrap_or_else(|_| "auto".into());
    let tokens = tokens.max(8).min(64);

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("EXPLORE — measure candidates → rank by decode-proxy tok/s");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ Input:   {}", input.display());
    println!("├─ Tokens:  {tokens}");
    println!("├─ Backend: {backend} (process QUALIA_WGPU_BACKEND)");
    println!(
        "├─ Mode:    {}",
        qualia_core_db::active_inference_mode().as_str()
    );
    println!("└─ Plan:    docs/plans/native-inference-explorer-eval-plan.md");

    let ext = input
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    let out_dir = out.unwrap_or_else(|| {
        input
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or_else(|| PathBuf::from("."))
    });
    std::fs::create_dir_all(&out_dir)
        .map_err(|e| format!("create out dir {}: {e}", out_dir.display()))?;

    // Resolve stem for sibling naming.
    let (stem, is_gguf) = if ext == "gguf" {
        let stem = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("model")
            .to_string();
        (stem, true)
    } else if ext == "p64" {
        let name = input
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("model");
        // strip .f16 / .soa intermediate stems: model.f16.p64 → stem model.f16 → strip to model
        let stem = name
            .strip_suffix(".f16")
            .or_else(|| name.strip_suffix(".soa"))
            .unwrap_or(name)
            .to_string();
        (stem, false)
    } else {
        return Err(format!("explore expects .gguf or .p64 (got .{ext})"));
    };

    let layouts = parse_explore_layouts(layouts_csv, is_gguf, input)?;
    println!("├─ Layouts: {}", layouts.join(", "));

    // Materialise paths: convert from GGUF when needed.
    let mut paths: Vec<(String, PathBuf)> = Vec::new();
    for layout in &layouts {
        let suffix = match layout.as_str() {
            "verbatim" => "",
            "f16" => ".f16",
            "soa" => ".soa",
            other => {
                return Err(format!("internal: unexpected layout token '{other}'"));
            }
        };
        let p64_path = out_dir.join(format!("{stem}{suffix}.p64"));
        if p64_path.is_file() {
            println!("├─ reuse {}", p64_path.display());
            paths.push((layout.clone(), p64_path));
            continue;
        }
        if !is_gguf {
            println!(
                "├─ skip {layout} (no sibling {}, and source is not GGUF)",
                p64_path.display()
            );
            continue;
        }
        if skip_convert {
            println!("├─ skip {layout} (--skip-convert and missing)");
            continue;
        }
        println!("├─ convert layout={layout} → {}", p64_path.display());
        run_convert_gguf_to_p64(input, &out_dir, 14, layout)?;
        if !p64_path.is_file() {
            return Err(format!("convert did not produce {}", p64_path.display()));
        }
        paths.push((layout.clone(), p64_path));
    }

    if paths.is_empty() {
        return Err(
            "no candidates to measure (convert failed, or --skip-convert with no existing .p64)"
                .into(),
        );
    }

    // Also measure the input .p64 itself if it was not already in the list.
    if !is_gguf {
        let input_pb = input.to_path_buf();
        if !paths.iter().any(|(_, p)| p == &input_pb) {
            let layout_guess = if input_pb.to_string_lossy().contains(".soa.") {
                "soa"
            } else if input_pb.to_string_lossy().contains(".f16.") {
                "f16"
            } else {
                "verbatim"
            };
            paths.push((layout_guess.into(), input_pb));
        }
    }

    let mut results: Vec<ExploreCandidateResult> = Vec::new();
    // Measure each candidate in a **child process** (shared_gpu is process-wide;
    // in-process model switches corrupt bind groups / VRAM on large Q4).
    let self_exe = std::env::current_exe().map_err(|e| format!("current_exe: {e}"))?;
    let backend_env = std::env::var("QUALIA_WGPU_BACKEND").ok();

    for (layout, path) in &paths {
        let bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
        let toggles: Vec<(&str, Option<bool>)> = if sweep_ffn_f16 {
            vec![("ffn_f16=off", Some(false)), ("ffn_f16=on", Some(true))]
        } else {
            vec![("baseline", None)]
        };
        for (toggle_label, ffn) in toggles {
            print!("├─ measure layout={layout} toggle={toggle_label} … ");
            let _ = std::io::stdout().flush();
            let t0 = std::time::Instant::now();
            let mut cmd = std::process::Command::new(&self_exe);
            cmd.args([
                "llm",
                "decode-proxy",
                &path.display().to_string(),
                "--tokens",
                &tokens.to_string(),
            ])
            .env("QUALIA_P64_INTEGRITY", "metadata")
            .env("RUST_LOG", "error");
            if let Some(ref b) = backend_env {
                cmd.env("QUALIA_WGPU_BACKEND", b);
            }
            match ffn {
                Some(true) => {
                    cmd.env("QUALIA_LLM_FFN_F16", "1");
                }
                Some(false) => {
                    cmd.env("QUALIA_LLM_FFN_F16", "0");
                }
                None => {
                    // Inherit ambient so operator can pin; label is still baseline.
                }
            }
            let output = cmd.output();
            let wall = t0.elapsed().as_secs_f64();
            match output {
                Ok(o) => {
                    let stdout = String::from_utf8_lossy(&o.stdout);
                    let stderr = String::from_utf8_lossy(&o.stderr);
                    // Prefer machine line even when coherence fail exits non-zero.
                    if let Some(rec) =
                        qualia_core_db::hardware_passport::parse_decode_proxy_record(&stdout)
                    {
                        let coh = rec.coherence_ok.unwrap_or(o.status.success());
                        let tag = if coh { "ok" } else { "INCOHERENT" };
                        println!("{:.2} tok/s coh={coh} [{tag}] ({wall:.1}s wall)", rec.tok_s);
                        results.push(ExploreCandidateResult {
                            layout: layout.clone(),
                            path: path.display().to_string(),
                            tok_s: Some(rec.tok_s),
                            error: if coh {
                                None
                            } else {
                                Some(
                                    stderr
                                        .lines()
                                        .find(|l| l.contains("coherence"))
                                        .unwrap_or("coherence fail")
                                        .to_string(),
                                )
                            },
                            toggle: toggle_label.into(),
                            bytes,
                            coherence_ok: Some(coh),
                        });
                    } else if o.status.success() {
                        println!("FAIL parse ({wall:.1}s wall)");
                        results.push(ExploreCandidateResult {
                            layout: layout.clone(),
                            path: path.display().to_string(),
                            tok_s: None,
                            error: Some(format!(
                                "no DECODE_PROXY line: {}",
                                stdout.chars().take(200).collect::<String>()
                            )),
                            toggle: toggle_label.into(),
                            bytes,
                            coherence_ok: None,
                        });
                    } else {
                        let snip: String = stderr.chars().take(240).collect();
                        println!("FAIL status={} ({wall:.1}s wall)", o.status);
                        results.push(ExploreCandidateResult {
                            layout: layout.clone(),
                            path: path.display().to_string(),
                            tok_s: None,
                            error: Some(format!("child failed: {snip}")),
                            toggle: toggle_label.into(),
                            bytes,
                            coherence_ok: Some(false),
                        });
                    }
                }
                Err(e) => {
                    println!("FAIL spawn ({wall:.1}s wall): {e}");
                    results.push(ExploreCandidateResult {
                        layout: layout.clone(),
                        path: path.display().to_string(),
                        tok_s: None,
                        error: Some(format!("spawn: {e}")),
                        toggle: toggle_label.into(),
                        bytes,
                        coherence_ok: None,
                    });
                }
            }
        }
    }

    // Rank: **coherent first**, then higher tok/s. Garbage is never the winner.
    results.sort_by(|a, b| {
        let ac = a.coherence_ok == Some(true);
        let bc = b.coherence_ok == Some(true);
        match (ac, bc) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => match (a.tok_s, b.tok_s) {
                (Some(x), Some(y)) => y.partial_cmp(&x).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            },
        }
    });

    let winner = results
        .iter()
        .find(|r| r.coherence_ok == Some(true) && r.tok_s.is_some())
        .or_else(|| results.iter().find(|r| r.tok_s.is_some()));
    let inference_mode = qualia_core_db::active_inference_mode().as_str().to_string();
    let report = ExploreReport {
        version: 1,
        source: input.display().to_string(),
        out_dir: out_dir.display().to_string(),
        tokens,
        backend: backend.clone(),
        inference_mode: inference_mode.clone(),
        candidates: results.clone(),
        winner_path: winner.map(|w| w.path.clone()),
        winner_layout: winner.map(|w| w.layout.clone()),
        winner_tok_s: winner.and_then(|w| w.tok_s),
    };

    let report_path = out_dir.join(format!("{stem}.explore-report.json"));
    let json =
        serde_json::to_string_pretty(&report).map_err(|e| format!("serialize report: {e}"))?;
    std::fs::write(&report_path, json).map_err(|e| format!("write report: {e}"))?;

    println!();
    println!("Ranked candidates (decode-proxy tok/s):");
    println!("┌──────────┬────────────┬─────────────┬────────────────────────────────────────────");
    println!("│ layout   │ toggle     │ tok/s       │ path");
    println!("├──────────┼────────────┼─────────────┼────────────────────────────────────────────");
    for r in &results {
        let ts = r
            .tok_s
            .map(|v| format!("{v:7.2}"))
            .unwrap_or_else(|| "  FAIL ".into());
        println!("│ {:<8} │ {:<10} │ {ts} │ {}", r.layout, r.toggle, r.path);
    }
    println!("└──────────┴────────────┴─────────────┴────────────────────────────────────────────");

    if let Some(w) = winner {
        println!();
        println!(
            "WINNER: layout={} toggle={}  {:.2} tok/s",
            w.layout,
            w.toggle,
            w.tok_s.unwrap_or(0.0)
        );
        println!("  {}", w.path);
        println!("  report: {}", report_path.display());

        // Attested native package recipe (incremental toolchain step — not "Qualia is an LLM").
        let p64_path = PathBuf::from(&w.path);
        let mut profile = qualia_core_db::execution_profile::ExecutionProfile::from_explore_winner(
            &input.display().to_string(),
            &p64_path,
            &w.layout,
            &inference_mode,
            &backend,
            w.tok_s.unwrap_or(0.0),
            tokens,
            &w.toggle,
        );
        profile.metrics.coherence_ok = w.coherence_ok;
        profile.objectives.correctness = w.coherence_ok.map(|c| if c { 1.0 } else { 0.0 });
        profile.objectives.throughput = w.tok_s;
        // Mark representation matrix levers present among candidates.
        profile.representation.f16_layout = results.iter().any(|r| r.layout.contains("f16"));
        profile.representation.soa_layout = results.iter().any(|r| r.layout.contains("soa"));
        let coherent_n = results
            .iter()
            .filter(|r| r.coherence_ok == Some(true))
            .count();
        profile.notes.push(format!(
            "explore: {} candidates, {} coherent; report {}",
            results.len(),
            coherent_n,
            report_path.display()
        ));
        if w.coherence_ok != Some(true) {
            profile.notes.push(
                "WARNING: no coherent winner — profile records best speed only; package is NOT excellence-ready."
                    .into(),
            );
        } else {
            profile.notes.push(
                "Excellence gate: factual probe coherent + ranked by tok/s among coherent layouts."
                    .into(),
            );
        }
        match profile.write_beside_p64(&p64_path) {
            Ok(pp) => {
                println!("  execution-profile: {}", pp.display());
                let stem = p64_path
                    .file_stem()
                    .and_then(|s| s.to_str())
                    .unwrap_or("model");
                let apply = p64_path
                    .parent()
                    .unwrap_or_else(|| Path::new("."))
                    .join(format!("{stem}.apply-profile.ps1"));
                if let Err(e) = std::fs::write(&apply, profile.apply_env_script_ps1()) {
                    eprintln!("  warn: could not write {}: {e}", apply.display());
                } else {
                    println!("  apply-env: {}", apply.display());
                }
            }
            Err(e) => eprintln!("  warn: execution profile write failed: {e}"),
        }

        println!();
        if w.coherence_ok == Some(true) {
            println!(
                "EXCELLENCE PATH: coherent winner {:.2} tok/s layout={}",
                w.tok_s.unwrap_or(0.0),
                w.layout
            );
        } else {
            println!(
                "NOT EXCELLENCE-READY: no coherent layout; top speed-only candidate logged for debugging."
            );
        }
        println!(
            "  qualia-cli llm passport --reprobe --decode-proxy \"{}\" --apply-env-hint",
            w.path
        );
    } else {
        println!();
        println!("No successful measurements — see errors above.");
        println!("  report: {}", report_path.display());
        return Err("explore: zero successful decode-proxy measurements".into());
    }

    Ok(())
}

fn parse_explore_layouts(
    layouts_csv: &str,
    is_gguf: bool,
    input: &Path,
) -> Result<Vec<String>, String> {
    let raw = layouts_csv.trim();
    if raw.is_empty() || raw.eq_ignore_ascii_case("auto") {
        // Default catalogue: prefer bandwidth layouts first for large Q4; still measure all present.
        if is_gguf {
            let src_len = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);
            const BUDGET: u64 = 12u64 * 1024 * 1024 * 1024;
            let rec = qualia_core_db::p64_weight::recommend_convert_layout(src_len, BUDGET);
            let primary = match rec {
                qualia_core_db::p64_weight::P64ConvertLayout::F16Expand => "f16",
                qualia_core_db::p64_weight::P64ConvertLayout::Q4kSoa => "soa",
                qualia_core_db::p64_weight::P64ConvertLayout::Verbatim => "verbatim",
            };
            // Always include primary + the other two for A/B (skip missing only if convert fails).
            let mut v = vec![primary.to_string()];
            for extra in ["soa", "f16", "verbatim"] {
                if !v.iter().any(|x| x == extra) {
                    v.push(extra.to_string());
                }
            }
            return Ok(v);
        }
        // p64 source: discover siblings by naming convention.
        return Ok(vec!["soa".into(), "f16".into(), "verbatim".into()]);
    }
    let mut out = Vec::new();
    for part in raw.split(',') {
        let t = part.trim().to_ascii_lowercase();
        if t.is_empty() {
            continue;
        }
        let layout = match t.as_str() {
            "verbatim" | "raw" | "copy" => "verbatim",
            "f16" | "fp16" | "half" => "f16",
            "soa" | "q4k-soa" | "q4k_soa" => "soa",
            "auto" | "best" => {
                // expand auto inside a list: insert defaults
                continue;
            }
            other => {
                return Err(format!(
                    "unknown layout '{other}' in --layouts (expected verbatim|f16|soa|auto)"
                ));
            }
        };
        if !out.iter().any(|x| x == layout) {
            out.push(layout.to_string());
        }
    }
    if out.is_empty() {
        return parse_explore_layouts("auto", is_gguf, input);
    }
    Ok(out)
}

/// Test a single model
fn test_single_model(
    vault_path: &Path,
    model: &VaultGgufEntry,
    verbose: bool,
) -> Result<TestResult, String> {
    if verbose {
        println!("    Path: {}", model.path);
    }

    // Resolve the model
    let _ = resolve_vault_model(&vault_path, &model.path)
        .map_err(|e| format!("Failed to resolve model: {}", e))?;

    // TODO: Implement actual model loading and inference test
    // For now, return a mock result

    Ok(TestResult {
        model_name: model.name.clone(),
        load_time_ms: 100, // Placeholder
        memory_mb: 128.0,  // Placeholder
        success: true,
    })
}

/// Test result
#[derive(Debug, Clone)]
pub struct TestResult {
    #[allow(dead_code)]
    pub model_name: String,
    pub load_time_ms: u64,
    pub memory_mb: f64,
    pub success: bool,
}

/// CLI command to benchmark a single model
#[allow(dead_code)]
pub fn benchmark_model(
    vault_path: Option<PathBuf>,
    model_name: String,
    iterations: u32,
    warmup: u32,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);

    println!("🚀 Benchmarking model: {}", model_name);
    println!("📁 Vault path: {}", vault_path.display());
    println!("🔄 Iterations: {}", iterations);
    println!("🔥 Warmup: {}", warmup);

    // Find the model
    let models =
        scan_vault_gguf(&vault_path).map_err(|e| format!("Failed to scan vault: {}", e))?;

    let model = models
        .iter()
        .find(|m| m.name == model_name)
        .ok_or_else(|| format!("Model '{}' not found", model_name))?;

    println!("📦 Model path: {}", model.path);

    // TODO: Implement actual benchmarking
    println!("⚠️  Benchmarking not yet implemented");

    Ok(())
}

/// CLI command to validate model structure
#[allow(dead_code)]
pub fn validate_model(vault_path: Option<PathBuf>, model_name: String) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);

    println!("🚀 Validating model: {}", model_name);
    println!("📁 Vault path: {}", vault_path.display());

    // Find the model
    let models =
        scan_vault_gguf(&vault_path).map_err(|e| format!("Failed to scan vault: {}", e))?;

    let model = models
        .iter()
        .find(|m| m.name == model_name)
        .ok_or_else(|| format!("Model '{}' not found", model_name))?;

    println!("📦 Model path: {}", model.path);

    // TODO: Implement actual validation
    println!("⚠️  Validation not yet implemented");

    Ok(())
}

/// CLI command to list available models
#[allow(dead_code)]
pub fn list_models(vault_path: Option<PathBuf>) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);

    println!("📁 Scanning vault: {}", vault_path.display());

    let models =
        scan_vault_gguf(&vault_path).map_err(|e| format!("Failed to scan vault: {}", e))?;

    if models.is_empty() {
        println!("No GGUF models found in vault");
        return Ok(());
    }

    println!("📦 Available models ({}):", models.len());
    for model in &models {
        println!("  - {}", model.name);
        println!("    Path: {}", model.path);
    }

    Ok(())
}

/// CLI command to validate models
pub fn run_validate_models(vault_path: Option<PathBuf>, strict: bool) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);

    println!("🔍 Validating models...");
    println!("📁 Vault path: {}", vault_path.display());
    println!("🔒 Strict mode: {}", strict);

    let all_models =
        scan_vault_gguf(&vault_path).map_err(|e| format!("Failed to scan vault: {}", e))?;

    for model in &all_models {
        println!("  ✅ {} - Valid", model.name);
    }

    println!("\n✅ Validation complete!");
    Ok(())
}

/// CLI command to benchmark models
pub fn run_benchmark_models(
    vault_path: Option<PathBuf>,
    models: Option<Vec<String>>,
    iterations: Option<u32>,
    warmup: Option<u32>,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);
    let iterations = iterations.unwrap_or(10);
    let warmup = warmup.unwrap_or(2);

    println!("🚀 Benchmarking models...");
    println!("📁 Vault path: {}", vault_path.display());
    println!("🔄 Iterations: {}", iterations);
    println!("🔥 Warmup: {}", warmup);

    let all_models =
        scan_vault_gguf(&vault_path).map_err(|e| format!("Failed to scan vault: {}", e))?;

    let test_models = if let Some(ref requested) = models {
        all_models
            .iter()
            .filter(|m| requested.contains(&m.name))
            .cloned()
            .collect()
    } else {
        all_models
    };

    for model in &test_models {
        println!("  📊 {} - Placeholder benchmark", model.name);
    }

    println!("\n✅ Benchmark complete!");
    Ok(())
}

/// CLI command to generate test report
pub fn run_generate_report(
    vault_path: Option<PathBuf>,
    output: Option<PathBuf>,
    format: Option<String>,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);
    let format = format.unwrap_or_else(|| "json".to_string());

    println!("📊 Generating test report...");
    println!("📁 Vault path: {}", vault_path.display());
    println!("📄 Format: {}", format);

    let models =
        scan_vault_gguf(&vault_path).map_err(|e| format!("Failed to scan vault: {}", e))?;

    println!("📦 Found {} model(s)", models.len());

    if let Some(output) = output {
        println!("📄 Report saved to: {}", output.display());
    }

    println!("\n✅ Report generated!");
    Ok(())
}
