//! Radiomics feature extractors (pure Rust).
//!
//! First-order intensity statistics, Gray-Level Co-occurrence Matrix (GLCM)
//! texture features at distance 1, and lite 2D/3D shape descriptors.

pub mod first_order_stats;
pub mod glcm_features;
pub mod shape_2d;
pub mod shape_3d_lite;

pub use first_order_stats::{
    first_order_stats, first_order_stats_with_bins, FirstOrderStats, RadiomicsError,
    DEFAULT_HIST_BINS,
};
pub use glcm_features::{
    glcm_features, glcm_features_d1, GlcmFeatures, GLCM_LEVELS_16, GLCM_LEVELS_32, GLCM_MAX_LEVELS,
};
pub use shape_2d::{shape_2d_features, Shape2d};
pub use shape_3d_lite::{
    shape_2d_from_mask, shape_3d_from_voxels, Shape2dFeatures, Shape3dFeatures,
};
