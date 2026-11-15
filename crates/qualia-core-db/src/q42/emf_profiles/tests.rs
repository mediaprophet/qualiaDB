use super::super::field_envelope::{EntryInput, EnvelopeInput, Family, Limits as StoreLimits};
use super::*;
#[derive(Clone)]
struct Meta {
    id: u32,
    schema: u16,
    fields: Vec<Vec<u8>>,
}
fn f(key: u16, tag: u8, count: u32, bytes: &[u8]) -> Vec<u8> {
    let mut v = Vec::new();
    v.extend_from_slice(&key.to_le_bytes());
    v.push(tag);
    v.push(1);
    v.extend_from_slice(&count.to_le_bytes());
    v.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
    v.extend_from_slice(&0u32.to_le_bytes());
    v.extend_from_slice(bytes);
    while v.len() % 8 != 0 {
        v.push(0);
    }
    v
}
fn n32(key: u16, tag: u8, n: u32) -> Vec<u8> {
    f(key, tag, 1, &n.to_le_bytes())
}
fn n64(key: u16, tag: u8, n: u64) -> Vec<u8> {
    f(key, tag, 1, &n.to_le_bytes())
}
fn s(key: u16, text: &str) -> Vec<u8> {
    f(key, 6, text.len() as u32, text.as_bytes())
}
fn numbers(key: u16, values: &[f64]) -> Vec<u8> {
    f(
        key,
        5,
        values.len() as u32,
        &values
            .iter()
            .flat_map(|v| v.to_le_bytes())
            .collect::<Vec<_>>(),
    )
}
fn put(bytes: &mut [u8], at: usize, value: f64) {
    bytes[at..at + 8].copy_from_slice(&value.to_le_bytes());
}
fn field_payload(tick: i64) -> Vec<u8> {
    let mut b = vec![0; 128];
    put(&mut b, 0, 1.0);
    put(&mut b, 32, 1.0 / spectrum::VACUUM_SPEED_OF_LIGHT_M_PER_S);
    b[72..80].copy_from_slice(&tick.to_le_bytes());
    b
}
fn ray_payload() -> Vec<u8> {
    let mut b = vec![0; 128];
    put(&mut b, 40, 1.0);
    put(&mut b, 48, 1.0);
    put(&mut b, 72, -1.0);
    put(&mut b, 80, 2.4e9);
    put(&mut b, 88, 20e6);
    put(&mut b, 96, 0.299792458);
    put(&mut b, 112, 0.1);
    b
}
fn spectrum_payload(tick: i64) -> Vec<u8> {
    let mut b = vec![0; 128];
    b[24..32].copy_from_slice(&tick.to_le_bytes());
    b[36..40].copy_from_slice(&6u32.to_le_bytes());
    for (i, v) in [1e-12, 2e-12, 3e-12, 4e-12].iter().enumerate() {
        put(&mut b, 64 + i * 8, *v);
    }
    b
}
fn metadata(mode: u16, temporal: bool) -> Vec<Meta> {
    let (profile, quantity, units, model) = match mode {
        0 => (
            FIELD_PROFILE,
            "electromagnetic_field_snapshot",
            "mixed:mode-registry",
            FIELD_MODEL_URI,
        ),
        1 => (RAY_PROFILE, "polarimetric_ray", "W m^-2", RAY_MODEL_URI),
        3 => (
            SPECTRUM_PROFILE,
            "spectral_irradiance",
            "W m^-2 Hz^-1",
            SPECTRUM_MODEL_URI,
        ),
        _ => panic!(),
    };
    let mut ext = vec![
        n32(1, 1, u32::from(mode)),
        n32(2, 1, 1),
        n32(3, 8, 1),
        n32(4, 8, 2),
        n32(5, 8, 3),
        s(6, profile),
        s(7, "linear"),
        f(
            8,
            2,
            3,
            &[1u64, 1, 1]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        ),
        f(
            9,
            1,
            3,
            &[1u32, 1, 1]
                .iter()
                .flat_map(|v| v.to_le_bytes())
                .collect::<Vec<_>>(),
        ),
        n32(10, 1, 0),
        n64(11, 3, if temporal { 2 } else { 0 }),
    ];
    if mode == 1 {
        ext.push(n32(16, 8, 6));
    }
    if mode == 3 {
        ext.push(n32(12, 8, 6));
        ext.push(n32(19, 1, 4));
    }
    let mut desc = vec![
        n32(2, 8, 2),
        n32(3, 8, 3),
        s(7, quantity),
        s(8, units),
        s(12, model),
        n32(18, 8, 4),
    ];
    if temporal {
        desc.push(n32(1, 8, 1));
        desc.push(n64(13, 2, 500));
        desc.push(n64(14, 2, 1));
    }
    let mut m = vec![
        Meta {
            id: 1,
            schema: 1,
            fields: vec![
                n32(1, 1, 0),
                n64(2, 3, 0),
                n32(3, 1, 0),
                n64(4, 2, 1000),
                n64(5, 2, 1),
            ],
        },
        Meta {
            id: 2,
            schema: 2,
            fields: vec![
                n32(1, 1, 0),
                numbers(3, &[1e9, 2e9, 3e9]),
                numbers(4, &[0.0, 0.0, 0.0, 1.0]),
                s(8, "right-handed-X-forward-Y-left-Z-up"),
            ],
        },
        Meta {
            id: 3,
            schema: 4,
            fields: vec![n32(1, 1, 0)],
        },
        Meta {
            id: 4,
            schema: 103,
            fields: ext,
        },
        Meta {
            id: 5,
            schema: 3,
            fields: desc,
        },
    ];
    if mode == 1 {
        m.push(Meta {
            id: 6,
            schema: 135,
            fields: vec![
                s(1, "deterministic_transverse_v1"),
                s(2, "I_Q_U_V_v1"),
                s(3, "W m^-2"),
                s(4, "time_averaged"),
            ],
        });
    }
    if mode == 3 {
        m.push(Meta {
            id: 6,
            schema: 132,
            fields: vec![
                s(1, "frequency_hz"),
                s(2, "point_density"),
                s(3, "spectral_irradiance"),
                s(4, "W m^-2 Hz^-1"),
                numbers(5, &[1e6, 2.4e9, 6e14, 3e20]),
                s(6, "explicit"),
                s(8, "none"),
            ],
        });
    }
    m
}
fn row<'a>(mode: u16, payload: &'a [u8], count: u64, temporal: bool) -> EntryInput<'a> {
    EntryInput {
        mode,
        revision: 1,
        flags: if temporal { 2 } else { 0 },
        stream_id: if temporal { 1 } else { 0 },
        frame_id: 2,
        start_tick: if temporal { -2 } else { 0 },
        time_count: if temporal { count } else { 0 },
        spatial_key: 0,
        record_count: count,
        stride: (payload.len() as u64 / count) as u32,
        codec: 0,
        descriptor_id: 5,
        payload,
    }
}
fn encode(metadata: &[Meta], rows: &[EntryInput<'_>]) -> Vec<u8> {
    let mut bytes = Vec::new();
    for m in metadata {
        let mut fields = m.fields.clone();
        fields.sort_by_key(|v| u16::from_le_bytes(v[..2].try_into().unwrap()));
        let body = fields.concat();
        bytes.extend_from_slice(&m.id.to_le_bytes());
        bytes.extend_from_slice(&m.schema.to_le_bytes());
        bytes.extend_from_slice(&1u16.to_le_bytes());
        bytes.extend_from_slice(&(body.len() as u32).to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes.extend_from_slice(&body);
    }
    let input = EnvelopeInput {
        family: Family::EmfV4Candidate,
        flags: 0,
        metadata: &bytes,
        entries: rows,
    };
    let mut out = vec![0; input.encoded_len(StoreLimits::default()).unwrap()];
    input.encode_into(&mut out, StoreLimits::default()).unwrap();
    out
}
fn admit<'a>(view: &StructuralView<'a>) -> Result<ProfileChunk<'a>, Error> {
    let mut metadata = [semantics::NodeState::default(); 32];
    let mut entries = [semantics::NodeState::default(); 8];
    admit_chunk(
        view,
        1,
        &mut Scratch {
            metadata: &mut metadata,
            entries: &mut entries,
        },
        Limits::default(),
    )
}
fn check(
    mode: u16,
    payload: &[u8],
    count: u64,
    temporal: bool,
    metadata: &[Meta],
) -> Result<(), Error> {
    let encoded = encode(metadata, &[row(mode, payload, count, temporal)]);
    let view =
        StructuralView::open(&encoded, Family::EmfV4Candidate, StoreLimits::default()).unwrap();
    admit(&view).map(|_| ())
}
fn replace(metadata: &mut [Meta], id: u32, key: u16, value: Vec<u8>) {
    let m = metadata.iter_mut().find(|m| m.id == id).unwrap();
    if let Some(f) = m
        .fields
        .iter_mut()
        .find(|f| u16::from_le_bytes(f[..2].try_into().unwrap()) == key)
    {
        *f = value;
    } else {
        m.fields.push(value);
    }
}
#[path = "profile_tests.rs"]
mod profile_tests;
#[path = "record_tests.rs"]
mod record_tests;

#[path = "validity_tests.rs"]
mod validity_tests;

#[path = "spatial_tests.rs"]
mod spatial_tests;

#[path = "field_writer_tests.rs"]
mod field_writer_tests;

#[path = "document_tests.rs"]
mod document_tests;
