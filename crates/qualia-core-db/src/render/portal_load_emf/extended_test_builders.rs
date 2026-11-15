use crate::container_10d::crc32c::crc32c;
use crate::q42::emf_profiles as profiles;
use crate::q42::field_envelope as env;

#[derive(Clone)]
pub(super) struct Tf {
    pub key: u16,
    pub tag: u8,
    pub count: u32,
    pub flags: u8,
    pub bytes: Vec<u8>,
}

pub(super) struct Tb {
    pub id: u32,
    pub schema: u16,
    pub fields: Vec<Tf>,
}

pub(super) fn tf(key: u16, tag: u8, bytes: Vec<u8>) -> Tf {
    let w = match tag {
        1 | 8 | 9 => 4,
        2 | 3 | 5 => 8,
        _ => 1,
    };
    Tf { key, tag, count: (bytes.len() / w) as u32, flags: 1, bytes }
}
pub(super) fn tu32(key: u16, v: u32) -> Tf { tf(key, 1, v.to_le_bytes().to_vec()) }
pub(super) fn tref(key: u16, id: u32) -> Tf { tf(key, 8, id.to_le_bytes().to_vec()) }
pub(super) fn tchunk(key: u16, id: u32) -> Tf { tf(key, 9, id.to_le_bytes().to_vec()) }
pub(super) fn tu64(key: u16, v: u64) -> Tf { tf(key, 2, v.to_le_bytes().to_vec()) }
pub(super) fn ti64(key: u16, v: i64) -> Tf { tf(key, 3, v.to_le_bytes().to_vec()) }
pub(super) fn tf64(key: u16, v: f64) -> Tf { tf(key, 5, v.to_le_bytes().to_vec()) }
pub(super) fn tstr(key: u16, s: &str) -> Tf { tf(key, 6, s.as_bytes().to_vec()) }
pub(super) fn tf64_arr(key: u16, xs: &[f64]) -> Tf {
    tf(key, 5, xs.iter().flat_map(|v| v.to_le_bytes()).collect())
}
pub(super) fn tu32_arr(key: u16, xs: &[u32]) -> Tf {
    tf(key, 1, xs.iter().flat_map(|v| v.to_le_bytes()).collect())
}
pub(super) fn tu64_arr(key: u16, xs: &[u64]) -> Tf {
    tf(key, 2, xs.iter().flat_map(|v| v.to_le_bytes()).collect())
}
pub(super) fn ti64_arr(key: u16, xs: &[i64]) -> Tf {
    tf(key, 3, xs.iter().flat_map(|v| v.to_le_bytes()).collect())
}

pub(super) fn seal_blocks(mut blocks: Vec<Tb>, mut entries: Vec<env::EntryInput<'_>>) -> Vec<u8> {
    blocks.sort_by_key(|b| b.id);
    for b in &mut blocks {
        b.fields.sort_by_key(|f| f.key);
    }
    let fields: Vec<Vec<env::FieldInput<'_>>> = blocks
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
                .collect()
        })
        .collect();
    let meta_blocks: Vec<env::BlockInput<'_>> = blocks
        .iter()
        .zip(&fields)
        .map(|(b, f)| env::BlockInput {
            id: b.id,
            schema_type: b.schema,
            revision: 1,
            flags: 1,
            fields: f,
        })
        .collect();
    let meta_in = env::MetadataInput { blocks: &meta_blocks };
    let mut meta = vec![0; meta_in.encoded_len(env::Limits::default()).unwrap()];
    meta_in.encode_into(&mut meta, env::Limits::default()).unwrap();
    entries.sort_by_key(|e| (e.stream_id, e.start_tick, e.spatial_key, e.mode, e.descriptor_id));
    let env_in = env::EnvelopeInput {
        family: env::Family::EmfV4Candidate,
        flags: 0,
        metadata: &meta,
        entries: &entries,
    };
    let mut out = vec![0; env_in.encoded_len(env::Limits::default()).unwrap()];
    env_in.encode_into(&mut out, env::Limits::default()).unwrap();
    out
}

