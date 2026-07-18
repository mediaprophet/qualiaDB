//! Pitch contours / Melodia predominant melody / contour segmentation.
//! Re-exports only (AU-PITCH-2).
//!
//! - [`track_contours`]: link per-frame salience peaks into time contours.
//! - [`select_melody_contour`]: pick the predominant-melody contour (salience + smoothness).
//! - [`predominant_melody`]: per-frame predominant F0 (mono melody, abstains on non-melody frames).
//! - [`segment_contour`]: carve a contour into stable-pitch note events.

pub mod contour_segmentation;
pub mod contour_tracking;
pub mod melodia;
pub mod predominant;

pub use contour_segmentation::segment_contour;
pub use contour_tracking::track_contours;
pub use melodia::select_melody_contour;
pub use predominant::predominant_melody;
