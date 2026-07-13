//! GPU-forge differential oracle: CPU references, the numeric comparator, the
//! [`GpuEvaluation`] evidence bundle, and the per-kernel evaluators that certify
//! emitted shaders against those references.
//!
//! Split by concern into submodules ([`params`], [`report`], [`reference`],
//! [`evaluate`], [`kernels`]); the full public surface is re-exported here so every
//! existing `crate::wgsl_forge::oracle::<Item>` path resolves exactly as before.

mod evaluate;
mod kernels;
mod params;
mod reference;
mod report;

pub use evaluate::*;
pub use kernels::*;
pub use params::*;
pub use reference::*;
pub use report::*;

#[cfg(test)]
mod tests;
