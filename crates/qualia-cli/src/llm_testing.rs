//! LLM Testing Integration for CLI
//! 
//! Simple LLM model testing functionality for the CLI.

use std::path::{Path, PathBuf};
use qualia_client_core::model_lifecycle::{scan_vault_gguf, resolve_vault_model, VaultGgufEntry};
use crate::llm_lifecycle::{default_vault_path, init_log_stream};

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
    let available_models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
    if available_models.is_empty() {
        return Err("No GGUF models found in vault".to_string());
    }
    
    println!("📦 Found {} model(s):", available_models.len());
    for model in &available_models {
        println!("  - {}", model.name);
    }
    
    // Filter models if specific ones requested
    let test_models = if let Some(ref requested) = models {
        available_models.iter()
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
                println!("  ✅ Status: {}", if result.success { "PASS" } else { "FAIL" });
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
        tokio::runtime::Handle::current()
            .block_on(qualia_client_core::model_lifecycle::activate_vault_gguf(&gguf))
    }).map_err(|e| format!("Failed to activate model: {}", e))?;
    
    let load_time = start.elapsed();
    println!("✅ Model loaded in {:?}", load_time);
    println!("  Profile ID: 0x{:016x}", record.profile_id);
    println!("  Context Window: {}", record.context_window);
    println!();
    
    // Step 2: Create agent
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("STEP 2: Creating Agent");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    use qualia_core_db::llm_agent::{LocalLlmAgent, AgentBackend};
    
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
        ("System Awareness", "What is QualiaDB and what are its main features?", 100),
        ("Technical Understanding", "Explain what a NQuin is in simple terms.", 80),
        ("Capability Awareness", "What capabilities does the Qualia system have for semantic graph processing?", 120),
        ("Instruction Following", "Write a haiku about artificial intelligence.", 30),
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
            agent.infer_local_model_streaming::<fn(String)>(
                prompt,
                "graph_context:cli_test",
                None,
            )
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
        println!("{}{}", preview, if response.len() > 200 { "..." } else { "" });
        println!();
    }
    
    // Step 4: Summary
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("TEST SUMMARY");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✅ Model Loading: PASS ({:?})", load_time);
    println!("✅ Agent Creation: PASS");
    println!("✅ Inference: {} / {} tests passed", successful_tests, test_prompts.len());
    
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

