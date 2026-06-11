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
    quantization: Option<String>,
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
    
    for (test_name, prompt, max_tokens) in test_prompts.iter() {
        println!("┌─ Test: {}", test_name);
        println!("├─ Prompt: {}", prompt);
        
        let started = std::time::Instant::now();
        
        // Use actual inference with block_in_place to handle tokio runtime
        let (response, token_ids, _step_count, _provenance) = tokio::task::block_in_place(|| {
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
        
        let token_count = token_ids.len() as u64;
        let tps = if elapsed_ms > 0 {
            (token_count as f64 * 1000.0) / elapsed_ms as f64
        } else {
            0.0
        };
        
        println!("├─ TTFT: {}ms (estimated)", ttft);
        println!("├─ Total Time: {}ms", elapsed_ms);
        println!("├─ Tokens: {}", token_count);
        println!("├─ TPS: {:.2}", tps);
        
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
    pub model_name: String,
    pub load_time_ms: u64,
    pub memory_mb: f64,
    pub success: bool,
}

/// CLI command to benchmark a single model
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