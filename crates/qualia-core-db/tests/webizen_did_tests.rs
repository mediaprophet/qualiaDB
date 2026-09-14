//! Integration tests for `did:webizen:` identity, post-quantum cryptography, and hybrid attestation.

use ed25519_dalek::SigningKey;
use qualia_core_db::crypto::network::dual_sign::{sign_dual, DualProof};
use qualia_core_db::crypto::network::mldsa::generate_keypair;
use qualia_core_db::crypto::network::types::{
    ED25519_PK_LEN, ED25519_SIG_LEN, ML_DSA_65_SIG_LEN,
};
use qualia_core_db::crypto::verifiable_credential::{
    issue_dual, issue_pq, verify_dual, verify_pq, Credential, VcError,
};
use qualia_core_db::identity::agency::{
    compute_scoped_merkle_root, sign_agency_root_dual, sign_agency_root_pq,
    verify_human_agency_dual, verify_human_agency_pq, AgencyError,
};
use qualia_core_db::identity::webizen_did::{
    derive_concept_uuid, derive_concept_uuid_pq, format_concept_urn, parse_did_webizen,
    quin_concept_token, verify_dual_signature, verify_pq_signature, WebizenDidError,
    WebizenRealm,
};
use qualia_core_db::NQuin;

#[test]
fn test_did_webizen_parsing_all_realms() {
    let c = parse_did_webizen(b"did:webizen:concept:FinancialInstitution").unwrap();
    assert_eq!(c.realm, WebizenRealm::Concept);
    assert_eq!(c.payload, b"FinancialInstitution");

    let o = parse_did_webizen(b"did:webizen:ont:perception@2.0.1").unwrap();
    assert_eq!(o.realm, WebizenRealm::Ont);
    assert_eq!(o.payload, b"perception@2.0.1");

    let a = parse_did_webizen(b"did:webizen:agreement:z6MkpTHR8VNs123").unwrap();
    assert_eq!(a.realm, WebizenRealm::Agreement);
    assert_eq!(a.payload, b"z6MkpTHR8VNs123");

    let ag = parse_did_webizen(b"did:webizen:agent:tim-berners-lee").unwrap();
    assert_eq!(ag.realm, WebizenRealm::Agent);
    assert_eq!(ag.payload, b"tim-berners-lee");
}

#[test]
fn test_did_webizen_parsing_rejects_malformed_and_foreign_dids() {
    // Foreign DID methods
    assert_eq!(
        parse_did_webizen(b"did:q42:z6MkpTHR8VNs"),
        Err(WebizenDidError::RejectedMethod)
    );
    assert_eq!(
        parse_did_webizen(b"did:qi:z6MkpTHR8VNs"),
        Err(WebizenDidError::RejectedMethod)
    );
    assert_eq!(
        parse_did_webizen(b"did:qualia:0x3f8a"),
        Err(WebizenDidError::RejectedMethod)
    );

    // Invalid prefix or malformed realms
    assert_eq!(
        parse_did_webizen(b"urn:webizen:concept:foo"),
        Err(WebizenDidError::InvalidPrefix)
    );
    assert_eq!(
        parse_did_webizen(b"did:webizen:"),
        Err(WebizenDidError::UnknownRealm)
    );
    assert_eq!(
        parse_did_webizen(b"did:webizen:foo:bar"),
        Err(WebizenDidError::UnknownRealm)
    );
    assert_eq!(
        parse_did_webizen(b"did:webizen:concept:"),
        Err(WebizenDidError::EmptyPayload)
    );
}

