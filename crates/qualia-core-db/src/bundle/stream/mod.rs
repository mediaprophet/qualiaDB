//! Bounded QBDL streaming admission. Structural checks do not confer authority.
pub mod index;
pub mod proof;
pub mod runtime;
pub mod descriptor;
pub mod publish;
pub mod storage;
pub mod policy;

pub mod selection;

pub mod formats;

// Protected publication requires native managed-object key custody.
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod protection;
