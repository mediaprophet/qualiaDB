//! Versioned Conditioning CBOR codec (Tag-4201).

mod decode;
mod encode;

pub const CODEC_VERSION: u16 = 1;
#[allow(dead_code)]
pub const TAG_CONDITIONING: u64 = 4201;

pub use decode::decode_plan_cbor;
pub use encode::encode_plan_cbor;