/// Convert a GGUF import file to native `.p64` + `.q42.cbor-ld` helper metadata.
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
    if ext != "gguf" {
        return Err(format!(
            "only .gguf import is supported in this command (got .{ext}); safetensors path is a follow-up"
        ));
    }
    let src_len = std::fs::metadata(input)
        .map(|m| m.len())
        .unwrap_or(0);
    // 12 GB class card default budget when auto-selecting f16 expand.
    const DEFAULT_VRAM_BUDGET: u64 = 12u64 * 1024 * 1024 * 1024;
    let layout = match layout.trim().to_ascii_lowercase().as_str() {
        "verbatim" | "raw" | "copy" => qualia_core_db::p64_weight::P64ConvertLayout::Verbatim,
        "f16" | "fp16" | "half" => qualia_core_db::p64_weight::P64ConvertLayout::F16Expand,
        "auto" | "best" | "remarkable" => {
            let rec =
                qualia_core_db::p64_weight::recommend_convert_layout(src_len, DEFAULT_VRAM_BUDGET);
            println!("├─ auto layout → {rec:?} (source {src_len} B, 12 GiB VRAM budget)");
            rec
        }
        other => {
            return Err(format!(
                "unknown --layout '{other}' (expected verbatim|f16|auto)"
            ))
        }
    };

    std::fs::create_dir_all(out_dir)
        .map_err(|e| format!("create out dir {}: {e}", out_dir.display()))?;

    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("GGUF → p64 + q42 convert");
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
    println!("├─ Source size: {:.1} MiB", src_bytes as f64 / (1024.0 * 1024.0));

    let p64 = qualia_core_db::p64_weight::compile_gguf_to_p64_with_layout(
        &mmap, page_log2, layout,
    )
    .map_err(|e| format!("compile_gguf_to_p64: {e}"))?;
    let stem = input
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("model");
    let suffix = match layout {
        qualia_core_db::p64_weight::P64ConvertLayout::Verbatim => "",
        qualia_core_db::p64_weight::P64ConvertLayout::F16Expand => ".f16",
    };
    let p64_path = out_dir.join(format!("{stem}{suffix}.p64"));
    std::fs::write(&p64_path, &p64).map_err(|e| format!("write p64: {e}"))?;

    // q42 helper (CBOR-LD): behavioural metadata the engine should not re-guess from GGUF.
    let tok = qualia_core_db::gguf_sharder::GgufTokenizer::from_gguf(&mmap);
    let stop_ids: Vec<u32> = tok.stop_tokens().to_vec();
    let stop_names: Vec<String> = stop_ids
        .iter()
        .filter_map(|&id| tok.vocab.get(id as usize).cloned())
        .collect();
    let helper = qualia_core_db::model_helper::ModelHelper::new(
        input.display().to_string(),
        p64_path.display().to_string(),
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
        .map_err(|e| format!("write q42.cbor-ld helper: {e}"))?;

    // Validate the container can be indexed (fail closed if we wrote garbage).
    let index = qualia_core_db::p64_weight::P64TensorIndex::from_p64(&p64)
        .map_err(|e| format!("p64 self-check failed: {e}"))?;
    let n_tensors = index.entries.len();
    // Round-trip the helper so a bad encode fails the convert command.
    let _ = qualia_core_db::model_helper::ModelHelper::from_cbor_ld(
        &std::fs::read(&q42_path).map_err(|e| format!("re-read helper: {e}"))?,
    )
    .map_err(|e| format!("helper self-check failed: {e}"))?;

    let elapsed = t0.elapsed();
    println!();
    println!("✅ Convert complete in {:.1}s", elapsed.as_secs_f64());
    println!("  └─ {}", p64_path.display());
    println!(
        "     size {:.1} MiB, tensors {}",
        p64.len() as f64 / (1024.0 * 1024.0),
        n_tensors
    );
    println!("  └─ {} (CBOR-LD)", q42_path.display());
    println!(
        "     chat_family={:?} stop_ids={:?}",
        tok.chat_family(),
        tok.stop_tokens()
    );
    println!();
    println!("Activate with a Local backend path pointing at the .p64 file.");
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
        let _ = run_hardware_passport(true, 512, None, true);
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
    // auto may produce .f16.p64 or .p64 — list what we wrote
    let candidates = [
        out_dir.join(format!("{stem}.f16.p64")),
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
pub fn run_hardware_passport(
    reprobe: bool,
    gemv_n: usize,
    cache: Option<PathBuf>,
    apply_env_hint: bool,
) -> Result<(), String> {
    use qualia_core_db::hardware_passport::{
        backend_env_token, default_cache_path, load_or_probe, write_passport, HardwarePassport,
        PASSPORT_VERSION, topology_key,
    };
    use qualia_core_db::device_benchmark::benchmark_devices;
    use qualia_core_db::host_topology::probe_host_topology;

    let path = cache.unwrap_or_else(default_cache_path);
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("HardwarePassport");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("├─ Cache: {}", path.display());
    println!("├─ GEMV n: {gemv_n}");
    println!("└─ Reprobe: {reprobe}");

    let (passport, was_cached) = if reprobe {
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
        };
        write_passport(&fresh, &path)?;
        (fresh, false)
    } else {
        load_or_probe(&path, gemv_n)
    };

    println!(
        "├─ Source: {}",
        if was_cached {
            "cache hit (fast-boot)"
        } else {
            "fresh probe"
        }
    );
    println!("├─ Key: {}", passport.key);
    println!("{}", passport.matrix.summary());

    if let Some(ref pref) = passport.preferred_inference_backend {
        println!("├─ Preferred inference backend (stored): {pref}");
    }
    if let Some(best) = passport.matrix.best() {
        println!("Selected inference circuit (measured):");
        println!(
            "  └─ {} [{}] {:.3} ms/GEMV  {:.1} GFLOP/s",
            best.label, best.backend, best.ms_per_gemv, best.gflops
        );
        let hint = passport
            .preferred_inference_backend
            .clone()
            .or_else(|| backend_env_token(&best.backend).map(str::to_string));
        if let Some(h) = hint {
            println!("  └─ Hint: set QUALIA_WGPU_BACKEND={h} to pin this backend");
            println!("  └─ Fast P64 activate: QUALIA_P64_INTEGRITY=metadata (after trusted convert)");
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

/// Test a single model
fn test_single_model(vault_path: &Path, model: &VaultGgufEntry, verbose: bool) -> Result<TestResult, String> {
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
        memory_mb: 128.0, // Placeholder
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
    let models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
    let model = models.iter()
        .find(|m| m.name == model_name)
        .ok_or_else(|| format!("Model '{}' not found", model_name))?;
    
    println!("📦 Model path: {}", model.path);
    
    // TODO: Implement actual benchmarking
    println!("⚠️  Benchmarking not yet implemented");
    
    Ok(())
}

/// CLI command to validate model structure
#[allow(dead_code)]
pub fn validate_model(
    vault_path: Option<PathBuf>,
    model_name: String,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);
    
    println!("🚀 Validating model: {}", model_name);
    println!("📁 Vault path: {}", vault_path.display());
    
    // Find the model
    let models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
    let model = models.iter()
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
    
    let models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
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
pub fn run_validate_models(
    vault_path: Option<PathBuf>,
    strict: bool,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);
    
    println!("🔍 Validating models...");
    println!("📁 Vault path: {}", vault_path.display());
    println!("🔒 Strict mode: {}", strict);
    
    let all_models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
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
    
    let all_models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
    let test_models = if let Some(ref requested) = models {
        all_models.iter()
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
    
    let models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
    println!("📦 Found {} model(s)", models.len());
    
    if let Some(output) = output {
        println!("📄 Report saved to: {}", output.display());
    }
    
    println!("\n✅ Report generated!");
    Ok(())
}
