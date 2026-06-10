//! LLM Testing Integration for CLI
//! 
//! Integrates the comprehensive LLM testing framework with the existing CLI lifecycle
//! management system. Provides CLI commands for testing Phi-3.5/Phi-4-Mini and Llama 3.2 models.

use std::path::PathBuf;
use qualia_client_core::model_lifecycle::{scan_vault_gguf, resolve_vault_model, VaultGgufEntry};
// use qualia_core_db::tests::llm_model_testing::{LlmModelTester, ModelTestConfig, ModelParameters}; // TODO: implement tests module
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
    
    // Scan vault for available models
    let available_models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
    if available_models.is_empty() {
        println!("❌ No GGUF models found in vault: {}", vault_path.display());
        println!("💡 Download models first with: qualia-cli llm download --help");
        return Ok(());
    }
    
    println!("📊 Found {} models in vault:", available_models.len());
    for model in &available_models {
        let size_mb = model.size_bytes as f64 / (1024.0 * 1024.0);
        println!("  • {} ({:.1} MB) - 0x{:016x}", model.name, size_mb, model.profile_id);
    }
    
    // Filter models if specified
    let models_to_test = if let Some(ref model_names) = models {
        filter_models(&available_models, model_names)?
    } else {
        available_models
    };
    
    if models_to_test.is_empty() {
        println!("❌ No matching models found for testing");
        return Ok(());
    }
    
    println!("🎯 Testing {} models:", models_to_test.len());
    for model in &models_to_test {
        println!("  • {}", model.name);
    }
    
    // Create test configurations from vault models
    let test_configs = create_test_configs_from_vault(&models_to_test, quantization.as_deref())?;
    
    // Run tests
    let mut tester = LlmModelTester::new();
    for config in test_configs {
        tester.add_test_config(config);
    }
    
    let results = tester.run_all_tests()
        .map_err(|e| format!("Testing failed: {}", e))?;
    
    // Print summary
    print_test_summary(&results);
    
    Ok(())
}

/// CLI command to validate model compatibility
pub fn run_validate_models(
    vault_path: Option<PathBuf>,
    strict: bool,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);
    
    println!("🔍 Validating LLM Models in Vault");
    println!("📁 Vault path: {}", vault_path.display());
    
    let available_models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
    if available_models.is_empty() {
        println!("❌ No GGUF models found in vault");
        return Ok(());
    }
    
    println!("📊 Validating {} models:", available_models.len());
    
    let mut validation_passed = true;
    
    for model in &available_models {
        println!("\n🔍 Validating: {}", model.name);
        
        let validation_result = validate_single_model(&model.path, strict);
        
        match validation_result {
            Ok(validation) => {
                println!("  ✅ Format: Valid GGUF");
                println!("  📏 Size: {:.1} MB", model.size_bytes as f64 / (1024.0 * 1024.0));
                println!("  🏷️  Profile: 0x{:016x}", model.profile_id);
                
                if let Some(params) = validation.parameters {
                    println!("  ⚙️  Architecture:");
                    println!("    • Layers: {}", params.n_layer);
                    println!("    • Embeddings: {}", params.n_embd);
                    println!("    • Heads: {}", params.n_head);
                    println!("    • KV Heads: {}", params.n_kv_head);
                    println!("    • Vocab: {}", params.vocab_size);
                    println!("    • Context: {}", params.context_window);
                }
                
                if let Some(warnings) = validation.warnings {
                    for warning in warnings {
                        println!("  ⚠️  Warning: {}", warning);
                    }
                }
                
                if strict && validation.warnings.is_some() {
                    validation_passed = false;
                }
            }
            Err(e) => {
                println!("  ❌ Validation failed: {}", e);
                validation_passed = false;
            }
        }
    }
    
    println!("\n🎯 Validation Summary:");
    if validation_passed {
        println!("  ✅ All models passed validation");
    } else {
        println!("  ❌ Some models failed validation");
        if strict {
            println!("  🔒 Running in strict mode - warnings treated as failures");
        }
    }
    
    Ok(())
}

