use crate::crypto::network::digest::sha384;
use crate::crypto::network::{
    secret_lease::SecretLease, transcript::Transcript, types::KeyPurpose,
};
use crate::net::qdnf::crypto::{
    finished::{finished_mac, verify_finished},
    schedule::{derive_handshake_keys, endpoint::EndpointSecrets},
};
use serde_json::Value;

fn hex(s: &str) -> Vec<u8> {
    assert_eq!(s.len() % 2, 0, "odd-length hex");
    (0..s.len() / 2)
        .map(|i| u8::from_str_radix(&s[i * 2..i * 2 + 2], 16).unwrap())
        .collect()
}
fn hexstr(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(DIGITS[(byte >> 4) as usize] as char);
        out.push(DIGITS[(byte & 15) as usize] as char);
    }
    out
}
fn corpus() -> Value {
    serde_json::from_str(include_str!("../fixtures/endpoint-v1.json")).unwrap()
}
fn arr32(bytes: &[u8]) -> [u8; 32] {
    bytes.try_into().unwrap()
}
fn arr48(bytes: &[u8]) -> [u8; 48] {
    bytes.try_into().unwrap()
}

#[derive(Clone)]
struct Inputs {
    intent: Vec<u8>,
    client_nonce: Vec<u8>,
    responder_nonce: Vec<u8>,
    client_offer: Vec<u8>,
    server_selection: Vec<u8>,
    initiator_share: Vec<u8>,
    responder_share: Vec<u8>,
    client_proof: Vec<u8>,
    server_proof: Vec<u8>,
    kem_ss: [u8; 32],
    dh_ss: [u8; 32],
    wrong_responder_role: bool,
}
impl Inputs {
    fn from_base(base: &Value) -> Self {
        Self {
            intent: hex(base["common_intent_hex"].as_str().unwrap()),
            client_nonce: hex(base["nonces_hex"]["client"].as_str().unwrap()),
            responder_nonce: hex(base["nonces_hex"]["responder"].as_str().unwrap()),
            client_offer: hex(base["offer_hex"].as_str().unwrap()),
            server_selection: hex(base["selection_hex"].as_str().unwrap()),
            initiator_share: hex(base["synthetic_share_inputs_hex"]["initiator"]
                .as_str()
                .unwrap()),
            responder_share: hex(base["synthetic_share_inputs_hex"]["responder"]
                .as_str()
                .unwrap()),
            client_proof: hex(base["synthetic_proof_envelopes_hex"]["client"]
                .as_str()
                .unwrap()),
            server_proof: hex(base["synthetic_proof_envelopes_hex"]["server"]
                .as_str()
                .unwrap()),
            kem_ss: arr32(&hex(
                base["schedule_inputs"]["ml_kem_shared_secret_synthetic_hex"]
                    .as_str()
                    .unwrap(),
            )),
            dh_ss: arr32(&hex(
                base["schedule_inputs"]["x25519_shared_secret_synthetic_hex"]
                    .as_str()
                    .unwrap(),
            )),
            wrong_responder_role: false,
        }
    }
}

struct Actual {
    client_signature_commitment: [u8; 48],
    server_signature_commitment: [u8; 48],
    t0: [u8; 48],
    t1: [u8; 48],
    t2: [u8; 48],
    responder_finished: [u8; 48],
    initiator_finished: [u8; 48],
    base_i2r: [u8; 32],
    base_r2i: [u8; 32],
    finished_i2r: [u8; 32],
    finished_r2i: [u8; 32],
    traffic_i2r: [u8; 32],
    traffic_r2i: [u8; 32],
    client_commit_bytes: Vec<u8>,
    server_commit_bytes: Vec<u8>,
    t0_bytes: Vec<u8>,
    t1_bytes: Vec<u8>,
    t2_bytes: Vec<u8>,
}

