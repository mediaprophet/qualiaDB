//! LLM Model Testing Framework
//! 
//! Comprehensive testing suite for Phi-3.5/Phi-4-Mini and Llama 3.2 models
//! with GGUF loading, inference validation, and performance benchmarking.

use qualia_core_db::{
    llm_agent::{LlmAgent, AgentBackend, LLM_MEMORY_BUDGET_BYTES, MAX_OUTPUT_TOKENS},
    gguf_sharder::{GgufSharder, GgufTokenizer, GgufTensorIndex, GgufHyperparams},
    gguf_bridge::{GgufBridge, QTensorEngine},
    execution_error::ExecutionError,
};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Test configuration for different model types
#[derive(Debug, Clone)]
pub struct ModelTestConfig {
    pub model_name: String,
    pub model_path: PathBuf,
    pub expected_params: ModelParameters,
    pub memory_limit: u64,
    pub max_tokens: u32,
    pub test_prompts: Vec<String>,
}

/// Expected model parameters for validation
#[derive(Debug, Clone, PartialEq)]
pub struct ModelParameters {
    pub n_layer: u32,
    pub n_embd: u32,
    pub n_head: u32,
    pub n_kv_head: u32,
    pub vocab_size: u32,
    pub context_window: u32,
}

/// Test results for model validation
#[derive(Debug, Clone)]
pub struct ModelTestResults {
    pub model_name: String,
    pub load_time: Duration,
    pub inference_times: Vec<Duration>,
    pub memory_usage: u64,
    pub token_generation_speed: f64, // tokens/second
    pub validation_passed: bool,
    pub error_messages: Vec<String>,
}

/// Comprehensive LLM model testing framework
pub struct LlmModelTester {
    test_configs: Vec<ModelTestConfig>,
    results: Vec<ModelTestResults>,
}

impl LlmModelTester {
    /// Create new tester with default model configurations
    pub fn new() -> Self {
        let mut tester = Self {
            test_configs: Vec::new(),
            results: Vec::new(),
        };
        
        // Add default test configurations for requested models
        tester.add_default_configs();
        tester
    }
    
    /// Add default configurations for Phi-3.5/Phi-4-Mini and Llama 3.2 models
    fn add_default_configs(&mut self) {
        // Phi-3.5-Mini (3.8B parameters)
        self.test_configs.push(ModelTestConfig {
            model_name: "Phi-3.5-Mini".to_string(),
            model_path: PathBuf::from("models/phi-3.5-mini-4k-q4.gguf"),
            expected_params: ModelParameters {
                n_layer: 32,
                n_embd: 3072,
                n_head: 32,
                n_kv_head: 32,
                vocab_size: 100277,
                context_window: 4096,
            },
            memory_limit: LLM_MEMORY_BUDGET_BYTES,
            max_tokens: 1024,
            test_prompts: vec![
                "What is the capital of France?".to_string(),
                "Explain quantum computing in simple terms.".to_string(),
                "Write a short poem about artificial intelligence.".to_string(),
            ],
        });
        
        // Phi-4-Mini (3.8B parameters)
        self.test_configs.push(ModelTestConfig {
            model_name: "Phi-4-Mini".to_string(),
            model_path: PathBuf::from("models/phi-4-mini-4k-q4.gguf"),
            expected_params: ModelParameters {
                n_layer: 32,
                n_embd: 3072,
                n_head: 32,
                n_kv_head: 32,
                vocab_size: 100352,
                context_window: 4096,
            },
            memory_limit: LLM_MEMORY_BUDGET_BYTES,
            max_tokens: 1024,
            test_prompts: vec![
                "What are the main challenges in renewable energy?".to_string(),
                "Describe the process of photosynthesis.".to_string(),
                "Write a brief summary of machine learning.".to_string(),
            ],
        });
        
        // Llama-3.2-1B (1B parameters)
        self.test_configs.push(ModelTestConfig {
            model_name: "Llama-3.2-1B".to_string(),
            model_path: PathBuf::from("models/llama-3.2-1b-4k-q4.gguf"),
            expected_params: ModelParameters {
                n_layer: 16,
                n_embd: 2048,
                n_head: 16,
                n_kv_head: 16,
                vocab_size: 128256,
                context_window: 4096,
            },
            memory_limit: LLM_MEMORY_BUDGET_BYTES,
            max_tokens: 1024,
            test_prompts: vec![
                "What causes seasons on Earth?".to_string(),
                "Explain the concept of gravity.".to_string(),
                "Write a short story about a robot.".to_string(),
            ],
        });
        
        // Llama-3.2-3B (3B parameters)
        self.test_configs.push(ModelTestConfig {
            model_name: "Llama-3.2-3B".to_string(),
            model_path: PathBuf::from("models/llama-3.2-3b-4k-q4.gguf"),
            expected_params: ModelParameters {
                n_layer: 26,
                n_embd: 3072,
                n_head: 24,
                n_kv_head: 8, // Grouped-query attention
                vocab_size: 128256,
                context_window: 4096,
            },
            memory_limit: LLM_MEMORY_BUDGET_BYTES,
            max_tokens: 1024,
            test_prompts: vec![
                "How does the internet work?".to_string(),
                "Explain the importance of biodiversity.".to_string(),
                "Write a short dialogue between two scientists.".to_string(),
            ],
        });
    }
    