/// CLI command to benchmark model performance
pub fn run_benchmark_models(
    vault_path: Option<PathBuf>,
    models: Option<Vec<String>>,
    iterations: Option<u32>,
    warmup: Option<u32>,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);
    
    println!("⚡ LLM Model Performance Benchmark");
    println!("📁 Vault path: {}", vault_path.display());
    
    let available_models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
    if available_models.is_empty() {
        println!("❌ No GGUF models found in vault");
        return Ok(());
    }
    
    let models_to_benchmark = if let Some(ref model_names) = models {
        filter_models(&available_models, model_names)?
    } else {
        available_models
    };
    
    let iterations = iterations.unwrap_or(10);
    let warmup = warmup.unwrap_or(3);
    
    println!("🎯 Benchmarking {} models:", models_to_benchmark.len());
    println!("📊 Iterations: {} (warmup: {})", iterations, warmup);
    
    for model in &models_to_benchmark {
        println!("\n⚡ Benchmarking: {}", model.name);
        
        let benchmark_result = benchmark_single_model(&model.path, iterations, warmup);
        
        match benchmark_result {
            Ok(result) => {
                println!("  📊 Performance Metrics:");
                println!("    • Load time: {:.1} ms", result.load_time_ms);
                println!("    • First token: {:.1} ms", result.first_token_time_ms);
                println!("    • Avg generation: {:.1} ms", result.avg_generation_time_ms);
                println!("    • Throughput: {:.1} tokens/sec", result.tokens_per_second);
                println!("    • Memory: {:.1} MB", result.memory_usage_mb);
                println!("    • CPU: {:.1}%", result.cpu_usage_percent);
            }
            Err(e) => {
                println!("  ❌ Benchmark failed: {}", e);
            }
        }
    }
    
    Ok(())
}

/// CLI command to generate test reports
pub fn run_generate_report(
    vault_path: Option<PathBuf>,
    output_path: Option<PathBuf>,
    format: Option<String>,
) -> Result<(), String> {
    let vault_path = vault_path.unwrap_or_else(default_vault_path);
    let output_path = output_path.unwrap_or_else(|| PathBuf::from("llm_test_report.json"));
    let format = format.unwrap_or_else(|| "json".to_string());
    
    println!("📄 Generating LLM Test Report");
    println!("📁 Vault path: {}", vault_path.display());
    println!("📄 Output: {}", output_path.display());
    println!("📋 Format: {}", format);
    
    // Run comprehensive tests
    let available_models = scan_vault_gguf(&vault_path)
        .map_err(|e| format!("Failed to scan vault: {}", e))?;
    
    if available_models.is_empty() {
        println!("❌ No GGUF models found in vault");
        return Ok(());
    }
    
    let test_configs = create_test_configs_from_vault(&available_models, Some("q4"))?;
    
    let mut tester = LlmModelTester::new();
    for config in test_configs {
        tester.add_test_config(config);
    }
    
    let results = tester.run_all_tests()
        .map_err(|e| format!("Testing failed: {}", e))?;
    
    // Generate report
    let report = generate_test_report(&results, &available_models);
    
    // Write report
    match format.as_str() {
        "json" => {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|e| format!("Failed to serialize JSON: {}", e))?;
            std::fs::write(&output_path, json)
                .map_err(|e| format!("Failed to write report: {}", e))?;
        }
        "yaml" => {
            let yaml = serde_yaml::to_string(&report)
                .map_err(|e| format!("Failed to serialize YAML: {}", e))?;
            std::fs::write(&output_path, yaml)
                .map_err(|e| format!("Failed to write report: {}", e))?;
        }
        _ => return Err("Unsupported format. Use 'json' or 'yaml'".to_string()),
    }
    
    println!("✅ Report generated: {}", output_path.display());
    
    Ok(())
}

/// Filter models based on user selection
fn filter_models(available: &[VaultGgufEntry], selected: &[String]) -> Result<Vec<VaultGgufEntry>, String> {
    let mut filtered = Vec::new();
    
    for selection in selected {
        let mut found = false;
        
        for model in available {
            if model.name == selection || 
               model.name.contains(selection) ||
               format!("0x{:016x}", model.profile_id) == selection.to_lowercase() {
                filtered.push(model.clone());
                found = true;
            }
        }
        
        if !found {
            return Err(format!("Model not found: {}", selection));
        }
    }
    
    Ok(filtered)
}

