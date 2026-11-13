//! Hand-authored little-endian bytes; no candidate serializer/parser is used.
pub fn header(version: u32, tensors: u64, kvs: u64) -> Vec<u8> {
    let mut out = b"GGUF".to_vec();
    out.extend_from_slice(&version.to_le_bytes());
    out.extend_from_slice(&tensors.to_le_bytes());
    out.extend_from_slice(&kvs.to_le_bytes());
    out
}
pub fn string(out: &mut Vec<u8>, value: &[u8]) {
    out.extend_from_slice(&(value.len() as u64).to_le_bytes());
    out.extend_from_slice(value);
}
pub fn kv(out: &mut Vec<u8>, key: &[u8], kind: u32, value: &[u8]) {
    string(out, key);
    out.extend_from_slice(&kind.to_le_bytes());
    out.extend_from_slice(value);
}
pub fn tensor(out: &mut Vec<u8>, name: &[u8], dims: &[u64], dtype: u32, offset: u64) {
    string(out, name);
    out.extend_from_slice(&(dims.len() as u32).to_le_bytes());
    for dimension in dims {
        out.extend_from_slice(&dimension.to_le_bytes());
    }
    out.extend_from_slice(&dtype.to_le_bytes());
    out.extend_from_slice(&offset.to_le_bytes());
}
pub fn payloads() -> Vec<(u32, Vec<u8>)> {
    let mut string_value = Vec::new();
    string(&mut string_value, b"a\xff\0");
    let mut array = 4u32.to_le_bytes().to_vec();
    array.extend_from_slice(&2u64.to_le_bytes());
    array.extend_from_slice(&7u32.to_le_bytes());
    array.extend_from_slice(&9u32.to_le_bytes());
    vec![
        (0, vec![255]),
        (1, vec![128]),
        (2, u16::MAX.to_le_bytes().to_vec()),
        (3, i16::MIN.to_le_bytes().to_vec()),
        (4, u32::MAX.to_le_bytes().to_vec()),
        (5, i32::MIN.to_le_bytes().to_vec()),
        (6, 0x7fc0_1234u32.to_le_bytes().to_vec()),
        (7, vec![127]),
        (8, string_value),
        (9, array),
        (10, u64::MAX.to_le_bytes().to_vec()),
        (11, i64::MIN.to_le_bytes().to_vec()),
        (12, (-0.0f64).to_bits().to_le_bytes().to_vec()),
    ]
}
pub fn complete() -> Vec<u8> {
    let mut out = header(3, 3, 13);
    for (kind, payload) in payloads() {
        kv(&mut out, b"duplicate.raw\xff", kind, &payload);
    }
    tensor(&mut out, b"x", &[2, 3, 4, 5, 6], u32::MAX, u64::MAX);
    tensor(&mut out, b"x", &[], 0, 0);
    tensor(&mut out, b"", &[0], 42, 17);
    out
}
pub fn nested(depth: usize) -> Vec<u8> {
    assert!(depth > 0);
    let mut value = 4u32.to_le_bytes().to_vec();
    value.extend_from_slice(&1u64.to_le_bytes());
    value.extend_from_slice(&7u32.to_le_bytes());
    for _ in 1..depth {
        let mut outer = 9u32.to_le_bytes().to_vec();
        outer.extend_from_slice(&1u64.to_le_bytes());
        outer.extend_from_slice(&value);
        value = outer;
    }
    let mut out = header(3, 0, 1);
    kv(&mut out, b"array", 9, &value);
    out
}
pub fn strict_divergences() -> Vec<(&'static str, Vec<u8>)> {
    // Actual old-source results are frozen in Qualia's gguf-reference-20261103-01
    // baseline.log: old index accepts all three rows. These new strict failures
    // are intentional additive safety differences; no caller adoption is implied.
    let mut unknown = header(3, 0, 1);
    kv(&mut unknown, b"unknown", 99, &[]);
    let mut array = 99u32.to_le_bytes().to_vec();
    array.extend_from_slice(&0u64.to_le_bytes());
    let mut empty_unknown = header(3, 0, 1);
    kv(&mut empty_unknown, b"empty", 9, &array);
    let mut partial = 8u32.to_le_bytes().to_vec();
    partial.extend_from_slice(&3u64.to_le_bytes());
    string(&mut partial, b"a");
    string(&mut partial, b"b");
    let mut strings = header(3, 0, 1);
    kv(&mut strings, b"tokenizer.ggml.tokens", 9, &partial);
    vec![
        ("unknown value", unknown),
        ("unknown empty array", empty_unknown),
        ("partial string array", strings),
    ]
}
