//! Opt-in v3 metadata and original PCM-f32LE tracks. Not a Bao proof or codec graph.
mod model;
mod codec;

pub use model::*;
pub use codec::{EmfExtensionView, EXTENSION_FOOTER_BYTES, EXTENSION_FOOTER_MAGIC};
pub(super) use codec::{parse_extension, prepare_extension, validate_field_directory};

#[cfg(test)]
mod tests;

mod range;
pub(crate) use range::{parse_extension_tail, validate_directory_bytes};