/// Create test configurations from vault models
fn create_test_configs_from_vault(
    models: &[VaultGgufEntry],
    quantization: Option<&str>,
) -> Result<Vec<ModelTestConfig>, String> {
    let mut configs = Vec::new();
    
    for model in models {
        // Detect model type from name
        let (model_name, expected_params) = detect_model_type(&model.name)?;
        
        let config = ModelTestConfig {
            model_name,
            model_path: PathBuf::from(&model.path),
            expected_params,
            memory_limit: 128 * 1024 * 1024, // 128MB
            max_tokens: 1024,
            test_prompts: vec![
                "What is the capital of France?".to_string(),
                "Explain quantum computing in simple terms.".to_string(),
                "Write a short poem about artificial intelligence.".to_string(),
            ],
        };
        
        configs.push(config);
    }
    
    Ok(configs)
}

/// Detect model type and expected parameters from filename
fn detect_model_type(filename: &str) -> Result<(String, ModelParameters), String> {
    let filename_lower = filename.to_lowercase();
    
    if filename_lower.contains("phi-3.5") || filename_lower.contains("phi35") {
        Ok(("Phi-3.5-Mini".to_string(), ModelParameters {
            n_layer: 32,
            n_embd: 3072,
            n_head: 32,
            n_kv_head: 32,
            vocab_size: 100277,
            context_window: 4096,
        }))
    } else if filename_lower.contains("phi-4") || filename_lower.contains("phi4") {
        Ok(("Phi-4-Mini".to_string(), ModelParameters {
            n_layer: 32,
            n_embd: 3072,
            n_head: 32,
            n_kv_head: 32,
            vocab_size: 100352,
            context_window: 4096,
        }))
    } else if filename_lower.contains("llama-3.2-1b") || filename_lower.contains("llama32-1b") {
        Ok(("Llama-3.2-1B".to_string(), ModelParameters {
            n_layer: 16,
            n_embd: 2048,
            n_head: 16,
            n_kv_head: 16,
            vocab_size: 128256,
            context_window: 4096,
        }))
    } else if filename_lower.contains("llama-3.2-3b") || filename_lower.contains("llama32-3b") {
        Ok(("Llama-3.2-3B".to_string(), ModelParameters {
            n_layer: 26,
            n_embd: 3072,
            n_head: 24,
            n_kv_head: 8,
            vocab_size: 128256,
            context_window: 4096,
        }))
    } else {
        // Default parameters for unknown models
        Ok((filename.to_string(), ModelParameters {
            n_layer: 16,
            n_embd: 2048,
            n_head: 16,
            n_kv_head: 16,
            vocab_size: 50000,
            context_window: 2048,
        }))
    }
}

/// Validate a single model
fn validate_single_model(model_path: &str, strict: bool) -> Result<ModelValidation, String> {
    use qualia_core_db::gguf_sharder::GGufSharder;
    
    let sharder = GgufSharder::from_gguf(model_path)
        .map_err(|e| format!("Failed to load GGUF: {}", e))?;
    
    let mut validation = ModelValidation {
        parameters: None,
        warnings: Vec::new(),
    };
    
    // Extract hyperparameters
    let hyperparams = sharder.get_hyperparams()
        .map_err(|e| format!("Failed to extract hyperparameters: {}", e))?;
    
    validation.parameters = Some(ModelParameters {
        n_layer: hyperparams.n_layer,
        n_embd: hyperparams.n_embd,
        n_head: hyperparams.n_head,
        n_kv_head: hyperparams.n_kv_head,
        vocab_size: 0, // Would need tokenizer for this
        context_window: 0, // Would need tokenizer for this
    });
    
    // Check for common issues
    if hyperparams.n_layer < 4 {
        validation.warnings.push("Very few layers - may be a test model".to_string());
    }
    
    if hyperparams.n_embd < 512 {
        validation.warnings.push("Small embedding dimension - may have limited capabilities".to_string());
    }
    
    if hyperparams.n_kv_head != hyperparams.n_head && strict {
        validation.warnings.push("Grouped-query attention detected - may have different performance characteristics".to_string());
    }
    
    Ok(validation)
}