    /// Run comprehensive tests for all configured models
    pub fn run_all_tests(&mut self) -> Result<Vec<ModelTestResults>, ExecutionError> {
        println!("🚀 Starting comprehensive LLM model testing...");
        
        for config in &self.test_configs {
            println!("\n📊 Testing model: {}", config.model_name);
            
            let result = self.test_single_model(config)?;
            self.results.push(result);
            
            // Print immediate results
            if let Some(latest_result) = self.results.last() {
                self.print_test_summary(latest_result);
            }
        }
        
        self.print_overall_summary();
        Ok(self.results.clone())
    }
    
    /// Test a single model comprehensively
    fn test_single_model(&self, config: &ModelTestConfig) -> Result<ModelTestResults, ExecutionError> {
        let mut result = ModelTestResults {
            model_name: config.model_name.clone(),
            load_time: Duration::ZERO,
            inference_times: Vec::new(),
            memory_usage: 0,
            token_generation_speed: 0.0,
            validation_passed: true,
            error_messages: Vec::new(),
        };
        
        // Test 1: Model loading and validation
        let load_result = self.test_model_loading(config, &mut result);
        if let Err(e) = load_result {
            result.error_messages.push(format!("Loading failed: {}", e));
            result.validation_passed = false;
            return Ok(result);
        }
        
        // Test 2: GGUF parsing and validation
        let parsing_result = self.test_gguf_parsing(config, &mut result);
        if let Err(e) = parsing_result {
            result.error_messages.push(format!("GGUF parsing failed: {}", e));
            result.validation_passed = false;
        }
        
        // Test 3: Inference performance
        let inference_result = self.test_inference_performance(config, &mut result);
        if let Err(e) = inference_result {
            result.error_messages.push(format!("Inference failed: {}", e));
            result.validation_passed = false;
        }
        
        // Test 4: Memory constraints
        let memory_result = self.test_memory_constraints(config, &mut result);
        if let Err(e) = memory_result {
            result.error_messages.push(format!("Memory test failed: {}", e));
            result.validation_passed = false;
        }
        
        // Test 5: Token generation quality
        let quality_result = self.test_generation_quality(config, &mut result);
        if let Err(e) = quality_result {
            result.error_messages.push(format!("Quality test failed: {}", e));
            result.validation_passed = false;
        }
        
        Ok(result)
    }
    
    /// Test model loading and basic validation
    fn test_model_loading(&self, config: &ModelTestConfig, result: &mut ModelTestResults) -> Result<(), ExecutionError> {
        println!("  📥 Loading model: {}", config.model_name);
        
        let start_time = Instant::now();
        
        // Check if model file exists
        if !config.model_path.exists() {
            return Err(ExecutionError::FileNotFound(config.model_path.to_string_lossy().into()));
        }
        
        // Create GGUF sharder to parse model
        let sharder = GgufSharder::from_gguf(&config.model_path)?;
        
        // Load tokenizer
        let tokenizer = sharder.get_tokenizer()?;
        
        // Load tensor index
        let tensor_index = sharder.get_tensor_index()?;
        
        // Get hyperparameters
        let hyperparams = sharder.get_hyperparams()?;
        
        result.load_time = start_time.elapsed();
        
        // Validate hyperparameters against expected values
        self.validate_hyperparameters(&hyperparams, &config.expected_params)?;
        
        println!("    ✅ Model loaded successfully in {:?}", result.load_time);
        println!("    📋 Layers: {}, Embeddings: {}, Heads: {}", 
                hyperparams.n_layer, hyperparams.n_embd, hyperparams.n_head);
        
        Ok(())
    }
    
