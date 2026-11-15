use super::*;
use env::{BlockInput, FieldInput, FieldValue as V, MetadataInput};
fn f(key: u16, value: V<'_>) -> FieldInput<'_> {
    FieldInput {
        key,
        flags: 1,
        value,
    }
}
fn b<'a>(id: u32, schema_type: u16, fields: &'a [FieldInput<'a>]) -> BlockInput<'a> {
    BlockInput {
        id,
        schema_type,
        revision: 1,
        flags: 1,
        fields,
    }
}
fn push<'a>(out: &mut [FieldInput<'a>], n: &mut usize, key: u16, value: V<'a>) {
    out[*n] = f(key, value);
    *n += 1;
}
pub(super) fn encode(
    document: &PcmDocument<'_>,
    records: u64,
    out: &mut [u8],
    limits: env::Limits,
) -> Result<usize, Error> {
    let t = document.timing;
    let scale = [t.scale];
    let epoch = [t.epoch_seconds];
    let ns = [t.epoch_nanoseconds];
    let ticks_n = [t.ticks.numerator];
    let ticks_d = [t.ticks.denominator];
    let clock = [
        f(1, V::U32(&scale)),
        f(2, V::I64(&epoch)),
        f(3, V::U32(&ns)),
        f(4, V::U64(&ticks_n)),
        f(5, V::U64(&ticks_d)),
    ];
    let empty = f(0, V::Bytes(&[]));
    let role = [document.provenance.role as u32];
    let digest = document
        .original
        .map(|raw| *blake3::hash(raw).as_bytes())
        .unwrap_or([0; 32]);
    let mut provenance = [empty; 7];
    let mut pn = 0;
    push(&mut provenance, &mut pn, 1, V::U32(&role));
    if document.original.is_some() {
        push(&mut provenance, &mut pn, 2, V::Bytes(&digest));
    }
    if let Some(s) = document.provenance.algorithm {
        push(&mut provenance, &mut pn, 3, V::Utf8(s));
    }
    if let Some(s) = document.provenance.algorithm_version {
        push(&mut provenance, &mut pn, 4, V::Utf8(s));
    }
    if let Some(s) = document.provenance.instrument_uri {
        push(&mut provenance, &mut pn, 5, V::Utf8(s));
    }
    if let Some(s) = document.provenance.lineage {
        push(&mut provenance, &mut pn, 7, V::Utf8(s));
    }
    let dimensions = [records, u64::from(document.channels)];
    let shape = [
        f(1, V::Utf8("urn:qualia:qac:pcm-preservation:1")),
        f(2, V::Utf8("1")),
        f(14, V::U64(&dimensions)),
        f(15, V::Utf8("sample")),
    ];
    let gain = [document.units_per_stored_sample];
    let offset = [document.stored_offset];
    let encoding = [
        f(1, V::Utf8(document.channel_order)),
        f(2, V::F64(&gain)),
        f(3, V::F64(&offset)),
        f(4, V::Utf8(document.purpose)),
        f(5, V::Utf8(document.relation.name())),
    ];
    let (quantity, units) = document.quantity.names();
    let clock_id = [1];
    let prov_id = [2];
    let channels = [document.channels];
    let scalar = [document.samples.scalar() as u32];
    let rate_n = [t.samples.numerator];
    let rate_d = [t.samples.denominator];
    let source_entry = [1];
    let extensions = [3, 4];
    let mut descriptor = [empty; 12];
    let mut dn = 0;
    for (key, value) in [
        (1, V::MetadataRefs(&clock_id)),
        (3, V::MetadataRefs(&prov_id)),
        (7, V::Utf8(quantity)),
        (8, V::Utf8(units)),
        (9, V::U32(&channels)),
        (10, V::Utf8("time-channel")),
        (11, V::U32(&scalar)),
        (12, V::Utf8("urn:qualia:qac:pcm-preservation:1")),
        (13, V::U64(&rate_n)),
        (14, V::U64(&rate_d)),
    ] {
        push(&mut descriptor, &mut dn, key, value);
    }
    if document.original.is_some() {
        push(&mut descriptor, &mut dn, 17, V::EntryRefs(&source_entry));
    }
    push(&mut descriptor, &mut dn, 18, V::MetadataRefs(&extensions));
    let opaque_role = [4];
    let original_len = [document.original.map_or(0, |s| s.len() as u64)];
    let original_prov = [6];
    let source_shape_id = [7];
    let one = [1];
    let byte_scalar = [7];
    let original_provenance = [f(1, V::U32(&opaque_role))];
    let original_shape = [
        f(1, V::Utf8("urn:qualia:qac:opaque-original:1")),
        f(2, V::Utf8("1")),
        f(14, V::U64(&original_len)),
        f(15, V::Utf8("byte")),
    ];
    let original_descriptor = [
        f(3, V::MetadataRefs(&original_prov)),
        f(7, V::Utf8("original_bytes")),
        f(8, V::Utf8("1")),
        f(9, V::U32(&one)),
        f(10, V::Utf8("byte")),
        f(11, V::U32(&byte_scalar)),
        f(12, V::Utf8("urn:qualia:qac:opaque-original:1")),
        f(18, V::MetadataRefs(&source_shape_id)),
    ];
    let blocks = [
        b(1, 1, &clock),
        b(2, 4, &provenance[..pn]),
        b(3, 205, &shape),
        b(4, 210, &encoding),
        b(5, 3, &descriptor[..dn]),
        b(6, 4, &original_provenance),
        b(7, 205, &original_shape),
        b(8, 3, &original_descriptor),
    ];
    let count = if document.original.is_some() { 8 } else { 5 };
    Ok(MetadataInput {
        blocks: &blocks[..count],
    }
    .encode_into(out, limits)?
    .len())
}
