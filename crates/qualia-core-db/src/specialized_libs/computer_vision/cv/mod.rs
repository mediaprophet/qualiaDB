//! Classical vision capability library (single-function files, subdirs).

pub mod buffer;
pub mod codecs;
pub mod color;
pub mod contours;
pub mod draw;
pub mod edges;
pub mod error;
pub mod features;
pub mod filter;
pub mod flow;
pub mod hist;
pub mod morph;
pub mod photo;
pub mod sr;
pub mod transform;
pub mod video;

pub use buffer::{GrayView, RgbView};
pub use color::rgb_to_gray_u8;
pub use contours::{find_external_blobs, BlobBox};
pub use draw::draw_rect_u8;
pub use edges::{canny_u8, sobel_mag_u8};
pub use error::CvError;
pub use features::{brief_desc_u8, fast_corners_u8, hamming_match, Keypoint, Match, DESC_LEN};
pub use filter::{box_blur_u8, gaussian_blur_u8, median_blur_u8};
pub use flow::lucas_kanade_step;
pub use hist::{equalize_hist_u8, histogram_u8};
pub use morph::{dilate_u8, erode_u8};
pub use photo::bilateral_denoise_u8;
pub use sr::{bicubic_u8, bilinear_u8, lanczos3_u8};
pub use transform::{warp_affine_u8, warp_perspective_u8};
pub use video::{synthetic_pulse_sequence, FrameSequence, MAX_SEQ_FRAMES};
