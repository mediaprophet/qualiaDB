//! Cold QAC v1 metadata & directory builder (tier-2 bounded construction).

use crate::q42::field_envelope::{
    BlockInput, EntryInput, EnvelopeInput, Family, FieldInput, FieldValue, Limits, MetadataInput,
};

use super::envelope::AdmittedEnvelope;
use super::types::*;

#[derive(Debug, Clone)]
pub enum OwnedValue {
    U32(Vec<u32>),
    U64(Vec<u64>),
    I64(Vec<i64>),
    F32(Vec<f32>),
    F64(Vec<f64>),
    Utf8(String),
    Bytes(Vec<u8>),
    MetadataRefs(Vec<u32>),
    EntryRefs(Vec<u32>),
}

impl OwnedValue {
    fn as_field_value(&self) -> FieldValue<'_> {
        match self {
            Self::U32(v) => FieldValue::U32(v),
            Self::U64(v) => FieldValue::U64(v),
            Self::I64(v) => FieldValue::I64(v),
            Self::F32(v) => FieldValue::F32(v),
            Self::F64(v) => FieldValue::F64(v),
            Self::Utf8(s) => FieldValue::Utf8(s),
            Self::Bytes(b) => FieldValue::Bytes(b),
            Self::MetadataRefs(r) => FieldValue::MetadataRefs(r),
            Self::EntryRefs(r) => FieldValue::EntryRefs(r),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedField {
    pub key: u16,
    pub critical: bool,
    pub value: OwnedValue,
}

impl OwnedField {
    pub fn crit(key: u16, value: OwnedValue) -> Self {
        Self {
            key,
            critical: true,
            value,
        }
    }

    pub fn opt(key: u16, value: OwnedValue) -> Self {
        Self {
            key,
            critical: false,
            value,
        }
    }
}

#[derive(Debug, Clone)]
pub struct OwnedBlock {
    pub id: u32,
    pub schema_type: u16,
    pub revision: u16,
    pub critical: bool,
    pub fields: Vec<OwnedField>,
}

/// Descriptor configuration for building `Schema 3` (`PayloadDescriptor`) blocks.
#[derive(Debug, Clone)]
pub struct DescriptorSpec<'a> {
    pub clock_id: Option<u32>,
    pub frame_id: Option<u32>,
    pub provenance_id: u32,
    pub medium_id: Option<u32>,
    pub transform_id: Option<u32>,
    pub calibration_id: Option<u32>,
    pub quantity: &'a str,
    pub units: &'a str,
    pub channels: Option<u32>,
    pub layout: Option<&'a str>,
    pub scalar: Option<AuxScalar>,
    pub model_uri: &'a str,
    pub sample_rate: Option<RationalRate>,
    pub valid_record_count: Option<u64>,
    pub validity_mask_entry: Option<u32>,
    pub source_entries: &'a [u32],
    pub extension_metadata: &'a [u32],
}

/// Model & tensor grid descriptor (`Schema 205`).
#[derive(Debug, Clone)]
pub struct ModelGridSpec<'a> {
    pub model_uri: &'a str,
    pub model_version: &'a str,
    pub discretization: Option<&'a str>,
    pub component_offsets_xyz: &'a [f64],
    pub component_time_offset_num: &'a [i64],
    pub component_time_offset_den: &'a [u64],
    pub timestep: Option<RationalRate>,
    pub tolerance_abs: Option<f64>,
    pub tolerance_rel: Option<f64>,
    pub tensor_shape: &'a [u64],
    pub component_names: &'a str,
}

/// Cold metadata builder enforcing ascending block IDs and field keys.
#[derive(Debug, Default, Clone)]
pub struct MetadataBuilder {
    pub blocks: Vec<OwnedBlock>,
}

impl MetadataBuilder {
    pub fn new() -> Self {
        Self { blocks: Vec::new() }
    }