#[test]
fn test_post_quantum_concept_uuidv8_derivation() {
    let term = b"hasDirectGrant";

    // 1. Post-quantum UUIDv8 (SHA-256)
    let u_pq1 = derive_concept_uuid_pq(term);
    let u_pq2 = derive_concept_uuid_pq(term);
    assert_eq!(u_pq1, u_pq2, "PQ UUIDv8 derivation must be deterministic");

    // Version 8 check: high 4 bits of octet 6 must be 0x8
    assert_eq!(
        u_pq1[6] >> 4,
        8,
        "UUID must be version 8 (custom/vendor post-quantum)"
    );
    // Variant check: high 2 bits of octet 8 must be 0b10 (RFC 4122 / 9562)
    assert_eq!(u_pq1[8] >> 6, 2, "UUID must be RFC variant");

    // 2. Classical UUIDv5 (SHA-1)
    let u_v5 = derive_concept_uuid(term);
    assert_eq!(u_v5[6] >> 4, 5, "UUID must be version 5");
    assert_ne!(u_pq1, u_v5, "PQ UUIDv8 and SHA-1 UUIDv5 must be distinct");

    // 3. Format URN and Quin Token
    let mut urn = [0u8; 45];
    format_concept_urn(&u_pq1, &mut urn);
    assert!(urn.starts_with(b"urn:uuid:"));

    let token = quin_concept_token(&urn);
    assert_eq!(
        token >> 63,
        0,
        "MSB must be 0 for dictionary concept entities"
    );
    assert_eq!(
        token >> 60,
        0,
        "Bits 60-63 must be 0 for inline type tag clearance"
    );
}

#[test]
fn test_post_quantum_mldsa65_signature_verification() {
    let (sk, pk) = generate_keypair().unwrap();
    let msg = b"qualia:webizen:manifest:attestation:v1";
    let ctx = b"webizen-test";

    let mut sig = [0u8; ML_DSA_65_SIG_LEN];
    qualia_core_db::crypto::network::mldsa::sign(&sk, msg, ctx, &mut sig).unwrap();

    // Verify using webizen_did verify_pq_signature
    assert!(
        verify_pq_signature(&pk, msg, &sig, ctx),
        "Valid ML-DSA-65 signature must verify"
    );

    // Tampered message fails
    assert!(
        !verify_pq_signature(&pk, b"tampered message", &sig, ctx),
        "Tampered message must fail verification"
    );

    // Tampered context fails
    assert!(
        !verify_pq_signature(&pk, msg, &sig, b"wrong-ctx"),
        "Wrong context must fail verification"
    );

    // Corrupted signature fails
    let mut bad_sig = sig;
    bad_sig[42] ^= 0xFF;
    assert!(
        !verify_pq_signature(&pk, msg, &bad_sig, ctx),
        "Corrupted signature must fail verification"
    );
}

#[test]
fn test_hybrid_dual_signature_verification() {
    let (mldsa_sk, mldsa_pk) = generate_keypair().unwrap();
    let ed_seed = [88u8; 32];
    let ed_signing = SigningKey::from_bytes(&ed_seed);
    let ed_pk_bytes: [u8; ED25519_PK_LEN] = ed_signing.verifying_key().to_bytes();

    let msg = b"webizen:dual:proof:payload";
    let ctx = b"qlink";

    let mut proof = DualProof {
        mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
        ed25519_sig: [0u8; ED25519_SIG_LEN],
    };
    sign_dual(&mldsa_sk, &ed_seed, msg, ctx, &mut proof).unwrap();

    // Valid dual signature passes
    assert!(
        verify_dual_signature(&mldsa_pk, &ed_pk_bytes, msg, ctx, &proof),
        "Valid DualProof must verify"
    );

    // Tampered Ed25519 signature fails
    let mut bad_ed_proof = proof;
    bad_ed_proof.ed25519_sig[10] ^= 0x01;
    assert!(
        !verify_dual_signature(&mldsa_pk, &ed_pk_bytes, msg, ctx, &bad_ed_proof),
        "Tampered Ed25519 half must fail dual verification"
    );

    // Tampered ML-DSA-65 signature fails
    let mut bad_ml_proof = proof;
    bad_ml_proof.mldsa_sig[50] ^= 0x01;
    assert!(
        !verify_dual_signature(&mldsa_pk, &ed_pk_bytes, msg, ctx, &bad_ml_proof),
        "Tampered ML-DSA-65 half must fail dual verification"
    );
}

