//! Deterministic VibeScript Studio Domain capability handlers.
//!
//! Submodule routing and re-exports for office, media/CAD, and sim/lifecycle domains.
//! All handlers emit bit-exact 48-byte `NQuin` structures backed by `studio_store::STUDIO_STORE`.
//! Follows AGENTS.md §0-B (< 120 lines).

pub mod receipt;
pub mod office_domains;
pub mod media_cad_domains;
pub mod sim_lifecycle_domains;

#[allow(unused_imports)]
pub use receipt::*;
pub use office_domains::*;
pub use media_cad_domains::*;
pub use sim_lifecycle_domains::*;
