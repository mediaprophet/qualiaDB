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
    /// Can ingest grounded conversational-DAG thread context (RequestPartKind::ChatGraphThread).
    pub chat_graph_aware: SupportLevel,
    /// Honours a reserved deliberation (`<think>`) token budget inside the context envelope.
    pub thinking_budget: SupportLevel,
    /// Sparse mixture-of-experts gating / expert dispatch execution path.
    pub moe_execution: SupportLevel,
    /// Recurrent SSM / GatedDeltaNet hybrid state execution path.
    pub hybrid_ssm: SupportLevel,
    /// Mid-decode Prolog Sentinel guard with Deny/Rollback control.
    pub sentinel_rollback: SupportLevel,
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
            chat_graph_aware: SupportLevel::Supported,
            thinking_budget: SupportLevel::Unknown,
            moe_execution: SupportLevel::Supported,
            hybrid_ssm: SupportLevel::Supported,
            sentinel_rollback: SupportLevel::Supported,
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
            chat_graph_aware: SupportLevel::Unknown,
            thinking_budget: SupportLevel::Unknown,
            moe_execution: SupportLevel::Unknown,
            hybrid_ssm: SupportLevel::Unknown,
            sentinel_rollback: SupportLevel::Unsupported,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn baselines_populate_stage_capability_dimensions() {
        let native = BackendCapabilities::native_gguf_baseline();
        assert_eq!(native.chat_graph_aware, SupportLevel::Supported);
        assert_eq!(native.sentinel_rollback, SupportLevel::Supported);
        assert_eq!(native.moe_execution, SupportLevel::Supported);
        assert_eq!(native.hybrid_ssm, SupportLevel::Supported);
        assert_eq!(native.thinking_budget, SupportLevel::Unknown);

        let mcp = BackendCapabilities::mcp_tool_baseline();
        assert_eq!(mcp.chat_graph_aware, SupportLevel::Unknown);
        assert_eq!(mcp.sentinel_rollback, SupportLevel::Unsupported);
        assert_eq!(mcp.moe_execution, SupportLevel::Unknown);
        assert_eq!(mcp.hybrid_ssm, SupportLevel::Unknown);
        assert_eq!(mcp.thinking_budget, SupportLevel::Unknown);
    }
}
