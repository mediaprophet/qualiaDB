//! Classical super-resolution kernels (RGB packed u8, integer scale 2|3|4).
//!
//! No GPU, no OpenCV, no learned weights — pure Rust floor for SR0 / Track B0.

pub mod bilinear_u8;
pub mod bicubic_u8;
pub mod lanczos3_u8;
pub mod edge_directed_lite;

pub use bilinear_u8::bilinear_u8;
pub use bicubic_u8::bicubic_u8;
pub use lanczos3_u8::lanczos3_u8;
pub use edge_directed_lite::edge_directed_lite;
