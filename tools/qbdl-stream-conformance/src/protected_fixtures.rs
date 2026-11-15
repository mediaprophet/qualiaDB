//! Independent Node bytes and primitives; synthetic KEM inputs are not KEM interop.
use crate::object_stream::*;
use crate::qdnf_types::StrongDigest as D;
use serde_json::Value;
fn corpus() -> Value {
    serde_json::from_str(include_str!("../fixtures/protected-v1.json")).unwrap()
}
fn hex(v: &Value) -> Vec<u8> {
    let s = v.as_str().unwrap();
    assert_eq!(s.len() % 2, 0);
    s.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn digest(v: &Value) -> D {
    D(hex(v).try_into().unwrap())
}
fn number(v: &Value) -> u64 {
    v.as_u64().unwrap()
}
#[test]
fn independent_context_descriptor_and_page_commitments() {
    let fixture = corpus();
    for case in fixture["cases"].as_array().unwrap() {
        let cb = hex(&case["context"]["hex"]);
        let db = hex(&case["protected_descriptor"]["hex"]);
        let c = Context::parse(&cb).unwrap();
        let d = Descriptor::parse(&db).unwrap();
        assert_eq!(c.id(), digest(&case["context"]["id_hex"]));
        assert_eq!(
            d.id(),
            digest(&case["protected_descriptor"]["descriptor_id_hex"])
        );
        assert_eq!(d.context().encoded(), cb);
        let refs: Vec<_> = (0..c.page_count())
            .map(|i| c.page_ref(i).unwrap())
            .collect();
        let roots: Vec<_> = (0..c.page_count())
            .map(|i| d.page_ref(i).unwrap())
            .collect();
        let mut out = vec![0; MAX_DESCRIPTOR];
        let n = encode_context(c.fields(), &refs, &mut out).unwrap();
        assert_eq!(&out[..n], cb);
        let n = encode_descriptor(c, d.metadata_root(), &roots, &mut out).unwrap();
        assert_eq!(&out[..n], db);
        let mut inventory = Inventory::new(d);
        for (i, p) in case["context"]["geometry_pages"]
            .as_array()
            .unwrap()
            .iter()
            .enumerate()
        {
            let gb = hex(&p["hex"]);
            let rb = hex(&case["protected_descriptor"]["root_pages"][i]["hex"]);
            let g = GeometryPage::parse(&gb).unwrap();
            let r = RootPage::parse(&rb).unwrap();
            let count = crate::allocation::measured(|| inventory.accept(g, r).unwrap());
            assert_eq!(count, 0);
            let rows: Vec<_> = (0..g.count()).map(|j| g.row(j).unwrap()).collect();
            let n = encode_geometry(i as u16, &rows, &mut out).unwrap();
            assert_eq!(&out[..n], gb);
            let rows: Vec<_> = (0..r.count()).map(|j| r.row(j).unwrap()).collect();
            let n = encode_roots(c.id(), i as u16, &rows, &mut out).unwrap();
            assert_eq!(&out[..n], rb);
            let mut bad = rb.clone();
            *bad.last_mut().unwrap() ^= 1;
            assert!(
                d.check_page_pair(i, g, RootPage::parse(&bad).unwrap())
                    .is_err()
            );
        }
        inventory.complete().unwrap();
        for bytes in [&cb, &db] {
            let mut trailing = bytes.clone();
            trailing.push(0);
            assert!(Context::parse(&trailing).is_err());
            assert!(Descriptor::parse(&trailing).is_err());
        }
    }
}
#[test]
fn independent_recipient_wrap_and_private_delivery_bytes() {
    let fixture = corpus();
    for case in fixture["cases"].as_array().unwrap() {
        let ab = hex(&case["recipient_authorization"]["hex"]);
        let wb = hex(&case["wrap"]["hex"]);
        let qb = hex(&case["private_delivery"]["hex"]);
        let a = RecipientRecord::parse(&ab).unwrap();
        let w = Wrap::parse(&wb).unwrap();
        let q = Delivery::parse(&qb).unwrap();
        assert_eq!(
            a.id(),
            digest(&case["recipient_authorization"]["auth_id_hex"])
        );
        assert_eq!(
            a.key_ids().ml_kem_768,
            digest(&case["recipient_authorization"]["ml_id_hex"])
        );
        assert_eq!(
            a.key_ids().x25519,
            digest(&case["recipient_authorization"]["dh_id_hex"])
        );
        let wc = WrapContext::new(
            a,
            number(&case["private_metadata"]["length"]),
            w.ephemeral,
            w.ciphertext,
        )
        .unwrap();
        let mut context = [0; WRAP_CONTEXT_BYTES];
        wc.encode(&mut context).unwrap();
        assert_eq!(&context[..], hex(&case["wrap"]["context_hex"]));
        let mut master = [0; 32];
        open_master_raw(wc, w, &[0x91; 32], &[0x92; 32], &mut master).unwrap();
        assert_eq!(&master[..], hex(&case["wrap"]["metadata_master_hex"]));
        let mut out = vec![0; MAX_DELIVERY];
        let n = seal_master_raw(wc, &[0x91; 32], &[0x92; 32], &master, &mut out).unwrap();
        assert_eq!(&out[..n], wb);
        let n = encode_recipient(a.fields(), &mut out).unwrap();
        assert_eq!(&out[..n], ab);
        let n = encode_delivery(q.request, a, q.proof, w, &mut out).unwrap();
        assert_eq!(&out[..n], qb);
        let mut bad = wb.clone();
        *bad.last_mut().unwrap() ^= 1;
        assert!(
            open_master_raw(
                wc,
                Wrap::parse(&bad).unwrap(),
                &[0x91; 32],
                &[0x92; 32],
                &mut master
            )
            .is_err()
        );
        assert_eq!(master, [0; 32]);
    }
}
fn check_leaf(
    v: &Value,
    context: D,
    master: &[u8; 32],
    id: u32,
    generation: u64,
    epoch: u32,
    plain: &[u8],
) {
    let s = ObjectScope::new(context, id, generation, epoch, number(&v["logical_length"])).unwrap();
    let index = number(&v["index"]);
    let segment = s.segment(index).unwrap();
    let mut info = [0; SEGMENT_INFO_BYTES];
    encode_segment_info(s, segment, &mut info).unwrap();
    assert_eq!(&info[..], hex(&v["segment_info_hex"]));
    let mut key = [0; 32];
    derive_segment(master, s, segment, &mut key).unwrap();
    assert_eq!(&key[..], hex(&v["key_hex"]));
    let mut aad = [0; LEAF_AAD_BYTES];
    encode_leaf_aad(s, index, &mut aad).unwrap();
    assert_eq!(&aad[..], hex(&v["aad_hex"]));
    assert_eq!(&leaf_nonce(s, index).unwrap()[..], hex(&v["nonce_hex"]));
    let mut bytes = plain.to_vec();
    let mut tag = [0; 16];
    let calls = crate::allocation::measured(|| {
        seal_leaf_raw(master, s, index, &mut bytes, &mut tag).unwrap()
    });
    assert_eq!(calls, 0);
    assert_eq!(bytes, hex(&v["ciphertext_hex"]));
    assert_eq!(&tag[..], hex(&v["tag_hex"]));
    let encrypted = bytes.clone();
    open_leaf_raw(master, s, index, &mut bytes, &tag).unwrap();
    assert_eq!(bytes, plain);
    for altered in [
        ObjectScope::new(D([0x44; 48]), id, generation, epoch, s.length()).unwrap(),
        ObjectScope::new(context, id, generation, epoch + 1, s.length()).unwrap(),
    ] {
        bytes.copy_from_slice(&encrypted);
        assert!(open_leaf_raw(master, altered, index, &mut bytes, &tag).is_err());
        assert!(bytes.iter().all(|b| *b == 0));
    }
    bytes.copy_from_slice(&encrypted);
    tag[15] ^= 1;
    assert!(open_leaf_raw(master, s, index, &mut bytes, &tag).is_err());
    assert!(bytes.iter().all(|b| *b == 0));
}
#[test]
fn independent_segment_keys_nonces_aad_and_aead_boundaries() {
    let f = corpus();
    let v = &f["crypto_vectors"];
    let b = &v["base"];
    let context = digest(&f["cases"][1]["context"]["id_hex"]);
    let master: [u8; 32] = hex(&b["master_hex"]).try_into().unwrap();
    let id = number(&b["id"]) as u32;
    let generation = number(&b["generation"]);
    let epoch = number(&b["epoch"]) as u32;
    check_leaf(
        b,
        context,
        &master,
        id,
        generation,
        epoch,
        &hex(&b["plaintext_hex"]),
    );
    for row in v["boundaries"].as_array().unwrap() {
        check_leaf(
            row,
            context,
            &master,
            id,
            generation,
            epoch,
            &vec![(number(&row["index"]) & 255) as u8; 16384],
        );
    }
    let empty = ObjectScope::new(context, id, generation, epoch, 0).unwrap();
    assert_eq!(empty.leaf_count(), 0);
    assert!(leaf_nonce(empty, 0).is_err());
    assert!(derive_segment(&master, empty, 0, &mut [0; 32]).is_err());
}
