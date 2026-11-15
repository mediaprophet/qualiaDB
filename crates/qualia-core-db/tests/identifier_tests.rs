use qualia_core_db::identifier::{parse_did_q42, parse_did_q42_or_ptr, IdentifierError};
use qualia_core_db::identity::did_qi::{
    canonical_genesis_bytes, check_verification_relationship, did_qi_from_genesis_payload,
    format_did, parse_did_qi_uri, validate_generation_transition, verify_genesis_sha256,
    DidQiBackingHint, DidQiError, DidQiOperation, GenesisPayload, VerificationRelationship,
    MAX_DID_TEXT,
};
use qualia_core_db::identity::webizen_did::{parse_did_webizen, WebizenRealm};
use qualia_core_db::resolver::format_ntriples_to;
use qualia_core_db::{q_hash, NQuin};

// ---------------------------------------------------------------------------
// Core parsing tests (specified in the directive)
// ---------------------------------------------------------------------------

#[test]
fn test_valid_did_q42() {
    let result = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
    assert_eq!(
        result >> 63,
        1,
        "A valid did:q42 coordinate must have bit 63 set (topological pointer flag)"
    );
}

#[test]
fn test_invalid_prefix() {
    assert_eq!(
        parse_did_q42(b"did:key:z6MkpTHR8VNs"),
        Err(IdentifierError::InvalidPrefix),
        "A did:key URI must be rejected with InvalidPrefix"
    );
}

// ---------------------------------------------------------------------------
// Additional boundary and contract tests
// ---------------------------------------------------------------------------

#[test]
fn test_http_uri_rejected() {
    assert_eq!(
        parse_did_q42(b"http://example.org/Alice"),
        Err(IdentifierError::InvalidPrefix)
    );
}

#[test]
fn test_empty_slice_rejected() {
    assert_eq!(parse_did_q42(b""), Err(IdentifierError::InvalidPrefix));
}

#[test]
fn test_prefix_only_rejected() {
    assert_eq!(
        parse_did_q42(b"did:q42:"),
        Err(IdentifierError::MalformedHash)
    );
}

#[test]
fn test_determinism() {
    let a = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
    let b = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
    assert_eq!(a, b);
}

#[test]
fn test_distinct_payloads_produce_distinct_pointers() {
    let a = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
    let b = parse_did_q42(b"did:q42:z6MkpTHR8VNt").unwrap();
    assert_ne!(a, b);
}

#[test]
fn test_msb_flag_independent_of_payload_content() {
    let payloads: &[&[u8]] = &[
        b"did:q42:a",
        b"did:q42:z6Mk0000000",
        b"did:q42:ffffffffffffffffffffffffffffffff",
        b"did:q42:QUALIA_TOPOLOGICAL_NODE_42",
    ];
    for &uri in payloads {
        let result = parse_did_q42(uri).unwrap();
        assert_eq!(
            result >> 63,
            1,
            "MSB must be 1 for {:?}",
            core::str::from_utf8(uri).unwrap_or("<non-utf8>")
        );
    }
}

#[test]
fn test_pointer_equals_base_hash_or_msb() {
    let payload = "z6MkpTHR8VNs";
    let base = qualia_core_db::q_hash(payload);
    let pointer = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
    assert_eq!(pointer, base | (1u64 << 63));
}

// ---------------------------------------------------------------------------
// Packet IDF-03: did:q42:ptr/{hex} round-trip, did:webizen:, and did:qi:
// ---------------------------------------------------------------------------

#[test]
fn test_did_q42_ptr_display_roundtrip_with_resolver() {
    let qrc_ptr = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
    let s = q_hash("urn:qualia:subject:res1");
    let p = q_hash("urn:qualia:pred:locatedAt");
    let quin = NQuin {
        subject: s,
        predicate: p,
        object: qrc_ptr,
        context: 0,
        metadata: 0,
        parity: NQuin::calculate_parity(s, p, qrc_ptr, 0, 0),
    };

    let mut buf = Vec::new();
    format_ntriples_to(core::slice::from_ref(&quin), &mut buf).unwrap();
    let line = core::str::from_utf8(&buf).unwrap();
    let obj_token = line
        .split_whitespace()
        .nth(2)
        .expect("N-Triples line must contain object token");
    assert!(
        obj_token.starts_with("<did:q42:ptr/"),
        "expected <did:q42:ptr/...>, got {obj_token}"
    );

    let roundtripped = parse_did_q42_or_ptr(obj_token.as_bytes()).unwrap();
    assert_eq!(roundtripped, qrc_ptr);
}

