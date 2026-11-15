//! Native DICOM OF/OD values retain exact source bits; validity is separate.
mod value;
mod view;
pub use value::{FloatBits, FloatWidth};
pub use view::{FloatPixelView, FloatSample};
#[cfg(test)]
mod tests;
