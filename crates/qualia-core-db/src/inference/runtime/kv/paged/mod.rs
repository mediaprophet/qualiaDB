//! Fixed-capacity paged KV planning.
//!
//! Construction may allocate bounded vectors. Append, fork, copy-on-write, release, and GPU table
//! export never grow them, so the decode and scheduling paths do not touch the global allocator.

mod attention;
mod config;
mod pool;
mod table;

pub use config::{PagedKvConfig, DEFAULT_BLOCK_SIZE, INVALID_BLOCK};
pub use pool::{BlockPool, CopyOnWrite, PoolError};
pub use table::{fill_identity_block_table, GpuBlockTablePlan, SequenceBlockTable, TableError};

#[cfg(test)]
mod tests;
pub use attention::{paged_gqa_attention_into, AttentionError};
