//! Untrusted retained-run material, never recovered quota/progress/authority.
//! Opening is cold bounded work under actual quota-root provenance. Host keeps
//! root/ancestor ACL, locality and namespace stability for every scoped use.
mod locator;
mod source;
mod types;
pub use source::{RangeRecoveryRead, RangeRecoverySource};
pub use types::{
    LookupLimits, LookupReport, RecoveryError, StagingLocator, MAX_ROOT_BYTES, MAX_RUN_NAME_BYTES,
};
#[cfg(test)]
pub(crate) mod tests;
