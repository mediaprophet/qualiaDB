//! Versioned authoring adaptation of the shared agency and task contracts.
//! Neither serialization nor policy success verifies adoption or grants access.
mod policy;
mod types;
pub use policy::{check_delegated_operation, check_parent_attenuation, validate_delegation};
pub use types::*;

#[cfg(test)]
mod tests;
