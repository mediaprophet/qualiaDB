use super::*;
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest as D;
fn fields(objects: u32) -> ContextFields {
    ContextFields {
        nonce: [1; 32],
        scope_token: [2; 32],
        generation: 0,
        key_epoch: 1,
        predecessor: D::ZERO,
        objects,
        metadata_length: 352,
    }
}
#[test]
fn maximum_geometry_and_descriptor_are_bounded_and_canonical() {
    let grefs: Vec<_> = (0..MAX_PAGES)
        .map(|p| {
            let n = if p == 64 { 1 } else { 128 };
            PageRef {
                page: p as u16,
                rows: n,
                length: 24 + 12 * u32::from(n),
                digest: D([3; 48]),
            }
        })
        .collect();
    let mut context = [0; MAX_CONTEXT];
    assert_eq!(
        encode_context(fields(8193), &grefs, &mut context).unwrap(),
        3804
    );
    let c = Context::parse(&context).unwrap();
    assert_eq!(c.page_count(), 65);
    let rrefs: Vec<_> = grefs
        .iter()
        .map(|r| PageRef {
            length: 72 + 52 * u32::from(r.rows),
            ..*r
        })
        .collect();
    let mut descriptor = [0; MAX_DESCRIPTOR];
    assert_eq!(
        encode_descriptor(c, D([4; 48]), &rrefs, &mut descriptor).unwrap(),
        7512
    );
    Descriptor::parse(&descriptor).unwrap();
    assert!(encode_context(fields(8194), &grefs, &mut context).is_err());
    assert!(Context::parse(&descriptor).is_err());
    let mut trailing = descriptor.to_vec();
    trailing.push(0);
    assert!(Descriptor::parse(&trailing).is_err());
    let mut bad = context;
    bad[15] = 1;
    assert!(Context::parse(&bad).is_err());
    assert!(encode_context(fields(8193), &grefs, &mut context[..3803]).is_err());
}
#[test]
fn page_identity_count_digest_and_monotonic_inventory_checks() {
    let mut gb = [0; 48];
    encode_geometry(
        0,
        &[
            GeometryRow { id: 7, length: 0 },
            GeometryRow { id: 9, length: 1 },
        ],
        &mut gb,
    )
    .unwrap();
    let gr = PageRef {
        page: 0,
        rows: 2,
        length: 48,
        digest: sha384(&gb),
    };
    let mut cb = [0; 220];
    encode_context(fields(2), &[gr], &mut cb).unwrap();
    let c = Context::parse(&cb).unwrap();
    let mut rb = [0; 176];
    encode_roots(
        c.id(),
        0,
        &[
            RootRow {
                id: 7,
                root: D([5; 48]),
            },
            RootRow {
                id: 9,
                root: D([6; 48]),
            },
        ],
        &mut rb,
    )
    .unwrap();
    let rr = PageRef {
        page: 0,
        rows: 2,
        length: 176,
        digest: sha384(&rb),
    };
    let mut db = [0; 344];
    encode_descriptor(c, D([7; 48]), &[rr], &mut db).unwrap();
    let d = Descriptor::parse(&db).unwrap();
    let mut inv = Inventory::new(d);
    assert!(inv.complete().is_err());
    inv.accept(
        GeometryPage::parse(&gb).unwrap(),
        RootPage::parse(&rb).unwrap(),
    )
    .unwrap();
    inv.complete().unwrap();
    assert!(
        inv.accept(
            GeometryPage::parse(&gb).unwrap(),
            RootPage::parse(&rb).unwrap()
        )
        .is_err()
    );
    assert!(inv.complete().is_err());
    assert!(
        encode_geometry(
            0,
            &[
                GeometryRow { id: 9, length: 1 },
                GeometryRow { id: 9, length: 1 }
            ],
            &mut gb
        )
        .is_err()
    );
    let mut changed = rb;
    changed[175] ^= 1;
    assert!(
        d.check_page_pair(
            0,
            GeometryPage::parse(&gb).unwrap(),
            RootPage::parse(&changed).unwrap()
        )
        .is_err()
    );
}
#[test]
fn segment_boundaries_empty_and_tag_failure() {
    let s = ObjectScope::new(D([8; 48]), 1, 3, 4, MAX_OBJECT_BYTES).unwrap();
    assert_eq!(s.segment(1048575).unwrap(), 0);
    assert_eq!(s.segment(1048576).unwrap(), 1);
    assert_eq!(s.segment(67108863).unwrap(), 63);
    assert!(s.segment(67108864).is_err());
    assert!(encode_segment_info(s, 64, &mut [0; 116]).is_err());
    let empty = ObjectScope::new(D([8; 48]), 1, 3, 4, 0).unwrap();
    assert_eq!(empty.leaf_count(), 0);
    assert!(leaf_nonce(empty, 0).is_err());
    assert!(ObjectScope::new(D([8; 48]), 0, 3, 4, 0).is_err());
    let short = ObjectScope::new(D([8; 48]), 1, 3, 4, SEGMENT_LEAVES * LEAF_BYTES + 1).unwrap();
    let index = SEGMENT_LEAVES;
    assert_eq!(leaf_nonce(short, index).unwrap()[4..], index.to_be_bytes());
    let mut info0 = [0; 116];
    let mut info1 = [0; 116];
    encode_segment_info(short, 0, &mut info0).unwrap();
    encode_segment_info(short, 1, &mut info1).unwrap();
    assert_ne!(info0, info1);
    let mut byte = [42];
    let mut tag = [0; 16];
    seal_leaf_raw(&[9; 32], short, index, &mut byte, &mut tag).unwrap();
    let ciphertext = byte;
    open_leaf_raw(&[9; 32], short, index, &mut byte, &tag).unwrap();
    assert_eq!(byte, [42]);
    byte = ciphertext;
    tag[0] ^= 1;
    assert!(open_leaf_raw(&[9; 32], short, index, &mut byte, &tag).is_err());
    assert_eq!(byte, [0]);
}
#[test]
fn private_record_bounds_and_final_bit() {
    let h = RecordHeader {
        transcript: D([1; 48]),
        request: [2; 16],
        total: 8192,
        offset: 7168,
        chunk: 1024,
        final_chunk: true,
    };
    let mut b = [0; 88];
    assert_eq!(h.encode(&mut b).unwrap(), 88);
    assert_eq!(RecordHeader::parse(&b).unwrap(), h);
    for bad in [
        RecordHeader { offset: 7169, ..h },
        RecordHeader { total: 8193, ..h },
        RecordHeader { chunk: 0, ..h },
        RecordHeader {
            final_chunk: false,
            ..h
        },
    ] {
        assert!(bad.encode(&mut b).is_err());
    }
    b[87] = 2;
    assert!(RecordHeader::parse(&b).is_err());
}