#[test]
fn test_post_quantum_and_dual_agency_merkle_verification() {
    let author_did = 0x9000_1234_5678_ABCD;
    let mut frame = [NQuin {
        subject: 10,
        predicate: 20,
        object: 30,
        context: author_did,
        metadata: 0,
        parity: 0,
    }; 4];

    let (mldsa_sk, mldsa_pk) = generate_keypair().unwrap();
    let ed_seed = [77u8; 32];
    let ed_signing = SigningKey::from_bytes(&ed_seed);
    let ed_pk_bytes: [u8; ED25519_PK_LEN] = ed_signing.verifying_key().to_bytes();

    let root = compute_scoped_merkle_root(&frame, author_did);

    // 1. Post-quantum ML-DSA-65 agency test
    let mut pq_sig = [0u8; ML_DSA_65_SIG_LEN];
    sign_agency_root_pq(&mldsa_sk, &root, &mut pq_sig).unwrap();
    assert_eq!(
        verify_human_agency_pq(&frame, author_did, &mldsa_pk, &pq_sig),
        Ok(()),
        "ML-DSA-65 human agency must verify"
    );

    // 2. Hybrid DualProof agency test
    let mut dual_proof = DualProof {
        mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
        ed25519_sig: [0u8; ED25519_SIG_LEN],
    };
    sign_agency_root_dual(&mldsa_sk, &ed_seed, &root, &mut dual_proof).unwrap();
    assert_eq!(
        verify_human_agency_dual(&frame, author_did, &ed_pk_bytes, &mldsa_pk, &dual_proof),
        Ok(()),
        "DualProof human agency must verify"
    );

    // 3. Foreign claim alteration causes verification failure
    frame[0].object = 999;
    assert_eq!(
        verify_human_agency_pq(&frame, author_did, &mldsa_pk, &pq_sig),
        Err(AgencyError::InvalidSignature)
    );
    assert_eq!(
        verify_human_agency_dual(&frame, author_did, &ed_pk_bytes, &mldsa_pk, &dual_proof),
        Err(AgencyError::InvalidSignature)
    );
}

#[test]
fn test_post_quantum_and_dual_verifiable_credentials() {
    let credential = Credential {
        issuer: 100,
        subject: 200,
        issued_at: 1_000,
        valid_until: 2_000,
        claims: vec![NQuin {
            subject: 200,
            predicate: 50,
            object: 60,
            context: 100,
            metadata: 0,
            parity: 0,
        }],
    };

    let (mldsa_sk, mldsa_pk) = generate_keypair().unwrap();
    let ed_seed = [44u8; 32];
    let ed_signing = SigningKey::from_bytes(&ed_seed);
    let ed_pk_bytes: [u8; ED25519_PK_LEN] = ed_signing.verifying_key().to_bytes();

    // 1. Post-Quantum ML-DSA-65 VC
    let mut pq_sig = [0u8; ML_DSA_65_SIG_LEN];
    issue_pq(&mldsa_sk, &credential, &mut pq_sig).unwrap();
    assert_eq!(
        verify_pq(&credential, &mldsa_pk, &pq_sig, 1_500),
        Ok(()),
        "Post-quantum VC must verify"
    );

    // 2. Hybrid DualProof VC
    let mut dual_proof = DualProof {
        mldsa_sig: [0u8; ML_DSA_65_SIG_LEN],
        ed25519_sig: [0u8; ED25519_SIG_LEN],
    };
    issue_dual(&mldsa_sk, &ed_seed, &credential, &mut dual_proof).unwrap();
    assert_eq!(
        verify_dual(&credential, &ed_pk_bytes, &mldsa_pk, &dual_proof, 1_500),
        Ok(()),
        "DualProof VC must verify"
    );

    // 3. Expired VC fails
    assert_eq!(
        verify_pq(&credential, &mldsa_pk, &pq_sig, 2_500),
        Err(VcError::Expired)
    );
    assert_eq!(
        verify_dual(&credential, &ed_pk_bytes, &mldsa_pk, &dual_proof, 2_500),
        Err(VcError::Expired)
    );
}
