//! Image codecs: PNG (encode + decode) and JPEG (decode only).
//!
//! Pure-Rust, permissive-licensed backends: the `png` crate and
//! `jpeg-decoder`. All entry points return packed RGB8 and fail closed to a
//! [`crate::specialized_libs::computer_vision::cv::error::CvError`] on malformed
//! input — no panics, no `unwrap` on the decode/encode paths.

pub mod jpeg_decode;
pub mod png_decode;
pub mod png_encode;

pub use jpeg_decode::decode_jpeg;
pub use png_decode::decode_png;
pub use png_encode::encode_png;
