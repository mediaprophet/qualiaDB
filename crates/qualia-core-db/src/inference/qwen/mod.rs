//! Qwen hybrid architecture execution and state management (Work Package F11).
//!
//! Implements GatedDeltaNet convolution, linear-attention recurrent updates,
//! and dual-state checkpointing for Qwen3.6 MoE on the RTX A2000.

pub mod checkpoint;
pub mod gated_deltanet;

pub use checkpoint::{QwenCheckpointRegistry, QwenDualCheckpoint};
pub use gated_deltanet::{
    step_causal_conv1d, step_gated_deltanet, QwenStateError, CAUSAL_CONV_KERNEL,
};
