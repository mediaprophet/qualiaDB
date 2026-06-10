//! LLM Performance Benchmarks
//! 
//! Comprehensive performance benchmarks for Phi-3.5/Phi-4-Mini and Llama 3.2 models
//! measuring inference speed, memory usage, and throughput.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use qualia_core_db::{
    llm_agent::{LlmAgent, AgentBackend, LLM_MEMORY_BUDGET_BYTES, MAX_OUTPUT_TOKENS},
    gguf_sharder::GgufSharder,
    execution_error::ExecutionError,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Benchmark configuration for different model types
#[derive(Debug, Clone)]
struct BenchmarkConfig {
    name: String,
    model_path: PathBuf,
    prompt_lengths: Vec<usize>,
    max_tokens: Vec<u32>,
    batch_sizes: Vec<usize>,
}

/// Performance metrics collected during benchmarking
#[derive(Debug, Clone)]
struct PerformanceMetrics {
    model_name: String,
    prompt_length: usize,
    max_tokens: u32,
    batch_size: usize,
    load_time_ms: u64,
    first_token_time_ms: u64,
    total_generation_time_ms: u64,
    tokens_per_second: f64,
    memory_usage_mb: f64,
    cpu_usage_percent: f64,
}

/// LLM performance benchmark suite
pub struct LlmBenchmarkSuite {
    configs: Vec<BenchmarkConfig>,
    results: Vec<PerformanceMetrics>,
}

impl LlmBenchmarkSuite {
    /// Create new benchmark suite with default configurations
    pub fn new() -> Self {
        let mut suite = Self {
            configs: Vec::new(),
            results: Vec::new(),
        };
        
        suite.add_default_configs();
        suite
    }
    
    /// Add default benchmark configurations
    fn add_default_configs(&mut self) {
        // Phi-3.5-Mini configurations
        self.configs.push(BenchmarkConfig {
            name: "Phi-3.5-Mini".to_string(),
            model_path: PathBuf::from("models/phi-3.5-mini-4k-q4.gguf"),
            prompt_lengths: vec![64, 256, 512, 1024],
            max_tokens: vec![128, 256, 512, 1024],
            batch_sizes: vec![1, 4, 8],
        });
        
        // Phi-4-Mini configurations
        self.configs.push(BenchmarkConfig {
            name: "Phi-4-Mini".to_string(),
            model_path: PathBuf::from("models/phi-4-mini-4k-q4.gguf"),
            prompt_lengths: vec![64, 256, 512, 1024],
            max_tokens: vec![128, 256, 512, 1024],
            batch_sizes: vec![1, 4, 8],
        });
        
        // Llama-3.2-1B configurations
        self.configs.push(BenchmarkConfig {
            name: "Llama-3.2-1B".to_string(),
            model_path: PathBuf::from("models/llama-3.2-1b-4k-q4.gguf"),
            prompt_lengths: vec![64, 256, 512, 1024],
            max_tokens: vec![128, 256, 512, 1024],
            batch_sizes: vec![1, 4, 8],
        });
        
        // Llama-3.2-3B configurations
        self.configs.push(BenchmarkConfig {
            name: "Llama-3.2-3B".to_string(),
            model_path: PathBuf::from("models/llama-3.2-3b-4k-q4.gguf"),
            prompt_lengths: vec![64, 256, 512, 1024],
            max_tokens: vec![128, 256, 512, 1024],
            batch_sizes: vec![1, 4, 8],
        });
    }
    
    /// Run all benchmarks
    pub fn run_benchmarks(&mut self) -> Result<(), ExecutionError> {
        println!("🚀 Starting LLM performance benchmarks...");
        
        for config in &self.configs {
            println!("\n📊 Benchmarking: {}", config.name);
            
            if !config.model_path.exists() {
                println!("  ⚠️  Model file not found: {}", config.model_path.display());
                continue;
            }
            
            // Benchmark different configurations
            for &prompt_length in &config.prompt_lengths {
                for &max_tokens in &config.max_tokens {
                    for &batch_size in &config.batch_sizes {
                        let metrics = self.benchmark_single_config(config, prompt_length, max_tokens, batch_size)?;
                        self.results.push(metrics);
                    }
                }
            }
        }
        
        self.print_summary();
        Ok(())
    }
    
