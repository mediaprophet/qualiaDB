use crate::container_10d::crc32c::crc32c;
use crate::q42::field_envelope::{
    align, BlockInput, EnvelopeInput, EntryInput, Error, Family, FieldInput, FieldValue, Limits,
    MetadataInput, DIRECTORY_BYTES, HEADER_BYTES, INDEX_DOMAIN,
};

pub(super) fn put_u16(buf: &mut [u8], offset: usize, val: u16) {
    buf[offset..offset + 2].copy_from_slice(&val.to_le_bytes());
}

pub(super) fn put_u32(buf: &mut [u8], offset: usize, val: u32) {
    buf[offset..offset + 4].copy_from_slice(&val.to_le_bytes());
}

pub(super) fn put_u64(buf: &mut [u8], offset: usize, val: u64) {
    buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
}

pub(super) fn put_i64(buf: &mut [u8], offset: usize, val: i64) {
    buf[offset..offset + 8].copy_from_slice(&val.to_le_bytes());
}

pub(super) fn get_u32(buf: &[u8], offset: usize) -> u32 {
    u32::from_le_bytes(buf[offset..offset + 4].try_into().unwrap())
}

pub(super) fn get_u64(buf: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(buf[offset..offset + 8].try_into().unwrap())
}

pub(super) fn crc_zeroed(block128: &[u8]) -> u32 {
    let mut work = [0u8; 128];
    work.copy_from_slice(block128);
    work[124..].fill(0);
    crc32c(&work)
}

/// Recomputes directory row CRCs (when `update_rows`), `INDEX_DOMAIN` commitment
/// (when `update_index`), and header CRC-32C after mutating structural test buffers.
pub(super) fn reseal(buf: &mut [u8], update_rows: bool, update_index: bool) {
    let count = get_u32(buf, 32) as usize;
    let meta_off = get_u64(buf, 40) as usize;
    let meta_len = get_u64(buf, 48) as usize;
    if update_rows {
        for i in 0..count {
            let base = HEADER_BYTES + i * DIRECTORY_BYTES;
            let row_crc = crc_zeroed(&buf[base..base + DIRECTORY_BYTES]);
            put_u32(buf, base + 124, row_crc);
        }
    }
    if update_index {
        let mut zero_hdr = [0u8; HEADER_BYTES];
        zero_hdr.copy_from_slice(&buf[..HEADER_BYTES]);
        zero_hdr[56..88].fill(0);
        zero_hdr[124..128].fill(0);
        let dir_end = HEADER_BYTES + count * DIRECTORY_BYTES;
        let mut hasher = blake3::Hasher::new();
        hasher.update(INDEX_DOMAIN);
        hasher.update(&zero_hdr);
        hasher.update(&buf[HEADER_BYTES..dir_end]);
        hasher.update(&buf[meta_off..meta_off + meta_len]);
        buf[56..88].copy_from_slice(hasher.finalize().as_bytes());
    }
    let hdr_crc = crc_zeroed(&buf[..HEADER_BYTES]);
    put_u32(buf, 124, hdr_crc);
}

pub(super) fn sample_metadata(f32_extra: Option<f32>, f64_extra: Option<f64>) -> Vec<u8> {
    let role = [4u32];
    let f32_val = [f32_extra.unwrap_or(1.0)];
    let f64_val = [f64_extra.unwrap_or(1.0)];
    let prov_ref = [1u32];
    let f_block1 = [FieldInput {
        key: 1,
        flags: 1,
        value: FieldValue::U32(&role),
    }];
    let f_block2 = [
        FieldInput {
            key: 3,
            flags: 1,
            value: FieldValue::MetadataRefs(&prov_ref),
        },
        FieldInput {
            key: 7,
            flags: 1,
            value: FieldValue::Utf8("v01-adversarial"),
        },
        FieldInput {
            key: 8,
            flags: 1,
            value: FieldValue::Utf8("1"),
        },
        FieldInput {
            key: 12,
            flags: 1,
            value: FieldValue::Utf8("urn:qualia:v01:test"),
        },
    ];
    let f_block3 = [
        FieldInput {
            key: 1,
            flags: 0,
            value: FieldValue::F32(&f32_val),
        },
        FieldInput {
            key: 2,
            flags: 0,
            value: FieldValue::F64(&f64_val),
        },
    ];
    let blocks = [
        BlockInput {
            id: 1,
            schema_type: 4,
            revision: 1,
            flags: 1,
            fields: &f_block1,
        },
        BlockInput {
            id: 4,
            schema_type: 3,
            revision: 1,
            flags: 1,
            fields: &f_block2,
        },
        BlockInput {
            id: 5,
            schema_type: 65000,
            revision: 1,
            flags: 0,
            fields: &f_block3,
        },
    ];
    let meta = MetadataInput { blocks: &blocks };
    let limits = Limits::default();
    let len = meta.encoded_len(limits).unwrap();
    let mut out = vec![0u8; len];
    meta.encode_into(&mut out, limits).unwrap();
    out
}

