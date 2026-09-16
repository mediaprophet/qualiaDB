//! Budget scalars.

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ConditioningBudget {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub tool_rounds: u16,
    pub max_bytes: u32,
}