    pub fn push_block(&mut self, mut block: OwnedBlock) -> Result<u32, ProfileError> {
        block.fields.sort_by_key(|f| f.key);
        for w in block.fields.windows(2) {
            if w[0].key >= w[1].key || w[0].key == 0 {
                return Err(ProfileError::InvalidOrdering);
            }
        }
        if block.id == 0 {
            return Err(ProfileError::InvalidOrdering);
        }
        let id = block.id;
        self.blocks.push(block);
        self.blocks.sort_by_key(|b| b.id);
        for w in self.blocks.windows(2) {
            if w[0].id >= w[1].id {
                return Err(ProfileError::InvalidOrdering);
            }
        }
        Ok(id)
    }

    pub fn push_clock(&mut self, id: u32, spec: &ClockSpec) -> Result<u32, ProfileError> {
        spec.validate()?;
        self.push_block(OwnedBlock {
            id,
            schema_type: 1,
            revision: 1,
            critical: true,
            fields: vec![
                OwnedField::crit(1, OwnedValue::U32(vec![spec.scale])),
                OwnedField::crit(2, OwnedValue::I64(vec![spec.epoch_seconds])),
                OwnedField::crit(3, OwnedValue::U32(vec![spec.epoch_nanoseconds])),
                OwnedField::crit(4, OwnedValue::U64(vec![spec.ticks_per_second.num])),
                OwnedField::crit(5, OwnedValue::U64(vec![spec.ticks_per_second.den])),
            ],
        })
    }

    pub fn push_frame(&mut self, id: u32, spec: &FrameSpec<'_>) -> Result<u32, ProfileError> {
        validate_identifier_str(spec.convention, "frame.convention")?;
        let mut fields = vec![
            OwnedField::crit(1, OwnedValue::U32(vec![spec.kind])),
            OwnedField::crit(3, OwnedValue::F64(spec.origin_m.to_vec())),
            OwnedField::crit(
                4,
                OwnedValue::F64(vec![
                    spec.rotation.x,
                    spec.rotation.y,
                    spec.rotation.z,
                    spec.rotation.w,
                ]),
            ),
            OwnedField::crit(8, OwnedValue::Utf8(spec.convention.to_owned())),
        ];
        if let Some(p) = spec.parent_block_id {
            fields.push(OwnedField::crit(2, OwnedValue::MetadataRefs(vec![p])));
        }
        if let Some(sp) = spec.grid_spacing_m {
            fields.push(OwnedField::crit(5, OwnedValue::F64(sp.to_vec())));
        }
        if let Some(dim) = spec.grid_dimensions {
            fields.push(OwnedField::crit(6, OwnedValue::U32(dim.to_vec())));
        }
        if let Some(ord) = spec.index_order {
            fields.push(OwnedField::opt(7, OwnedValue::U32(vec![ord])));
        }
        if let Some(crs) = spec.crs_definition {
            fields.push(OwnedField::opt(9, OwnedValue::Bytes(crs.to_vec())));
        }
        self.push_block(OwnedBlock {
            id,
            schema_type: 2,
            revision: 1,
            critical: true,
            fields,
        })
    }

    pub fn push_instant(&mut self, id: u32, spec: &InstantSpec) -> Result<u32, ProfileError> {
        let checked = InstantSpec::new(
            spec.clock_block_id,
            spec.tick,
            spec.subtick_num,
            spec.subtick_den,
        )?;
        self.push_block(OwnedBlock {
            id,
            schema_type: 5,
            revision: 1,
            critical: true,
            fields: vec![
                OwnedField::crit(1, OwnedValue::MetadataRefs(vec![checked.clock_block_id])),
                OwnedField::crit(2, OwnedValue::I64(vec![checked.tick])),
                OwnedField::crit(3, OwnedValue::U64(vec![checked.subtick_num])),
                OwnedField::crit(4, OwnedValue::U64(vec![checked.subtick_den])),
            ],
        })
    }