pub(super) fn encode_envelope(
    family: Family,
    flags: u32,
    metadata: &[u8],
    entries: &[EntryInput<'_>],
) -> Vec<u8> {
    let input = EnvelopeInput {
        family,
        flags,
        metadata,
        entries,
    };
    let limits = Limits::default();
    let len = input.encoded_len(limits).unwrap();
    let mut out = vec![0u8; len];
    input.encode_into(&mut out, limits).unwrap();
    out
}

pub(super) fn entry_timed<'a>(
    stream_id: u32,
    start_tick: i64,
    mode: u16,
    descriptor_id: u32,
    payload: &'a [u8],
) -> EntryInput<'a> {
    EntryInput {
        mode,
        revision: 1,
        flags: 3,
        stream_id,
        frame_id: 0,
        start_tick,
        time_count: 1,
        spatial_key: 0,
        record_count: payload.len() as u64,
        stride: 1,
        codec: 0,
        descriptor_id,
        payload,
    }
}

pub(super) fn base_single(family: Family) -> Vec<u8> {
    let meta = sample_metadata(None, None);
    let e = [entry_timed(0, -5, 0x0104, 4, b"V01PAYLD")];
    encode_envelope(family, 0, &meta, &e)
}

pub(super) fn base_two(family: Family) -> Vec<u8> {
    let meta = sample_metadata(None, None);
    let e = [
        entry_timed(0, -5, 0x0104, 4, b"STREAM_0"),
        entry_timed(1, -5, 0x0104, 4, b"STREAM_1"),
    ];
    encode_envelope(family, 0, &meta, &e)
}

/// Crafts a valid 0-entry envelope around raw unvalidated metadata bytes so
/// negative TLV parser cases bypass `MetadataInput` / `EnvelopeInput` guards.
pub(super) fn raw_meta_envelope(family: Family, raw_meta: &[u8]) -> Vec<u8> {
    let meta_end = HEADER_BYTES as u64 + raw_meta.len() as u64;
    let total = align(meta_end, 64).unwrap() as usize;
    let mut out = vec![0u8; total];
    out[..8].copy_from_slice(family.magic());
    put_u16(&mut out, 8, family.version());
    put_u16(&mut out, 10, HEADER_BYTES as u16);
    put_u32(&mut out, 12, 0);
    put_u64(&mut out, 16, total as u64);
    put_u64(&mut out, 24, HEADER_BYTES as u64);
    put_u32(&mut out, 32, 0);
    put_u16(&mut out, 36, DIRECTORY_BYTES as u16);
    put_u16(&mut out, 38, 1);
    put_u64(&mut out, 40, HEADER_BYTES as u64);
    put_u64(&mut out, 48, raw_meta.len() as u64);
    out[HEADER_BYTES..HEADER_BYTES + raw_meta.len()].copy_from_slice(raw_meta);
    reseal(&mut out, false, true);
    out
}

pub(super) fn assert_both_families(mut check: impl FnMut(Family)) {
    check(Family::EmfV4Candidate);
    check(Family::QacV1Candidate);
}

pub(super) fn expect_open_err(bytes: &[u8], family: Family, expected: Error) {
    assert_eq!(
        crate::q42::field_envelope::StructuralView::open(bytes, family, Limits::default())
            .unwrap_err(),
        expected
    );
}
