//! Exact signed plaintext-entry selection; this is not current disclosure,
//! consumer, serving or storage authority. Recheck actual owning gates at use.
mod entry;
pub use entry::{select_entry, SelectedPlainEntry, SelectionError};
#[cfg(test)]
mod tests;
