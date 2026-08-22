//! 0.1 host bindings.
//!
//! `Host` is the seam Poet (and other environments) implement.
//! [`LocalHost`] is the in-process host used by the Vibe REPL, WASM playground,
//! and tests: real catalog kernels, in-memory graph/pulse/time.

mod math;
mod quin;
mod rdf;
mod host;
mod local;
mod dispatch;

pub use math::call_math;
pub use quin::call_quin;
pub use rdf::call_rdf;
pub use host::{AccelerationTier, Host, HostEnvironment};
pub use local::LocalHost;
pub use dispatch::dispatch;

#[cfg(test)]
mod tests;
