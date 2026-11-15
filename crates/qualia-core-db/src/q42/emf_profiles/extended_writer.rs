//! Zero-heap canonical builder and envelope writer for all EMF profiles (0x0100..=0x0107 / 0..=7).
//! Uses caller-supplied arenas only; preserves strict block-ID and TLV-key ascending ordering.

#[path = "extended_writer_helpers.rs"]
mod extended_writer_helpers;
use extended_writer_helpers::*;

use super::super::field_envelope::{
    BlockInput, EntryInput, EnvelopeInput, Family, FieldInput, FieldValue as V, MetadataInput,
};
use super::dose_biofield::{BIOFIELD_MODEL_URI, BIOFIELD_PROFILE, DOSE_MODEL_URI, DOSE_PROFILE};
use super::iq::{iq_record_stride, IQ_MODEL_URI, IQ_PROFILE};
use super::mueller::{MUELLER_MODEL_URI, MUELLER_PROFILE};
use super::yee::{YEE_MODEL_URI, YEE_PROFILE};
use super::*;

#[derive(Clone, Copy, Debug)]
pub struct CalibrationSpec<'a> {
    pub instrument_identifier: &'a str,
    pub content_digest: [u8; 32],
    pub quantity_unit: &'a str,
    pub scale: f64,
    pub offset: f64,
    pub uncertainty: Option<(f64, &'a str)>,
    pub validity_ticks: Option<[i64; 2]>,
}

#[derive(Clone, Copy, Debug)]
pub struct ModelEvidenceSpec<'a> {
    pub model_identifier_version: &'a str,
    pub domain_applicability: &'a str,
    pub parameter_source_digest: [u8; 32],
    pub numerical_convention: &'a str,
    pub epistemic_class: &'a str,
    pub uncertainty: Option<&'a str>,
    pub external_reference: Option<&'a str>,
    pub qualification_receipt_id: Option<&'a str>,
    pub particle_species: Option<&'a str>,
    pub integration_ticks: Option<[i64; 2]>,
    pub weighting_derating_model: Option<&'a str>,
}

#[derive(Clone, Copy, Debug)]
pub struct ConstitutiveSpec<'a> {
    pub model: &'a str,
    pub epsilon_infinity_relative: f64,
    pub mu_relative: f64,
    pub conductivity_s_per_m: f64,
    pub delta_epsilon: &'a [f64],
    pub relaxation_time_s: &'a [f64],
    pub resonance_rad_s: &'a [f64],
    pub damping_s1: &'a [f64],
}

#[derive(Clone, Copy, Debug)]
pub struct AcquisitionSpec<'a> {
    pub sample_rate_num: u64,
    pub sample_rate_den: u64,
    pub signal_kind: &'a str,
    pub unit: &'a str,
    pub complex_convention: &'a str,
    pub window: Option<&'a str>,
    pub fft_size: Option<u32>,
    pub hop: Option<u32>,
    pub fft_normalization: Option<&'a str>,
    pub carrier_hz: Option<f64>,
    pub channel_count: Option<u32>,
}

#[derive(Clone, Copy, Debug)]
pub struct BioContextSpec<'a> {
    pub anatomical_ontology_version: &'a str,
    pub anatomical_identifier: &'a str,
    pub observation_type: &'a str,
    pub consent_receipt_digest: Option<[u8; 32]>,
}

#[derive(Clone, Copy, Debug)]
pub struct MaterialSpec<'a> {
    pub container_digest: [u8; 32],
    pub table_digest: [u8; 32],
    pub table_epoch: u64,
    pub table_schema_version: &'a str,
    pub material_ids: &'a [u64],
}

#[derive(Clone, Copy, Debug)]
pub struct SpectralAxisSpec<'a> {
    pub axis: &'a str,
    pub sample_semantics: &'a str,
    pub physical_quantity: &'a str,
    pub unit: &'a str,
    pub coordinates: &'a [f64],
    pub sigma_interpretation: &'a str,
    pub sigma_bounds: Option<[f64; 2]>,
}

#[derive(Clone, Copy, Debug)]
pub enum ExtendedModeSpec<'a> {
    Field { carrier_hz_note: bool },
    Ray { units: &'a str },
    Yee {
        constitutive: ConstitutiveSpec<'a>,
        time_offset_num: &'a [i64],
        time_offset_den: &'a [u64],
        material: Option<MaterialSpec<'a>>,
    },
    Spectrum {
        axis: SpectralAxisSpec<'a>,
        bins: u32,
        response_cal: Option<CalibrationSpec<'a>>,
    },
    Mueller {
        evidence: ModelEvidenceSpec<'a>,
        units: &'a str,
        material: Option<MaterialSpec<'a>>,
    },
    Iq {
        acquisition: AcquisitionSpec<'a>,
        calibration: CalibrationSpec<'a>,
        samples_per_packet: u32,
    },
    Dose {
        evidence: ModelEvidenceSpec<'a>,
        calibration: CalibrationSpec<'a>,
        material: Option<MaterialSpec<'a>>,
    },
    Biofield {
        bio: BioContextSpec<'a>,
        evidence: ModelEvidenceSpec<'a>,
        calibration: CalibrationSpec<'a>,
    },
}