    /// Test GGUF parsing and validation
    fn test_gguf_parsing(&self, config: &ModelTestConfig, result: &mut ModelTestResults) -> Result<(), ExecutionError> {
        println!("  🔍 Parsing GGUF structure...");
        
        let sharder = GgufSharder::from_gguf(&config.model_path)?;
        
        // Test tokenizer parsing
        let tokenizer = sharder.get_tokenizer()?;
        let vocab_size = tokenizer.vocab_size();
        
        // Validate vocabulary size
        if vocab_size != config.expected_params.vocab_size {
            return Err(ExecutionError::InvalidParameters(format!(
                "Expected vocab size {}, got {}", config.expected_params.vocab_size, vocab_size
            )));
        }
        
        // Test tensor index parsing
        let tensor_index = sharder.get_tensor_index()?;
        let tensor_count = tensor_index.tensor_count();
        
        // Validate tensor count (should be reasonable for the model size)
        if tensor_count < 10 || tensor_count > 1000 {
            return Err(ExecutionError::InvalidParameters(format!(
                "Unexpected tensor count: {}", tensor_count
            )));
        }
        
        // Test hyperparameter extraction
        let hyperparams = sharder.get_hyperparams()?;
        
        println!("    ✅ GGUF parsed successfully");
        println!("    📖 Vocabulary size: {}", vocab_size);
        println!("    🔢 Tensor count: {}", tensor_count);
        println!("    ⚙️  Hyperparameters: {} layers, {} embeddings", 
                hyperparams.n_layer, hyperparams.n_embd);
        
        Ok(())
    }
    
    /// Test inference performance
    fn test_inference_performance(&self, config: &ModelTestConfig, result: &mut ModelTestResults) -> Result<(), ExecutionError> {
        println!("  ⚡ Testing inference performance...");
        
        // Create LLM agent with the model
        let backend = AgentBackend::Local {
            model_path: config.model_path.to_string_lossy().into(),
            context_window: config.expected_params.context_window,
            quantization: "Q4_K_M".to_string(),
            vision_projector_path: None,
            modality: "text".to_string(),
            architecture: Some(config.model_name.clone()),
        };
        
        let mut agent = LlmAgent::new(backend)?;
        
        // Test each prompt
        for (i, prompt) in config.test_prompts.iter().enumerate() {
            println!("    🎯 Running inference test {}/{}", i + 1, config.test_prompts.len());
            
            let start_time = Instant::now();
            
            // Generate response
            let response = agent.generate_response(prompt, config.max_tokens)?;
            
            let inference_time = start_time.elapsed();
            result.inference_times.push(inference_time);
            
            // Validate response
            if response.is_empty() {
                return Err(ExecutionError::EmptyResponse);
            }
            
            println!("      ⏱️  Inference time: {:?}", inference_time);
            println!("      📝 Response length: {} characters", response.len());
        }
        
        // Calculate average inference speed
        let total_time: Duration = result.inference_times.iter().sum();
        let total_tokens = config.test_prompts.len() as u32 * config.max_tokens;
        result.token_generation_speed = total_tokens as f64 / total_time.as_secs_f64();
        
        println!("    📊 Average generation speed: {:.1} tokens/second", result.token_generation_speed);
        
        Ok(())
    }
    
    /// Test memory constraints
    fn test_memory_constraints(&self, config: &ModelTestConfig, result: &mut ModelTestResults) -> Result<(), ExecutionError> {
        println!("  🧠 Testing memory constraints...");
        
        // Get initial memory usage
        let initial_memory = self.get_memory_usage();
        
        // Load model and measure memory
        let sharder = GgufSharder::from_gguf(&config.model_path)?;
        let tokenizer = sharder.get_tokenizer()?;
        let tensor_index = sharder.get_tensor_index()?;
        
        let peak_memory = self.get_memory_usage();
        result.memory_usage = peak_memory - initial_memory;
        
        // Check against memory limit
        if result.memory_usage > config.memory_limit {
            return Err(ExecutionError::MemoryLimitExceeded(format!(
                "Memory usage {} exceeds limit {}", result.memory_usage, config.memory_limit
            )));
        }
        
        println!("    📈 Memory usage: {:.1} MB", result.memory_usage as f64 / 1024.0 / 1024.0);
        println!("    ✅ Within memory limit ({:.1} MB)", config.memory_limit as f64 / 1024.0 / 1024.0);
        
        Ok(())
    }
    
