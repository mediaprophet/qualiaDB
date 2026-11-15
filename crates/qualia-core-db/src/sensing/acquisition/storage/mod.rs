//! Approved common schemas6/7 over exact opaque-original chunks in current families.
//! No device authority, calibration execution or container-membership proof is implied.
mod model;
mod schema;
mod values;
mod context;
mod layout;
mod registry;
mod lineage;
mod read;
mod write;
mod write_metadata;
mod write_view;
#[cfg(not(target_arch = "wasm32"))]
pub mod custody;
pub use model::*;
pub use lineage::Lineage;
pub use read::admit;
pub use registry::AcquisitionRegistry;
pub use write::encode;
pub use crate::q42::field_envelope::Family;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_vectors;
#[cfg(test)]
mod tests_negative;
