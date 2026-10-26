//! Type and effect checker implementation and compatibility exports.
#[path = "check/mod.rs"]
mod implementation;
pub use implementation::*;
pub(crate) use implementation::{check_submission, CheckEnv};
