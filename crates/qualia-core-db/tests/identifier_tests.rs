use qualia_core_db::identifier::{parse_did_q42, IdentifierError};

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
    assert_eq!(
        parse_did_q42(b""),
        Err(IdentifierError::InvalidPrefix)
    );
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
    // Several structurally valid payloads — all must have MSB = 1.
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
    // The pointer must be exactly fnv1a(payload) | (1 << 63).
    // We verify this using the public q_hash (same algorithm, same seed).
    let payload = "z6MkpTHR8VNs";
    let base = qualia_core_db::q_hash(payload);
    let pointer = parse_did_q42(b"did:q42:z6MkpTHR8VNs").unwrap();
    assert_eq!(pointer, base | (1u64 << 63));
}
