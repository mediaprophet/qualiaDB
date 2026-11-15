//! Helper functions, scratch structs, and mode builders for extended EMF profiles.

use super::*;

pub(super) fn empty_f() -> FieldInput<'static> {
    FieldInput { key: 0, flags: 0, value: V::Bytes(&[]) }
}

pub(super) fn push<'a>(arr: &mut [FieldInput<'a>], n: &mut usize, key: u16, critical: bool, value: V<'a>) {
    arr[*n] = FieldInput { key, flags: u8::from(critical), value };
    *n += 1;
}

pub(super) fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 { let r = a % b; a = b; b = r; }
    a
}

pub(super) fn build_cal_fields<'a>(
    spec: &'a CalibrationSpec<'a>,
    scale_arr: &'a [f64; 1],
    off_arr: &'a [f64; 1],
    unc_arr: &'a [f64; 1],
    clk_ref: &'a [u32; 1],
    out: &mut [FieldInput<'a>; 9],
) -> usize {
    let mut n = 0;
    push(out, &mut n, 1, true, V::Utf8(spec.instrument_identifier));
    push(out, &mut n, 2, true, V::Bytes(&spec.content_digest));
    push(out, &mut n, 3, true, V::Utf8(spec.quantity_unit));
    push(out, &mut n, 4, true, V::F64(scale_arr));
    push(out, &mut n, 5, true, V::F64(off_arr));
    if let Some((_, model)) = spec.uncertainty {
        push(out, &mut n, 6, false, V::F64(unc_arr));
        push(out, &mut n, 7, false, V::Utf8(model));
    }
    if let Some(vt) = &spec.validity_ticks {
        push(out, &mut n, 8, false, V::I64(vt));
        push(out, &mut n, 9, false, V::MetadataRefs(clk_ref));
    }
    n
}

pub(super) fn build_ev_fields<'a>(
    spec: &'a ModelEvidenceSpec<'a>,
    clk_ref: &'a [u32; 1],
    out: &mut [FieldInput<'a>; 12],
) -> usize {
    let mut n = 0;
    push(out, &mut n, 1, true, V::Utf8(spec.model_identifier_version));
    push(out, &mut n, 2, true, V::Utf8(spec.domain_applicability));
    push(out, &mut n, 3, true, V::Bytes(&spec.parameter_source_digest));
    push(out, &mut n, 4, true, V::Utf8(spec.numerical_convention));
    if let Some(u) = spec.uncertainty { push(out, &mut n, 5, false, V::Utf8(u)); }
    if let Some(r) = spec.external_reference { push(out, &mut n, 6, false, V::Utf8(r)); }
    if let Some(q) = spec.qualification_receipt_id { push(out, &mut n, 7, false, V::Utf8(q)); }
    push(out, &mut n, 8, true, V::Utf8(spec.epistemic_class));
    if let Some(sp) = spec.particle_species { push(out, &mut n, 9, true, V::Utf8(sp)); }
    if let Some(it) = &spec.integration_ticks { push(out, &mut n, 10, true, V::I64(it)); }
    if let Some(wm) = spec.weighting_derating_model { push(out, &mut n, 11, false, V::Utf8(wm)); }
    if spec.integration_ticks.is_some() { push(out, &mut n, 12, true, V::MetadataRefs(clk_ref)); }
    n
}

#[derive(Default)]
pub(super) struct ModeScratch {
    pub f64_a: [f64; 1], pub f64_b: [f64; 1], pub f64_c: [f64; 1], pub f64_d: [f64; 1],
    pub u64_a: [u64; 1], pub u64_b: [u64; 1], pub u64_c: [u64; 1],
    pub u32_a: [u32; 1], pub u32_b: [u32; 1], pub u32_c: [u32; 1], pub u32_d: [u32; 1],
}

