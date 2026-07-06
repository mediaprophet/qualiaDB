//! SHACL extensions for QualiaDB.
//!
//! Two concerns, one submodule each:
//!
//! - [`config`] — client-side configuration shapes (logging, system tray, storage /
//!   network / tax-recipient / security settings). The opcode builders here are
//!   off-hot-path (they allocate a `Vec<SlgOpcode>` at config-compile time).
//! - [`identity`] — human-centric **identity & data-rights** SHACL: identity as
//!   an enumerated multi-identifier state (never a single definitive identifier),
//!   decentralized shape-target routing (no central aggregation), real-time severity
//!   degradation for off-grid partial-subgraph utilization, and Verifiable-Credential
//!   gating of SHACL target nodes. The runtime validators here are zero-heap.
//!
//! The public surface of both is re-exported so the historical path
//! `crate::modalities::logic::shacl_extensions::*` is preserved.

pub mod config;
pub mod identity;

pub use config::*;
pub use identity::*;
