//! Thin MCP 2025-11-25 authoring protocol. Native authority and domain operations
//! remain in the supplied backend; request JSON never authenticates a principal.
//! No legacy engine-tool or synthetic Poet fallback is called by this module.

mod codec;
mod protocol;
mod stdio;
mod types;

pub use protocol::AuthoringServer;
pub use stdio::{run_stdio, serve};
pub use types::*;

#[cfg(test)]
mod tests;