pub(super) struct ModeStaticRefs {
    pub one32: [u32; 1], pub two32: [u32; 1], pub three32: [u32; 1],
    pub four32: [u32; 1], pub six32: [u32; 1], pub seven32: [u32; 1],
    pub eight32: [u32; 1], pub nine32: [u32; 1], pub consent_role: [u32; 1],
}

pub(super) struct SubBlocks<'a> {
    pub sub6: [FieldInput<'a>; 13],
    pub sub7: [FieldInput<'a>; 13],
    pub sub8: [FieldInput<'a>; 12],
    pub sub9: [FieldInput<'a>; 9],
    pub s6n: usize, pub s7n: usize, pub s8n: usize, pub s9n: usize,
    pub s6_schema: u16, pub s7_schema: u16, pub s8_schema: u16, pub s9_schema: u16,
}

impl<'a> SubBlocks<'a> {
    pub fn new() -> Self {
        Self {
            sub6: [empty_f(); 13], sub7: [empty_f(); 13],
            sub8: [empty_f(); 12], sub9: [empty_f(); 9],
            s6n: 0, s7n: 0, s8n: 0, s9n: 0,
            s6_schema: 0, s7_schema: 0, s8_schema: 0, s9_schema: 0,
        }
    }
}

pub(super) struct ModeDescriptors<'a> {
    pub quantity_str: &'a str,
    pub unit_str: &'a str,
    pub model_uri_str: &'static str,
    pub stride_u32: u32,
}

fn push_grid_ext_fields<'a>(
    ext_fields: &mut [FieldInput<'a>],
    ec: &mut usize,
    profile_str: &'static str,
    indexing_str: &'a str,
    dims64: &'a [u64; 3],
    bricks: &'a [u32; 3],
    order: &'a [u32; 1],
    step_i64: &'a [i64; 1],
) {
    push(ext_fields, ec, 6, true, V::Utf8(profile_str));
    push(ext_fields, ec, 7, true, V::Utf8(indexing_str));
    push(ext_fields, ec, 8, true, V::U64(dims64));
    push(ext_fields, ec, 9, true, V::U32(bricks));
    push(ext_fields, ec, 10, true, V::U32(order));
    push(ext_fields, ec, 11, true, V::I64(step_i64));
}

fn push_material_subblock<'a>(
    sub: &mut [FieldInput<'a>],
    sn: &mut usize,
    schema: &mut u16,
    mat: &'a MaterialSpec<'a>,
    u64_a: &'a mut [u64; 1],
    two32: &'a [u32; 1],
) {
    *schema = 105;
    *u64_a = [mat.table_epoch];
    push(sub, sn, 1, true, V::Bytes(&mat.container_digest));
    push(sub, sn, 2, true, V::Utf8("BLAKE3-256"));
    push(sub, sn, 3, true, V::Bytes(&mat.table_digest));
    push(sub, sn, 4, true, V::U64(u64_a));
    push(sub, sn, 5, true, V::MetadataRefs(two32));
    push(sub, sn, 8, true, V::Utf8(mat.table_schema_version));
    push(sub, sn, 9, true, V::U64(mat.material_ids));
}

fn push_cal_subblock<'a>(
    sub: &mut [FieldInput<'a>],
    sn: &mut usize,
    schema: &mut u16,
    cal: &'a CalibrationSpec<'a>,
    f64_a: &'a mut [f64; 1],
    f64_b: &'a mut [f64; 1],
    f64_c: &'a mut [f64; 1],
    one32: &'a [u32; 1],
) {
    *schema = 107;
    *f64_a = [cal.scale];
    *f64_b = [cal.offset];
    *f64_c = [cal.uncertainty.map_or(0.0, |(u, _)| u)];
    let mut cal_fields = [empty_f(); 9];
    let n = build_cal_fields(cal, f64_a, f64_b, f64_c, one32, &mut cal_fields);
    sub[..n].copy_from_slice(&cal_fields[..n]);
    *sn = n;
}

fn push_ev_subblock<'a>(
    sub: &mut [FieldInput<'a>],
    sn: &mut usize,
    schema: &mut u16,
    ev: &'a ModelEvidenceSpec<'a>,
    one32: &'a [u32; 1],
) {
    *schema = 134;
    let mut ev_fields = [empty_f(); 12];
    let n = build_ev_fields(ev, one32, &mut ev_fields);
    sub[..n].copy_from_slice(&ev_fields[..n]);
    *sn = n;
}

#[allow(clippy::too_many_arguments)]
pub(super) fn populate_extended_mode<'a>(
    spec: &'a ExtendedModeSpec<'a>,
    indexing: GridIndexing,
    dims64: &'a [u64; 3],
    bricks: &'a [u32; 3],
    order: &'a [u32; 1],
    step_i64: &'a [i64; 1],
    ext_fields: &mut [FieldInput<'a>],
    ec: &mut usize,
    sub: &mut SubBlocks<'a>,
    scratch: &'a mut ModeScratch,
    refs: &'a ModeStaticRefs,
) -> Result<ModeDescriptors<'a>, Error> {
    let (profile_str, quantity_str, unit_str, model_uri_str, stride_u32): (&str, &str, &str, &str, u32);

    match spec {
        ExtendedModeSpec::Field { .. } => {
            profile_str = FIELD_PROFILE;
            quantity_str = "electromagnetic_field_snapshot";
            unit_str = "mixed:mode-registry";
            model_uri_str = FIELD_MODEL_URI;
            stride_u32 = 128;
            push_grid_ext_fields(ext_fields, ec, profile_str, indexing.name(), dims64, bricks, order, step_i64);
        }
        ExtendedModeSpec::Ray { units } => {
            profile_str = RAY_PROFILE;
            quantity_str = "polarimetric_ray";
            unit_str = units;
            model_uri_str = RAY_MODEL_URI;
            stride_u32 = 128;
            push_grid_ext_fields(ext_fields, ec, profile_str, indexing.name(), dims64, bricks, order, step_i64);
            push(ext_fields, ec, 16, true, V::MetadataRefs(&refs.six32));
            sub.s6_schema = 135;
            push(&mut sub.sub6, &mut sub.s6n, 1, true, V::Utf8("deterministic_transverse_v1"));
            push(&mut sub.sub6, &mut sub.s6n, 2, true, V::Utf8("I_Q_U_V_v1"));
            push(&mut sub.sub6, &mut sub.s6n, 3, true, V::Utf8(units));
            push(&mut sub.sub6, &mut sub.s6n, 4, true, V::Utf8("time_averaged"));
        }
        ExtendedModeSpec::Yee { constitutive, time_offset_num, time_offset_den, material } => {
            profile_str = YEE_PROFILE;
            quantity_str = "yee_cartesian_cell";
            unit_str = "mixed:mode-registry";
            model_uri_str = YEE_MODEL_URI;
            stride_u32 = 128;
            push_grid_ext_fields(ext_fields, ec, profile_str, indexing.name(), dims64, bricks, order, step_i64);
            if material.is_some() {
                push(ext_fields, ec, 13, false, V::MetadataRefs(&refs.seven32));
            }
            push(ext_fields, ec, 14, true, V::MetadataRefs(&refs.six32));
            if !time_offset_num.is_empty() {
                push(ext_fields, ec, 27, false, V::I64(time_offset_num));
                push(ext_fields, ec, 28, false, V::U64(time_offset_den));
            }
            sub.s6_schema = 133;
            scratch.f64_a = [constitutive.epsilon_infinity_relative];
            scratch.f64_b = [constitutive.mu_relative];
            scratch.f64_c = [constitutive.conductivity_s_per_m];
            push(&mut sub.sub6, &mut sub.s6n, 1, true, V::Utf8(constitutive.model));
            push(&mut sub.sub6, &mut sub.s6n, 2, true, V::F64(&scratch.f64_a));
            push(&mut sub.sub6, &mut sub.s6n, 3, true, V::F64(&scratch.f64_b));
            push(&mut sub.sub6, &mut sub.s6n, 4, true, V::F64(&scratch.f64_c));
            if !constitutive.delta_epsilon.is_empty() {
                push(&mut sub.sub6, &mut sub.s6n, 5, false, V::F64(constitutive.delta_epsilon));
            }
            if !constitutive.relaxation_time_s.is_empty() {
                push(&mut sub.sub6, &mut sub.s6n, 6, false, V::F64(constitutive.relaxation_time_s));
            }
            if !constitutive.resonance_rad_s.is_empty() {
                push(&mut sub.sub6, &mut sub.s6n, 7, false, V::F64(constitutive.resonance_rad_s));
            }
            if !constitutive.damping_s1.is_empty() {
                push(&mut sub.sub6, &mut sub.s6n, 8, false, V::F64(constitutive.damping_s1));
            }
            if material.is_some() {
                push(&mut sub.sub6, &mut sub.s6n, 10, false, V::MetadataRefs(&refs.seven32));
            }
            if let Some(mat) = material {
                push_material_subblock(&mut sub.sub7, &mut sub.s7n, &mut sub.s7_schema, mat, &mut scratch.u64_a, &refs.two32);
            }
        }
        ExtendedModeSpec::Spectrum { axis, bins, response_cal } => {
            profile_str = SPECTRUM_PROFILE;
            quantity_str = axis.physical_quantity;
            unit_str = axis.unit;
            model_uri_str = SPECTRUM_MODEL_URI;
            if !(1..=4096).contains(bins) {
                return Err(Error::Shape);
            }
            stride_u32 = 64 + (((*bins * 8) + 63) & !63);
            scratch.u32_a = [*bins];
            push_grid_ext_fields(ext_fields, ec, profile_str, "linear", dims64, bricks, order, step_i64);
            push(ext_fields, ec, 12, true, V::MetadataRefs(&refs.six32));
            push(ext_fields, ec, 19, true, V::U32(&scratch.u32_a));
            sub.s6_schema = 132;
            push(&mut sub.sub6, &mut sub.s6n, 1, true, V::Utf8(axis.axis));
            push(&mut sub.sub6, &mut sub.s6n, 2, true, V::Utf8(axis.sample_semantics));
            push(&mut sub.sub6, &mut sub.s6n, 3, true, V::Utf8(axis.physical_quantity));
            push(&mut sub.sub6, &mut sub.s6n, 4, true, V::Utf8(axis.unit));
            push(&mut sub.sub6, &mut sub.s6n, 5, true, V::F64(axis.coordinates));
            push(&mut sub.sub6, &mut sub.s6n, 6, true, V::Utf8("explicit"));
            if response_cal.is_some() {
                push(&mut sub.sub6, &mut sub.s6n, 7, false, V::MetadataRefs(&refs.seven32));
            }
            push(&mut sub.sub6, &mut sub.s6n, 8, true, V::Utf8(axis.sigma_interpretation));
            if let Some(ref sb) = axis.sigma_bounds {
                push(&mut sub.sub6, &mut sub.s6n, 9, true, V::F64(sb));
            }
            if let Some(rcal) = response_cal {
                push_cal_subblock(&mut sub.sub7, &mut sub.s7n, &mut sub.s7_schema, rcal, &mut scratch.f64_a, &mut scratch.f64_b, &mut scratch.f64_c, &refs.one32);
            }
        }
        ExtendedModeSpec::Mueller { evidence, units, material } => {
            profile_str = MUELLER_PROFILE;
            quantity_str = "mueller_scattering_matrix";
            unit_str = units;
            model_uri_str = MUELLER_MODEL_URI;
            stride_u32 = 256;
            push_grid_ext_fields(ext_fields, ec, profile_str, "linear", dims64, bricks, order, step_i64);
            if material.is_some() {
                push(ext_fields, ec, 13, false, V::MetadataRefs(&refs.eight32));
            }
            push(ext_fields, ec, 16, true, V::MetadataRefs(&refs.six32));
            push(ext_fields, ec, 17, true, V::MetadataRefs(&refs.seven32));
            sub.s6_schema = 135;
            push(&mut sub.sub6, &mut sub.s6n, 1, true, V::Utf8("deterministic_transverse_v1"));
            push(&mut sub.sub6, &mut sub.s6n, 2, true, V::Utf8("I_Q_U_V_v1"));
            push(&mut sub.sub6, &mut sub.s6n, 3, true, V::Utf8(units));
            push(&mut sub.sub6, &mut sub.s6n, 4, true, V::Utf8("time_averaged"));
            push_ev_subblock(&mut sub.sub7, &mut sub.s7n, &mut sub.s7_schema, evidence, &refs.one32);
            if let Some(mat) = material {
                push_material_subblock(&mut sub.sub8, &mut sub.s8n, &mut sub.s8_schema, mat, &mut scratch.u64_a, &refs.two32);
            }
        }
        ExtendedModeSpec::Iq { acquisition, calibration, samples_per_packet } => {
            profile_str = IQ_PROFILE;
            quantity_str = "rf_iq_samples";
            unit_str = acquisition.unit;
            model_uri_str = IQ_MODEL_URI;
            stride_u32 = u32::try_from(iq_record_stride(*samples_per_packet)?).map_err(|_| Error::Shape)?;
            scratch.u32_a = [*samples_per_packet];
            push_grid_ext_fields(ext_fields, ec, profile_str, "linear", dims64, bricks, order, step_i64);
            push(ext_fields, ec, 15, true, V::MetadataRefs(&refs.seven32));
            push(ext_fields, ec, 18, true, V::U32(&scratch.u32_a));
            push(ext_fields, ec, 30, true, V::MetadataRefs(&refs.six32));
            push_cal_subblock(&mut sub.sub6, &mut sub.s6n, &mut sub.s6_schema, calibration, &mut scratch.f64_a, &mut scratch.f64_b, &mut scratch.f64_c, &refs.one32);

            sub.s7_schema = 136;
            scratch.u64_b = [acquisition.sample_rate_num];
            scratch.u64_c = [acquisition.sample_rate_den];
            push(&mut sub.sub7, &mut sub.s7n, 1, true, V::MetadataRefs(&refs.one32));
            push(&mut sub.sub7, &mut sub.s7n, 2, true, V::U64(&scratch.u64_b));
            push(&mut sub.sub7, &mut sub.s7n, 3, true, V::U64(&scratch.u64_c));
            push(&mut sub.sub7, &mut sub.s7n, 4, true, V::Utf8(acquisition.signal_kind));
            push(&mut sub.sub7, &mut sub.s7n, 5, true, V::Utf8(acquisition.unit));
            push(&mut sub.sub7, &mut sub.s7n, 6, true, V::MetadataRefs(&refs.six32));
            push(&mut sub.sub7, &mut sub.s7n, 7, true, V::Utf8(acquisition.complex_convention));
            if let Some(w) = acquisition.window { push(&mut sub.sub7, &mut sub.s7n, 8, false, V::Utf8(w)); }
            if let Some(n) = acquisition.fft_size {
                scratch.u32_b = [n];
                push(&mut sub.sub7, &mut sub.s7n, 9, false, V::U32(&scratch.u32_b));
            }
            if let Some(h) = acquisition.hop {
                scratch.u32_c = [h];
                push(&mut sub.sub7, &mut sub.s7n, 10, false, V::U32(&scratch.u32_c));
            }
            if let Some(nm) = acquisition.fft_normalization { push(&mut sub.sub7, &mut sub.s7n, 11, false, V::Utf8(nm)); }
            if let Some(fc_hz) = acquisition.carrier_hz {
                scratch.f64_d = [fc_hz];
                push(&mut sub.sub7, &mut sub.s7n, 12, false, V::F64(&scratch.f64_d));
            }
            if let Some(ch) = acquisition.channel_count {
                scratch.u32_d = [ch];
                push(&mut sub.sub7, &mut sub.s7n, 13, false, V::U32(&scratch.u32_d));
            }
        }
        ExtendedModeSpec::Dose { evidence, calibration, material } => {
            profile_str = DOSE_PROFILE;
            quantity_str = "ionizing_radiation_dose";
            unit_str = "mixed:mode-registry";
            model_uri_str = DOSE_MODEL_URI;
            stride_u32 = 128;
            push_grid_ext_fields(ext_fields, ec, profile_str, "linear", dims64, bricks, order, step_i64);
            if material.is_some() {
                push(ext_fields, ec, 13, false, V::MetadataRefs(&refs.eight32));
            }
            push(ext_fields, ec, 17, true, V::MetadataRefs(&refs.seven32));
            push(ext_fields, ec, 30, true, V::MetadataRefs(&refs.six32));
            push_cal_subblock(&mut sub.sub6, &mut sub.s6n, &mut sub.s6_schema, calibration, &mut scratch.f64_a, &mut scratch.f64_b, &mut scratch.f64_c, &refs.one32);
            push_ev_subblock(&mut sub.sub7, &mut sub.s7n, &mut sub.s7_schema, evidence, &refs.one32);
            if let Some(mat) = material {
                push_material_subblock(&mut sub.sub8, &mut sub.s8n, &mut sub.s8_schema, mat, &mut scratch.u64_a, &refs.two32);
            }
        }
        ExtendedModeSpec::Biofield { bio, evidence, calibration } => {
            profile_str = BIOFIELD_PROFILE;
            quantity_str = "bioelectric_current_dipole";
            unit_str = "A m";
            model_uri_str = BIOFIELD_MODEL_URI;
            stride_u32 = 128;
            push_grid_ext_fields(ext_fields, ec, profile_str, "linear", dims64, bricks, order, step_i64);
            push(ext_fields, ec, 17, true, V::MetadataRefs(&refs.seven32));
            push(ext_fields, ec, 22, true, V::MetadataRefs(&refs.eight32));
            push(ext_fields, ec, 30, true, V::MetadataRefs(&refs.six32));
            push_cal_subblock(&mut sub.sub6, &mut sub.s6n, &mut sub.s6_schema, calibration, &mut scratch.f64_a, &mut scratch.f64_b, &mut scratch.f64_c, &refs.one32);
            push_ev_subblock(&mut sub.sub7, &mut sub.s7n, &mut sub.s7_schema, evidence, &refs.one32);
            sub.s8_schema = 137;
            push(&mut sub.sub8, &mut sub.s8n, 1, true, V::Utf8(bio.anatomical_ontology_version));
            push(&mut sub.sub8, &mut sub.s8n, 2, true, V::Utf8(bio.anatomical_identifier));
            push(&mut sub.sub8, &mut sub.s8n, 3, true, V::Utf8(bio.observation_type));
            push(&mut sub.sub8, &mut sub.s8n, 4, true, V::MetadataRefs(&refs.three32));
            push(&mut sub.sub8, &mut sub.s8n, 5, false, V::MetadataRefs(&refs.six32));
            push(&mut sub.sub8, &mut sub.s8n, 6, false, V::MetadataRefs(&refs.seven32));
            if let Some(ref receipt) = bio.consent_receipt_digest {
                push(&mut sub.sub8, &mut sub.s8n, 7, false, V::MetadataRefs(&refs.nine32));
                sub.s9_schema = 4;
                push(&mut sub.sub9, &mut sub.s9n, 1, true, V::U32(&refs.consent_role));
                push(&mut sub.sub9, &mut sub.s9n, 6, false, V::Bytes(receipt));
            }
        }
    }

    Ok(ModeDescriptors {
        quantity_str,
        unit_str,
        model_uri_str,
        stride_u32,
    })
}