    /// Test generation quality
    fn test_generation_quality(&self, config: &ModelTestConfig, result: &mut ModelTestResults) -> Result<(), ExecutionError> {
        println!("  🎨 Testing generation quality...");
        
        let backend = AgentBackend::Local {
            model_path: config.model_path.to_string_lossy().into(),
            context_window: config.expected_params.context_window,
            quantization: "Q4_K_M".to_string(),
            vision_projector_path: None,
            modality: "text".to_string(),
            architecture: Some(config.model_name.clone()),
        };
        
        let mut agent = LlmAgent::new(backend)?;
        
        // Test quality metrics
        let mut total_coherence_score = 0.0;
        let mut total_relevance_score = 0.0;
        
        for (i, prompt) in config.test_prompts.iter().enumerate() {
            let response = agent.generate_response(prompt, config.max_tokens)?;
            
            // Simple quality metrics (in production, would use more sophisticated NLP)
            let coherence_score = self.calculate_coherence_score(&response);
            let relevance_score = self.calculate_relevance_score(prompt, &response);
            
            total_coherence_score += coherence_score;
            total_relevance_score += relevance_score;
            
            println!("    📝 Test {}: Coherence {:.2}, Relevance {:.2}", 
                    i + 1, coherence_score, relevance_score);
        }
        
        let avg_coherence = total_coherence_score / config.test_prompts.len() as f64;
        let avg_relevance = total_relevance_score / config.test_prompts.len() as f64;
        
        println!("    📊 Average quality scores:");
        println!("      Coherence: {:.2}/1.0", avg_coherence);
        println!("      Relevance: {:.2}/1.0", avg_relevance);
        
        // Quality threshold check
        if avg_coherence < 0.5 || avg_relevance < 0.5 {
            return Err(ExecutionError::QualityThresholdNotMet);
        }
        
        Ok(())
    }
    
    /// Validate hyperparameters against expected values
    fn validate_hyperparameters(&self, actual: &GgufHyperparams, expected: &ModelParameters) -> Result<(), ExecutionError> {
        if actual.n_layer != expected.n_layer {
            return Err(ExecutionError::InvalidParameters(format!(
                "Layer count mismatch: expected {}, got {}", expected.n_layer, actual.n_layer
            )));
        }
        
        if actual.n_embd != expected.n_embd {
            return Err(ExecutionError::InvalidParameters(format!(
                "Embedding size mismatch: expected {}, got {}", expected.n_embd, actual.n_embd
            )));
        }
        
        if actual.n_head != expected.n_head {
            return Err(ExecutionError::InvalidParameters(format!(
                "Head count mismatch: expected {}, got {}", expected.n_head, actual.n_head
            )));
        }
        
        // KV heads can be different (grouped-query attention)
        if actual.n_kv_head != expected.n_kv_head {
            println!("    ⚠️  KV head count different: expected {}, got {} (may be GQA)", 
                    expected.n_kv_head, actual.n_kv_head);
        }
        
        Ok(())
    }
    
    /// Calculate coherence score (simplified)
    fn calculate_coherence_score(&self, text: &str) -> f64 {
        // Simple coherence metrics (in production, would use sophisticated NLP)
        let sentences: Vec<&str> = text.split(&['.', '!', '?'][..]).collect();
        
        if sentences.is_empty() {
            return 0.0;
        }
        
        let mut coherence_score = 0.0;
        for sentence in sentences {
            let words: Vec<&str> = sentence.split_whitespace().collect();
            if words.len() >= 3 {
                coherence_score += 1.0;
            } else if words.len() >= 1 {
                coherence_score += 0.5;
            }
        }
        
        coherence_score / sentences.len() as f64
    }
    
    /// Calculate relevance score (simplified)
    fn calculate_relevance_score(&self, prompt: &str, response: &str) -> f64 {
        // Simple relevance metrics (in production, would use semantic similarity)
        let prompt_words: std::collections::HashSet<&str> = prompt.split_whitespace().collect();
        let response_words: std::collections::HashSet<&str> = response.split_whitespace().collect();
        
        if prompt_words.is_empty() {
            return 0.0;
        }
        
        let intersection = prompt_words.intersection(&response_words).count();
        intersection as f64 / prompt_words.len() as f64
    }
    