    pub fn push_provenance(&mut self, id: u32, spec: &ProvenanceSpec<'_>) -> Result<u32, ProfileError> {
        let mut fields = vec![OwnedField::crit(1, OwnedValue::U32(vec![spec.role]))];
        if !spec.source_digests.is_empty() {
            let mut cat = Vec::with_capacity(spec.source_digests.len() * 32);
            for d in spec.source_digests {
                cat.extend_from_slice(d);
            }
            fields.push(OwnedField::crit(2, OwnedValue::Bytes(cat)));
        }
        if let Some(alg) = spec.algorithm {
            fields.push(OwnedField::opt(3, OwnedValue::Utf8(alg.to_owned())));
        }
        if let Some(ver) = spec.algorithm_version {
            fields.push(OwnedField::opt(4, OwnedValue::Utf8(ver.to_owned())));
        }
        if let Some(uri) = spec.instrument_uri {
            fields.push(OwnedField::opt(5, OwnedValue::Utf8(uri.to_owned())));
        }
        if let Some(auth) = spec.authority_receipt_digest {
            fields.push(OwnedField::opt(6, OwnedValue::Bytes(auth.to_vec())));
        }
        if let Some(lin) = spec.lineage {
            fields.push(OwnedField::opt(7, OwnedValue::Utf8(lin.to_owned())));
        }
        self.push_block(OwnedBlock {
            id,
            schema_type: 4,
            revision: 1,
            critical: true,
            fields,
        })
    }

    pub fn push_calibration(&mut self, id: u32, spec: &CalibrationSpec<'_>) -> Result<u32, ProfileError> {
        spec.validate()?;
        let mut fields = vec![
            OwnedField::crit(1, OwnedValue::Utf8(spec.input_quantity.to_owned())),
            OwnedField::crit(2, OwnedValue::Utf8(spec.output_units.to_owned())),
            OwnedField::crit(3, OwnedValue::F64(vec![spec.scale])),
            OwnedField::crit(4, OwnedValue::F64(vec![spec.offset])),
        ];
        if let Some(p0) = spec.reference_pressure_pa {
            fields.push(OwnedField::opt(5, OwnedValue::F64(vec![p0])));
        }
        if let Some(resp) = spec.response_entry {
            fields.push(OwnedField::opt(6, OwnedValue::EntryRefs(vec![resp])));
        }
        if let Some(unc) = spec.uncertainty_entry {
            fields.push(OwnedField::opt(7, OwnedValue::EntryRefs(vec![unc])));
        }
        if let Some(d) = spec.calibration_digest {
            fields.push(OwnedField::opt(8, OwnedValue::Bytes(d.to_vec())));
        }
        if let Some(vf) = spec.valid_from {
            fields.push(OwnedField::opt(9, OwnedValue::MetadataRefs(vec![vf])));
        }
        if let Some(vu) = spec.valid_until {
            fields.push(OwnedField::opt(10, OwnedValue::MetadataRefs(vec![vu])));
        }
        self.push_block(OwnedBlock {
            id,
            schema_type: 202,
            revision: 1,
            critical: true,
            fields,
        })
    }

    pub fn push_descriptor(&mut self, id: u32, spec: &DescriptorSpec<'_>) -> Result<u32, ProfileError> {
        let mut fields = Vec::with_capacity(18);
        if let Some(clk) = spec.clock_id {
            fields.push(OwnedField::crit(1, OwnedValue::MetadataRefs(vec![clk])));
        }
        if let Some(fr) = spec.frame_id {
            fields.push(OwnedField::crit(2, OwnedValue::MetadataRefs(vec![fr])));
        }
        fields.push(OwnedField::crit(3, OwnedValue::MetadataRefs(vec![spec.provenance_id])));
        if let Some(med) = spec.medium_id {
            fields.push(OwnedField::crit(4, OwnedValue::MetadataRefs(vec![med])));
        }
        if let Some(tr) = spec.transform_id {
            fields.push(OwnedField::crit(5, OwnedValue::MetadataRefs(vec![tr])));
        }
        if let Some(cal) = spec.calibration_id {
            fields.push(OwnedField::opt(6, OwnedValue::MetadataRefs(vec![cal])));
        }
        fields.push(OwnedField::crit(7, OwnedValue::Utf8(spec.quantity.to_owned())));
        fields.push(OwnedField::crit(8, OwnedValue::Utf8(spec.units.to_owned())));
        if let Some(ch) = spec.channels {
            fields.push(OwnedField::crit(9, OwnedValue::U32(vec![ch])));
        }
        if let Some(lay) = spec.layout {
            fields.push(OwnedField::crit(10, OwnedValue::Utf8(lay.to_owned())));
        }
        if let Some(sc) = spec.scalar {
            fields.push(OwnedField::crit(11, OwnedValue::U32(vec![sc as u32])));
        }
        fields.push(OwnedField::crit(12, OwnedValue::Utf8(spec.model_uri.to_owned())));
        if let Some(sr) = spec.sample_rate {
            fields.push(OwnedField::crit(13, OwnedValue::U64(vec![sr.num])));
            fields.push(OwnedField::crit(14, OwnedValue::U64(vec![sr.den])));
        }
        if let Some(vrc) = spec.valid_record_count {
            fields.push(OwnedField::crit(15, OwnedValue::U64(vec![vrc])));
        }
        if let Some(vm) = spec.validity_mask_entry {
            fields.push(OwnedField::crit(16, OwnedValue::EntryRefs(vec![vm])));
        }
        if !spec.source_entries.is_empty() {
            fields.push(OwnedField::opt(17, OwnedValue::EntryRefs(spec.source_entries.to_vec())));
        }
        if !spec.extension_metadata.is_empty() {
            fields.push(OwnedField::crit(18, OwnedValue::MetadataRefs(spec.extension_metadata.to_vec())));
        }
        self.push_block(OwnedBlock {
            id,
            schema_type: 3,
            revision: 1,
            critical: true,
            fields,
        })
    }

