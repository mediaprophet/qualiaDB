//! Exact saved-package workflow. Source grants never imply package authority.
mod policy;
mod records;
mod types;
pub use policy::{authorize_package, validate_package, PackagePermit};
pub use types::*;
#[cfg(test)]
mod tests;