pub(super) fn base_blocks(grid: Option<([f64; 3], [u32; 3])>) -> Vec<Tb> {
    let mut frame_fields = vec![
        tu32(1, 0),
        tf64_arr(3, &[0.0, 0.0, 0.0]),
        tf64_arr(4, &[0.0, 0.0, 0.0, 1.0]),
        tstr(8, "right-handed-X-forward-Y-left-Z-up"),
    ];
    if let Some((sp, dims)) = grid {
        frame_fields.push(tf64_arr(5, &sp));
        frame_fields.push(tu32_arr(6, &dims));
        frame_fields.push(tu32(7, 0));
    }
    vec![
        Tb {
            id: 1,
            schema: 1,
            fields: vec![tu32(1, 0), ti64(2, 0), tu32(3, 0), tu64(4, 1000), tu64(5, 1)],
        },
        Tb { id: 2, schema: 2, fields: frame_fields },
        Tb { id: 3, schema: 4, fields: vec![tu32(1, 1)] },
    ]
}

pub(super) fn desc_block(id: u32, qty: &str, unit: &str, ext_id: Option<u32>, mask_chunk: Option<(u32, u64)>) -> Tb {
    let mut fields = vec![
        tref(2, 2),
        tref(3, 3),
        tstr(7, qty),
        tstr(8, unit),
        tstr(12, "qualia://emf/v4"),
    ];
    if let Some((_, valid_n)) = mask_chunk {
        fields.push(tu64(15, valid_n));
    }
    if let Some((mask_idx, _)) = mask_chunk {
        fields.push(tchunk(16, mask_idx));
    }
    if let Some(eid) = ext_id {
        fields.push(tref(18, eid));
    }
    Tb { id, schema: 3, fields }
}

pub(super) fn ext_block(id: u32, mode: u32, profile: &str, grid_dims: [u64; 3], brick: [u32; 3], extra: Vec<Tf>) -> Tb {
    let mut fields = vec![
        tu32(1, mode),
        tu32(2, 1),
        tref(3, 1),
        tref(4, 2),
        tref(5, 3),
        tstr(6, profile),
        tstr(7, "linear"),
        tu64_arr(8, &grid_dims),
        tu32_arr(9, &brick),
        tu32(10, 0),
        ti64(11, 0),
    ];
    fields.extend(extra);
    Tb { id, schema: 103, fields }
}

pub(super) fn pol_block(id: u32, unit: &str) -> Tb {
    Tb {
        id,
        schema: 135,
        fields: vec![
            tstr(1, "deterministic_transverse_v1"),
            tstr(2, "I_Q_U_V_v1"),
            tstr(3, unit),
            tstr(4, "time_averaged"),
        ],
    }
}

pub(super) fn ev_block(id: u32, ionizing: bool) -> Tb {
    let mut fields = vec![
        tstr(1, "e02-fixture-v1"),
        tstr(2, "local-domain"),
        tf(3, 7, vec![0xAA; 32]),
        tstr(4, "si-right-handed"),
        tstr(8, "empirical"),
    ];
    if ionizing {
        fields.push(tstr(9, "photon"));
        fields.push(ti64_arr(10, &[0, 100]));
        fields.push(tref(12, 1));
    }
    Tb { id, schema: 134, fields }
}

pub(super) fn calib_block(id: u32, unit: &str, scale: f64, offset: f64) -> Tb {
    Tb {
        id,
        schema: 107,
        fields: vec![
            tstr(1, "e02-sensor"),
            tf(2, 7, vec![0xBB; 32]),
            tstr(3, unit),
            tf64(4, scale),
            tf64(5, offset),
        ],
    }
}

pub(super) fn wr_f64(buf: &mut [u8], off: usize, v: f64) {
    buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
}
pub(super) fn wr_u32(buf: &mut [u8], off: usize, v: u32) {
    buf[off..off + 4].copy_from_slice(&v.to_le_bytes());
}
pub(super) fn wr_i64(buf: &mut [u8], off: usize, v: i64) {
    buf[off..off + 8].copy_from_slice(&v.to_le_bytes());
}