#[test]
fn recipient_wrap_and_delivery_reject_cross_binding_and_noncanonical_bytes() {
    let f = RecipientFields {
        descriptor: D([1; 48]),
        context: D([2; 48]),
        recipient: D([3; 48]),
        publisher: D([4; 48]),
        service: D([5; 48]),
        purpose: D([6; 48]),
        instrument: D([7; 48]),
        scope: D([8; 48]),
        operation: [9; 16],
        closure: D([10; 48]),
        policy: D([11; 48]),
        consumer: D([12; 48]),
        source: D([13; 48]),
        source_length: 64,
        key_epoch: 1,
        not_before: 10,
        expires: 20,
        ml_public: &[14; 1184],
        dh_public: &[15; 32],
    };
    let mut rb = [0; RECIPIENT_BYTES];
    encode_recipient(f, &mut rb).unwrap();
    let record = RecipientRecord::parse(&rb).unwrap();
    assert!(record.valid_at(9).is_err());
    record.valid_at(10).unwrap();
    assert!(record.valid_at(20).is_err());
    assert!(encode_recipient(RecipientFields { expires: 10, ..f }, &mut rb).is_err());
    let record = RecipientRecord::parse(&rb).unwrap();
    let wc = WrapContext::new(record, 352, &[16; 32], &[17; 1088]).unwrap();
    let mut wb = [0; WRAP_BYTES];
    seal_master_raw(wc, &[18; 32], &[19; 32], &[20; 32], &mut wb).unwrap();
    let wrap = Wrap::parse(&wb).unwrap();
    let mut master = [0; 32];
    open_master_raw(wc, wrap, &[18; 32], &[19; 32], &mut master).unwrap();
    assert_eq!(master, [20; 32]);
    assert!(open_master_raw(wc, wrap, &[21; 32], &[19; 32], &mut master).is_err());
    assert_eq!(master, [0; 32]);
    let mut delivery = [0; MAX_DELIVERY];
    let n = encode_delivery([22; 16], record, &[23; 4608], wrap, &mut delivery).unwrap();
    assert_eq!(n, MAX_DELIVERY);
    Delivery::parse(&delivery).unwrap();
    assert!(encode_delivery([22; 16], record, &[], wrap, &mut delivery).is_err());
    assert!(encode_delivery([22; 16], record, &[23; 4609], wrap, &mut delivery).is_err());
    for offset in [
        12usize,
        76,
        1934,
        1936 + 4608,
        MAX_DELIVERY - WRAP_BYTES + 16,
    ] {
        let mut bad = delivery;
        bad[offset] ^= 1;
        assert!(Delivery::parse(&bad).is_err(), "offset {offset}");
    }
    for offset in [14usize, 15] {
        let mut bad = rb;
        bad[offset] = 1;
        assert!(RecipientRecord::parse(&bad).is_err());
        let mut bad = wb;
        bad[offset] = 1;
        assert!(Wrap::parse(&bad).is_err());
    }
    let mut mismatch = wb;
    mismatch[16] ^= 1;
    assert!(wc.matches(Wrap::parse(&mismatch).unwrap()).is_err());
    assert!(Wrap::parse(&wb[..WRAP_BYTES - 1]).is_err());
    assert!(RecipientRecord::parse(&rb[..RECIPIENT_BYTES - 1]).is_err());
}
