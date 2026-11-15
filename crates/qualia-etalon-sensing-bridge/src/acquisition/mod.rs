//! Existing-authority Etalon observation -> current acquisition views.
//! Envelope/layout are retained declarations, not original serialized source bytes.
//! No device origin, calibration validity or new authorization follows from a hash.
mod mapping;
mod view;
pub use mapping::{map_layout, AcquisitionMapping};
pub use view::{
    admit_observation, AcquisitionAuthority, AcquisitionInput, BridgedAcquisition, Error,
};
#[cfg(test)]
mod tests;
