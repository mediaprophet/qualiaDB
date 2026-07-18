//! Non-intrusive MOS (DNSMOS/NISQA) — NeedsWeights; fails closed, never fabricates a MOS. Re-exports only (AU-AQA).

pub mod dnsmos;
pub mod nisqa;

pub use dnsmos::dnsmos;
pub use nisqa::nisqa;