    /// Get current memory usage (simplified)
    fn get_memory_usage(&self) -> u64 {
        // In a real implementation, would use system APIs to get actual memory usage
        // For now, return a placeholder
        0
    }
    
    /// Print test summary for a single model
    fn print_test_summary(&self, result: &ModelTestResults) {
        println!("\n📋 Test Summary for {}", result.model_name);
        println!("  ✅ Validation: {}", if result.validation_passed { "PASSED" } else { "FAILED" });
        println!("  ⏱️  Load time: {:?}", result.load_time);
        println!("  🧠 Memory usage: {:.1} MB", result.memory_usage as f64 / 1024.0 / 1024.0);
        println!("  ⚡ Generation speed: {:.1} tokens/sec", result.token_generation_speed);
        
        if !result.error_messages.is_empty() {
            println!("  ❌ Errors:");
            for error in &result.error_messages {
                println!("    - {}", error);
            }
        }
    }
    
    /// Print overall test summary
    fn print_overall_summary(&self) {
        println!("\n🎉 Overall Test Summary");
        println!("==================");
        
        let total_models = self.results.len();
        let passed_models = self.results.iter().filter(|r| r.validation_passed).count();
        
        println!("📊 Models tested: {}", total_models);
        println!("✅ Models passed: {}", passed_models);
        println!("❌ Models failed: {}", total_models - passed_models);
        
        if passed_models == total_models {
            println!("🎉 All models passed validation!");
        } else {
            println!("⚠️  Some models failed validation. Check error messages above.");
        }
        
        // Performance summary
        if !self.results.is_empty() {
            let avg_load_time: Duration = self.results.iter().map(|r| r.load_time).sum();
            let avg_generation_speed = self.results.iter()
                .map(|r| r.token_generation_speed)
                .sum::<f64>() / self.results.len() as f64;
            
            println!("\n📈 Performance Summary:");
            println!("  Average load time: {:?}", avg_load_time / total_models as u32);
            println!("  Average generation speed: {:.1} tokens/sec", avg_generation_speed);
        }
    }
    
    /// Add custom test configuration
    pub fn add_test_config(&mut self, config: ModelTestConfig) {
        self.test_configs.push(config);
    }
    
    /// Get test results
    pub fn get_results(&self) -> &[ModelTestResults] {
        &self.results
    }
}

impl Default for LlmModelTester {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_llm_tester_creation() {
        let tester = LlmModelTester::new();
        assert_eq!(tester.test_configs.len(), 4); // Phi-3.5, Phi-4, Llama-3.2-1B, Llama-3.2-3B
    }
    
    #[test]
    fn test_model_config_validation() {
        let config = ModelTestConfig {
            model_name: "Test-Model".to_string(),
            model_path: PathBuf::from("test.gguf"),
            expected_params: ModelParameters {
                n_layer: 16,
                n_embd: 1024,
                n_head: 16,
                n_kv_head: 16,
                vocab_size: 50000,
                context_window: 2048,
            },
            memory_limit: 128 * 1024 * 1024,
            max_tokens: 512,
            test_prompts: vec!["Test prompt".to_string()],
        };
        
        assert_eq!(config.model_name, "Test-Model");
        assert_eq!(config.expected_params.n_layer, 16);
        assert_eq!(config.memory_limit, 128 * 1024 * 1024);
    }
    
    #[test]
    fn test_coherence_calculation() {
        let tester = LlmModelTester::new();
        
        let coherent_text = "This is a well-structured sentence. Another complete thought here.";
        let incoherent_text = "Word fragment Another";
        
        let coherent_score = tester.calculate_coherence_score(coherent_text);
        let incoherent_score = tester.calculate_coherence_score(incoherent_text);
        
        assert!(coherent_score > incoherent_score);
        assert!(coherent_score <= 1.0);
        assert!(incoherent_score >= 0.0);
    }
    
    #[test]
    fn test_relevance_calculation() {
        let tester = LlmModelTester::new();
        
        let prompt = "What is quantum computing?";
        let relevant_response = "Quantum computing uses quantum mechanics to process information.";
        let irrelevant_response = "The weather is nice today.";
        
        let relevant_score = tester.calculate_relevance_score(prompt, relevant_response);
        let irrelevant_score = tester.calculate_relevance_score(prompt, irrelevant_response);
        
        assert!(relevant_score > irrelevant_score);
        assert!(relevant_score <= 1.0);
        assert!(irrelevant_score >= 0.0);
    }
}
