use super::*;
use crate::net::qdnf::types::StrongDigest;
use serde_json::Value;
fn bytes(v: &Value) -> Vec<u8> {
    let h = v.as_str().unwrap();
    assert_eq!(h.len() % 2, 0);
    h.as_bytes()
        .chunks_exact(2)
        .map(|p| u8::from_str_radix(core::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
fn array<const N: usize>(v: &Value) -> [u8; N] {
    bytes(v).try_into().unwrap()
}
#[test]
fn actual_intent_codec_and_client_transcript_match_independent_fixture() {
    // Independent Node fixture has synthetic public shares/proofs, not real agreement.
    let vectors: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../tools/qbdl-stream-conformance/fixtures/endpoint-v1.json"
    )))
    .unwrap();
    let v = &vectors["base"];
    let f = &v["intent_fields"];
    let intent = Intent {
        initiator: StrongDigest(array(&f["initiator_hex"])),
        responder: StrongDigest(array(&f["responder_hex"])),
        service: StrongDigest(array(&f["service_hex"])),
        purpose: StrongDigest(array(&f["purpose_hex"])),
        instrument: StrongDigest(array(&f["instrument_hex"])),
        scope: StrongDigest(array(&f["scope_hex"])),
        operation: OperationId(array(&f["operation_hex"])),
        initiator_keys: cose::KeyIds {
            ml_dsa_65: array(&f["initiator_ml_key_id_hex"]),
            ed25519: array(&f["initiator_ed_key_id_hex"]),
        },
        responder_keys: cose::KeyIds {
            ml_dsa_65: array(&f["responder_ml_key_id_hex"]),
            ed25519: array(&f["responder_ed_key_id_hex"]),
        },
        initiator_epoch: u64::from_be_bytes(array(&f["initiator_controller_epoch_hex"])),
        responder_epoch: u64::from_be_bytes(array(&f["responder_controller_epoch_hex"])),
        expires_unix: u64::from_be_bytes(array(&f["expires_unix_hex"])),
    };
    let mut encoded = [0; INTENT_BYTES];
    assert_eq!(intent.encode(&mut encoded).unwrap(), 520);
    assert_eq!(encoded.as_slice(), bytes(&v["common_intent_hex"]));
    let offer = super::super::wire::offer(1400);
    assert_eq!(offer.as_slice(), bytes(&v["offer_hex"]));
    assert_eq!(
        super::super::wire::selection(1500, 1400).as_slice(),
        bytes(&v["selection_hex"])
    );
    let transcript = super::super::transcript::client(
        intent,
        &array(&v["nonces_hex"]["client"]),
        &offer,
        &bytes(&v["synthetic_share_inputs_hex"]["initiator"]),
    )
    .unwrap();
    assert_eq!(
        transcript.as_bytes(),
        bytes(&v["transcript"][0]["encoded_items_hex"])
    );
    assert_eq!(
        transcript.digest().0,
        array::<48>(&v["transcript"][0]["digest_hex"])
    );
    assert_eq!(transcript.as_bytes().len(), 1887);
}
