//! Image codecs: PNG (encode + decode) and JPEG (decode only).
//!
//! Pure-Rust, permissive-licensed backends: the `png` crate and
//! `jpeg-decoder`. All entry points return packed RGB8 and fail closed to a
//! [`crate::specialized_libs::computer_vision::cv::error::CvError`] on malformed
//! input — no panics, no `unwrap` on the decode/encode paths.

#[cfg(not(target_arch = "wasm32"))]
pub mod jpeg_decode;
pub mod png_decode;
pub mod png_encode;

#[cfg(not(target_arch = "wasm32"))]
pub use jpeg_decode::decode_jpeg;
#[cfg(target_arch = "wasm32")]
pub fn decode_jpeg(
    _bytes: &[u8],
) -> Result<(Vec<u8>, u32, u32), crate::specialized_libs::computer_vision::cv::error::CvError> {
    Err(crate::specialized_libs::computer_vision::cv::error::CvError::EmptyInput)
}
pub use png_decode::decode_png;
pub use png_encode::encode_png;
