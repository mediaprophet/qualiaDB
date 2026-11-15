//! EMF full-profile codecs, semantic admission rules, and zero-heap envelope writers
//! covering canonical modes `0x0100..=0x0107` (with compatibility mapping for `0..=7`).
//! Every measurement/model carrier preserves explicit units, calibration/evidence/provenance
//! links, frame conventions, and physical realizability checks; unsupported profiles fail closed
//! without recasting, and dose/biofield storage never emits universal safety verdicts.
use super::field_envelope::{Block, Entry, Field, StructuralView};
use super::field_semantics::{
    self as semantics, field, Context, DomainRegistry, FieldRule, Scratch,
};
use super::field_spectrum::{self as spectrum, AxisView, SigmaInterpretation};
mod acquisition_bridge;
mod bindings;
mod chunk;
mod document;
mod dose_biofield;
mod extended_writer;
mod field_metadata;
mod field_writer;
mod frame_transforms;
mod iq;
mod mueller;
mod records;
mod registry;
mod schemas;
mod spatial;
mod yee;
pub use acquisition_bridge::*;
pub use chunk::{admit_chunk, ProfileChunk};
pub use document::ProfileReader;
pub use dose_biofield::{
    absorbed_dose_gy, dipole_potential_infinite_homogeneous, encode_biofield_records,
    encode_dose_records, equivalent_dose_sv, kev_to_joules, linear_from_mass_attenuation,
    photon_uncollided_fluence, verify_empirical_join_allowed, BiofieldRecord, DoseTransportRecord,
    TransportSpecies, BIOFIELD_MODEL_URI, BIOFIELD_PROFILE, BIO_FLAG_CARRIER, BIO_FLAG_POTENTIAL,
    BIO_FLAG_UNCERTAINTY, DOSE_ALL_VALID_MASK, DOSE_MODEL_URI, DOSE_PROFILE,
    DOSE_VALID_ABSORBED_GY, DOSE_VALID_EQUIVALENT_SV, DOSE_VALID_FLUENCE, DOSE_VALID_LET,
    DOSE_VALID_LINEAR_ATTEN, DOSE_VALID_MASS_ATTEN, DOSE_VALID_PHOTON_ENERGY,
    DOSE_VALID_UNCERTAINTY,
};
pub use extended_writer::{
    write_extended_envelope, AcquisitionSpec, BioContextSpec, CalibrationSpec, ConstitutiveSpec,
    ExtendedModeSpec, MaterialSpec, ModelEvidenceSpec, SpectralAxisSpec,
};
pub use field_writer::{
    encode_field_samples, field_entry_slot, field_metadata_len, prepare_field_document,
    FieldChunkInput, FieldClock, FieldDocument, FieldFrame, FieldProvenance, FieldSample,
    FieldWriteLimits, PreparedField,
};
pub use frame_transforms::{det_3x3, invert_3x3, FramePose, Schema106Transform};
pub use iq::{
    check_complex_nyquist_band, convert_carrier_sign_conjugate, encode_iq_packet,
    fft_bins_to_ascending_physical, iq_dft_forward_unscaled, iq_idft_scaled, iq_record_crc32c,
    iq_record_stride, reconstruct_rf_real_signal, signed_baseband_offset_hz, window_weight,
    IqHeader, IqPacketView, FLAG_LO_PHASE_KNOWN, IQ_CONVENTION, IQ_MODEL_URI, IQ_PROFILE,
    MAX_IQ_SAMPLES, STFT_FFT_NORMALIZATION,
};
pub use mueller::{
    apply_mueller, apply_spectral_response, build_log_band_edges, cloude_coherency_matrix,
    cloude_eigenvalues, convert_density_hz, convert_density_wavelength_to_hz,
    encode_mueller_records, integrate_spectrum_over_axis, jones_to_mueller, rotate_mueller_passive,
    validate_band_edges, verify_mueller_physical_realizability, DensityTarget, MuellerRecord,
    MuellerUnits, SpectralPrecisionProfile, BAND_COUNT_24, BAND_COUNT_28, MUELLER_MODEL_URI,
    MUELLER_PROFILE,
};
pub use records::{FieldRecord, RayRecord, SpectrumRecord, SpectrumValues};
pub use registry::{canonical_emf_mode, extension_rules, polarization_rules, ProfileRegistry};
pub use schemas::{
    acquisition_rules, bio_context_rules, calibration_rules, constitutive_model_rules,
    coordinate_transform_rules, evaluate_calibration, material_binding_contains,
    material_binding_rules, model_evidence_rules, validate_acquisition, validate_bio_context,
    validate_calibration, validate_constitutive_model, validate_coordinate_transform,
    validate_material_binding, validate_model_evidence, ConstitutiveKind, EpistemicClass,
    ProfileResponseValidator, SignalKind, TransformKind, WindowKind,
};
pub use spatial::{BrickLayout, GridIndexing, GridLayout, GridSpec};
pub use yee::{
    encode_yee_records, energy_density_isotropic, energy_density_vacuum_eb,
    evaluate_instantaneous_phasor, optical_path_delay_seconds, peak_to_rms, poynting_e_cross_h,
    poynting_from_eb_vacuum, rms_to_peak, rotate_stokes_passive, validate_yee_extension,
    yee_check_cfl, yee_cfl_max_dt_seconds, yee_discrete_dispersion_omega, yee_step_periodic,
    yee_total_energy_j, HarmonicSign, JonesVector, PhasorAmplitudeKind, YeeParsePolicy, YeeRecord,
    SPEED_OF_LIGHT_M_PER_S, VACUUM_IMPEDANCE_OHM, VACUUM_PERMEABILITY_H_PER_M,
    VACUUM_PERMITTIVITY_F_PER_M, YEE_MODEL_URI, YEE_PROFILE,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Error {
    Shared(semantics::Error),
    Storage(super::field_envelope::Error),
    Spectrum(spectrum::Error),
    UnsupportedProfile,
    Shape,
    Descriptor,
    Frame,
    Time,
    Flags,
    NonFinite,
    Value,
    Reserved,
    Validity,
    Index,
    WrongMode,
    Limit,
    Overflow,
}
#[derive(Clone, Copy, Debug)]
pub struct Limits {
    pub shared: semantics::Limits,
    pub records: u64,
    pub payload_work: u64,
}
impl Default for Limits {
    fn default() -> Self {
        Self {
            shared: semantics::Limits::default(),
            records: 1_000_000,
            payload_work: 64 << 20,
        }
    }
}
pub const VALIDITY_PROFILE: &str = "validity_v1";
pub const VALIDITY_MODEL_URI: &str = "urn:qualia:emf:validity_v1";
pub const FIELD_PROFILE: &str = "instantaneous_E_B_v1";
pub const RAY_PROFILE: &str = "polarimetric_ray_v1";
pub const SPECTRUM_PROFILE: &str = "sampled_spectrum_v1";
pub const FIELD_MODEL_URI: &str = "urn:qualia:emf:instantaneous_E_B_v1";
pub const RAY_MODEL_URI: &str = "urn:qualia:emf:polarimetric_ray_v1";
pub const SPECTRUM_MODEL_URI: &str = "urn:qualia:emf:sampled_spectrum_v1";
fn required<'a>(b: Block<'a>, k: u16) -> Result<Field<'a>, Error> {
    field(b, k).ok_or(Error::Descriptor)
}
fn u32_at(b: &[u8], i: usize) -> u32 {
    u32::from_le_bytes(b[i..i + 4].try_into().unwrap())
}
fn u64_at(b: &[u8], i: usize) -> u64 {
    u64::from_le_bytes(b[i..i + 8].try_into().unwrap())
}
fn f64_at(b: &[u8], i: usize) -> f64 {
    f64::from_bits(u64_at(b, i))
}
fn scalar32(b: Block<'_>, k: u16) -> Result<u32, Error> {
    Ok(u32_at(required(b, k)?.value(), 0))
}
fn scalar64(b: Block<'_>, k: u16) -> Result<u64, Error> {
    Ok(u64_at(required(b, k)?.value(), 0))
}
fn text(f: Field<'_>) -> Result<&str, Error> {
    std::str::from_utf8(f.value()).map_err(|_| Error::Descriptor)
}
fn triple(bytes: &[u8], at: usize) -> [f64; 3] {
    [
        f64_at(bytes, at),
        f64_at(bytes, at + 8),
        f64_at(bytes, at + 16),
    ]
}
fn finite(bytes: &[u8], start: usize, end: usize) -> Result<(), Error> {
    for at in (start..end).step_by(8) {
        if !f64_at(bytes, at).is_finite() {
            return Err(Error::NonFinite);
        }
    }
    Ok(())
}
fn zeros(bytes: &[u8]) -> Result<(), Error> {
    if bytes.iter().any(|&b| b != 0) {
        Err(Error::Reserved)
    } else {
        Ok(())
    }
}
fn shared_error(e: semantics::Error) -> Error {
    if e == semantics::Error::UnsupportedDomain {
        Error::UnsupportedProfile
    } else {
        Error::Shared(e)
    }
}
#[cfg(test)]
mod dose_biofield_tests;
#[cfg(test)]
mod iq_tests;
#[cfg(test)]
mod mueller_tests;
#[cfg(test)]
mod schemas_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod yee_tests;
