//! Source-bound medical reconstruction (CT FBP, MRI k-space/SENSE, PET OSEM,
//! Ultrasound DAS), quantitative calibration/inference, and anatomy registration.
pub mod anatomy_reg;
pub mod fbp;
pub mod inference;
pub mod mri;
pub mod pet_osem;
pub mod spectral_fft;
pub mod ultrasound_das;

pub use anatomy_reg::{
    register_patient_to_anatomy, AnatomyRegError, AnatomyTransform,
    CrossFrameRegistrationCertificate, RegisteredAnatomyPoint,
};
pub use fbp::{
    filter_projection_fft, reconstruct_ct_fbp_into, FbpConfig, FbpError, FbpFilter,
    FbpProvenance, ProjectionGeometry, SinogramView,
};
pub use inference::{
    calibrate_and_infer_into, InferenceError, InferenceProvenance, NonVisualDomain,
    UncertaintyBudget,
};
pub use mri::{
    combine_coils_sense, combine_coils_sos, reconstruct_mri_kspace_2d,
    reconstruct_mri_kspace_3d, MriComplexVolume, MriError, MriKspaceConfig, SenseOutput,
};
pub use pet_osem::{
    forward_project_pet, reconstruct_pet_osem_into, trace_ray_into, OsemConfig, OsemError,
    OsemResult, PetSinogram,
};
pub use spectral_fft::{
    analytic_hilbert_envelope_1d, apodization_weight, fft_1d, fft_2d, fft_3d, fftshift_1d,
    fftshift_3d, ifft_1d, ifftshift_1d, SpectralError, WindowKind,
};
pub use ultrasound_das::{
    beamform_ultrasound_das_into, DasBeamformResult, DasConfig, DasError, TxFocusMode,
    UltrasoundArray,
};

#[cfg(test)]
mod tests;