/// Encodes an extended single-chunk EMF v4 envelope into `output` via zero-heap staging
/// and self-admits through `ProfileReader` before final atomic copy.
#[allow(clippy::too_many_arguments)]
pub fn write_extended_envelope<'out>(
    clock: FieldClock,
    frame: FieldFrame,
    provenance: FieldProvenance<'_>,
    mode_id: u16,
    temporal_step_ticks: Option<i64>,
    start_tick: i64,
    time_count: u64,
    spatial_key: Option<u64>,
    record_count: u64,
    payload: &[u8],
    spec: &ExtendedModeSpec<'_>,
    meta_buf: &mut [u8],
    staging: &mut [u8],
    output: &'out mut [u8],
    scratch: &mut Scratch<'_>,
    limits: FieldWriteLimits,
) -> Result<&'out mut [u8], Error> {
    if payload.is_empty() || record_count == 0 {
        return Err(Error::Shape);
    }
    if temporal_step_ticks.is_some_and(|s| s <= 0) {
        return Err(Error::Time);
    }
    let scale = [clock.scale];
    let epoch = [clock.epoch_seconds];
    let nanos = [clock.epoch_nanoseconds];
    let cnum = [clock.ticks_per_second_num];
    let cden = [clock.ticks_per_second_den];
    let clock_fields = [
        FieldInput { key: 1, flags: 1, value: V::U32(&scale) },
        FieldInput { key: 2, flags: 1, value: V::I64(&epoch) },
        FieldInput { key: 3, flags: 1, value: V::U32(&nanos) },
        FieldInput { key: 4, flags: 1, value: V::U64(&cnum) },
        FieldInput { key: 5, flags: 1, value: V::U64(&cden) },
    ];

    let zero32 = [0u32];
    let mut frame_fields = [empty_f(); 7];
    let mut fc = 0usize;
    push(&mut frame_fields, &mut fc, 1, true, V::U32(&zero32));
    push(&mut frame_fields, &mut fc, 3, true, V::F64(&frame.origin_m));
    push(&mut frame_fields, &mut fc, 4, true, V::F64(&frame.rotation_xyzw));
    let grid = frame.grid;
    let dims = grid.map_or([1; 3], |g| g.dimensions);
    let dims64 = dims.map(u64::from);
    let bricks = grid.map_or([1; 3], |g| g.brick_dimensions);
    let spacing = grid.map_or([1.0; 3], |g| g.spacing_m);
    let indexing = grid.map_or(GridIndexing::Linear, |g| g.indexing);
    let order = [indexing.order()];
    let common_order = [indexing.common_order().unwrap_or(0)];
    if let Some(g) = grid {
        g.validate()?;
        push(&mut frame_fields, &mut fc, 5, true, V::F64(&spacing));
        push(&mut frame_fields, &mut fc, 6, true, V::U32(&dims));
        if indexing.common_order().is_some() {
            push(&mut frame_fields, &mut fc, 7, true, V::U32(&common_order));
        }
    }
    push(&mut frame_fields, &mut fc, 8, true, V::Utf8("right-handed-X-forward-Y-left-Z-up"));

    let mut prov_fields = [empty_f(); 4];
    let mut pc = 0usize;
    let role = [provenance.role];
    push(&mut prov_fields, &mut pc, 1, true, V::U32(&role));
    if let Some(s) = provenance.algorithm {
        push(&mut prov_fields, &mut pc, 3, false, V::Utf8(s));
    }
    if let Some(s) = provenance.algorithm_version {
        push(&mut prov_fields, &mut pc, 4, false, V::Utf8(s));
    }
    if let Some(s) = provenance.lineage {
        push(&mut prov_fields, &mut pc, 7, false, V::Utf8(s));
    }

    let mode_u32 = [u32::from(mode_id)];
    let step_i64 = [temporal_step_ticks.unwrap_or(0)];
    let (rate_num, rate_den) = if let Some(s) = temporal_step_ticks {
        let su = s as u64;
        let g = gcd(clock.ticks_per_second_num, su);
        let rn = clock.ticks_per_second_num / g;
        let rd = u128::from(clock.ticks_per_second_den) * u128::from(su / g);
        (rn, u64::try_from(rd).map_err(|_| Error::Time)?)
    } else {
        (0, 0)
    };
    let rn_arr = [rate_num];
    let rd_arr = [rate_den];

    let refs = ModeStaticRefs {
        one32: [1],
        two32: [2],
        three32: [3],
        four32: [4],
        six32: [6],
        seven32: [7],
        eight32: [8],
        nine32: [9],
        consent_role: [4],
    };

    let mut scratch_scalars = ModeScratch::default();
    let mut sub = SubBlocks::new();

    let mut ext_fields = [empty_f(); 20];
    let mut ec = 0usize;
    push(&mut ext_fields, &mut ec, 1, true, V::U32(&mode_u32));
    push(&mut ext_fields, &mut ec, 2, true, V::U32(&refs.one32));
    push(&mut ext_fields, &mut ec, 3, true, V::MetadataRefs(&refs.one32));
    push(&mut ext_fields, &mut ec, 4, true, V::MetadataRefs(&refs.two32));
    push(&mut ext_fields, &mut ec, 5, true, V::MetadataRefs(&refs.three32));

    let desc = populate_extended_mode(
        spec,
        indexing,
        &dims64,
        &bricks,
        &order,
        &step_i64,
        &mut ext_fields,
        &mut ec,
        &mut sub,
        &mut scratch_scalars,
        &refs,
    )?;

    let mut desc_fields = [empty_f(); 10];
    let mut dc = 0usize;
    if temporal_step_ticks.is_some() {
        push(&mut desc_fields, &mut dc, 1, true, V::MetadataRefs(&refs.one32));
    }
    push(&mut desc_fields, &mut dc, 2, false, V::MetadataRefs(&refs.two32));
    push(&mut desc_fields, &mut dc, 3, true, V::MetadataRefs(&refs.three32));
    push(&mut desc_fields, &mut dc, 7, true, V::Utf8(desc.quantity_str));
    push(&mut desc_fields, &mut dc, 8, true, V::Utf8(desc.unit_str));
    push(&mut desc_fields, &mut dc, 12, true, V::Utf8(desc.model_uri_str));
    if temporal_step_ticks.is_some() {
        push(&mut desc_fields, &mut dc, 13, true, V::U64(&rn_arr));
        push(&mut desc_fields, &mut dc, 14, true, V::U64(&rd_arr));
    }
    push(&mut desc_fields, &mut dc, 18, true, V::MetadataRefs(&refs.four32));

    let empty_block = BlockInput {
        id: 0,
        schema_type: 0,
        revision: 1,
        flags: 0,
        fields: &[],
    };
    let mut blocks = [empty_block; 9];
    blocks[0] = BlockInput { id: 1, schema_type: 1, revision: 1, flags: 1, fields: &clock_fields };
    blocks[1] = BlockInput { id: 2, schema_type: 2, revision: 1, flags: 1, fields: &frame_fields[..fc] };
    blocks[2] = BlockInput { id: 3, schema_type: 4, revision: 1, flags: 1, fields: &prov_fields[..pc] };
    blocks[3] = BlockInput { id: 4, schema_type: 103, revision: 1, flags: 1, fields: &ext_fields[..ec] };
    blocks[4] = BlockInput { id: 5, schema_type: 3, revision: 1, flags: 1, fields: &desc_fields[..dc] };
    let mut bc = 5usize;
    for (id, schema, len, fields) in [
        (6u32, sub.s6_schema, sub.s6n, &sub.sub6[..]),
        (7u32, sub.s7_schema, sub.s7n, &sub.sub7[..]),
        (8u32, sub.s8_schema, sub.s8n, &sub.sub8[..]),
        (9u32, sub.s9_schema, sub.s9n, &sub.sub9[..]),
    ] {
        if schema != 0 {
            blocks[bc] = BlockInput {
                id,
                schema_type: schema,
                revision: 1,
                flags: 1,
                fields: &fields[..len],
            };
            bc += 1;
        }
    }

    let meta = MetadataInput {
        blocks: &blocks[..bc],
    }
    .encode_into(meta_buf, limits.storage)
    .map_err(Error::Storage)?;

    let entry = [EntryInput {
        mode: mode_id,
        revision: 1,
        flags: if temporal_step_ticks.is_some() { 2 } else { 0 }
            | if spatial_key.is_some() { 4 } else { 0 },
        stream_id: 1,
        frame_id: 2,
        start_tick,
        time_count,
        spatial_key: spatial_key.unwrap_or(0),
        record_count,
        stride: desc.stride_u32,
        codec: 0,
        descriptor_id: 5,
        payload,
    }];
    let env = EnvelopeInput {
        family: Family::EmfV4Candidate,
        flags: 0,
        metadata: meta,
        entries: &entry,
    };
    let len = env.encoded_len(limits.storage).map_err(Error::Storage)?;
    if output.len() < len {
        return Err(Error::Storage(super::super::field_envelope::Error::Extent));
    }
    let staged = env.encode_into(staging, limits.storage).map_err(Error::Storage)?;
    let view = StructuralView::open(staged, Family::EmfV4Candidate, limits.storage)
        .map_err(Error::Storage)?;
    let mut reader = ProfileReader::open(&view, scratch, limits.profiles, limits.total_payload_work)?;
    let _ = reader.chunk(1)?;
    output[..staged.len()].copy_from_slice(staged);
    Ok(&mut output[..staged.len()])
}