fn actual_schedule(input: &Inputs) -> Actual {
    assert_eq!(input.intent.len(), 520);
    assert_eq!(input.client_nonce.len(), 32);
    assert_eq!(input.responder_nonce.len(), 32);
    assert_eq!(input.client_offer.len(), 8);
    assert_eq!(input.server_selection.len(), 10);
    assert_eq!(input.initiator_share.len(), 1216);
    assert_eq!(input.responder_share.len(), 1120);

    let mut transcript = Transcript::new();
    transcript
        .append(b"endpoint-profile", b"qpr-pq-1/endpoint-v1")
        .unwrap();
    transcript.append(b"intent", &input.intent).unwrap();
    transcript
        .append(b"client-nonce", &input.client_nonce)
        .unwrap();
    transcript
        .append(b"client-offer", &input.client_offer)
        .unwrap();
    transcript
        .append(b"initiator-share", &input.initiator_share)
        .unwrap();
    let client_signature_commitment = *transcript.digest().as_bytes();
    let client_commit_bytes = transcript.as_bytes().to_vec();

    let client_proof_digest = sha384(&input.client_proof);
    transcript
        .append(b"client-proof", client_proof_digest.as_bytes())
        .unwrap();
    transcript
        .append(b"responder-nonce", &input.responder_nonce)
        .unwrap();
    transcript
        .append(b"server-selection", &input.server_selection)
        .unwrap();
    transcript
        .append(b"responder-share", &input.responder_share)
        .unwrap();
    let server_signature_commitment = *transcript.digest().as_bytes();
    let server_commit_bytes = transcript.as_bytes().to_vec();

    let server_proof_digest = sha384(&input.server_proof);
    transcript
        .append(b"server-proof", server_proof_digest.as_bytes())
        .unwrap();
    let t0_digest = transcript.digest();
    let t0 = *t0_digest.as_bytes();
    let t0_bytes = transcript.as_bytes().to_vec();

    // Fixed synthetic vector secrets, transferred through the same typed lease
    // accepted by the actual EndpointSecrets implementation. They are not live KEM/DH outputs.
    let mut shared = [0u8; 64];
    shared[..32].copy_from_slice(&input.kem_ss);
    shared[32..].copy_from_slice(&input.dh_ss);
    let shared_lease = SecretLease::new(KeyPurpose::QSessionEphemeral, &shared).unwrap();
    let base = derive_handshake_keys(&input.kem_ss, &input.dh_ss, &transcript).unwrap();
    assert_eq!(base.transcript_digest, t0_digest);
    let endpoint = EndpointSecrets::derive(&shared_lease, &transcript).unwrap();
    let base_i2r = base.initiator_to_responder;
    let base_r2i = base.responder_to_initiator;
    let finished_i2r = *endpoint.finished_key(true).unwrap();
    let finished_r2i = *endpoint.finished_key(false).unwrap();

    let responder_finished =
        finished_mac(&finished_r2i, &t0_digest, input.wrong_responder_role).unwrap();
    let responder_finished = *responder_finished.as_bytes();
    if !input.wrong_responder_role {
        verify_finished(
            &finished_r2i,
            &t0_digest,
            false,
            &crate::net::qdnf::types::StrongDigest(responder_finished),
        )
        .unwrap();
        assert!(
            verify_finished(
                &finished_r2i,
                &t0_digest,
                true,
                &crate::net::qdnf::types::StrongDigest(responder_finished)
            )
            .is_err()
        );
    }
    transcript
        .append(b"responder-finished", &responder_finished)
        .unwrap();
    let t1_digest = transcript.digest();
    let t1 = *t1_digest.as_bytes();
    let t1_bytes = transcript.as_bytes().to_vec();
    let initiator_finished = finished_mac(&finished_i2r, &t1_digest, true).unwrap();
    let initiator_finished = *initiator_finished.as_bytes();
    if !input.wrong_responder_role {
        verify_finished(
            &finished_i2r,
            &t1_digest,
            true,
            &crate::net::qdnf::types::StrongDigest(initiator_finished),
        )
        .unwrap();
    }
    transcript
        .append(b"initiator-finished", &initiator_finished)
        .unwrap();
    let t2_digest = transcript.digest();
    let t2 = *t2_digest.as_bytes();
    let t2_bytes = transcript.as_bytes().to_vec();
    let traffic = endpoint.traffic(t2_digest).unwrap();
    let traffic_bytes = traffic.expose(KeyPurpose::QSessionAead).unwrap();
    assert_eq!(traffic_bytes.len(), 64);

    Actual {
        client_signature_commitment,
        server_signature_commitment,
        t0,
        t1,
        t2,
        responder_finished,
        initiator_finished,
        base_i2r,
        base_r2i,
        finished_i2r,
        finished_r2i,
        traffic_i2r: arr32(&traffic_bytes[..32]),
        traffic_r2i: arr32(&traffic_bytes[32..]),
        client_commit_bytes,
        server_commit_bytes,
        t0_bytes,
        t1_bytes,
        t2_bytes,
    }
}

