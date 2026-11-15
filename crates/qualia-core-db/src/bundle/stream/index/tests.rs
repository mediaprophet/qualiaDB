//! Independent byte fixtures for the structural admission profile.
use super::*;

#[derive(Clone, Copy)]
enum Value<'a> {
    Text(&'a str),
    Uint(u64),
    Bytes(&'a [u8]),
    Array(&'a [u8]),
    Null,
}
fn major(out: &mut Vec<u8>, tag: u8, n: u64) {
    let prefix = tag << 5;
    match n {
        0..=23 => out.push(prefix | n as u8),
        24..=255 => {
            out.extend_from_slice(&[prefix | 24, n as u8]);
        }
        256..=65535 => {
            out.push(prefix | 25);
            out.extend_from_slice(&(n as u16).to_be_bytes());
        }
        65536..=4294967295 => {
            out.push(prefix | 26);
            out.extend_from_slice(&(n as u32).to_be_bytes());
        }
        _ => {
            out.push(prefix | 27);
            out.extend_from_slice(&n.to_be_bytes());
        }
    }
}
fn value(out: &mut Vec<u8>, v: Value<'_>) {
    match v {
        Value::Text(s) => {
            major(out, 3, s.len() as u64);
            out.extend_from_slice(s.as_bytes());
        }
        Value::Uint(n) => major(out, 0, n),
        Value::Bytes(b) => {
            major(out, 2, b.len() as u64);
            out.extend_from_slice(b);
        }
        Value::Array(b) => {
            major(out, 4, b.len() as u64);
            for n in b {
                major(out, 0, u64::from(*n));
            }
        }
        Value::Null => out.push(0xf6),
    }
}
fn encoded_row(fields: &[(&str, Value<'_>)]) -> Vec<u8> {
    let mut out = Vec::new();
    major(&mut out, 5, fields.len() as u64);
    for (key, v) in fields {
        value(&mut out, Value::Text(key));
        value(&mut out, *v);
    }
    out
}
fn row(key: &str, offset: u64, len: u64) -> Vec<u8> {
    encoded_row(&[
        ("key", Value::Text(key)),
        ("kind", Value::Text("10d")),
        ("offset", Value::Uint(offset)),
        ("length", Value::Uint(len)),
        ("sha256", Value::Array(&[0; 32])),
    ])
}
fn index(rows: &[Vec<u8>]) -> Vec<u8> {
    let mut out = Vec::new();
    major(&mut out, 4, rows.len() as u64);
    for row in rows {
        out.extend_from_slice(row);
    }
    out
}
fn header(offset: u64, index_len: usize, count: u32, flags: u16) -> [u8; 64] {
    let mut out = [0; 64];
    out[..4].copy_from_slice(b"QBDL");
    out[4..6].copy_from_slice(&1u16.to_le_bytes());
    out[6..8].copy_from_slice(&flags.to_le_bytes());
    out[8..12].copy_from_slice(&count.to_le_bytes());
    out[12..20].copy_from_slice(&offset.to_le_bytes());
    out[20..28].copy_from_slice(&(index_len as u64).to_le_bytes());
    out[28..36].copy_from_slice(&(offset + index_len as u64).to_le_bytes());
    out
}
fn check(h: &[u8], bytes: &[u8]) -> Result<(), IndexError> {
    let extent = u64::from_le_bytes(h[28..36].try_into().unwrap());
    let checked = admit_header(
        h,
        extent,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )?;
    let count = measure_index(&checked, bytes, IndexReadLimits::default())?.entry_count();
    let mut scratch = vec![EntryScratch::EMPTY; count];
    admit_index(&checked, bytes, IndexReadLimits::default(), &mut scratch)?;
    Ok(())
}

#[test]
fn flags_and_independent_extent_are_checked_before_index() {
    let bytes = index(&[row("a", 64, 3)]);
    let raw = header(512, bytes.len(), 1, 0);
    assert!(check(&raw, &bytes).is_ok());
    let extent = u64::from_le_bytes(raw[28..36].try_into().unwrap());
    assert_eq!(
        admit_header(
            &raw,
            extent + 1,
            ExtentLimit::default(),
            IndexReadLimits::default()
        )
        .unwrap_err(),
        IndexError::Extent
    );
    assert_eq!(
        admit_header(
            &raw,
            extent,
            ExtentLimit {
                max_logical_bytes: extent - 1
            },
            IndexReadLimits::default()
        )
        .unwrap_err(),
        IndexError::ExtentLimit
    );
    for bit in 1..16 {
        let mut bad = raw;
        bad[6..8].copy_from_slice(&(1u16 << bit).to_le_bytes());
        assert_eq!(check(&bad, &bytes), Err(IndexError::Flags));
    }
    for n in 40..64 {
        let mut bad = raw;
        bad[n] = 1;
        assert_eq!(check(&bad, &bytes), Err(IndexError::Reserved));
    }
    let bytes = index(&[row("a", 4096, 4)]);
    assert!(check(&header(8192, bytes.len(), 1, 1), &bytes).is_ok());
    assert_eq!(
        check(&header(512, bytes.len(), 1, 1), &bytes),
        Err(IndexError::Alignment)
    );
}

#[test]
fn exact_header_and_footer_geometry_fail_closed() {
    let bytes = index(&[]);
    let raw = header(64, bytes.len(), 0, 0);
    assert!(check(&raw, &bytes).is_ok());
    assert_eq!(
        admit_header(
            &raw[..63],
            65,
            ExtentLimit::default(),
            IndexReadLimits::default()
        )
        .unwrap_err(),
        IndexError::HeaderLength
    );
    let mut long = raw.to_vec();
    long.push(0);
    assert_eq!(
        admit_header(
            &long,
            65,
            ExtentLimit::default(),
            IndexReadLimits::default()
        )
        .unwrap_err(),
        IndexError::HeaderLength
    );
    for (range, word) in [(12..20, 0u64), (20..28, 0u64), (28..36, 66u64)] {
        let mut bad = raw;
        bad[range].copy_from_slice(&word.to_le_bytes());
        assert!(check(&bad, &bytes).is_err());
    }
    let mut bad = raw;
    bad[12..20].copy_from_slice(&(u64::MAX - 7).to_le_bytes());
    bad[20..28].copy_from_slice(&20u64.to_le_bytes());
    bad[28..36].copy_from_slice(&u64::MAX.to_le_bytes());
    assert_eq!(
        admit_header(
            &bad,
            u64::MAX,
            ExtentLimit {
                max_logical_bytes: u64::MAX
            },
            IndexReadLimits::default()
        )
        .unwrap_err(),
        IndexError::Overflow
    );
    let mut bad = raw;
    bad[0] = 0;
    assert_eq!(check(&bad, &bytes), Err(IndexError::BadMagic));
    let mut bad = raw;
    bad[4] = 2;
    assert_eq!(check(&bad, &bytes), Err(IndexError::UnsupportedVersion(2)));
}

#[test]
fn remote_u64_extent_does_not_size_resident_scratch() {
    let bytes = index(&[row("remote", 1u64 << 34, 3)]);
    let raw = header(1u64 << 35, bytes.len(), 1, 0);
    let extent = (1u64 << 35) + bytes.len() as u64;
    let h = admit_header(
        &raw,
        extent,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let size = measure_index(&h, &bytes, IndexReadLimits::default()).unwrap();
    assert_eq!(size.entry_count(), 1);
    assert!(size.aggregate_bytes() < 4096);
    let mut scratch = [EntryScratch::EMPTY; 1];
    let admitted = admit_index(&h, &bytes, IndexReadLimits::default(), &mut scratch).unwrap();
    assert_eq!(
        admitted.entries().next().unwrap().unwrap().offset(),
        1u64 << 34
    );
}

#[test]
fn scratch_and_resident_limits_are_independent() {
    let bytes = index(&[row("a", 64, 3)]);
    let raw = header(512, bytes.len(), 1, 0);
    let extent = 512 + bytes.len() as u64;
    let h = admit_header(
        &raw,
        extent,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    assert_eq!(
        admit_index(&h, &bytes, IndexReadLimits::default(), &mut []).unwrap_err(),
        IndexError::Scratch
    );
    let size = measure_index(&h, &bytes, IndexReadLimits::default()).unwrap();
    let mut l = IndexReadLimits::default();
    l.decoded_index_bytes = size.aggregate_bytes() - 1;
    assert_eq!(measure_index(&h, &bytes, l), Err(IndexError::ResidentLimit));
    l.decoded_index_bytes = size.aggregate_bytes();
    assert!(measure_index(&h, &bytes, l).is_ok());
    for field in 0..6 {
        let mut l = IndexReadLimits::default();
        match field {
            0 => l.index_bytes = bytes.len() - 1,
            1 => l.entries = 0,
            2 => l.key_bytes = 0,
            3 => l.kind_bytes = 2,
            4 => l.meta_bytes = 0,
            _ => l.decoded_index_bytes = 0,
        }
        if field != 4 {
            assert!(measure_index(&h, &bytes, l).is_err());
        }
    }
}

#[test]
fn duplicate_keys_and_nonempty_overlaps_are_rejected() {
    for rows in [
        vec![row("a", 64, 2), row("a", 128, 2)],
        vec![row("a", 64, 128), row("b", 128, 4)],
        vec![row("a", 64, 256), row("b", 128, 64)],
        vec![row("a", 64, 3), row("b", 64, 3)],
    ] {
        let bytes = index(&rows);
        assert!(check(&header(512, bytes.len(), 2, 0), &bytes).is_err());
    }
    let bytes = index(&[row("later", 128, 3), row("first", 64, 3)]);
    let raw = header(512, bytes.len(), 2, 0);
    assert!(check(&raw, &bytes).is_ok());
    let h = admit_header(
        &raw,
        512 + bytes.len() as u64,
        ExtentLimit::default(),
        IndexReadLimits::default(),
    )
    .unwrap();
    let mut scratch = [EntryScratch::EMPTY; 2];
    let admitted = admit_index(&h, &bytes, IndexReadLimits::default(), &mut scratch).unwrap();
    assert_eq!(admitted.entries().next().unwrap().unwrap().key(), "later");
}

#[test]
fn empty_ranges_never_reset_occupied_frontier() {
    let bytes = index(&[row("full", 64, 256), row("empty", 128, 0)]);
    assert!(check(&header(512, bytes.len(), 2, 0), &bytes).is_ok());
    let bytes = index(&[
        row("full", 64, 256),
        row("empty", 128, 0),
        row("overlap", 192, 3),
    ]);
    assert_eq!(
        check(&header(512, bytes.len(), 3, 0), &bytes),
        Err(IndexError::Overlap)
    );
    let bytes = index(&[row("start", 64, 0), row("data", 64, 3), row("end", 512, 0)]);
    assert!(check(&header(512, bytes.len(), 3, 0), &bytes).is_ok());
}

#[test]
fn bounds_alignment_and_range_overflow_are_rejected() {
    for (off, len) in [(0, 3), (65, 3), (512, 1), (448, 65), (u64::MAX - 63, 128)] {
        let bytes = index(&[row("bad", off, len)]);
        assert!(check(&header(512, bytes.len(), 1, 0), &bytes).is_err());
    }
}

#[test]
fn both_byte_forms_and_arbitrary_field_order_materialize_without_reencoding() {
    let digest = [7u8; 32];
    let meta = [0u8, 255, 42];
    for array in [false, true] {
        let bytes = index(&[encoded_row(&[
            (
                "sha256",
                if array {
                    Value::Array(&digest)
                } else {
                    Value::Bytes(&digest)
                },
            ),
            (
                "meta",
                if array {
                    Value::Bytes(&meta)
                } else {
                    Value::Array(&meta)
                },
            ),
            ("length", Value::Uint(0)),
            ("offset", Value::Uint(64)),
            ("kind", Value::Text("10d")),
            ("key", Value::Text("a")),
        ])]);
        let raw = header(512, bytes.len(), 1, 0);
        let h = admit_header(
            &raw,
            512 + bytes.len() as u64,
            ExtentLimit::default(),
            IndexReadLimits::default(),
        )
        .unwrap();
        let mut scratch = [EntryScratch::EMPTY; 1];
        let admitted = admit_index(&h, &bytes, IndexReadLimits::default(), &mut scratch).unwrap();
        assert_eq!(admitted.encoded_bytes(), &bytes);
        let rows = materialize_entries(&admitted, IndexReadLimits::default()).unwrap();
        assert_eq!(rows[0].sha256.as_slice(), digest.as_slice());
        assert_eq!(rows[0].meta.as_deref(), Some(meta.as_slice()));
        let mut l = IndexReadLimits::default();
        l.meta_bytes = 2;
        assert_eq!(measure_index(&h, &bytes, l), Err(IndexError::ResidentLimit));
    }
}

#[test]
fn invalid_required_fields_and_digests_refuse() {
    for len in [0, 31, 33] {
        let digest = vec![0; len];
        let bytes = index(&[encoded_row(&[
            ("key", Value::Text("a")),
            ("kind", Value::Text("10d")),
            ("offset", Value::Uint(64)),
            ("length", Value::Uint(0)),
            ("sha256", Value::Array(&digest)),
        ])]);
        assert_eq!(
            check(&header(512, bytes.len(), 1, 0), &bytes),
            Err(IndexError::Digest)
        );
    }
    for fields in [
        vec![
            ("key", Value::Text("a")),
            ("kind", Value::Text("10d")),
            ("offset", Value::Uint(64)),
            ("length", Value::Uint(0)),
            ("sha256", Value::Null),
        ],
        vec![
            ("key", Value::Text("a")),
            ("key", Value::Text("b")),
            ("offset", Value::Uint(64)),
            ("length", Value::Uint(0)),
            ("sha256", Value::Bytes(&[0; 32])),
        ],
        vec![
            ("key", Value::Text("a")),
            ("kind", Value::Text("10d")),
            ("offset", Value::Uint(64)),
            ("length", Value::Uint(0)),
            ("unknown", Value::Null),
        ],
        vec![
            ("key", Value::Text("")),
            ("kind", Value::Text("10d")),
            ("offset", Value::Uint(64)),
            ("length", Value::Uint(0)),
            ("sha256", Value::Bytes(&[0; 32])),
        ],
    ] {
        let bytes = index(&[encoded_row(&fields)]);
        assert!(check(&header(512, bytes.len(), 1, 0), &bytes).is_err());
    }
}

#[test]
fn malformed_counts_indefinite_trailing_and_size_bombs_refuse() {
    for bytes in [
        vec![0x9f, 0xff],
        vec![0x9b, 255, 255, 255, 255, 255, 255, 255, 255],
        vec![
            0x81, 0xa5, 0x63, b'k', b'e', b'y', 0x7b, 255, 255, 255, 255, 255, 255, 255, 255,
        ],
    ] {
        assert!(check(&header(512, bytes.len(), 1, 0), &bytes).is_err());
    }
    let mut bytes = index(&[row("a", 64, 0)]);
    bytes.push(0xf6);
    assert_eq!(
        check(&header(512, bytes.len(), 1, 0), &bytes),
        Err(IndexError::Malformed)
    );
    let bytes = index(&[row("a", 64, 0)]);
    assert_eq!(
        check(&header(512, bytes.len(), 2, 0), &bytes),
        Err(IndexError::Count)
    );
    let mut truncated = bytes.clone();
    truncated.pop();
    assert!(check(&header(512, truncated.len(), 1, 0), &truncated).is_err());
}

#[test]
fn invalid_utf8_and_missing_required_fields_refuse() {
    let mut bytes=index(&[row("a",64,0)]);
    let marker=b"key\x61a";
    let position=bytes.windows(marker.len()).position(|w|w==marker).unwrap()+marker.len()-1;
    bytes[position]=255;
    assert_eq!(check(&header(512,bytes.len(),1,0),&bytes),Err(IndexError::Malformed));
    let bytes=index(&[encoded_row(&[("key",Value::Text("a")),("kind",Value::Text("10d")),
        ("offset",Value::Uint(64)),("length",Value::Uint(0))])]);
    assert_eq!(check(&header(512,bytes.len(),1,0),&bytes),Err(IndexError::MissingField));
    let bytes=index(&[row("wrong-alignment",64,0)]);
    assert_eq!(check(&header(8192,bytes.len(),1,1),&bytes),Err(IndexError::Alignment));
}
