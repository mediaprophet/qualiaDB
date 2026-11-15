use super::*;
#[path = "fixture_registry.rs"]
mod fixture_registry;
fn load(path: &str) -> Vec<u8> {
    fixture_registry::load(path).to_vec()
}
fn open(b: &[u8], f: Family) -> Result<StructuralView<'_>, Error> {
    StructuralView::open(b, f, Limits::default())
}
#[test]
fn independent_candidate_fixtures_and_unaligned_memory() {
    for (stem, ext, f) in [
        ("qac_candidate", "qac", Family::QacV1Candidate),
        ("emf_candidate", "emf", Family::EmfV4Candidate),
    ] {
        for name in ["metadata-only", "opaque", "aligned4096", "two-entries"] {
            let b = load(&format!("candidate/{stem}-{name}.{ext}"));
            let mut unaligned = vec![0];
            unaligned.extend_from_slice(&b);
            let v = open(&unaligned[1..], f).unwrap();
            for i in 1..=v.entry_count() {
                v.verify_chunk(i).unwrap();
            }
            for block in v.blocks() {
                for field in block.fields() {
                    let _ = field.value();
                }
            }
            assert_eq!(v.entry(0).unwrap_err(), Error::EntryIndex);
        }
    }
    let b = load("candidate/qac_candidate-pcm-signed-zero.qac");
    open(&b, Family::QacV1Candidate)
        .unwrap()
        .verify_chunk(1)
        .unwrap();
}
#[test]
fn independent_structural_corruptions() {
    for (stem, ext, f) in [
        ("qac_candidate", "qac", Family::QacV1Candidate),
        ("emf_candidate", "emf", Family::EmfV4Candidate),
    ] {
        for name in [
            "count-overflow",
            "count-product-overflow",
            "directory-crc",
            "duplicate",
            "header-crc",
            "metadata-duplicate-id",
            "padding",
            "payload-overlap",
            "reordered",
            "reserved-directory",
            "reserved-header",
            "root-tail",
            "tlv-count",
            "tlv-padding",
            "tlv-reserved",
            "tlv-zero-key",
            "truncated",
            "unknown-codec",
        ] {
            let b = load(&format!("malformed/{stem}-{name}.{ext}"));
            assert!(open(&b, f).is_err(), "{stem}-{name}");
        }
        for name in ["payload", "payload-digest-tail"] {
            let b = load(&format!("malformed/{stem}-{name}.{ext}"));
            let v = open(&b, f).unwrap();
            assert!(
                (1..=v.entry_count()).any(|i| v.verify_chunk(i) == Err(Error::PayloadDigest)),
                "{name}"
            );
        }
        // Intentional boundary: unknown critical schema is not scientific admission.
        let b = load(&format!("malformed/{stem}-unknown-critical-schema.{ext}"));
        open(&b, f).unwrap();
    }
}
#[test]
fn budgets_family_and_checked_arithmetic() {
    let b = load("candidate/qac_candidate-opaque.qac");
    for limits in [
        Limits {
            file_bytes: 0,
            ..Limits::default()
        },
        Limits {
            entries: 0,
            ..Limits::default()
        },
        Limits {
            metadata_bytes: 0,
            ..Limits::default()
        },
        Limits {
            blocks: 0,
            ..Limits::default()
        },
        Limits {
            fields_per_block: 0,
            ..Limits::default()
        },
        Limits {
            chunk_bytes: 0,
            ..Limits::default()
        },
        Limits {
            aggregate_decoded_bytes: 0,
            ..Limits::default()
        },
    ] {
        assert_eq!(
            StructuralView::open(&b, Family::QacV1Candidate, limits).unwrap_err(),
            Error::Limit
        );
    }
    assert_eq!(open(&b, Family::EmfV4Candidate).unwrap_err(), Error::Family);
    assert_eq!(align(u64::MAX, 64), Err(Error::Overflow));
    assert_eq!(extent(&b, u64::MAX, 1), Err(Error::Overflow));
    for end in 0..128 {
        assert!(open(&b[..end], Family::QacV1Candidate).is_err());
    }
}
#[test]
fn numeric_metadata_and_unknown_types() {
    fn block(tag: u8, value: &[u8], count: u32) -> Vec<u8> {
        let mut b = vec![0; 32 + ((value.len() + 7) & !7)];
        b[..4].copy_from_slice(&1u32.to_le_bytes());
        let body = (b.len() - 16) as u32;
        b[8..12].copy_from_slice(&body.to_le_bytes());
        b[16..18].copy_from_slice(&1u16.to_le_bytes());
        b[18] = tag;
        b[20..24].copy_from_slice(&count.to_le_bytes());
        b[24..28].copy_from_slice(&(value.len() as u32).to_le_bytes());
        b[32..32 + value.len()].copy_from_slice(value);
        b
    }
    for (tag, value) in [
        (4, f32::NAN.to_le_bytes().to_vec()),
        (5, f64::INFINITY.to_le_bytes().to_vec()),
    ] {
        assert_eq!(
            metadata::validate(&block(tag, &value, 1), Limits::default()),
            Err(Error::Numeric)
        );
    }
    assert_eq!(
        metadata::validate(&block(1, &[0; 4], 2), Limits::default()),
        Err(Error::Numeric)
    );
    assert_eq!(
        metadata::validate(&block(6, b"a\0", 2), Limits::default()),
        Err(Error::Metadata)
    );
    metadata::validate(&block(255, b"opaque", 99), Limits::default()).unwrap();
}