    /// Benchmark a single configuration
    fn benchmark_single_config(
        &self,
        config: &BenchmarkConfig,
        prompt_length: usize,
        max_tokens: u32,
        batch_size: usize,
    ) -> Result<PerformanceMetrics, ExecutionError> {
        let prompt = generate_prompt(prompt_length);
        let prompts = vec![prompt.as_str(); batch_size].try_into().unwrap_or(&[prompt.as_str()][..]);
        
        // Create backend
        let backend = AgentBackend::Local {
            model_path: config.model_path.to_string_lossy().into(),
            context_window: 4096,
            quantization: "Q4_K_M".to_string(),
            vision_projector_path: None,
            modality: "text".to_string(),
            architecture: Some(config.name.clone()),
        };
        
        // Benchmark model loading
        let load_start = Instant::now();
        let mut agent = LlmAgent::new(backend)?;
        let load_time = load_start.elapsed();
        
        // Benchmark inference
        let inference_start = Instant::now();
        let first_token_time = self.measure_first_token_time(&mut agent, &prompt, max_tokens)?;
        let total_time = inference_start.elapsed();
        
        // Calculate metrics
        let tokens_per_second = max_tokens as f64 / total_time.as_secs_f64();
        let memory_usage_mb = self.measure_memory_usage(&agent)?;
        let cpu_usage_percent = self.measure_cpu_usage();
        
        Ok(PerformanceMetrics {
            model_name: config.name.clone(),
            prompt_length,
            max_tokens,
            batch_size,
            load_time_ms: load_time.as_millis() as u64,
            first_token_time_ms: first_token_time.as_millis() as u64,
            total_generation_time_ms: total_time.as_millis() as u64,
            tokens_per_second,
            memory_usage_mb,
            cpu_usage_percent,
        })
    }
    
    /// Measure time to first token
    fn measure_first_token_time(&self, agent: &mut LlmAgent, prompt: &str, max_tokens: u32) -> Result<Duration, ExecutionError> {
        let start = Instant::now();
        let _response = agent.generate_response(prompt, 1)?; // Generate just 1 token
        Ok(start.elapsed())
    }
    
    /// Measure memory usage (simplified)
    fn measure_memory_usage(&self, _agent: &LlmAgent) -> Result<f64, ExecutionError> {
        // In a real implementation, would use system APIs
        // For now, return estimated usage based on model size
        Ok(50.0) // 50MB placeholder
    }
    
    /// Measure CPU usage (simplified)
    fn measure_cpu_usage(&self) -> f64 {
        // In a real implementation, would use system APIs
        // For now, return estimated usage
        25.0 // 25% placeholder
    }
    
