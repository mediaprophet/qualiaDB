//! Comprehensive `DomainRegistry` implementation covering QAC v1 schemas 200–210.
//!
//! Bridges the metadata admission gap left by `audio/qac/metadata/registry.rs`
//! (which only implements schemas 201, 202, 205, 210 and omits `descriptor_target_schema(4)`).
//! All unit validation, schema field rules, reference schema targets, and medium/model invariants
//! are enforced fail-closed during `q42::field_semantics::admit_shared`.

use crate::q42::{
    field_envelope::{Block, Field},
    field_semantics as sem,
};

use super::types::{gcd_u64, has_control_chars, MAX_CLOCK_DEN, MAX_CLOCK_NUM};

#[path = "registry_helpers.rs"]
mod registry_helpers;
use registry_helpers::*;

const fn r(key: u16, type_tag: u8, count: Option<u32>, required: bool) -> sem::FieldRule {
    sem::FieldRule {
        key,
        type_tag,
        count,
        required,
    }
}

// Schema 200: Medium
const S200: &[sem::FieldRule] = &[
    r(1, 6, None, true),      // model
    r(2, 5, Some(1), false),  // density_kg_m3
    r(3, 5, Some(1), false),  // bulk_modulus_pa
    r(4, 5, Some(1), false),  // shear_modulus_pa
    r(5, 5, Some(1), false),  // temperature_k
    r(6, 5, Some(1), false),  // ambient_pressure_pa
    r(7, 5, Some(3), false),  // flow_m_s
    r(8, 7, Some(32), false), // matter_digest
    r(9, 7, Some(32), false), // material_table_digest
    r(10, 2, Some(1), false), // material_table_epoch
    r(11, 2, None, false),    // material_ids
    r(12, 8, Some(1), false), // constitutive_extension
    r(13, 9, Some(1), false), // domain_mask
];

// Schema 201: Transform
const S201: &[sem::FieldRule] = &[
    r(1, 6, None, true),     // kind
    r(2, 1, Some(1), false), // fft_size
    r(3, 1, Some(1), false), // hop_samples
    r(4, 6, None, false),    // window_name
    r(5, 9, Some(1), false), // window_coefficients
    r(6, 6, None, false),    // normalization
    r(7, 1, Some(1), false), // one_sided
    r(8, 9, Some(1), false), // bin_centers_hz
    r(9, 6, None, false),    // phase_convention
    r(10, 3, Some(1), false),// frame_origin_samples
    r(11, 6, None, false),   // padding_policy
    r(12, 6, None, false),   // channel_order
    r(13, 1, Some(1), true), // bins
    r(14, 2, Some(1), false),// input_sample_rate_num
    r(15, 2, Some(1), false),// input_sample_rate_den
    r(16, 4, None, false),   // window_coefficients_inline
    r(17, 6, None, false),   // origin_basis
];

// Schema 202: Calibration
const S202: &[sem::FieldRule] = &[
    r(1, 6, None, true),      // input_quantity
    r(2, 6, None, true),      // output_units
    r(3, 5, Some(1), true),   // scale
    r(4, 5, Some(1), true),   // offset
    r(5, 5, Some(1), false),  // reference_pressure_pa
    r(6, 9, Some(1), false),  // response
    r(7, 9, Some(1), false),  // uncertainty
    r(8, 7, Some(32), false), // calibration_digest
    r(9, 8, Some(1), false),  // valid_from (Schema 5)
    r(10, 8, Some(1), false), // valid_until (Schema 5)
];

// Schema 203: FilterbankFeatures
const S203: &[sem::FieldRule] = &[
    r(1, 6, None, true),     // algorithm_URI
    r(2, 6, None, true),     // algorithm_version
    r(3, 1, Some(1), true),  // bands
    r(4, 6, None, true),     // scale
    r(5, 9, Some(1), false), // centers_hz
    r(6, 9, Some(1), false), // coefficients
    r(7, 8, Some(1), false), // window
    r(8, 5, Some(1), false), // reference_pressure_pa
    r(9, 9, Some(1), false), // channel_weights
    r(10, 6, None, false),   // gating_profile
    r(11, 6, None, false),   // feature_names
];

// Schema 204: SpatialHrir
const S204: &[sem::FieldRule] = &[
    r(1, 6, None, true),     // representation ('HOA', 'HRIR', 'RIR')
    r(2, 1, Some(1), false), // hoa_order
    r(3, 6, None, false),    // hoa_ordering
    r(4, 6, None, false),    // normalization
    r(5, 9, Some(1), false), // receiver_positions
    r(6, 9, Some(1), false), // source_positions
    r(7, 9, Some(1), false), // delays_samples
    r(8, 1, Some(1), false), // taps
    r(9, 1, Some(1), false), // receivers
    r(10, 1, Some(1), false),// measurements
    r(11, 9, Some(1), false),// sofa_original
    r(12, 6, None, false),   // sofa_convention
    r(13, 8, Some(1), false),// listener_frame (Schema 2)
    r(14, 9, Some(1), false),// pose_times
    r(15, 9, Some(1), false),// RT60_bands_s
    r(16, 9, Some(1), false),// portal_metadata
    r(17, 2, Some(1), false),// impulse_sample_rate_num
    r(18, 2, Some(1), false),// impulse_sample_rate_den
    r(19, 6, None, false),   // delay_reference
];

