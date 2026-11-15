use crate::q42::{emf_profiles::*, field_envelope as env, field_semantics as sem};
#[derive(Clone, Copy)]
pub struct Part {
    pub key: u64,
    pub start: i64,
    pub times: u64,
    pub stream: u32,
    pub electric: [f64; 3],
}
pub fn spec() -> GridSpec {
    GridSpec {
        dimensions: [2, 1, 1],
        brick_dimensions: [2, 1, 1],
        spacing_m: [0.5, 2.0, 3.0],
        indexing: GridIndexing::Linear,
    }
}
pub fn encode(
    spec: GridSpec,
    parts: &[Part],
    step: Option<i64>,
    clock: FieldClock,
    rotation: [f64; 4],
) -> Vec<u8> {
    let layout = spec.validate().unwrap();
    let mut ordered = parts.to_vec();
    ordered.sort_by_key(|p| (p.stream, p.start, p.key));
    let mut payloads = Vec::new();
    for part in &ordered {
        let brick = layout.brick(part.key).unwrap();
        let mut samples = Vec::new();
        for time in 0..if step.is_some() { part.times } else { 1 } {
            for local in 0..brick.cell_count() {
                samples.push(FieldSample {
                    electric_v_per_m: part.electric,
                    magnetic_tesla: [0.0; 3],
                    position_m: brick.local_position_m(local).unwrap(),
                    tick: if let Some(step) = step {
                        part.start + time as i64 * step
                    } else {
                        0
                    },
                    carrier_hz: None,
                    stored_poynting_w_per_m2: None,
                    confidence: None,
                });
            }
        }
        let mut payload = vec![0; samples.len() * 128];
        encode_field_samples(&samples, &mut payload, Limits::default()).unwrap();
        payloads.push(payload);
    }
    let chunks = ordered
        .iter()
        .zip(&payloads)
        .map(|(p, data)| FieldChunkInput {
            stream_id: p.stream,
            start_tick: p.start,
            time_count: p.times,
            spatial_key: Some(p.key),
            payload: data,
        })
        .collect::<Vec<_>>();
    let document = FieldDocument {
        clock,
        frame: FieldFrame {
            origin_m: [1e12, -2e12, 3e12],
            rotation_xyzw: rotation,
            grid: Some(spec),
        },
        provenance: FieldProvenance {
            role: 1,
            algorithm: Some("explicit-portal-fixture"),
            algorithm_version: Some("1"),
            lineage: None,
        },
        temporal_step_ticks: step,
        chunks: &chunks,
        align_payloads_to_4096: false,
    };
    let limits = FieldWriteLimits::default();
    let mut metadata = vec![0; field_metadata_len(&document, limits).unwrap()];
    let mut entries = vec![field_entry_slot(); chunks.len()];
    let prepared = prepare_field_document(&document, &mut metadata, &mut entries, limits).unwrap();
    let n = prepared.encoded_len().unwrap();
    let mut stage = vec![0; n];
    let mut out = vec![0; n];
    let mut m = [sem::NodeState::default(); 16];
    let mut e = vec![sem::NodeState::default(); chunks.len()];
    prepared
        .encode_into(
            &mut stage,
            &mut out,
            &mut sem::Scratch {
                metadata: &mut m,
                entries: &mut e,
            },
        )
        .unwrap();
    out
}
pub fn clock(scale: u32) -> FieldClock {
    FieldClock {
        scale,
        epoch_seconds: -7,
        epoch_nanoseconds: 123,
        ticks_per_second_num: 1000,
        ticks_per_second_den: 1,
    }
}
pub fn simple(electric: [f64; 3]) -> Vec<u8> {
    encode(
        spec(),
        &[Part {
            key: 0,
            start: 0,
            times: 0,
            stream: 0,
            electric,
        }],
        None,
        clock(0),
        [0.0, 0.0, 0.0, 1.0],
    )
}
#[derive(Clone)]
struct F {
    key: u16,
    tag: u8,
    count: u32,
    flags: u8,
    bytes: Vec<u8>,
}
struct B {
    id: u32,
    schema: u16,
    revision: u16,
    flags: u32,
    fields: Vec<F>,
}
fn fields(view: &env::StructuralView<'_>, offset: u32) -> Vec<B> {
    view.blocks()
        .map(|b| B {
            id: b.id() + offset,
            schema: b.schema_type(),
            revision: b.revision(),
            flags: b.flags(),
            fields: b
                .fields()
                .map(|f| {
                    let mut bytes = f.value().to_vec();
                    if f.type_tag() == 8 {
                        for v in bytes.chunks_exact_mut(4) {
                            let id = u32::from_le_bytes(v.try_into().unwrap()) + offset;
                            v.copy_from_slice(&id.to_le_bytes());
                        }
                    }
                    F {
                        key: f.key(),
                        tag: f.type_tag(),
                        count: f.element_count(),
                        flags: f.flags(),
                        bytes,
                    }
                })
                .collect(),
        })
        .collect()
}
fn metadata(blocks: &[B]) -> Vec<u8> {
    let fields = blocks
        .iter()
        .map(|b| {
            b.fields
                .iter()
                .map(|f| env::FieldInput {
                    key: f.key,
                    flags: f.flags,
                    value: env::FieldValue::Raw {
                        type_tag: f.tag,
                        element_count: f.count,
                        bytes: &f.bytes,
                    },
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let blocks = blocks
        .iter()
        .zip(&fields)
        .map(|(b, f)| env::BlockInput {
            id: b.id,
            schema_type: b.schema,
            revision: b.revision,
            flags: b.flags,
            fields: f,
        })
        .collect::<Vec<_>>();
    let input = env::MetadataInput { blocks: &blocks };
    let mut bytes = vec![0; input.encoded_len(env::Limits::default()).unwrap()];
    input
        .encode_into(&mut bytes, env::Limits::default())
        .unwrap();
    bytes
}
fn entry<'a>(e: env::Entry<'_>, payload: &'a [u8], offset: u32) -> env::EntryInput<'a> {
    env::EntryInput {
        mode: e.mode(),
        revision: e.revision(),
        flags: e.flags(),
        stream_id: e.stream_id(),
        frame_id: if e.frame_id() == 0 {
            0
        } else {
            e.frame_id() + offset
        },
        start_tick: e.start_tick(),
        time_count: e.time_count(),
        spatial_key: e.spatial_key(),
        record_count: e.record_count(),
        stride: e.stride(),
        codec: 0,
        descriptor_id: e.descriptor_id() + offset,
        payload,
    }
}
fn seal(metadata: &[u8], entries: &[env::EntryInput<'_>]) -> Vec<u8> {
    let input = env::EnvelopeInput {
        family: env::Family::EmfV4Candidate,
        flags: 0,
        metadata,
        entries,
    };
    let mut out = vec![0; input.encoded_len(env::Limits::default()).unwrap()];
    input.encode_into(&mut out, env::Limits::default()).unwrap();
    out
}
pub fn merge(a: &[u8], b: &[u8]) -> Vec<u8> {
    let av =
        env::StructuralView::open(a, env::Family::EmfV4Candidate, env::Limits::default()).unwrap();
    let bv =
        env::StructuralView::open(b, env::Family::EmfV4Candidate, env::Limits::default()).unwrap();
    let mut blocks = fields(&av, 0);
    blocks.extend(fields(&bv, 5));
    let meta = metadata(&blocks);
    let mut entries = Vec::new();
    for (v, offset) in [(&av, 0), (&bv, 5)] {
        for i in 1..=v.entry_count() {
            entries.push(entry(
                v.entry(i).unwrap(),
                v.verify_chunk(i).unwrap(),
                offset,
            ));
        }
    }
    entries.sort_by_key(|e| {
        (
            e.stream_id,
            e.start_tick,
            e.spatial_key,
            e.mode,
            e.descriptor_id,
        )
    });
    seal(&meta, &entries)
}
fn f(key: u16, tag: u8, bytes: Vec<u8>) -> F {
    let width = match tag {
        1 | 8 | 9 => 4,
        2 | 3 => 8,
        _ => 1,
    };
    F {
        key,
        tag,
        count: (bytes.len() / width) as u32,
        flags: 1,
        bytes,
    }
}
fn n(key: u16, tag: u8, value: u32) -> F {
    f(key, tag, value.to_le_bytes().to_vec())
}
fn s(key: u16, text: &str) -> F {
    f(key, 6, text.as_bytes().to_vec())
}
pub fn masked(source: &[u8], mask: &[u8], orphan: bool) -> Vec<u8> {
    let view =
        env::StructuralView::open(source, env::Family::EmfV4Candidate, env::Limits::default())
            .unwrap();
    assert_eq!(view.entry_count(), 1);
    let mut blocks = fields(&view, 0);
    let mut payload = view.verify_chunk(1).unwrap().to_vec();
    let count = mask.iter().filter(|b| **b == 1).count() as u64;
    if !orphan {
        for (i, value) in mask.iter().enumerate() {
            if *value == 0 {
                payload[i * 128..i * 128 + 128].fill(0);
            }
        }
        let descriptor = blocks.iter_mut().find(|b| b.id == 5).unwrap();
        descriptor
            .fields
            .push(f(15, 2, count.to_le_bytes().to_vec()));
        descriptor.fields.push(n(16, 9, 2));
        descriptor.fields.sort_by_key(|f| f.key);
    }
    blocks.push(B {
        id: 6,
        schema: 103,
        revision: 1,
        flags: 1,
        fields: vec![
            n(1, 1, 0x106),
            n(2, 1, 1),
            n(3, 8, 1),
            n(5, 8, 3),
            s(6, VALIDITY_PROFILE),
            s(7, "linear"),
            f(
                8,
                2,
                [1u64, 1, 1].iter().flat_map(|v| v.to_le_bytes()).collect(),
            ),
            f(
                9,
                1,
                [1u32; 3].iter().flat_map(|v| v.to_le_bytes()).collect(),
            ),
            n(10, 1, 0),
            f(11, 3, 0i64.to_le_bytes().to_vec()),
        ],
    });
    blocks.push(B {
        id: 7,
        schema: 3,
        revision: 1,
        flags: 1,
        fields: vec![
            n(3, 8, 3),
            s(7, "validity"),
            s(8, "1"),
            n(9, 1, 1),
            s(10, "byte"),
            n(11, 1, 7),
            s(12, VALIDITY_MODEL_URI),
            n(18, 8, 6),
        ],
    });
    let meta = metadata(&blocks);
    let entries = [
        entry(view.entry(1).unwrap(), &payload, 0),
        env::EntryInput {
            mode: 0x106,
            revision: 1,
            flags: 0,
            stream_id: 0,
            frame_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: mask.len() as u64,
            stride: 1,
            codec: 0,
            descriptor_id: 7,
            payload: mask,
        },
    ];
    seal(&meta, &entries)
}
