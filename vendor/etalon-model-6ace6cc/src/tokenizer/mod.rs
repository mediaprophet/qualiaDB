//! Caller-buffered GPT-2/SmolLM tokenization over strict borrowed GGUF metadata.
//!
//! Raw encoding adds no template or BOS. Prompt encoding only applies the declared
//! BOS flag. EOS is exposed; heuristic chat stop sets are deliberately not inferred.
//! Supplied table, workspace and output capacities are charged in full, excluding
//! borrowed source/input storage. The GGUF parser has its own prior parse budget.
//! Capacity-budget preflight leaves buffers untouched. Later errors may leave
//! uncommitted partial scratch/output; callers must discard it. Success never truncates.
//! Work units conservatively charge byte scans/copies and loop/comparison steps;
//! they are deterministic bounds, not elapsed-time or processor-instruction limits.
//! Incomplete byte alphabets are allowed (including the retained SmolLM vocabulary).
//! An unresolved final byte-symbol fallback refuses instead of silently losing input.
//! Version 1 consistently uses the legacy safe scalar whitespace classification,
//! including vertical tab. It does not reproduce every runtime-selected SIMD edge.

mod alphabet;
mod budget;
mod decode;
mod encode;
mod index;
mod metadata;
mod plan;
mod pretoken;
mod types;

pub use plan::{prepare, requirements, TokenizerPlan};
pub use types::*;

/// Bind this version alongside model/profile and prompt settings in host requests.
/// Version 1 uses consistent safe scalar GPT-2/SmolLM splitting and refuses an
/// unresolved final byte-symbol fallback instead of legacy silent input loss.
/// It specifies neither chat formatting nor heuristic stop-token selection.
pub const TOKENIZER_SEMANTICS_VERSION: u32 = 1;

#[cfg(test)]
mod tests;