// Schema 205: ModelGrid
const S205: &[sem::FieldRule] = &[
    r(1, 6, None, true),     // model_URI
    r(2, 6, None, true),     // model_version
    r(3, 6, None, false),    // discretization
    r(4, 5, None, false),    // component_offsets_xyz
    r(5, 3, None, false),    // component_time_offset_num
    r(6, 9, Some(1), false), // boundary_conditions
    r(7, 9, Some(1), false), // source_terms
    r(8, 9, Some(1), false), // internal_state
    r(9, 9, Some(1), false), // constitutive_state
    r(10, 2, Some(1), false),// timestep_num
    r(11, 2, Some(1), false),// timestep_den
    r(12, 5, Some(1), false),// tolerance_abs
    r(13, 5, Some(1), false),// tolerance_rel
    r(14, 2, None, true),    // tensor_shape
    r(15, 6, None, true),    // component_names
    r(16, 2, None, false),   // component_time_offset_den
];

// Schema 206: RayModal
const S206: &[sem::FieldRule] = &[
    r(1, 6, None, true),      // kind
    r(2, 9, Some(1), false),  // band_centers_hz
    r(3, 6, None, false),     // gain_quantity
    r(4, 5, Some(1), false),  // reference_distance_m
    r(5, 8, Some(1), false),  // launch_time (Schema 5)
    r(6, 7, Some(32), false), // geometry_digest
    r(7, 7, Some(32), false), // material_digest
    r(8, 9, Some(1), false),  // excitation
    r(9, 9, Some(1), false),  // mode_shapes
    r(10, 6, None, false),    // diffraction_model
    r(11, 9, Some(1), false), // ray_weights
    r(12, 9, Some(1), false), // band_values
];

// Schema 207: Haptic
const S207: &[sem::FieldRule] = &[
    r(1, 6, None, true),      // role
    r(2, 8, Some(1), true),   // contact_frame (Schema 2)
    r(3, 5, Some(1), false),  // contact_area_m2
    r(4, 6, None, false),     // receptor_model_URI
    r(5, 7, Some(32), false), // device_profile_digest
    r(6, 7, Some(32), false), // authority_receipt_digest
    r(7, 8, Some(1), false),  // command_expiry (Schema 5)
    r(8, 8, Some(1), false),  // calibration (Schema 202)
    r(9, 9, Some(1), false),  // estimated_receptor_output
];

// Schema 208: UltrasoundAcquisition
const S208: &[sem::FieldRule] = &[
    r(1, 6, None, true),      // role
    r(2, 5, Some(1), false),  // center_frequency_hz
    r(3, 9, Some(1), false),  // element_positions
    r(4, 9, Some(1), false),  // channel_map
    r(5, 9, Some(1), false),  // transmit_delays_s
    r(6, 5, Some(1), false),  // demodulation_phase_rad
    r(7, 2, Some(1), false),  // PRF_num
    r(8, 2, Some(1), false),  // PRF_den
    r(9, 9, Some(1), false),  // pulse_definition
    r(10, 6, None, false),    // derating_model_URI
    r(11, 6, None, false),    // exposure_context
    r(12, 6, None, false),    // thermal_model_URI
    r(13, 9, Some(1), false), // uncertainty
    r(14, 7, Some(32), false),// instrument_digest
    r(15, 7, Some(32), false),// authority_receipt_digest
    r(16, 8, Some(1), false), // target_frame (Schema 2)
    r(17, 9, Some(1), false), // raw_source
];

// Schema 209: LegacyRetention
const S209: &[sem::FieldRule] = &[
    r(1, 6, None, true),     // original_family ('Q4AU')
    r(2, 1, Some(1), true),  // original_version (1..=3)
    r(3, 9, Some(1), true),  // original_bytes (mode 0x0104)
    r(4, 7, Some(32), true), // original_digest
    r(5, 6, None, true),     // migration_algorithm
    r(6, 6, None, true),     // migration_version
    r(7, 6, None, false),    // interpreted_fields
    r(8, 6, None, false),    // unknown_fields
];

// Schema 210: SignalEncoding
const S210: &[sem::FieldRule] = &[
    r(1, 6, None, true),    // channel_order
    r(2, 5, Some(1), true), // units_per_stored_sample
    r(3, 5, Some(1), true), // stored_offset
    r(4, 6, None, true),    // purpose
    r(5, 6, None, true),    // relation
];

