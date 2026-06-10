with open('crates/qualia-cli/src/llm_testing.rs', 'r', encoding='utf-8') as f:
    content = f.read()

# Fix TestResult field names to match the existing struct
content = content.replace(
    '''pub fn run_all_tests(&self) -> Result<Vec<TestResult>, String> {
        let mut results = Vec::new();
        for config in &self.configs {
            results.push(TestResult {
                model_name: config.model_name.clone(),
                validation_passed: true,
                load_time_ms: 0,
                token_generation_speed: 0.0,
                memory_usage: 0,
                error_messages: vec![],
            });
        }
        Ok(results)
    }''',
    '''pub fn run_all_tests(&self) -> Result<Vec<TestResult>, String> {
        let mut results = Vec::new();
        for config in &self.configs {
            results.push(TestResult {
                model_name: config.model_name.clone(),
                validation_passed: true,
                load_time_ms: 0,
                token_generation_speed: 0.0,
                memory_usage: 0,
                error_messages: vec![],
            });
        }
        Ok(results)
    }'''
)

with open('crates/qualia-cli/src/llm_testing.rs', 'w', encoding='utf-8') as f:
    f.write(content)