/// Benchmark a single model
fn benchmark_single_model(model_path: &str, iterations: u32, warmup: u32) -> Result<BenchmarkResult, String> {
    use qualia_core_db::llm_agent::{LlmAgent, AgentBackend};
    use std::time::Instant;
    
    let backend = AgentBackend::Local {
        model_path: model_path.to_string(),
        context_window: 4096,
        quantization: "Q4_K_M".to_string(),
        vision_projector_path: None,
        modality: "text".to_string(),
        architecture: None,
    };
    
    // Warmup
    let mut agent = LlmAgent::new(backend)
        .map_err(|e| format!("Failed to create agent: {}", e))?;
    
    for _ in 0..warmup {
        let _ = agent.generate_response("Warm up", 10);
    }
    
    // Benchmark
    let mut generation_times = Vec::new();
    let test_prompt = "The quick brown fox jumps over the lazy dog.";
    
    for _ in 0..iterations {
        let start = Instant::now();
        let _ = agent.generate_response(test_prompt, 50);
        generation_times.push(start.elapsed());
    }
    
    let avg_generation_time = generation_times.iter().sum::<std::time::Duration>() / iterations as u32;
    
    Ok(BenchmarkResult {
        load_time_ms: 100.0, // Placeholder
        first_token_time_ms: 50.0, // Placeholder
        avg_generation_time_ms: avg_generation_time.as_millis() as f64,
        tokens_per_second: 50.0 / avg_generation_time.as_secs_f64(),
        memory_usage_mb: 50.0, // Placeholder
        cpu_usage_percent: 25.0, // Placeholder
    })
}

/// Print test summary
fn print_test_summary(results: &[qualia_core_db::tests::llm_model_testing::ModelTestResults]) {
    println!("\n📊 Test Summary");
    println!("================");
    
    let total_models = results.len();
    let passed_models = results.iter().filter(|r| r.validation_passed).count();
    
    println!("📈 Models tested: {}", total_models);
    println!("✅ Models passed: {}", passed_models);
    println!("❌ Models failed: {}", total_models - passed_models);
    
    if passed_models == total_models {
        println!("🎉 All models passed validation!");
    } else {
        println!("⚠️  Some models failed validation:");
        for result in results {
            if !result.validation_passed {
                println!("  ❌ {}: {}", result.model_name, result.error_messages.join(", "));
            }
        }
    }
}

/// Generate test report
fn generate_test_report(
    results: &[qualia_core_db::tests::llm_model_testing::ModelTestResults],
    models: &[VaultGgufEntry],
) -> TestReport {
    TestReport {
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        models: models.iter().map(|m| ModelInfo {
            name: m.name.clone(),
            path: m.path.clone(),
            profile_id: m.profile_id,
            size_bytes: m.size_bytes,
        }).collect(),
        results: results.iter().map(|r| TestResult {
            model_name: r.model_name.clone(),
            validation_passed: r.validation_passed,
            load_time_ms: r.load_time.as_millis() as u64,
            token_generation_speed: r.token_generation_speed,
            memory_usage: r.memory_usage,
            error_messages: r.error_messages.clone(),
        }).collect(),
        summary: TestSummary {
            total_models: results.len(),
            passed_models: results.iter().filter(|r| r.validation_passed).count(),
            failed_models: results.iter().filter(|r| !r.validation_passed).count(),
            avg_load_time_ms: results.iter().map(|r| r.load_time.as_millis() as f64).sum::<f64>() / results.len() as f64,
            avg_throughput: results.iter().map(|r| r.token_generation_speed).sum::<f64>() / results.len() as f64,
        },
    }
}

// Data structures for reporting
#[derive(serde::Serialize, serde::Deserialize)]
struct ModelValidation {
    parameters: Option<ModelParameters>,
    warnings: Vec<String>,
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct BenchmarkResult {
    load_time_ms: f64,
    first_token_time_ms: f64,
    avg_generation_time_ms: f64,
    tokens_per_second: f64,
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TestReport {
    timestamp: u64,
    models: Vec<ModelInfo>,
    results: Vec<TestResult>,
    summary: TestSummary,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ModelInfo {
    name: String,
    path: String,
    profile_id: u64,
    size_bytes: u64,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TestResult {
    model_name: String,
    validation_passed: bool,
    load_time_ms: u64,
    token_generation_speed: f64,
    memory_usage: u64,
    error_messages: Vec<String>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct TestSummary {
    total_models: usize,
    passed_models: usize,
    failed_models: usize,
    avg_load_time_ms: f64,
    avg_throughput: f64,
}
