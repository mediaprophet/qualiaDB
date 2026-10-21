//! `qualia-mcp` — Model Context Protocol (MCP) server library for QualiaDB.
//!
//! Provides a standards-compliant JSON-RPC 2.0 interface exposing:
//! - Semantic Graph & SPARQL queries (including codebase.q42 audit inspection)
//! - VibeScript DSL compiler, typechecker, and pretty-printer
//! - Physics, geomechanics, space weather, and fire ecology simulations
//! - Geospatial 3D frames and Hilbert curve indexing
//! - Biosensing, rPPG pulse estimation, EVM magnification, and liveness analysis
//! - 3D spatial audio HRTF positioning and EBU R128 loudness metering
//! - Logic modalities (LTL, ASP, Deontic, Epistemic, Paraconsistent)
//! - Scientific compute, CAS, ODE solvers, and computational geometry
//! - QApps host surface runtime and cooperative governance
//!
//! Designed per `AGENTS.md` rules: zero-heap in hot paths, files under 500 lines.

pub mod dispatch;
pub mod listener;
pub mod protocol;
pub mod registry;
pub mod resources;
pub mod tools;
pub mod types;

pub use listener::start_mcp_listener_with_flags;
pub use protocol::{process_jsonrpc_message, MCP_PROTOCOL_VERSION};
pub use registry::{stable_mcp_tools, tool_list_json};
pub use types::{McpIntentFrame, McpRuntimeState, McpSystemError, McpToolDescriptor, RawToolPayload};
