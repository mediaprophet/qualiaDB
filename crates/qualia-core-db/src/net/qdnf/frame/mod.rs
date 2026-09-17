//! QFrame codec. `mod.rs` routes only.

pub mod decode;
pub mod encode;
pub mod errors;
pub mod extensions;
pub mod header;

pub use decode::{copy_payload, decode_frame};
pub use encode::encode_frame;
pub use errors::FrameError;
pub use extensions::{parse_extensions, Extension};
pub use header::{FrameHeader, BASE_HEADER_LEN, MAGIC, VERSION};
