use super::*;

#[test]
fn ciphertext_reference_is_exactly_one_quin_wide() {
    assert_eq!(core::mem::size_of::<HeCiphertextRef>(), 48);
    assert_eq!(core::mem::align_of::<HeCiphertextRef>(), 8);
}

#[test]
fn fixed_point_codec_is_caller_buffered_and_bounded() {
    let input = [1.25, -2.5, 0.125];
    let mut encoded = [0_i64; 3];
    encode_fixed_point_into(&input, 1_000, 10_000, &mut encoded).unwrap();
    assert_eq!(encoded, [1_250, -2_500, 125]);
    let mut decoded = [0.0; 3];
    decode_fixed_point_into(&encoded, 1_000, &mut decoded).unwrap();
    assert_eq!(decoded, input);
}

#[test]
fn bfv_encrypt_add_multiply_dot_and_external_round_trip() {
    let engine = BfvEngine::generate_test_context(0xA11CE).unwrap();
    let left = engine.encrypt_i64(1, &[2, 3, 4]).unwrap();
    let right = engine.encrypt_i64(2, &[5, 7, 11]).unwrap();

    let sum = engine.add(3, &left, &right).unwrap();
    let mut sum_out = [0_i64; 3];
    engine.decrypt_i64_into(&sum, &mut sum_out).unwrap();
    assert_eq!(sum_out, [7, 10, 15]);

    let product = engine.multiply(4, &left, &right).unwrap();
    let mut product_out = [0_i64; 3];
    engine.decrypt_i64_into(&product, &mut product_out).unwrap();
    assert_eq!(product_out, [10, 21, 44]);

    let dot = engine.dot_product(5, &left, &right).unwrap();
    let mut dot_out = [0_i64; 1];
    engine.decrypt_i64_into(&dot, &mut dot_out).unwrap();
    assert_eq!(dot_out, [75]);

    let required = product.inner.to_bytes().len();
    let mut storage = vec![0_u8; required];
    let written = engine.serialize_into(&product, &mut storage).unwrap();
    let restored = engine.deserialize(6, 3, &storage[..written]).unwrap();
    let mut restored_out = [0_i64; 3];
    engine
        .decrypt_i64_into(&restored, &mut restored_out)
        .unwrap();
    assert_eq!(restored_out, product_out);
    assert_eq!(restored.reference().slot_count(), 3);
    assert_eq!(restored.reference().scheme(), Some(HeScheme::Bfv));

    let verified = engine
        .deserialize_verified(product.reference(), &storage[..written])
        .unwrap();
    engine
        .decrypt_i64_into(&verified, &mut restored_out)
        .unwrap();
    storage[written / 2] ^= 1;
    assert!(matches!(
        engine.deserialize_verified(product.reference(), &storage[..written]),
        Err(HeError::CommitmentMismatch)
    ));
}

#[test]
fn fresh_bfv_encryptions_have_distinct_commitments() {
    let engine = BfvEngine::generate_test_context(7).unwrap();
    let first = engine.encrypt_i64(1, &[42]).unwrap().reference();
    let second = engine.encrypt_i64(2, &[42]).unwrap().reference();
    assert_ne!(
        (first.commitment_lo, first.commitment_hi),
        (second.commitment_lo, second.commitment_hi)
    );
}

#[test]
#[ignore = "production 128-bit BFV key generation is intentionally expensive in debug builds"]
fn production_parameter_set_encrypts_and_decrypts() {
    let engine = BfvEngine::generate_128_bit(9).unwrap();
    let ciphertext = engine.encrypt_i64(1, &[7, -3]).unwrap();
    let mut out = [0_i64; 2];
    engine.decrypt_i64_into(&ciphertext, &mut out).unwrap();
    assert_eq!(out, [7, -3]);
    assert_eq!(engine.parameters.degree(), MAX_BFV_PACKED_SLOTS);
    assert!(engine.serialized_context_bytes() <= MAX_SERIALIZED_HE_CONTEXT_BYTES);
    eprintln!(
        "serialized BFV context: {} bytes",
        engine.serialized_context_bytes()
    );
}