    pub fn push_model_grid(&mut self, id: u32, spec: &ModelGridSpec<'_>) -> Result<u32, ProfileError> {
        let mut fields = vec![
            OwnedField::crit(1, OwnedValue::Utf8(spec.model_uri.to_owned())),
            OwnedField::crit(2, OwnedValue::Utf8(spec.model_version.to_owned())),
            OwnedField::crit(14, OwnedValue::U64(spec.tensor_shape.to_vec())),
            OwnedField::crit(15, OwnedValue::Utf8(spec.component_names.to_owned())),
        ];
        if let Some(disc) = spec.discretization {
            fields.push(OwnedField::opt(3, OwnedValue::Utf8(disc.to_owned())));
        }
        if !spec.component_offsets_xyz.is_empty() {
            fields.push(OwnedField::opt(4, OwnedValue::F64(spec.component_offsets_xyz.to_vec())));
        }
        if !spec.component_time_offset_num.is_empty() {
            fields.push(OwnedField::opt(5, OwnedValue::I64(spec.component_time_offset_num.to_vec())));
        }
        if let Some(ts) = spec.timestep {
            fields.push(OwnedField::opt(10, OwnedValue::U64(vec![ts.num])));
            fields.push(OwnedField::opt(11, OwnedValue::U64(vec![ts.den])));
        }
        if let Some(ta) = spec.tolerance_abs {
            fields.push(OwnedField::opt(12, OwnedValue::F64(vec![ta])));
        }
        if let Some(tr) = spec.tolerance_rel {
            fields.push(OwnedField::opt(13, OwnedValue::F64(vec![tr])));
        }
        if !spec.component_time_offset_den.is_empty() {
            fields.push(OwnedField::opt(16, OwnedValue::U64(spec.component_time_offset_den.to_vec())));
        }
        self.push_block(OwnedBlock {
            id,
            schema_type: 205,
            revision: 1,
            critical: true,
            fields,
        })
    }

    pub fn encode(&self, limits: Limits) -> Result<Vec<u8>, ProfileError> {
        let field_inputs_per_block: Vec<Vec<FieldInput<'_>>> = self
            .blocks
            .iter()
            .map(|b| {
                b.fields
                    .iter()
                    .map(|f| FieldInput {
                        key: f.key,
                        flags: u8::from(f.critical),
                        value: f.value.as_field_value(),
                    })
                    .collect()
            })
            .collect();
        let block_inputs: Vec<BlockInput<'_>> = self
            .blocks
            .iter()
            .zip(&field_inputs_per_block)
            .map(|(b, fields)| BlockInput {
                id: b.id,
                schema_type: b.schema_type,
                revision: b.revision,
                flags: u32::from(b.critical),
                fields,
            })
            .collect();
        let input = MetadataInput {
            blocks: &block_inputs,
        };
        let len = input.encoded_len(limits)?;
        let mut buf = vec![0u8; len];
        let written = input.encode_into(&mut buf, limits)?.len();
        buf.truncate(written);
        Ok(buf)
    }
}

