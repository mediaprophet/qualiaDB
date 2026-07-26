//! Medical volume helpers (pure Rust).
//!
//! HU window/level display, max-intensity projection, isotropic nearest-neighbour
//! resample, and non-negative spectral unmixing for multi-channel fluorescence.

pub mod hu_window;
pub mod isotropic_resample_nn;
pub mod mip_project;
pub mod spectral_unmix_nnls;

pub use hu_window::apply_hu_window_i16 as hu_window_i16;
pub use hu_window::{apply_hu_window_f32, apply_hu_window_i16, MedicalError};
pub use isotropic_resample_nn::isotropic_resample_2d_nn as isotropic_resample_nn_2d;
pub use isotropic_resample_nn::{isotropic_resample_2d_nn, isotropic_resample_3d_nn};
pub use mip_project::{mip_project_axis, MipAxis};
pub use spectral_unmix_nnls::{
    spectral_unmix_nnls, spectral_unmix_per_pixel, spectral_unmix_roi_mean, UnmixResult,
};

/// MIP along Z (classic axial max-intensity projection).
pub fn mip_project_z(
    voxels: &[f32],
    width: usize,
    height: usize,
    depth: usize,
    out: &mut [f32],
) -> Result<(usize, usize), MedicalError> {
    mip_project_axis(voxels, width, height, depth, MipAxis::Z, out)
}
