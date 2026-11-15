use super::*;
#[test]
fn checkpoint_exact_layout_roundtrip_and_every_truncation_refuses() {
    let identity = StorageIdentity::new(fields(1, Representation::Plaintext)).unwrap();
    let mut payload = [0; BITMAP_BYTES];
    let page = CandidateBitmapPage::new(identity, 0, &mut payload).unwrap();
    let mut out = [0xaa; CHECKPOINT_BYTES];
    assert_eq!(
        encode_checkpoint(&page, 0x0102030405060708, &mut out),
        Ok(400)
    );
    assert!(CHECKPOINT_BYTES <= 512);
    assert_eq!(&out[..16], b"QBSCP01\0\0\x01\0\0\0\0\0\0");
    let mut embedded = [0; IDENTITY_BYTES];
    encode_identity(&identity, &mut embedded).unwrap();
    assert_eq!(&out[16..272], &embedded);
    assert_eq!(&out[272..280], &[0; 8]);
    assert_eq!(&out[280..284], &[0, 0, 16, 0]);
    assert_eq!(&out[336..344], &[1, 2, 3, 4, 5, 6, 7, 8]);
    assert_eq!(
        &out[288..336],
        crate::crypto::network::digest::sha384(page.payload()).as_bytes()
    );
    assert_eq!(
        &out[352..400],
        crate::crypto::network::digest::sha384(&out[..352]).as_bytes()
    );
    let decoded = decode_checkpoint(&out).unwrap();
    assert_eq!(decoded.sequence(), 0x0102030405060708);
    let hint = decoded.bind_payload(&identity, 0, page.payload()).unwrap();
    assert_eq!(hint.claims_leaf(0), Ok(false));
    for end in 0..400 {
        assert_eq!(
            decode_checkpoint(&out[..end]).err(),
            Some(StorageError::Length)
        );
    }
    let mut extended = out.to_vec();
    extended.push(0);
    assert_eq!(
        decode_checkpoint(&extended).err(),
        Some(StorageError::Length)
    );
    assert_eq!(
        encode_checkpoint(&page, 1, &mut out[..399]),
        Err(StorageError::Capacity)
    );
}
#[test]
fn wrong_identity_page_payload_and_nonzero_tail_are_never_accepted_as_resume_binding() {
    let bytes = [3u8; 1];
    let identity = one_leaf(&bytes, Representation::Plaintext, 0);
    let view = verify_leaf_view(&identity, 0, &bytes, &[]).unwrap();
    let mut payload = [0; BITMAP_BYTES];
    let mut page = CandidateBitmapPage::new(identity, 0, &mut payload).unwrap();
    page.record_integrity(&view).unwrap();
    let mut raw = [0; CHECKPOINT_BYTES];
    encode_checkpoint(&page, 7, &mut raw).unwrap();
    drop(page);
    let decoded = decode_checkpoint(&raw).unwrap();
    let mut f = *identity.fields();
    f.policy_generation ^= 1;
    let wrong = StorageIdentity::new(f).unwrap();
    assert_eq!(
        decoded.bind_payload(&wrong, 0, &payload).err(),
        Some(StorageError::IdentityMismatch)
    );
    assert_eq!(
        decoded.bind_payload(&identity, 1, &payload).err(),
        Some(StorageError::PageMismatch)
    );
    let mut changed = payload;
    changed[0] ^= 1;
    assert_eq!(
        decoded.bind_payload(&identity, 0, &changed).err(),
        Some(StorageError::DigestMismatch)
    );
    let mut bad = payload;
    bad[0] |= 2;
    raw[288..336].copy_from_slice(crate::crypto::network::digest::sha384(&bad).as_bytes());
    checksum_record(&mut raw);
    let decoded = decode_checkpoint(&raw).unwrap();
    assert_eq!(
        decoded.bind_payload(&identity, 0, &bad).err(),
        Some(StorageError::Bounds)
    );
}
#[test]
fn attacker_recomputed_checkpoint_remains_an_untrusted_hint_and_cannot_seed_current_marks() {
    let bytes = [4u8; 1];
    let identity = one_leaf(&bytes, Representation::Plaintext, 0);
    let mut payload = [0; BITMAP_BYTES];
    let page = CandidateBitmapPage::new(identity, 0, &mut payload).unwrap();
    let mut raw = [0; CHECKPOINT_BYTES];
    encode_checkpoint(&page, 1, &mut raw).unwrap();
    drop(page);
    payload[0] = 1;
    raw[288..336].copy_from_slice(crate::crypto::network::digest::sha384(&payload).as_bytes());
    checksum_record(&mut raw);
    let decoded = decode_checkpoint(&raw).unwrap();
    let hint = decoded.bind_payload(&identity, 0, &payload).unwrap();
    assert_eq!(hint.claims_leaf(0), Ok(true));
    drop(hint);
    let fresh = CandidateBitmapPage::new(identity, 0, &mut payload).unwrap();
    assert_eq!(fresh.has_integrity_mark(0), Ok(false));
}
#[test]
fn checkpoint_flags_reserved_digest_and_embedded_identity_are_all_checked() {
    let identity = StorageIdentity::new(fields(1, Representation::Plaintext)).unwrap();
    let mut payload = [0; BITMAP_BYTES];
    let page = CandidateBitmapPage::new(identity, 0, &mut payload).unwrap();
    let mut raw = [0; CHECKPOINT_BYTES];
    encode_checkpoint(&page, 0, &mut raw).unwrap();
    for bit in 0..16 {
        let mut bad = raw;
        bad[10..12].copy_from_slice(&(1u16 << bit).to_be_bytes());
        checksum_record(&mut bad);
        assert_eq!(decode_checkpoint(&bad).err(), Some(StorageError::Flags));
    }
    for i in (12..16).chain(284..288).chain(344..352) {
        let mut bad = raw;
        bad[i] = 1;
        checksum_record(&mut bad);
        assert_eq!(decode_checkpoint(&bad).err(), Some(StorageError::Reserved));
    }
    let mut bad = raw;
    bad[32] ^= 1;
    assert_eq!(
        decode_checkpoint(&bad).err(),
        Some(StorageError::DigestMismatch)
    );
    let mut bad = raw;
    bad[16 + 125] = 2;
    checksum_record(&mut bad);
    assert_eq!(
        decode_checkpoint(&bad).err(),
        Some(StorageError::Unsupported)
    );
    let mut bad = raw;
    bad[272..280].copy_from_slice(&u64::MAX.to_be_bytes());
    checksum_record(&mut bad);
    assert_eq!(decode_checkpoint(&bad).err(), Some(StorageError::Overflow));
    let mut bad = raw;
    bad[280..284].copy_from_slice(&0u32.to_be_bytes());
    checksum_record(&mut bad);
    assert_eq!(decode_checkpoint(&bad).err(), Some(StorageError::Length));
}
