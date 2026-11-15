use super::super::field_envelope::{
    EnvelopeInput, Family, Limits as StorageLimits, StructuralView,
};
use super::*;
fn field_bytes(key: u16, tag: u8, count: u32, critical: bool, value: &[u8]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&key.to_le_bytes());
    out.push(tag);
    out.push(u8::from(critical));
    out.extend_from_slice(&count.to_le_bytes());
    out.extend_from_slice(&(value.len() as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(value);
    while out.len() % 8 != 0 {
        out.push(0);
    }
    out
}
fn text_field(key: u16, text: &str) -> Vec<u8> {
    field_bytes(key, 6, text.len() as u32, true, text.as_bytes())
}
fn float_field(key: u16, values: &[f64]) -> Vec<u8> {
    field_bytes(
        key,
        5,
        values.len() as u32,
        true,
        &values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )
}
fn axis_fields(
    axis: &str,
    semantics: &str,
    quantity: &str,
    unit: &str,
    coords: &[f64],
) -> Vec<Vec<u8>> {
    vec![
        text_field(1, axis),
        text_field(2, semantics),
        text_field(3, quantity),
        text_field(4, unit),
        float_field(5, coords),
        text_field(6, "explicit"),
        text_field(8, "none"),
    ]
}
fn block_bytes(id: u32, schema: u16, fields: &[Vec<u8>]) -> Vec<u8> {
    let body = fields.concat();
    let mut out = Vec::new();
    out.extend_from_slice(&id.to_le_bytes());
    out.extend_from_slice(&schema.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(body.len() as u32).to_le_bytes());
    out.extend_from_slice(&1u32.to_le_bytes());
    out.extend_from_slice(&body);
    out
}
fn storage(metadata: &[u8]) -> Vec<u8> {
    let input = EnvelopeInput {
        family: Family::EmfV4Candidate,
        flags: 0,
        metadata,
        entries: &[],
    };
    let mut bytes = vec![0; input.encoded_len(StorageLimits::default()).unwrap()];
    input
        .encode_into(&mut bytes, StorageLimits::default())
        .unwrap();
    bytes
}
fn encoded(fields: &[Vec<u8>]) -> Vec<u8> {
    storage(&block_bytes(1, 132, fields))
}
fn with_axis<T>(
    fields: &[Vec<u8>],
    samples: u64,
    run: impl FnOnce(Result<AxisView<'_>, Error>) -> T,
) -> T {
    let bytes = encoded(fields);
    let view =
        StructuralView::open(&bytes, Family::EmfV4Candidate, StorageLimits::default()).unwrap();
    run(admit_axis(
        view.blocks().next().unwrap(),
        samples,
        Limits::default(),
        &NoResponse,
    ))
}
fn approx(a: f64, b: f64) {
    assert!(
        (a - b).abs() <= b.abs().max(f64::MIN_POSITIVE) * 8.0 * f64::EPSILON,
        "{a} != {b}"
    );
}
#[path = "axis_tests.rs"]
mod axis_tests;
#[path = "conversion_tests.rs"]
mod conversion_tests;
