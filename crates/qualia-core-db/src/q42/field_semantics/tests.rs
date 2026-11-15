use super::super::field_envelope::{EntryInput, EnvelopeInput, Family, Limits as StorageLimits};
use super::*;
#[derive(Clone)]
struct F {
    key: u16,
    tag: u8,
    critical: bool,
    count: u32,
    value: Vec<u8>,
}
#[derive(Clone)]
struct B {
    id: u32,
    schema: u16,
    revision: u16,
    critical: bool,
    fields: Vec<F>,
}
fn f(key: u16, tag: u8, count: u32, value: Vec<u8>) -> F {
    F {
        key,
        tag,
        count,
        value,
        critical: true,
    }
}
fn n32(key: u16, tag: u8, value: u32) -> F {
    f(key, tag, 1, value.to_le_bytes().to_vec())
}
fn n64(key: u16, tag: u8, value: u64) -> F {
    f(key, tag, 1, value.to_le_bytes().to_vec())
}
fn string(key: u16, value: &str) -> F {
    f(key, 6, value.len() as u32, value.as_bytes().to_vec())
}
fn floats(key: u16, values: &[f64]) -> F {
    f(
        key,
        5,
        values.len() as u32,
        values.iter().flat_map(|v| v.to_le_bytes()).collect(),
    )
}
fn block(id: u32, schema: u16, fields: Vec<F>) -> B {
    B {
        id,
        schema,
        revision: 1,
        critical: true,
        fields,
    }
}
fn good() -> Vec<B> {
    vec![
        block(
            1,
            1,
            vec![
                n32(1, 1, 0),
                n64(2, 3, 0),
                n32(3, 1, 0),
                n64(4, 2, 96000),
                n64(5, 2, 1),
            ],
        ),
        block(
            2,
            2,
            vec![
                n32(1, 1, 0),
                floats(3, &[0.0, 0.0, 0.0]),
                floats(4, &[0.0, 0.0, 0.0, 1.0]),
                floats(5, &[1.0, 1.0, 1.0]),
                f(
                    6,
                    1,
                    3,
                    [2u32, 3, 4].iter().flat_map(|v| v.to_le_bytes()).collect(),
                ),
                n32(7, 1, 0),
                string(8, "right-handed-X-forward-Y-left-Z-up"),
            ],
        ),
        block(3, 4, vec![n32(1, 1, 4)]),
        block(
            4,
            3,
            vec![
                n32(2, 8, 2),
                n32(3, 8, 3),
                string(7, "opaque-original"),
                string(8, "1"),
                string(12, "urn:test:opaque"),
            ],
        ),
        block(
            5,
            5,
            vec![
                n32(1, 8, 1),
                n64(2, 3, (-1i64) as u64),
                n64(3, 2, 1),
                n64(4, 2, 2),
            ],
        ),
    ]
}
fn row() -> EntryInput<'static> {
    EntryInput {
        mode: 0x104,
        revision: 1,
        flags: 4,
        stream_id: 0,
        frame_id: 2,
        start_tick: 0,
        time_count: 0,
        spatial_key: 23,
        record_count: 1,
        stride: 1,
        codec: 0,
        descriptor_id: 4,
        payload: b"a",
    }
}
fn bytes(blocks: &[B], rows: &[EntryInput<'_>]) -> Vec<u8> {
    let mut metadata = Vec::new();
    for b in blocks {
        let mut body = Vec::new();
        for f in &b.fields {
            body.extend_from_slice(&f.key.to_le_bytes());
            body.push(f.tag);
            body.push(u8::from(f.critical));
            body.extend_from_slice(&f.count.to_le_bytes());
            body.extend_from_slice(&(f.value.len() as u32).to_le_bytes());
            body.extend_from_slice(&0u32.to_le_bytes());
            body.extend_from_slice(&f.value);
            while body.len() % 8 != 0 {
                body.push(0);
            }
        }
        metadata.extend_from_slice(&b.id.to_le_bytes());
        metadata.extend_from_slice(&b.schema.to_le_bytes());
        metadata.extend_from_slice(&b.revision.to_le_bytes());
        metadata.extend_from_slice(&(body.len() as u32).to_le_bytes());
        metadata.extend_from_slice(&u32::from(b.critical).to_le_bytes());
        metadata.extend_from_slice(&body);
        while metadata.len() % 8 != 0 {
            metadata.push(0);
        }
    }
    let input = EnvelopeInput {
        family: Family::QacV1Candidate,
        flags: 0,
        metadata: &metadata,
        entries: rows,
    };
    let mut out = vec![0; input.encoded_len(StorageLimits::default()).unwrap()];
    input
        .encode_into(&mut out, StorageLimits::default())
        .unwrap();
    out
}
fn result_with(bytes: &[u8], registry: &impl DomainRegistry, limits: Limits) -> Result<(), Error> {
    let view =
        StructuralView::open(bytes, Family::QacV1Candidate, StorageLimits::default()).unwrap();
    let mut m = [NodeState::default(); 64];
    let mut e = [NodeState::default(); 64];
    admit_shared(
        &view,
        registry,
        &mut Scratch {
            metadata: &mut m,
            entries: &mut e,
        },
        limits,
    )
    .map(|_| ())
}
fn result(b: &[B], r: &[EntryInput<'_>]) -> Result<(), Error> {
    result_with(&bytes(b, r), &SharedOnly, Limits::default())
}
fn change<'a>(blocks: &'a mut [B], schema: u16, key: u16) -> &'a mut F {
    blocks
        .iter_mut()
        .find(|b| b.schema == schema)
        .unwrap()
        .fields
        .iter_mut()
        .find(|f| f.key == key)
        .unwrap()
}
#[path = "graph_tests.rs"]
mod graph_tests;
#[path = "schema_tests.rs"]
mod schema_tests;
#[path = "validity_tests.rs"]
mod validity_tests;

#[path = "spatial_tests.rs"]
mod spatial_tests;
