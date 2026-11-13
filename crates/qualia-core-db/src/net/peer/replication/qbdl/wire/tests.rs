use super::*;
use crate::net::qdnf::types::StrongDigest as D;
fn limits() -> Limits {
    Limits {
        record: 65776,
        page: 65536,
        cose: 4608,
        index: 8388608,
        metadata: 8388608,
        closure: 1048576,
        entries: 4096,
        inflight: 30,
        window: 33554432,
        cold_rows: 64,
        extent: 1 << 40,
        lifetime_ms: 30000,
    }
}
fn header(kind: Kind) -> Header {
    Header {
        kind,
        descriptor: D([1; 48]),
        context: D([2; 48]),
        publication_generation: 0,
        policy_generation: 3,
        cancellation: 0,
        transfer_id: [4; 16],
        entry: 0,
        leaf: 0,
        count: 0,
        credit_serial: 0,
        credit_bytes: 0,
        deadline: 123456789,
    }
}
fn case(kind: Kind) -> (Header, Vec<u8>) {
    let mut h = header(kind);
    let mut body = match kind {
        Kind::Describe => {
            h.credit_serial = 1;
            h.credit_bytes = 65776;
            let mut b = vec![0; 96];
            limits().encode(&mut b).unwrap();
            ObjectSelector {
                class: 1,
                number: 0,
                total: 0,
                offset: 0,
                chunk: 65536,
            }
            .encode(true, &mut b[64..])
            .unwrap();
            b
        }
        Kind::DescriptorPage => {
            h.credit_serial = 1;
            h.credit_bytes = 304;
            let mut b = vec![0; 96];
            ObjectSelector {
                class: 6,
                number: 0,
                total: 64,
                offset: 0,
                chunk: 64,
            }
            .encode(false, &mut b)
            .unwrap();
            b
        }
        Kind::Select => vec![0; 8],
        Kind::Open => {
            let mut b = vec![0; 72];
            limits().encode(&mut b).unwrap();
            b
        }
        Kind::Grant => {
            let mut b = vec![0; 80];
            b[1] = 2;
            limits().encode(&mut b[8..]).unwrap();
            b
        }
        Kind::Credit | Kind::RequestLeaves => {
            h.count = 1;
            h.credit_serial = 1;
            h.credit_bytes = 18137;
            vec![]
        }
        Kind::LeafData => {
            h.count = 1;
            h.credit_serial = 1;
            h.credit_bytes = 18137;
            let mut b = vec![0; 10];
            b[3] = 1;
            b[5] = 1;
            b[8] = 42;
            b
        }
        Kind::Cancel => {
            h.cancellation = 1;
            h.count = 1;
            let mut b = vec![0; 8];
            b[1] = 1;
            b
        }
        Kind::Close => {
            h.cancellation = 1;
            vec![0; 8]
        }
        Kind::Resume => {
            let mut b = vec![0; 24];
            b[1] = 2;
            b[23] = 9;
            b
        }
        Kind::Outcome => {
            h.count = 1;
            let mut b = vec![0; 16];
            b[3] = 3;
            b[11] = 1;
            b
        }
    };
    // Keep explicit ownership of each fixture; no parser-generated oracle bytes.
    body.shrink_to_fit();
    (h, body)
}
#[test]
fn every_variant_exact_prefix_and_roundtrip() {
    for kind in [
        Kind::Describe,
        Kind::DescriptorPage,
        Kind::Select,
        Kind::Open,
        Kind::Grant,
        Kind::RequestLeaves,
        Kind::LeafData,
        Kind::Credit,
        Kind::Cancel,
        Kind::Resume,
        Kind::Outcome,
        Kind::Close,
    ] {
        let (h, b) = case(kind);
        let mut out = vec![0; 208 + b.len()];
        assert_eq!(encode(&h, &b, &mut out).unwrap(), out.len());
        assert_eq!(
            &out[..12],
            &[b'Q', b'B', b'S', b'1', 0, 1, 0, kind as u8, 0, 1, 0, 0]
        );
        assert_eq!(&out[16..64], &[1; 48]);
        assert_eq!(&out[64..112], &[2; 48]);
        assert_eq!(&out[136..152], &[4; 16]);
        assert_eq!(u32_at(&out, 180) as usize, b.len());
        let decoded = Message::parse(&out).unwrap();
        assert_eq!(decoded.header, h);
        assert_eq!(decoded.body, b);
        for n in 0..out.len() {
            assert!(Message::parse(&out[..n]).is_err(), "kind={kind:?} cut={n}");
        }
        out.push(0);
        assert!(Message::parse(&out).is_err());
    }
}
#[test]
fn header_reserved_versions_and_unused_associations_refuse_without_output() {
    let (h, b) = case(Kind::Describe);
    let mut good = vec![0; 208 + b.len()];
    encode(&h, &b, &mut good).unwrap();
    for at in [0, 4, 6, 8, 10, 12, 156, 157, 160] {
        let mut bad = good.clone();
        bad[at] = 255;
        assert!(Message::parse(&bad).is_err(), "offset={at}");
    }
    let mut bad = h;
    bad.entry = 1;
    let mut out = vec![0xa5; good.len()];
    assert!(encode(&bad, &b, &mut out).is_err());
    assert!(out.iter().all(|x| *x == 0xa5));
    bad = h;
    bad.credit_serial = 0;
    assert!(encode(&bad, &b, &mut out).is_err());
    bad = h;
    bad.cancellation = 1;
    assert!(encode(&bad, &b, &mut out).is_err());
}
#[test]
fn leaf_and_descriptor_maxima_match_exact_allocations() {
    let mut h = header(Kind::LeafData);
    h.count = 1;
    h.credit_serial = 7;
    h.credit_bytes = 18137;
    let mut body = vec![0; 8 + 16384 + 1537];
    body[..4].copy_from_slice(&16384u32.to_be_bytes());
    body[4..6].copy_from_slice(&1537u16.to_be_bytes());
    body[8 + 16384] = 32;
    let mut out = vec![0; MAX_RECORD_BYTES];
    assert_eq!(encode(&h, &body, &mut out).unwrap(), MAX_LEAF_RECORD_BYTES);
    body[8 + 16384] = 31;
    assert!(encode(&h, &body, &mut out).is_err());
    body[8 + 16384] = 32;
    h.credit_bytes -= 1;
    assert!(encode(&h, &body, &mut out).is_err());
    h = header(Kind::DescriptorPage);
    h.credit_serial = 8;
    h.credit_bytes = 65776;
    let mut page = vec![0; 32 + 65536];
    ObjectSelector {
        class: 9,
        number: 31,
        total: 65536,
        offset: 0,
        chunk: 65536,
    }
    .encode(false, &mut page)
    .unwrap();
    assert_eq!(encode(&h, &page, &mut out).unwrap(), MAX_RECORD_BYTES);
    page.push(0);
    assert!(encode(&h, &page, &mut out).is_err());
}
#[test]
fn selector_final_chunks_limits_and_resume_order_are_canonical() {
    let s = ObjectSelector {
        class: 7,
        number: 0,
        total: 70000,
        offset: 65536,
        chunk: 4464,
    };
    assert!(s.validate(false).is_ok());
    assert!(ObjectSelector { chunk: 4465, ..s }.validate(false).is_err());
    assert!(ObjectSelector {
        offset: u64::MAX,
        ..s
    }
    .validate(true)
    .is_err());
    assert!(ObjectSelector {
        class: 2,
        total: 4609,
        offset: 0,
        chunk: 4609,
        ..s
    }
    .validate(false)
    .is_err());
    let mut l = limits();
    l.record -= 1;
    assert!(l.validate().is_err());
    let (h, mut b) = case(Kind::Resume);
    b[23] = 0;
    assert!(encode(&h, &b, &mut [0; 512]).is_err());
    let (h, mut b) = case(Kind::Grant);
    b[1] = 1;
    b[79] = 1;
    assert!(encode(&h, &b, &mut [0; 512]).is_err());
}
#[test]
fn service_record_domain_geometry_and_final_chunk_are_exact() {
    let h = ServiceRecordHeader {
        transcript: D([3; 48]),
        message_id: [4; 16],
        total: 65776,
        offset: 65536,
        chunk: 240,
        final_chunk: true,
    };
    let mut out = [0; 88];
    assert_eq!(h.encode(&mut out).unwrap(), 88);
    assert_eq!(&out[..12], b"QBDL-SCT-V1\0");
    assert_eq!(ServiceRecordHeader::parse(&out).unwrap(), h);
    assert_eq!(u32_at(&out, 76), 65776);
    assert_eq!(u32_at(&out, 80), 65536);
    for n in 0..88 {
        assert!(ServiceRecordHeader::parse(&out[..n]).is_err());
    }
    out[5] = b'P';
    assert!(ServiceRecordHeader::parse(&out).is_err());
    h.encode(&mut out).unwrap();
    out[87] = 0;
    assert!(ServiceRecordHeader::parse(&out).is_err());
    assert!(ServiceRecordHeader { total: 65777, ..h }
        .validate()
        .is_err());
    assert!(ServiceRecordHeader { offset: 65535, ..h }
        .validate()
        .is_err());
}

#[test]
fn service_codec_hot_success_and_failure_allocate_zero() {
    use crate::specialized_libs::computational_geometry::allocation_counter::AllocGuard;
    let (h, b) = case(Kind::LeafData);
    let mut out = [0; 512];
    let guard = AllocGuard::begin("qbdl-service-wire", true);
    let n = encode(&h, &b, &mut out).unwrap();
    assert_eq!(Message::parse(&out[..n]).unwrap().header, h);
    assert!(Message::parse(&out[..n - 1]).is_err());
    assert!(encode(&h, &b, &mut out[..1]).is_err());
    guard.check().unwrap();
}
