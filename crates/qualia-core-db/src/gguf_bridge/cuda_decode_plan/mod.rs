//! Cold-built CUDA decode plan.
//!
//! GGUF tensor discovery, dimension validation, raw byte-range resolution, and norm
//! dequantization happen once here. The run module only borrows prepared slices and invokes the
//! CUDA executor.

mod build;
pub(crate) mod context;
mod run;
#[cfg(test)]
mod tests;
mod types;

pub(crate) use types::CudaDecodePlanState;
