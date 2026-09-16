//! Endpoint/model capability evidence.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SupportLevel {
    Supported,
    Unsupported,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackendCapabilities {
    pub role_separation: SupportLevel,
    pub tools: SupportLevel,
    pub output_grammar: SupportLevel,
    pub exact_token_count: SupportLevel,
    pub exact_prefix_kv: SupportLevel,
    pub streaming: SupportLevel,
    pub cancellation: SupportLevel,
    pub usage_reporting: SupportLevel,
    pub learned_prompts: SupportLevel,
}

impl BackendCapabilities {
    pub const fn native_gguf_baseline() -> Self {
        Self {
            role_separation: SupportLevel::Supported,
            tools: SupportLevel::Unknown,
            output_grammar: SupportLevel::Supported,
            exact_token_count: SupportLevel::Supported,
            exact_prefix_kv: SupportLevel::Unknown,
            streaming: SupportLevel::Supported,
            cancellation: SupportLevel::Supported,
            usage_reporting: SupportLevel::Supported,
            learned_prompts: SupportLevel::Unsupported,
        }
    }

    pub const fn mcp_tool_baseline() -> Self {
        Self {
            role_separation: SupportLevel::Unsupported,
            tools: SupportLevel::Supported,
            output_grammar: SupportLevel::Unknown,
            exact_token_count: SupportLevel::Unknown,
            exact_prefix_kv: SupportLevel::Unsupported,
            streaming: SupportLevel::Unknown,
            cancellation: SupportLevel::Unknown,
            usage_reporting: SupportLevel::Unknown,
            learned_prompts: SupportLevel::Unsupported,
        }
    }
}