/// Full QAC v1 schema registry for all acoustic/continuum/spatial/modal/perceptual/haptic/ultrasound profiles.
#[derive(Debug, Default, Clone, Copy)]
pub struct QacProfileRegistry;

impl sem::DomainRegistry for QacProfileRegistry {
    fn rules(&self, schema: u16, revision: u16) -> Option<&'static [sem::FieldRule]> {
        if revision != 1 {
            return None;
        }
        match schema {
            200 => Some(S200),
            201 => Some(S201),
            202 => Some(S202),
            203 => Some(S203),
            204 => Some(S204),
            205 => Some(S205),
            206 => Some(S206),
            207 => Some(S207),
            208 => Some(S208),
            209 => Some(S209),
            210 => Some(S210),
            _ => None,
        }
    }

    fn descriptor_target_schema(&self, key: u16) -> Option<u16> {
        match key {
            4 => Some(200),
            5 => Some(201),
            6 => Some(202),
            _ => None,
        }
    }

    fn validate_units(&self, units: &str, _ctx: &sem::Context<'_, '_>) -> Result<(), sem::Error> {
        if has_control_chars(units) {
            return Err(sem::Error::Value);
        }
        match units {
            "Pa"
            | "m/s"
            | "Pa*m/s"
            | "W/m^2"
            | "W m^-2"
            | "W/m2"
            | "J/m^3"
            | "J/m3"
            | "N"
            | "N*m"
            | "m"
            | "m^2"
            | "Hz"
            | "s"
            | "1"
            | "dB-SPL"
            | "sone"
            | "LUFS"
            | "mel"
            | "Pa/Pa"
            | "raw_counts"
            | "mixed:mode-registry"
            | "complex_digital_spectrum"
            | "complex_pressure_spectrum" => Ok(()),
            _ => Err(sem::Error::Value),
        }
    }

    fn validate_block(
        &self,
        block: Block<'_>,
        ctx: &sem::Context<'_, '_>,
    ) -> Result<(), sem::Error> {
        match block.schema_type() {
            200 => validate_medium(block, ctx),
            201 => validate_transform(block),
            202 => validate_calibration(block, ctx),
            203 => validate_filterbank(block),
            204 => validate_spatial_hrir(block, ctx),
            205 => validate_model_grid(block),
            206 => validate_ray_modal(block, ctx),
            207 => validate_haptic(block, ctx),
            208 => validate_ultrasound(block, ctx),
            209 => validate_legacy_retention(block, ctx),
            210 => validate_signal_encoding(block),
            _ => Ok(()),
        }
    }
}

// ---------------------------------------------------------------------------
// Schema block helpers & validators
// ---------------------------------------------------------------------------

pub(crate) fn field<'a>(block: Block<'a>, key: u16) -> Option<Field<'a>> {
    block.fields().find(|f| f.key() == key)
}

pub(crate) fn text_val<'a>(f: Field<'a>) -> Result<&'a str, sem::Error> {
    let s = std::str::from_utf8(f.value()).map_err(|_| sem::Error::Value)?;
    if has_control_chars(s) {
        return Err(sem::Error::Value);
    }
    Ok(s)
}

pub(crate) fn u32_val(f: Field<'_>) -> Result<u32, sem::Error> {
    let b = f.value();
    if b.len() != 4 {
        return Err(sem::Error::FieldType);
    }
    Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
}

pub(crate) fn u64_val(f: Field<'_>) -> Result<u64, sem::Error> {
    let b = f.value();
    if b.len() != 8 {
        return Err(sem::Error::FieldType);
    }
    Ok(u64::from_le_bytes([
        b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7],
    ]))
}

pub(crate) fn f64_val(f: Field<'_>) -> Result<f64, sem::Error> {
    let b = f.value();
    if b.len() != 8 {
        return Err(sem::Error::FieldType);
    }
    let v = f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]]);
    if !v.is_finite() {
        return Err(sem::Error::Value);
    }
    Ok(v)
}

pub(crate) fn find_block_by_schema<'a>(
    ctx: &sem::Context<'_, 'a>,
    id: u32,
    expected_schema: u16,
) -> Result<Block<'a>, sem::Error> {
    for b in ctx.blocks() {
        let block = b?;
        if block.id() == id {
            if block.schema_type() != expected_schema {
                return Err(sem::Error::ReferenceSchema);
            }
            return Ok(block);
        }
    }
    Err(sem::Error::Reference)
}

pub(crate) fn validate_reduced_rate(num: u64, den: u64) -> Result<(), sem::Error> {
    if num == 0 || den == 0 || gcd_u64(num, den) != 1 || num > MAX_CLOCK_NUM || den > MAX_CLOCK_DEN {
        return Err(sem::Error::Time);
    }
    Ok(())
}
