//! Cold CUDA decode tuning configuration.
//!
//! Candidate selection is resolved once before graph capture. The token path reads only this
//! immutable record; it never reads environment variables or rebuilds schedule state.

mod q8_config;

pub(crate) use q8_config::{cuda_q8_tuning_for_model, CudaQ8Tuning};

#[cfg(test)]
mod tests;
