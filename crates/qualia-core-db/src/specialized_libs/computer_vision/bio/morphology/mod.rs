//! Native pure-Rust morphology / cell-detection kernels.
//!
//! Single-function files; uses `cv::morph` erode/dilate for top-hat.
//! No Python / OpenCV.

pub mod extended_minima;
pub mod morphological_tophat;
pub mod nucleus_features;
pub mod positive_od_threshold;
pub mod voronoi_otsu_label;
pub mod watershed_markers;

pub use extended_minima::extended_minima;
pub use morphological_tophat::{morphological_tophat, TopHatKind};
pub use nucleus_features::{nucleus_features, NucleusFeature, MAX_NUCLEUS_LABELS};
pub use positive_od_threshold::{
    intensity_to_od_u8, positive_od_threshold, OdPositiveIndex, MAX_OD_LABELS,
};
pub use voronoi_otsu_label::{otsu_threshold_from_hist, voronoi_otsu_label};
pub use watershed_markers::watershed_markers;
