//! Canonical P64 vision writer and unchanged compatibility record.
pub use etalon_model::p64::convert::{
    transcode_vision_tensors_to_p64, RawVisionTensor, P64_ROLE_VISION_BN, P64_ROLE_VISION_CONV2D,
    P64_ROLE_VISION_FC,
};
