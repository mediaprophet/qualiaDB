//! Paged-KV configuration, scalar oracle, and backend kernel sources.

mod config;
pub mod kernels;
mod oracle;

pub use config::{PagedKvConfig, DEFAULT_BLOCK_SIZE, INVALID_BLOCK};
pub use oracle::{paged_gqa_attention_into, AttentionError};

#[cfg(test)]
mod tests;
