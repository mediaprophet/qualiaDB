//! Compatibility adapter for canonical ordinary tensor execution.
#![allow(unused_imports)]
pub(super) use etalon_inference::ordinary::experts::*;

#[cfg(test)]
#[path = "moe_ffn_tests.rs"]
mod tests;
