use crate::q42::field_envelope::{self as env, EntryInput, EnvelopeInput, Family};
#[derive(Clone)]
pub struct F {
    pub key: u16,
    pub tag: u8,
    pub count: u32,
    pub critical: u8,
    pub bytes: Vec<u8>,
}
pub struct B {
    pub id: u32,
    pub schema: u16,
    pub fields: Vec<F>,
}
pub fn f(key: u16, tag: u8, bytes: Vec<u8>) -> F {
    let width = match tag {
        1 | 4 | 8 | 9 | 11 => 4,
        2 | 3 | 5 => 8,
        10 => 2,
        _ => 1,
    };
    F {
        key,
        tag,
        count: (bytes.len() / width) as u32,
        critical: 1,
        bytes,
    }
}
pub fn u32f(k: u16, v: u32) -> F {
    f(k, 1, v.to_le_bytes().to_vec())
}
pub fn ref_f(k: u16, v: u32) -> F {
    f(k, 8, v.to_le_bytes().to_vec())
}
pub fn u64f(k: u16, v: u64) -> F {
    f(k, 2, v.to_le_bytes().to_vec())
}
pub fn strf(k: u16, v: &str) -> F {
    f(k, 6, v.as_bytes().to_vec())
}
pub fn block(id: u32, schema: u16, fields: Vec<F>) -> B {
    B { id, schema, fields }
}
pub fn field<'a>(bs: &'a mut [B], id: u32, k: u16) -> &'a mut F {
    bs.iter_mut()
        .find(|b| b.id == id)
        .unwrap()
        .fields
        .iter_mut()
        .find(|f| f.key == k)
        .unwrap()
}
fn tlv(head: &[u8; 16], body: &[u8], out: &mut Vec<u8>) {
    out.extend_from_slice(head);
    out.extend_from_slice(body);
    while out.len() % 8 != 0 {
        out.push(0)
    }
}
pub fn metadata(bs: &[B]) -> Vec<u8> {
    let mut out = Vec::new();
    for b in bs {
        let mut body = Vec::new();
        for f in &b.fields {
            let mut h = [0; 16];
            h[..2].copy_from_slice(&f.key.to_le_bytes());
            h[2] = f.tag;
            h[3] = f.critical;
            h[4..8].copy_from_slice(&f.count.to_le_bytes());
            h[8..12].copy_from_slice(&(f.bytes.len() as u32).to_le_bytes());
            tlv(&h, &f.bytes, &mut body);
        }
        let mut h = [0; 16];
        h[..4].copy_from_slice(&b.id.to_le_bytes());
        h[4..6].copy_from_slice(&b.schema.to_le_bytes());
        h[6..8].copy_from_slice(&1u16.to_le_bytes());
        h[8..12].copy_from_slice(&(body.len() as u32).to_le_bytes());
        h[12..16].copy_from_slice(&1u32.to_le_bytes());
        tlv(&h, &body, &mut out);
    }
    out
}
#[derive(Clone, Copy)]
pub struct Config {
    pub scalar: u32,
    pub channels: u32,
    pub spectral: bool,
    pub source: bool,
    pub start: i64,
}
impl Default for Config {
    fn default() -> Self {
        Self {
            scalar: 1,
            channels: 1,
            spectral: false,
            source: false,
            start: 0,
        }
    }
}
pub const ORIGINAL: &[u8] = include_bytes!("../fixtures/q4au-v2-contract.qac");
pub fn build(payload: &[u8], c: Config, change: impl FnOnce(&mut Vec<B>)) -> Vec<u8> {
    let width = match c.scalar {
        1 | 5 => 4,
        2 | 6 => 8,
        3 => 2,
        4 => 3,
        _ => 1,
    };
    let bins = if c.spectral { 4 } else { 1 };
    let stride = c.channels * bins * width;
    let records = payload.len() as u64 / u64::from(stride);
    let raw = matches!(c.scalar, 3 | 4 | 5);
    let channels = (0..c.channels)
        .map(|i| format!("channel-{i}"))
        .collect::<Vec<_>>()
        .join(",");
    let rate = if c.spectral {
        (2, 1)
    } else {
        (48_000_000, 1001)
    };
    let ticks = if c.spectral { (4, 1) } else { rate };
    let model = if c.spectral {
        "urn:qualia:qac:stft-preservation:1"
    } else {
        "urn:qualia:qac:pcm-preservation:1"
    };
    let quantity = if c.spectral {
        if c.scalar == 6 {
            "complex_digital_spectrum"
        } else {
            "magnitude_digital_spectrum"
        }
    } else {
        if raw {
            "raw_sample_count"
        } else {
            "digital_sample"
        }
    };
    let mut shape = vec![records, u64::from(c.channels)];
    if c.spectral {
        shape.push(4)
    }
    let mut bs = vec![
        block(
            1,
            1,
            vec![
                u32f(1, 0),
                f(2, 3, 0i64.to_le_bytes().to_vec()),
                u32f(3, 0),
                u64f(4, ticks.0),
                u64f(5, ticks.1),
            ],
        ),
        block(2, 4, vec![u32f(1, if c.source { 2 } else { 0 })]),
        block(
            3,
            205,
            vec![
                strf(1, model),
                strf(2, "1"),
                f(14, 2, shape.iter().flat_map(|x| x.to_le_bytes()).collect()),
                strf(
                    15,
                    if c.spectral {
                        if c.scalar == 6 {
                            "real,imag"
                        } else {
                            "magnitude"
                        }
                    } else {
                        "sample"
                    },
                ),
            ],
        ),
        block(
            4,
            210,
            vec![
                strf(1, &channels),
                f(2, 5, 1f64.to_le_bytes().to_vec()),
                f(3, 5, 0f64.to_le_bytes().to_vec()),
                strf(4, "original-observation"),
                strf(
                    5,
                    if raw {
                        "raw-counts"
                    } else {
                        "digital-unrelated-to-pressure"
                    },
                ),
            ],
        ),
    ];
    if c.spectral {
        bs.push(block(
            5,
            201,
            vec![
                strf(
                    1,
                    if c.scalar == 6 {
                        "STFT-complex"
                    } else {
                        "STFT-magnitude"
                    },
                ),
                u32f(2, 4),
                u32f(3, 2),
                strf(4, "rectangular"),
                strf(6, "forward-unscaled-inverse-1/N"),
                u32f(7, 0),
                strf(9, "forward-negative-exponent"),
                f(10, 3, 0i64.to_le_bytes().to_vec()),
                strf(11, "none"),
                strf(12, &channels),
                u32f(13, 4),
                u64f(14, 4),
                u64f(15, 1),
            ],
        ));
    }
    let mut df = vec![ref_f(1, 1), ref_f(3, 2)];
    if c.spectral {
        df.push(ref_f(5, 5));
    }
    df.extend([
        strf(7, quantity),
        strf(8, "1"),
        u32f(9, c.channels),
        strf(
            10,
            if c.spectral {
                "frame-channel-bin"
            } else {
                "time-channel"
            },
        ),
        u32f(11, c.scalar),
        strf(12, model),
        u64f(13, rate.0),
        u64f(14, rate.1),
    ]);
    if c.source {
        df.push(f(17, 9, 1u32.to_le_bytes().to_vec()));
        bs[1]
            .fields
            .push(f(2, 7, blake3::hash(ORIGINAL).as_bytes().to_vec()));
    }
    df.push(f(18, 8, [3u32.to_le_bytes(), 4u32.to_le_bytes()].concat()));
    bs.push(block(6, 3, df));
    if c.source {
        bs.extend([
            block(7, 4, vec![u32f(1, 4)]),
            block(
                8,
                205,
                vec![
                    strf(1, "urn:qualia:qac:opaque-original:1"),
                    strf(2, "1"),
                    u64f(14, ORIGINAL.len() as u64),
                    strf(15, "byte"),
                ],
            ),
            block(
                9,
                3,
                vec![
                    ref_f(3, 7),
                    strf(7, "original_bytes"),
                    strf(8, "1"),
                    u32f(9, 1),
                    strf(10, "byte"),
                    u32f(11, 7),
                    strf(12, "urn:qualia:qac:opaque-original:1"),
                    f(18, 8, 8u32.to_le_bytes().to_vec()),
                ],
            ),
        ]);
    }
    change(&mut bs);
    for b in &mut bs {
        for f in &mut b.fields {
            let width = match f.tag {
                1 | 4 | 8 | 9 | 11 => 4,
                2 | 3 | 5 => 8,
                10 => 2,
                _ => 1,
            };
            f.count = (f.bytes.len() / width) as u32;
        }
    }
    let meta = metadata(&bs);
    let mut entries = Vec::new();
    if c.source {
        entries.push(EntryInput {
            mode: 0x104,
            revision: 1,
            flags: 0,
            stream_id: 0,
            frame_id: 0,
            start_tick: 0,
            time_count: 0,
            spatial_key: 0,
            record_count: ORIGINAL.len() as u64,
            stride: 1,
            codec: 0,
            descriptor_id: 9,
            payload: ORIGINAL,
        });
    }
    entries.push(EntryInput {
        mode: if c.spectral { 0x101 } else { 0x100 },
        revision: 1,
        flags: 2,
        stream_id: 1,
        frame_id: 0,
        start_tick: c.start,
        time_count: records,
        spatial_key: 0,
        record_count: records,
        stride,
        codec: 0,
        descriptor_id: 6,
        payload,
    });
    let input = EnvelopeInput {
        family: Family::QacV1Candidate,
        flags: 0,
        metadata: &meta,
        entries: &entries,
    };
    let mut out = vec![0; input.encoded_len(env::Limits::default()).unwrap()];
    input.encode_into(&mut out, env::Limits::default()).unwrap();
    out
}