pub(super) fn rec_mode0(e: [f64; 3], pos: [f64; 3], tick: i64, hz: Option<f64>, phase: Option<f64>) -> [u8; 128] {
    let mut b = [0u8; 128];
    for i in 0..3 {
        wr_f64(&mut b, i * 8, e[i]);
        wr_f64(&mut b, 48 + i * 8, pos[i]);
    }
    wr_i64(&mut b, 72, tick);
    let mut fl = 0u32;
    if let Some(f) = hz {
        fl |= 1;
        wr_f64(&mut b, 80, f);
    }
    if let Some(p) = phase {
        fl |= 2;
        wr_f64(&mut b, 88, p);
    }
    wr_u32(&mut b, 120, fl);
    b
}

pub(super) fn rec_mode1(pos: [f64; 3], dir: [f64; 3], stokes: [f64; 4], hz: f64, path: f64, phase: Option<f64>) -> [u8; 128] {
    let mut b = [0u8; 128];
    for i in 0..3 {
        wr_f64(&mut b, i * 8, pos[i]);
        wr_f64(&mut b, 24 + i * 8, dir[i]);
    }
    for i in 0..4 {
        wr_f64(&mut b, 48 + i * 8, stokes[i]);
    }
    wr_f64(&mut b, 80, hz);
    wr_f64(&mut b, 96, path);
    let mut fl = 0u32;
    if let Some(p) = phase {
        fl |= 1;
        wr_f64(&mut b, 104, p);
    }
    wr_u32(&mut b, 124, fl);
    b
}

pub(super) fn rec_mode2_yee(e: [f64; 3], h: [f64; 3], eps_r: f64, mu_r: f64) -> [u8; 128] {
    let mut b = [0u8; 128];
    for i in 0..3 {
        wr_f64(&mut b, i * 8, e[i]);
        wr_f64(&mut b, 24 + i * 8, h[i]);
    }
    wr_f64(&mut b, 88, eps_r);
    wr_f64(&mut b, 96, mu_r);
    b
}

pub(super) fn rec_mode3(pos: [f64; 3], tick: i64, axis_id: u32, vals: &[f64]) -> Vec<u8> {
    let stride = 64 + ((vals.len() * 8 + 63) & !63);
    let mut b = vec![0u8; stride];
    for i in 0..3 {
        wr_f64(&mut b, i * 8, pos[i]);
    }
    wr_i64(&mut b, 24, tick);
    wr_u32(&mut b, 36, axis_id);
    for (i, v) in vals.iter().enumerate() {
        wr_f64(&mut b, 64 + i * 8, *v);
    }
    b
}

pub(super) fn rec_mode4_mueller(pos: [f64; 3], hz: f64, col0: [f64; 4]) -> [u8; 256] {
    let mut b = [0u8; 256];
    for i in 0..3 {
        wr_f64(&mut b, i * 8, pos[i]);
    }
    wr_f64(&mut b, 24, 1.0);
    wr_f64(&mut b, 48, 1.0);
    wr_f64(&mut b, 72, hz);
    for r in 0..4 {
        wr_f64(&mut b, 128 + r * 32, col0[r]);
    }
    b
}

pub(super) fn rec_mode5_iq(tick: i64, ch: u32, hz: f64, phase: f64, iq: &[(f64, f64)]) -> Vec<u8> {
    let stride = 64 + ((16 * iq.len() + 63) & !63);
    let mut b = vec![0u8; stride];
    wr_i64(&mut b, 0, tick);
    wr_u32(&mut b, 8, iq.len() as u32);
    wr_u32(&mut b, 12, ch);
    wr_f64(&mut b, 16, hz);
    wr_f64(&mut b, 24, phase);
    wr_f64(&mut b, 32, 1.0);
    wr_u32(&mut b, 48, 1);
    for (idx, (iv, qv)) in iq.iter().enumerate() {
        wr_f64(&mut b, 64 + idx * 16, *iv);
        wr_f64(&mut b, 64 + idx * 16 + 8, *qv);
    }
    let crc = crc32c(&b);
    wr_u32(&mut b, 60, crc);
    b
}