fn assert_actual_matches_vector(actual: &Actual, vector: &Value) {
    let schedule = &vector["schedule_outputs"];
    assert_eq!(
        hexstr(&actual.client_signature_commitment),
        schedule["client_signature_commitment_hex"]
            .as_str()
            .unwrap_or_else(|| vector["transcript"][0]["digest_hex"].as_str().unwrap())
    );
    assert_eq!(
        hexstr(&actual.server_signature_commitment),
        vector["transcript"][1]["digest_hex"].as_str().unwrap()
    );
    assert_eq!(hexstr(&actual.t0), schedule["T0_hex"].as_str().unwrap());
    assert_eq!(hexstr(&actual.t1), schedule["T1_hex"].as_str().unwrap());
    assert_eq!(hexstr(&actual.t2), schedule["T2_hex"].as_str().unwrap());
    assert_eq!(
        hexstr(&actual.responder_finished),
        schedule["responder_finished_r2i_hmac_sha384_hex"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        hexstr(&actual.initiator_finished),
        schedule["initiator_finished_i2r_hmac_sha384_hex"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        hexstr(&actual.base_i2r),
        schedule["base_i2r_hkdf_sha384_hex"].as_str().unwrap()
    );
    assert_eq!(
        hexstr(&actual.base_r2i),
        schedule["base_r2i_hkdf_sha384_hex"].as_str().unwrap()
    );
    assert_eq!(
        hexstr(&actual.finished_i2r),
        schedule["finished_key_i2r_hkdf_sha384_hex"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        hexstr(&actual.finished_r2i),
        schedule["finished_key_r2i_hkdf_sha384_hex"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        hexstr(&actual.traffic_i2r),
        schedule["traffic_key_i2r_hkdf_sha384_hex"]
            .as_str()
            .unwrap()
    );
    assert_eq!(
        hexstr(&actual.traffic_r2i),
        schedule["traffic_key_r2i_hkdf_sha384_hex"]
            .as_str()
            .unwrap()
    );
    for (bytes, expected) in [
        (
            &actual.client_commit_bytes,
            &vector["transcript"][0]["encoded_items_hex"],
        ),
        (
            &actual.server_commit_bytes,
            &vector["transcript"][1]["encoded_items_hex"],
        ),
        (
            &actual.t0_bytes,
            &vector["transcript"][2]["encoded_items_hex"],
        ),
        (
            &actual.t1_bytes,
            &vector["transcript"][3]["encoded_items_hex"],
        ),
        (
            &actual.t2_bytes,
            &vector["transcript"][4]["encoded_items_hex"],
        ),
    ] {
        assert_eq!(hexstr(bytes), expected.as_str().unwrap());
    }
}

#[test]
fn independent_transcript_hkdf_finished_and_traffic_vectors_match_actual_sources() {
    let c = corpus();
    let base_vector = &c["base"];
    let inputs = Inputs::from_base(base_vector);
    let actual = actual_schedule(&inputs);
    assert_actual_matches_vector(&actual, base_vector);

    let client_hello = hex(base_vector["client_hello_wire_hex"].as_str().unwrap());
    let server_hello = hex(base_vector["server_hello_wire_hex"].as_str().unwrap());
    let finished_wire = hex(base_vector["initiator_finished_wire_hex"].as_str().unwrap());
    assert_eq!(
        client_hello.len(),
        base_vector["actual_lengths"]["client_hello"]
            .as_u64()
            .unwrap() as usize
    );
    assert_eq!(
        server_hello.len(),
        base_vector["actual_lengths"]["server_hello"]
            .as_u64()
            .unwrap() as usize
    );
    assert_eq!(
        finished_wire.len(),
        base_vector["actual_lengths"]["initiator_finished"]
            .as_u64()
            .unwrap() as usize
    );
    assert_eq!(actual.t2_bytes.len(), 3389);
    assert!(actual.t2_bytes.len() <= 4096);
    let magic = b"QSESSION-ENDPOINT-V1\0";
    assert_eq!(&client_hello[..magic.len()], magic);
    assert_eq!(client_hello[magic.len()], 1);
    assert_eq!(&server_hello[..magic.len()], magic);
    assert_eq!(server_hello[magic.len()], 2);
    assert_eq!(&finished_wire[..magic.len()], magic);
    assert_eq!(finished_wire[magic.len()], 3);
    assert_eq!(
        &server_hello[magic.len() + 1..magic.len() + 1 + 48],
        sha384(&client_hello).as_bytes()
    );
    assert_eq!(
        &server_hello[magic.len() + 1 + 48..magic.len() + 1 + 48 + 32],
        inputs.client_nonce.as_slice()
    );
    assert_eq!(
        &server_hello[server_hello.len() - 48..],
        actual.responder_finished.as_slice()
    );
    let finish_body = &finished_wire[magic.len() + 1..];
    assert_eq!(&finish_body[..32], inputs.client_nonce.as_slice());
    assert_eq!(&finish_body[32..64], inputs.responder_nonce.as_slice());
    assert_eq!(&finish_body[64..112], actual.t1.as_slice());
    assert_eq!(&finish_body[112..], actual.initiator_finished.as_slice());

    assert_eq!(
        base_vector["actual_lengths"]["final_transcript"]
            .as_u64()
            .unwrap(),
        3389
    );
    assert_eq!(
        c["arithmetic"]["client_hello_max_bytes"].as_u64().unwrap(),
        6408
    );
    assert_eq!(
        c["arithmetic"]["server_hello_max_bytes"].as_u64().unwrap(),
        5922
    );
    assert_eq!(
        c["arithmetic"]["initiator_finished_max_bytes"]
            .as_u64()
            .unwrap(),
        182
    );
    assert_eq!(
        c["arithmetic"]["mtu_negotiation"]["selected"]
            .as_u64()
            .unwrap(),
        1400
    );
}

#[test]
fn endpoint_transcript_and_schedule_mutations_match_independent_effect_vectors() {
    let c = corpus();
    let base_vector = &c["base"];
    let base_inputs = Inputs::from_base(base_vector);
    let base_actual = actual_schedule(&base_inputs);
    let base_outputs = [
        (
            "clientSignatureCommitment",
            hexstr(&base_actual.client_signature_commitment),
        ),
        (
            "serverSignatureCommitment",
            hexstr(&base_actual.server_signature_commitment),
        ),
        ("T0", hexstr(&base_actual.t0)),
        ("T1", hexstr(&base_actual.t1)),
        ("T2", hexstr(&base_actual.t2)),
        ("responderFinished", hexstr(&base_actual.responder_finished)),
        ("initiatorFinished", hexstr(&base_actual.initiator_finished)),
        ("baseI2R", hexstr(&base_actual.base_i2r)),
        ("baseR2I", hexstr(&base_actual.base_r2i)),
        ("trafficI2R", hexstr(&base_actual.traffic_i2r)),
        ("trafficR2I", hexstr(&base_actual.traffic_r2i)),
    ];
    let mutations = c["mutations"].as_array().unwrap();
    assert_eq!(mutations.len(), 8);
    for vector in mutations {
        let id = vector["id"].as_str().unwrap();
        let mut input = base_inputs.clone();
        match id {
            "changed-intent" => input.intent[0] ^= 1,
            "changed-initiator-share" => input.initiator_share[700] ^= 1,
            "changed-client-proof" => input.client_proof[17] ^= 1,
            "changed-responder-share" => input.responder_share[900] ^= 1,
            "changed-server-proof" => input.server_proof[31] ^= 1,
            "changed-ml-kem-secret-input" => input.kem_ss[0] ^= 1,
            "changed-x25519-secret-input" => input.dh_ss[0] ^= 1,
            "wrong-responder-finished-role" => input.wrong_responder_role = true,
            _ => panic!("unexpected independent mutation vector {id}"),
        }
        let actual = actual_schedule(&input);
        for (name, value) in [
            (
                "clientSignatureCommitment",
                hexstr(&actual.client_signature_commitment),
            ),
            (
                "serverSignatureCommitment",
                hexstr(&actual.server_signature_commitment),
            ),
            ("T0", hexstr(&actual.t0)),
            ("T1", hexstr(&actual.t1)),
            ("T2", hexstr(&actual.t2)),
            ("responderFinished", hexstr(&actual.responder_finished)),
            ("initiatorFinished", hexstr(&actual.initiator_finished)),
            ("baseI2R", hexstr(&actual.base_i2r)),
            ("baseR2I", hexstr(&actual.base_r2i)),
            ("trafficI2R", hexstr(&actual.traffic_i2r)),
            ("trafficR2I", hexstr(&actual.traffic_r2i)),
        ] {
            assert_eq!(
                value,
                vector["outputs"][name].as_str().unwrap(),
                "{id} {name}"
            );
        }
        for (name, base) in &base_outputs {
            let changed = vector["outputs"][*name].as_str().unwrap() != base.as_str();
            assert_eq!(
                vector["effect"]["changed"][*name].as_bool().unwrap(),
                changed,
                "{id} {name} effect"
            );
        }
    }
}
