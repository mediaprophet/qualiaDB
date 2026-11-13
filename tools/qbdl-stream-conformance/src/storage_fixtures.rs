use crate::allocation::measured;
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::types::StrongDigest;
use crate::stream_storage as storage;
use serde_json::Value;

fn hex(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0, "odd-length hex");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}
fn corpus() -> Value {
    serde_json::from_str(include_str!("../fixtures/storage-v1.json")).unwrap()
}
fn bytes_at<'a>(v: &'a Value, key: &str) -> Vec<u8> {
    hex(v[key].as_str().unwrap())
}
fn expect_storage_error<T>(
    result: Result<T, storage::StorageError>,
    expected: storage::StorageError,
) {
    match result {
        Err(actual) => assert_eq!(actual, expected),
        Ok(_) => panic!("expected storage error {expected:?}"),
    }
}

fn assert_fixed_identity(
    id: &str,
    decoded: &storage::StorageIdentity,
    expected_length: u64,
    expected_repr: u8,
    expected_class: u8,
    expected_epoch: u32,
) {
    let fields = decoded.fields();
    assert_eq!(fields.descriptor, StrongDigest([0x11; 48]));
    assert_eq!(fields.context, StrongDigest([0x22; 48]));
    assert_eq!(
        fields.entry,
        match id {
            "empty-entry-no-bitmap-pages" => 0x01020305,
            "last-partial-page-two-page-identity" => 0x01020306,
            _ => 0x01020304,
        }
    );
    assert_eq!(fields.generation, 0x0102030405060708);
    assert_eq!(fields.representation as u8, expected_repr);
    assert_eq!(fields.storage_class as u8, expected_class);
    assert_eq!(fields.key_epoch, expected_epoch);
    assert_eq!(fields.logical_length, expected_length);
    assert_eq!(fields.root, StrongDigest([0x33; 48]));
    assert_eq!(fields.privacy_partition, StrongDigest([0x44; 48]));
    assert_eq!(fields.policy_generation, 0x1112131415161718);
    assert_eq!(fields.storage_generation, 0x2122232425262728);
}

