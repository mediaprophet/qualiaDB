//! Controller-consent candidate verification over native admitted associations.
//! No request or inspection receipt installs a grant or restores a session.
mod inspection;
mod verify;
pub use inspection::ControllerDelegationInspection;
pub use verify::{authoring_delegation_digest, AUTHORING_DELEGATION_CONTEXT};

#[cfg(test)]
mod tests;