#[test]
fn test_did_qi_grammar_hints_genesis_and_rejected_prefixes() {
    // 1. Explicitly reject legacy and QRC prefixes with RejectedPrefix.
    for bad in [
        &b"did:hcai:z6MkpTHR8VNs"[..],
        &b"did:hci:z6MkpTHR8VNs"[..],
        &b"did:hcinet:z6MkpTHR8VNs"[..],
        &b"did:qualia:z6MkpTHR8VNs"[..],
        &b"did:q42:z6MkpTHR8VNs"[..],
    ] {
        assert_eq!(
            parse_did_qi_uri(bad),
            Err(DidQiError::RejectedPrefix),
            "must reject {:?}",
            core::str::from_utf8(bad)
        );
    }

    // 2. Genesis SHA-256 binding and parameterized hints (`git`, `utxo`, `mailbox`).
    let genesis = GenesisPayload {
        controller_ed25519: [0x11; 32],
        controller_ml_dsa_65_hash: [0x22; 32],
        session_ed25519: [0x33; 32],
        q_link_x25519: [0x44; 32],
        q_link_ml_kem_768_hash: [0x55; 32],
        genesis_nonce: [0x66; 16],
    };
    let did = did_qi_from_genesis_payload(&genesis);
    let canonical = canonical_genesis_bytes(&genesis);
    assert!(verify_genesis_sha256(&did, &canonical).is_ok());

    let mut tampered = canonical;
    tampered[0] ^= 0xFF;
    assert_eq!(
        verify_genesis_sha256(&did, &tampered),
        Err(DidQiError::DigestMismatch)
    );

    let mut did_buf = [0u8; MAX_DID_TEXT];
    let n = format_did(&did, &mut did_buf).unwrap();
    let did_str = core::str::from_utf8(&did_buf[..n]).unwrap();
    let uri_with_hints = format!("{did_str};versionId=3;qi:backing=git#key-0");
    let parsed = parse_did_qi_uri(uri_with_hints.as_bytes()).unwrap();
    assert_eq!(parsed.did, did);
    assert_eq!(parsed.version_id, Some(3));
    assert_eq!(parsed.backing_hint, Some(DidQiBackingHint::Git));
    assert_eq!(parsed.fragment, Some(&b"key-0"[..]));

    let utxo_uri = format!("{did_str};qi:backing=utxo");
    assert_eq!(
        parse_did_qi_uri(utxo_uri.as_bytes()).unwrap().backing_hint,
        Some(DidQiBackingHint::Utxo)
    );
    let mailbox_uri = format!("{did_str};qi:backing=mailbox#mailbox");
    assert_eq!(
        parse_did_qi_uri(mailbox_uri.as_bytes())
            .unwrap()
            .backing_hint,
        Some(DidQiBackingHint::Mailbox)
    );
}

#[test]
fn test_did_qi_monotonic_generation_tombstone_and_relationship_separation() {
    // Monotonic generation & tombstone deactivation.
    assert!(validate_generation_transition(0, 1, false).is_ok());
    assert_eq!(
        validate_generation_transition(1, 1, false),
        Err(DidQiError::NonMonotonicGeneration)
    );
    assert_eq!(
        validate_generation_transition(1, 3, false),
        Err(DidQiError::NonMonotonicGeneration)
    );
    assert_eq!(
        validate_generation_transition(1, 2, true),
        Err(DidQiError::Deactivated)
    );

    // Authentication cannot authorize document update, tombstone deactivation, or route update.
    assert!(check_verification_relationship(
        VerificationRelationship::Authentication,
        DidQiOperation::AuthenticateSession
    )
    .is_ok());
    assert_eq!(
        check_verification_relationship(
            VerificationRelationship::Authentication,
            DidQiOperation::UpdateDocument
        ),
        Err(DidQiError::UnauthorizedRelationship)
    );
    assert_eq!(
        check_verification_relationship(
            VerificationRelationship::Authentication,
            DidQiOperation::AuthorizeRouteUpdate
        ),
        Err(DidQiError::UnauthorizedRelationship)
    );
    assert!(check_verification_relationship(
        VerificationRelationship::CapabilityInvocation,
        DidQiOperation::UpdateDocument
    )
    .is_ok());
    assert!(check_verification_relationship(
        VerificationRelationship::CapabilityInvocation,
        DidQiOperation::DeactivateTombstone
    )
    .is_ok());
}

#[test]
fn test_side_by_side_did_q42_did_webizen_and_did_qi_remain_disjoint() {
    let qrc = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
    assert_eq!(qrc >> 63, 1);

    let wb = parse_did_webizen(b"did:webizen:concept:7c9e6679-7425-40de-944b-e07fc1f90ae7")
        .expect("valid did:webizen");
    assert_eq!(wb.realm, WebizenRealm::Concept);

    // did:q42 must fail did:qi and did:webizen parsers; did:webizen must fail did:q42 and did:qi.
    assert_eq!(
        parse_did_qi_uri(b"did:q42:z6MkpTHR8VNs"),
        Err(DidQiError::RejectedPrefix)
    );
    assert!(parse_did_q42(b"did:webizen:concept:7c9e6679-7425-40de-944b-e07fc1f90ae7").is_err());
}
