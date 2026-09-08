//! Compiled contract admission. Unknown required semantics cannot become Allow.

pub mod compile;

pub use compile::{compile_decision, ContractBundle};
