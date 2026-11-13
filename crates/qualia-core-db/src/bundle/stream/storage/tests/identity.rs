use super::*;
#[test]
fn exact_identity_bytes_offsets_and_big_endian_fields() {
    let identity = StorageIdentity::new(fields(1, Representation::Plaintext)).unwrap();
    let mut out = [0xaa; IDENTITY_BYTES];
    assert_eq!(encode_identity(&identity, &mut out), Ok(256));
    let mut expected = [0; 256];
    expected[..8].copy_from_slice(b"QBSID01\0");
    expected[8..10].copy_from_slice(&[0, 1]);
    expected[16..64].fill(0x11);
    expected[64..112].fill(0x22);
    expected[112..116].copy_from_slice(&[1, 2, 3, 4]);
    expected[116..124].copy_from_slice(&[1, 2, 3, 4, 5, 6, 7, 8]);
    expected[132..140].copy_from_slice(&[0, 0, 0, 0, 0, 0, 0, 1]);
    expected[140..188].fill(0x33);
    expected[188..236].fill(0x44);
    expected[236..244].copy_from_slice(&[0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17, 0x18]);
    expected[244..252].copy_from_slice(&[0x21, 0x22, 0x23, 0x24, 0x25, 0x26, 0x27, 0x28]);
    assert_eq!(out, expected);
    assert_eq!(decode_identity(&expected), Ok(identity));
    let mut short = [0x55; 255];
    assert_eq!(
        encode_identity(&identity, &mut short),
        Err(StorageError::Capacity)
    );
    assert_eq!(short, [0x55; 255]);
    for end in 0..256 {
        assert_eq!(decode_identity(&out[..end]), Err(StorageError::Length));
    }
    let mut extra = out.to_vec();
    extra.push(0);
    assert_eq!(decode_identity(&extra), Err(StorageError::Length));
}
#[test]
fn identity_flags_reserved_classes_and_tree_ceiling_fail_closed() {
    let identity = StorageIdentity::new(fields(1, Representation::Plaintext)).unwrap();
    let mut raw = [0; 256];
    encode_identity(&identity, &mut raw).unwrap();
    for bit in 0..16 {
        let mut bad = raw;
        bad[10..12].copy_from_slice(&(1u16 << bit).to_be_bytes());
        assert_eq!(decode_identity(&bad), Err(StorageError::Flags));
    }
    for i in (12..16).chain(126..128).chain(252..256) {
        let mut bad = raw;
        bad[i] = 1;
        assert_eq!(decode_identity(&bad), Err(StorageError::Reserved));
    }
    for class in 1..=255 {
        let mut bad = raw;
        bad[125] = class;
        assert_eq!(decode_identity(&bad), Err(StorageError::Unsupported));
    }
    let mut bad = raw;
    bad[124] = 2;
    assert_eq!(decode_identity(&bad), Err(StorageError::Unsupported));
    let mut f = fields(1, Representation::Plaintext);
    f.storage_class = StorageClass::PrivatePlaintext;
    assert_eq!(StorageIdentity::new(f), Err(StorageError::Unsupported));
    let mut f = fields(1, Representation::Plaintext);
    f.key_epoch = 1;
    assert_eq!(
        StorageIdentity::new(f),
        Err(StorageError::Proof(ProofError::InvalidSpec))
    );
    assert!(StorageIdentity::new(fields(MAX_LOGICAL_BYTES, Representation::Plaintext)).is_ok());
    assert_eq!(
        StorageIdentity::new(fields(MAX_LOGICAL_BYTES + 1, Representation::Plaintext)),
        Err(StorageError::Proof(ProofError::Limit))
    );
    let mut bad = raw;
    bad[8..10].copy_from_slice(&2u16.to_be_bytes());
    assert_eq!(decode_identity(&bad), Err(StorageError::Version));
    let mut bad = raw;
    bad[0] = 0;
    assert_eq!(decode_identity(&bad), Err(StorageError::Magic));
}

#[test]
fn ciphertext_epoch_zero_is_supported_without_any_encryption_authority_claim() {
    let identity = StorageIdentity::new(fields(1, Representation::Ciphertext)).unwrap();
    let mut raw = [0; IDENTITY_BYTES];
    encode_identity(&identity, &mut raw).unwrap();
    assert_eq!(raw[124], 1);
    assert_eq!(raw[125], 1);
    assert_eq!(decode_identity(&raw), Ok(identity));
}