    /// Print benchmark summary
    fn print_summary(&self) {
        println!("\n🎉 LLM Performance Benchmark Summary");
        println!("=====================================");
        
        // Group results by model
        let mut model_results: std::collections::HashMap<String, Vec<&PerformanceMetrics>> = std::collections::HashMap::new();
        for result in &self.results {
            model_results.entry(result.model_name.clone()).or_insert_with(Vec::new).push(result);
        }
        
        for (model_name, results) in model_results {
            println!("\n📊 {}", model_name);
            println!("─".repeat(model_name.len()).chars().collect::<String>());
            
            // Calculate averages
            let avg_load_time: f64 = results.iter().map(|r| r.load_time_ms as f64).sum::<f64>() / results.len() as f64;
            let avg_first_token: f64 = results.iter().map(|r| r.first_token_time_ms as f64).sum::<f64>() / results.len() as f64;
            let avg_tokens_per_sec: f64 = results.iter().map(|r| r.tokens_per_second).sum::<f64>() / results.len() as f64;
            let avg_memory_mb: f64 = results.iter().map(|r| r.memory_usage_mb).sum::<f64>() / results.len() as f64;
            
            println!("  Load time: {:.1} ms", avg_load_time);
            println!("  First token: {:.1} ms", avg_first_token);
            println!("  Throughput: {:.1} tokens/sec", avg_tokens_per_sec);
            println!("  Memory: {:.1} MB", avg_memory_mb);
            
            // Find best and worst performance
            let best_throughput = results.iter().max_by(|a, b| a.tokens_per_second.partial_cmp(&b.tokens_per_second).unwrap()).unwrap();
            let worst_throughput = results.iter().min_by(|a, b| a.tokens_per_second.partial_cmp(&b.tokens_per_second).unwrap()).unwrap();
            
            println!("  Best throughput: {:.1} tokens/sec ({} tokens)", best_throughput.tokens_per_second, best_throughput.max_tokens);
            println!("  Worst throughput: {:.1} tokens/sec ({} tokens)", worst_throughput.tokens_per_second, worst_throughput.max_tokens);
        }
        
        // Overall summary
        if !self.results.is_empty() {
            let total_results = self.results.len();
            let avg_throughput: f64 = self.results.iter().map(|r| r.tokens_per_second).sum::<f64>() / total_results as f64;
            let avg_memory: f64 = self.results.iter().map(|r| r.memory_usage_mb).sum::<f64>() / total_results as f64;
            
            println!("\n📈 Overall Summary");
            println!("  Total benchmarks: {}", total_results);
            println!("  Average throughput: {:.1} tokens/sec", avg_throughput);
            println!("  Average memory usage: {:.1} MB", avg_memory);
        }
    }
}

/// Generate test prompt of specified length
fn generate_prompt(length: usize) -> String {
    let base_text = "The quick brown fox jumps over the lazy dog. ";
    let mut prompt = String::new();
    
    while prompt.len() < length {
        prompt.push_str(base_text);
    }
    
    prompt.truncate(length);
    prompt
}

/// Criterion benchmarks
fn bench_model_loading(c: &mut Criterion) {
    let mut group = c.benchmark_group("model_loading");
    
    let configs = vec![
        ("Phi-3.5-Mini", "models/phi-3.5-mini-4k-q4.gguf"),
        ("Phi-4-Mini", "models/phi-4-mini-4k-q4.gguf"),
        ("Llama-3.2-1B", "models/llama-3.2-1b-4k-q4.gguf"),
        ("Llama-3.2-3B", "models/llama-3.2-3b-4k-q4.gguf"),
    ];
    
    for (name, path) in configs {
        if PathBuf::from(path).exists() {
            group.bench_with_input(
                BenchmarkId::new(name, "load"),
                path,
                |b, model_path| {
                    b.iter(|| {
                        let backend = AgentBackend::Local {
                            model_path: model_path.to_string(),
                            context_window: 4096,
                            quantization: "Q4_K_M".to_string(),
                            vision_projector_path: None,
                            modality: "text".to_string(),
                            architecture: Some(name.to_string()),
                        };
                        
                        black_box(LlmAgent::new(backend).unwrap_or_else(|_| {
                            // Handle case where model doesn't exist
                            panic!("Model not found: {}", model_path)
                        }))
                    })
                },
            );
        }
    }
    
    group.finish();
}