#[test]
fn identity_and_checkpoint_encodings_match_independent_wire_vectors() {
    let c = corpus();
    let identities = c["identity_vectors"].as_array().unwrap();
    assert_eq!(identities.len(), 5);
    for v in identities {
        let raw = bytes_at(v, "identity_hex");
        let identity = storage::decode_identity(&raw).unwrap();
        let expected_len = v["logical_length"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        let repr = v["representation"].as_u64().unwrap() as u8;
        let class = v["storage_class"].as_u64().unwrap() as u8;
        let epoch = v["key_epoch"].as_u64().unwrap() as u32;
        assert_fixed_identity(
            v["id"].as_str().unwrap(),
            &identity,
            expected_len,
            repr,
            class,
            epoch,
        );
        assert_eq!(
            identity.leaf_count(),
            expected_len / 16_384 + if expected_len % 16_384 == 0 { 0 } else { 1 }
        );
        let mut encoded = [0u8; storage::IDENTITY_BYTES];
        assert_eq!(
            storage::encode_identity(&identity, &mut encoded).unwrap(),
            storage::IDENTITY_BYTES
        );
        assert_eq!(encoded.as_slice(), raw.as_slice());
    }

    for v in c["malformed_identity_vectors"].as_array().unwrap() {
        let raw = bytes_at(v, "identity_hex");
        assert!(
            storage::decode_identity(&raw).is_err(),
            "{}",
            v["id"].as_str().unwrap()
        );
    }
    for v in c["malformed_checkpoint_vectors"].as_array().unwrap() {
        let raw = bytes_at(v, "checkpoint_hex");
        assert!(
            storage::decode_checkpoint(&raw).is_err(),
            "{}",
            v["id"].as_str().unwrap()
        );
    }

    let checkpoints = c["valid_checkpoints"].as_array().unwrap();
    assert_eq!(checkpoints.len(), 2);
    for v in checkpoints {
        let identity_bytes = bytes_at(v, "identity_hex");
        let checkpoint_bytes = bytes_at(v, "checkpoint_hex");
        let bitmap_bytes = bytes_at(v, "bitmap_hex");
        assert_eq!(identity_bytes.len(), storage::IDENTITY_BYTES);
        assert_eq!(checkpoint_bytes.len(), storage::CHECKPOINT_BYTES);
        assert_eq!(bitmap_bytes.len(), storage::BITMAP_BYTES);
        assert_eq!(
            sha384(&bitmap_bytes).as_bytes().as_slice(),
            hex(v["bitmap_sha384_hex"].as_str().unwrap()).as_slice()
        );
        assert_eq!(
            sha384(&checkpoint_bytes[..352]).as_bytes().as_slice(),
            hex(v["checkpoint_prefix_sha384_hex"].as_str().unwrap()).as_slice()
        );

        let identity = storage::decode_identity(&identity_bytes).unwrap();
        let decoded = storage::decode_checkpoint(&checkpoint_bytes).unwrap();
        assert_eq!(decoded.identity(), &identity);
        assert_eq!(
            decoded.page_number(),
            v["expected_page_number"].as_u64().unwrap()
        );
        let sequence = u64::from_be_bytes(checkpoint_bytes[336..344].try_into().unwrap());
        assert_eq!(decoded.sequence(), sequence);
        assert_eq!(
            decoded.page_number(),
            v["expected_page_number"].as_u64().unwrap()
        );
        let mut bitmap: [u8; storage::BITMAP_BYTES] = [0; storage::BITMAP_BYTES];
        bitmap.copy_from_slice(&bitmap_bytes);
        let hint = decoded
            .bind_payload(&identity, decoded.page_number(), &bitmap)
            .unwrap();
        assert_eq!(hint.identity(), &identity);
        assert_eq!(hint.page_number(), decoded.page_number());
        assert_eq!(hint.sequence(), sequence);
        let id = v["id"].as_str().unwrap();
        if id == "plaintext-empty-bitmap-page" {
            assert!(!hint.claims_leaf(0).unwrap());
            // CandidateBitmapPage::new is allowed to encode this cleared page.
            let mut fresh_payload = [0u8; storage::BITMAP_BYTES];
            let page = storage::CandidateBitmapPage::new(identity, 0, &mut fresh_payload).unwrap();
            let mut encoded = [0u8; storage::CHECKPOINT_BYTES];
            assert_eq!(
                storage::encode_checkpoint(&page, sequence, &mut encoded).unwrap(),
                storage::CHECKPOINT_BYTES
            );
            assert_eq!(encoded.as_slice(), checkpoint_bytes.as_slice());
        } else {
            assert!(hint.claims_leaf(storage::LEAVES_PER_PAGE).unwrap());
            assert_eq!(
                hint.claims_leaf(0),
                Err(storage::StorageError::PageMismatch)
            );
            assert_eq!(
                hint.claims_leaf(storage::LEAVES_PER_PAGE + 1),
                Err(storage::StorageError::Bounds)
            );
            // Nonzero bitmap bits are decoded as untrusted hints here. This harness
            // does not forge a CandidateBitmapPage or claim leaf verification.
        }
    }
}

#[test]
fn bound_resume_hints_match_expected_refusals() {
    let c = corpus();
    for v in c["bind_refusal_vectors"].as_array().unwrap() {
        let id = v["id"].as_str().unwrap();
        let checkpoint_bytes = bytes_at(v, "checkpoint_hex");
        let checkpoint = storage::decode_checkpoint(&checkpoint_bytes);
        let stage = v["stage"].as_str().unwrap();
        if stage == "checkpoint-decode" {
            assert!(
                checkpoint.is_err(),
                "{} should fail during checkpoint decode",
                id
            );
            continue;
        }
        let checkpoint =
            checkpoint.unwrap_or_else(|_| panic!("{} must pass decode before bind", id));
        let expected_identity_bytes = bytes_at(v, "expected_identity_hex");
        let expected_identity = storage::decode_identity(&expected_identity_bytes).unwrap();
        let page = v["expected_page_number"].as_u64().unwrap();
        let leaf_count = v["entry_leaf_count"]
            .as_str()
            .unwrap()
            .parse::<u64>()
            .unwrap();
        if id == "bind-bitmap-truncated" {
            let bitmap = bytes_at(v, "bitmap_hex");
            assert_eq!(bitmap.len(), storage::BITMAP_BYTES - 1);
            assert!(<&[u8; storage::BITMAP_BYTES]>::try_from(bitmap.as_slice()).is_err());
            assert!(leaf_count > 0);
            continue;
        }
        let bitmap_bytes = bytes_at(v, "bitmap_hex");
        let mut bitmap = [0u8; storage::BITMAP_BYTES];
        bitmap.copy_from_slice(&bitmap_bytes);
        assert!(
            checkpoint
                .bind_payload(&expected_identity, page, &bitmap)
                .is_err(),
            "{}",
            id
        );
        match id {
            "bind-stale-full-identity" => expect_storage_error(
                checkpoint.bind_payload(&expected_identity, page, &bitmap),
                storage::StorageError::IdentityMismatch,
            ),
            "bind-wrong-expected-page" => expect_storage_error(
                checkpoint.bind_payload(&expected_identity, page, &bitmap),
                storage::StorageError::PageMismatch,
            ),
            "bind-bitmap-digest-mismatch" => expect_storage_error(
                checkpoint.bind_payload(&expected_identity, page, &bitmap),
                storage::StorageError::DigestMismatch,
            ),
            "bind-final-page-nonzero-unused-tail" => expect_storage_error(
                checkpoint.bind_payload(&expected_identity, page, &bitmap),
                storage::StorageError::Bounds,
            ),
            _ => panic!("unexpected bind-stage vector {id}"),
        }
    }
}

#[test]
fn storage_decoders_and_zero_bitmap_checkpoint_encoder_allocate_nothing_after_setup() {
    let c = corpus();
    let valid_identity = c["identity_vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "plaintext-class-zero")
        .unwrap();
    let identity_bytes = bytes_at(valid_identity, "identity_hex");
    let identity = storage::decode_identity(&identity_bytes).unwrap();
    let valid_checkpoint = c["valid_checkpoints"]
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["id"] == "plaintext-empty-bitmap-page")
        .unwrap();
    let checkpoint_bytes = bytes_at(valid_checkpoint, "checkpoint_hex");
    let bitmap_bytes = bytes_at(valid_checkpoint, "bitmap_hex");
    let bad_identity = bytes_at(&c["malformed_identity_vectors"][0], "identity_hex");
    let bad_checkpoint = bytes_at(&c["malformed_checkpoint_vectors"][0], "checkpoint_hex");
    let mut payload = [0u8; storage::BITMAP_BYTES];
    payload.copy_from_slice(&bitmap_bytes);
    let candidate = storage::CandidateBitmapPage::new(identity, 0, &mut payload).unwrap();
    let sequence = 0x0102030405060708u64;
    let mut identity_out = [0u8; storage::IDENTITY_BYTES];
    let mut checkpoint_out = [0u8; storage::CHECKPOINT_BYTES];
    let allocations = measured(|| {
        let decoded_identity = storage::decode_identity(&identity_bytes).unwrap();
        assert_eq!(
            storage::encode_identity(&decoded_identity, &mut identity_out).unwrap(),
            storage::IDENTITY_BYTES
        );
        assert_eq!(identity_out.as_slice(), identity_bytes.as_slice());
        let decoded_checkpoint = storage::decode_checkpoint(&checkpoint_bytes).unwrap();
        let hint = decoded_checkpoint
            .bind_payload(&decoded_identity, 0, candidate.payload())
            .unwrap();
        assert!(!hint.claims_leaf(0).unwrap());
        assert_eq!(
            storage::encode_checkpoint(&candidate, sequence, &mut checkpoint_out).unwrap(),
            storage::CHECKPOINT_BYTES
        );
        assert!(storage::decode_identity(&bad_identity).is_err());
        assert!(storage::decode_checkpoint(&bad_checkpoint).is_err());
    });
    assert_eq!(
        allocations, 0,
        "storage decode/bind/encode allocated after fixture preparation"
    );
    assert_eq!(checkpoint_out.as_slice(), checkpoint_bytes.as_slice());
}