pub(super) fn rec_mode6_ionizing(pos: [f64; 3], tick: i64, energy_j: f64, fluence: f64) -> [u8; 128] {
    let mut b = [0u8; 128];
    for i in 0..3 {
        wr_f64(&mut b, i * 8, pos[i]);
    }
    wr_i64(&mut b, 24, tick);
    wr_f64(&mut b, 32, energy_j);
    wr_f64(&mut b, 56, fluence);
    wr_u32(&mut b, 108, 0b1001);
    let crc = crc32c(&b[..124]);
    wr_u32(&mut b, 124, crc);
    b
}

pub(super) fn rec_mode7_bio(pos: [f64; 3], dipole: [f64; 3], pot_v: f64, tick: i64, hz: f64) -> [u8; 128] {
    let mut b = [0u8; 128];
    for i in 0..3 {
        wr_f64(&mut b, i * 8, pos[i]);
        wr_f64(&mut b, 24 + i * 8, dipole[i]);
    }
    wr_f64(&mut b, 48, pot_v);
    wr_i64(&mut b, 56, tick);
    wr_f64(&mut b, 64, hz);
    wr_u32(&mut b, 96, 0b11);
    let crc = crc32c(&b[..124]);
    wr_u32(&mut b, 124, crc);
    b
}

pub(super) fn entry_ungridded(mode: u16, desc: u32, rc: u64, stride: u32, payload: &[u8]) -> env::EntryInput<'_> {
    env::EntryInput {
        mode,
        revision: 1,
        flags: 0,
        stream_id: 0,
        frame_id: 2,
        start_tick: 0,
        time_count: 0,
        spatial_key: 0,
        record_count: rc,
        stride,
        codec: 0,
        descriptor_id: desc,
        payload,
    }
}

pub(super) fn spectrum_axis_block(id: u32, coords_hz: &[f64]) -> Tb {
    Tb {
        id,
        schema: 132,
        fields: vec![
            tstr(1, "frequency_hz"),
            tstr(2, "bin_integral"),
            tstr(3, "bin_integrated_irradiance"),
            tstr(4, "W m^-2"),
            tf64_arr(5, coords_hz),
            tstr(6, "explicit"),
            tstr(8, "none"),
        ],
    }
}

pub(super) fn constitutive_block(id: u32) -> Tb {
    Tb {
        id,
        schema: 133,
        fields: vec![
            tstr(1, "isotropic_nondispersive"),
            tf64(2, 2.25),
            tf64(3, 1.0),
            tf64(4, 0.0),
        ],
    }
}

pub(super) fn acq_block(id: u32, calib_id: u32) -> Tb {
    Tb {
        id,
        schema: 136,
        fields: vec![
            tref(1, 1),
            tu64(2, 1000),
            tu64(3, 1),
            tstr(4, "complex_baseband"),
            tstr(5, "V"),
            tref(6, calib_id),
            tstr(7, "I_plus_iQ_exp_minus_iwt"),
        ],
    }
}

pub(super) fn bio_block(id: u32) -> Tb {
    Tb {
        id,
        schema: 137,
        fields: vec![tstr(1, "FMA-5"), tstr(2, "FMA_7088"), tstr(3, "current_dipole"), tref(4, 3)],
    }
}

pub(super) fn make_mask_desc_pair(ext_id: u32, desc_id: u32) -> (Tb, Tb) {
    (
        ext_block(ext_id, 0x106, profiles::VALIDITY_PROFILE, [1, 1, 1], [1, 1, 1], vec![]),
        Tb {
            id: desc_id,
            schema: 3,
            fields: vec![
                tref(3, 3),
                tstr(7, "validity"),
                tstr(8, "1"),
                tstr(12, profiles::VALIDITY_MODEL_URI),
                tref(18, ext_id),
            ],
        },
    )
}