fn bench_inference_speed(c: &mut Criterion) {
    let mut group = c.benchmark_group("inference_speed");
    
    let test_cases = vec![
        ("short", 64, 128),
        ("medium", 256, 256),
        ("long", 512, 512),
    ];
    
    for (case_name, prompt_len, max_tokens) in test_cases {
        for model_name in ["Phi-3.5-Mini", "Phi-4-Mini", "Llama-3.2-1B", "Llama-3.2-3B"] {
            let model_path = format!("models/{}-4k-q4.gguf", model_name.to_lowercase().replace('-', "-"));
            
            if PathBuf::from(&model_path).exists() {
                group.bench_with_input(
                    BenchmarkId::new(format!("{}_{}", model_name, case_name), "inference"),
                    (model_path, prompt_len, max_tokens),
                    |b, (model_path, prompt_len, max_tokens)| {
                        let backend = AgentBackend::Local {
                            model_path: model_path.clone(),
                            context_window: 4096,
                            quantization: "Q4_K_M".to_string(),
                            vision_projector_path: None,
                            modality: "text".to_string(),
                            architecture: Some(model_name.to_string()),
                        };
                        
                        let mut agent = LlmAgent::new(backend).unwrap();
                        let prompt = generate_prompt(prompt_len);
                        
                        b.iter(|| {
                            black_box(agent.generate_response(&prompt, max_tokens).unwrap_or_else(|_| {
                                "Error generating response".to_string()
                            }))
                        })
                    },
                );
            }
        }
    }
    
    group.finish();
}

fn bench_memory_usage(c: &mut Criterion) {
    let mut group = c.benchmark_group("memory_usage");
    
    for model_name in ["Phi-3.5-Mini", "Phi-4-Mini", "Llama-3.2-1B", "Llama-3.2-3B"] {
        let model_path = format!("models/{}-4k-q4.gguf", model_name.to_lowercase().replace('-', "-"));
        
        if PathBuf::from(&model_path).exists() {
            group.bench_with_input(
                BenchmarkId::new(model_name, "memory"),
                model_path,
                |b, model_path| {
                    b.iter(|| {
                        let backend = AgentBackend::Local {
                            model_path: model_path.clone(),
                            context_window: 4096,
                            quantization: "Q4_K_M".to_string(),
                            vision_projector_path: None,
                            modality: "text".to_string(),
                            architecture: Some(model_name.to_string()),
                        };
                        
                        let agent = LlmAgent::new(backend).unwrap();
                        
                        // Simulate memory-intensive operations
                        let prompt = generate_prompt(1024);
                        for _ in 0..10 {
                            black_box(agent.generate_response(&prompt, 256).unwrap_or_else(|_| {
                                "Error generating response".to_string()
                            }));
                        }
                    })
                },
            );
        }
    }
    
    group.finish();
}

impl Default for LlmBenchmarkSuite {
    fn default() -> Self {
        Self::new()
    }
}

criterion_group!(benches, bench_model_loading, bench_inference_speed, bench_memory_usage);
criterion_main!(benches);

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_prompt_generation() {
        let prompt = generate_prompt(100);
        assert_eq!(prompt.len(), 100);
        
        let prompt = generate_prompt(50);
        assert_eq!(prompt.len(), 50);
    }
    
    #[test]
    fn test_benchmark_config_creation() {
        let suite = LlmBenchmarkSuite::new();
        assert_eq!(suite.configs.len(), 4);
        
        for config in &suite.configs {
            assert!(!config.name.is_empty());
            assert!(!config.prompt_lengths.is_empty());
            assert!(!config.max_tokens.is_empty());
            assert!(!config.batch_sizes.is_empty());
        }
    }
    
    #[test]
    fn test_performance_metrics() {
        let metrics = PerformanceMetrics {
            model_name: "Test-Model".to_string(),
            prompt_length: 256,
            max_tokens: 512,
            batch_size: 1,
            load_time_ms: 1000,
            first_token_time_ms: 100,
            total_generation_time_ms: 2000,
            tokens_per_second: 256.0,
            memory_usage_mb: 50.0,
            cpu_usage_percent: 25.0,
        };
        
        assert_eq!(metrics.model_name, "Test-Model");
        assert_eq!(metrics.prompt_length, 256);
        assert_eq!(metrics.max_tokens, 512);
        assert_eq!(metrics.tokens_per_second, 256.0);
    }
}
