//! Budget scalars.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConditioningBudget {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub tool_rounds: u16,
    pub max_bytes: u32,
    /// Reserved deliberation (`<think>`) tokens for thinking-model decode.
    /// `None` means the model is not deliberation-capable or no reservation
    /// was requested; the value is added to `output_tokens` when checking the
    /// total context envelope.
    #[serde(default)]
    pub thinking_token_budget: Option<u32>,
}