/// Directory entry build descriptor.
#[derive(Debug, Clone)]
pub struct OwnedEntry {
    pub mode: u16,
    pub flags: u32,
    pub stream_id: u32,
    pub frame_id: u32,
    pub start_tick: i64,
    pub time_count: u64,
    pub spatial_key: u64,
    pub record_count: u64,
    pub stride: u32,
    pub descriptor_id: u32,
    pub payload: Vec<u8>,
}

/// Appends standard `validity_mask` descriptor + entry (`mode 0x0106`, stride 1).
pub fn push_validity_mask_entry(
    meta: &mut MetadataBuilder,
    descriptor_id: u32,
    prov_id: u32,
    stream_id: u32,
    start_tick: i64,
    time_count: u64,
    mask: &[u8],
) -> Result<OwnedEntry, ProfileError> {
    if mask.is_empty() || mask.iter().any(|&b| b > 1) {
        return Err(ProfileError::InvalidValidityBits);
    }
    let flags = if time_count > 0 { 3 } else { 1 };
    let (clock_id, sample_rate) = if time_count > 0 {
        (Some(1), Some(RationalRate::new(1, 1)?))
    } else {
        (None, None)
    };
    meta.push_descriptor(
        descriptor_id,
        &DescriptorSpec {
            clock_id,
            frame_id: None,
            provenance_id: prov_id,
            medium_id: None,
            transform_id: None,
            calibration_id: None,
            quantity: "validity-mask",
            units: "1",
            channels: Some(1),
            layout: Some("byte"),
            scalar: Some(AuxScalar::U8),
            model_uri: "urn:qac:validity-mask:v1",
            sample_rate,
            valid_record_count: None,
            validity_mask_entry: None,
            source_entries: &[],
            extension_metadata: &[],
        },
    )?;
    Ok(OwnedEntry {
        mode: MODE_VALIDITY_MASK,
        flags,
        stream_id,
        frame_id: 0,
        start_tick: if time_count > 0 { start_tick } else { 0 },
        time_count,
        spatial_key: 0,
        record_count: mask.len() as u64,
        stride: 1,
        descriptor_id,
        payload: mask.to_vec(),
    })
}

/// Encodes, validates, and returns canonical QAC v1 bytes.
pub fn assemble_qac_v1(
    meta: &MetadataBuilder,
    entries: &[OwnedEntry],
    limits: ProfileLimits,
) -> Result<Vec<u8>, ProfileError> {
    for w in entries.windows(2) {
        let ka = (
            w[0].stream_id,
            w[0].start_tick,
            w[0].spatial_key,
            w[0].mode,
            w[0].descriptor_id,
        );
        let kb = (
            w[1].stream_id,
            w[1].start_tick,
            w[1].spatial_key,
            w[1].mode,
            w[1].descriptor_id,
        );
        if ka >= kb {
            return Err(ProfileError::InvalidOrdering);
        }
    }
    let meta_bytes = meta.encode(limits.envelope)?;
    let entry_inputs: Vec<EntryInput<'_>> = entries
        .iter()
        .map(|e| EntryInput {
            mode: e.mode,
            revision: 1,
            flags: e.flags,
            stream_id: e.stream_id,
            frame_id: e.frame_id,
            start_tick: e.start_tick,
            time_count: e.time_count,
            spatial_key: e.spatial_key,
            record_count: e.record_count,
            stride: e.stride,
            codec: 0,
            descriptor_id: e.descriptor_id,
            payload: &e.payload,
        })
        .collect();
    let env_input = EnvelopeInput {
        family: Family::QacV1Candidate,
        flags: 0,
        metadata: &meta_bytes,
        entries: &entry_inputs,
    };
    let len = env_input.encoded_len(limits.envelope)?;
    let mut out = vec![0u8; len];
    let written = env_input.encode_into(&mut out, limits.envelope)?.len();
    out.truncate(written);
    let _ = AdmittedEnvelope::open_alloc(&out, limits)?;
    Ok(out)
}
