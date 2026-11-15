use super::*;
use crate::governance::consent_contract::ConsentScope;

struct Fixture {
    signer: SigningKey,
    grant: ConsentGrant,
    request: SensingRequest,
}

fn fixture() -> Fixture {
    let signer = SigningKey::from_bytes(&[5; 32]);
    let grant = ConsentGrant::new_signed(
        [4; 32],
        &signer,
        [8; 32],
        ConsentScope(ConsentScope::ALL),
        77,
        10,
        500,
        3,
    );
    let request = SensingRequest::new(
        [8; 32],
        19,
        SensingModality::Wifi,
        SensingAction::Acquire,
        77,
    )
    .unwrap();
    Fixture {
        signer,
        grant,
        request,
    }
}

#[test]
fn signs_and_verifies_exact_grant_bound_request() {
    let f = fixture();
    let scope = SignedSensingScope::sign(&f.grant, &f.signer, f.request).unwrap();
    scope
        .verify_scope(&f.grant, &f.grant.compute_digest(), &f.request)
        .unwrap();
    assert_eq!(scope.grant_id(), f.grant.grant_id);
    assert_eq!(scope.grant_digest(), f.grant.compute_digest());
    assert_eq!(scope.actor_id(), f.request.actor_id());
    assert_eq!(scope.source_id(), f.request.source_id());
}

#[test]
fn rejects_each_request_field_when_changed() {
    let f = fixture();
    let scope = SignedSensingScope::sign(&f.grant, &f.signer, f.request).unwrap();
    let changed = [
        SensingRequest::new(
            [9; 32],
            19,
            SensingModality::Wifi,
            SensingAction::Acquire,
            77,
        )
        .unwrap(),
        SensingRequest::new(
            [8; 32],
            20,
            SensingModality::Wifi,
            SensingAction::Acquire,
            77,
        )
        .unwrap(),
        SensingRequest::new(
            [8; 32],
            19,
            SensingModality::Radar,
            SensingAction::Acquire,
            77,
        )
        .unwrap(),
        SensingRequest::new(
            [8; 32],
            19,
            SensingModality::Wifi,
            SensingAction::Process,
            77,
        )
        .unwrap(),
        SensingRequest::new(
            [8; 32],
            19,
            SensingModality::Wifi,
            SensingAction::Acquire,
            78,
        )
        .unwrap(),
    ];
    for request in changed {
        assert_eq!(
            scope.verify_scope(&f.grant, &f.grant.compute_digest(), &request),
            Err(AuthorizationError::UnsupportedSensorScope)
        );
    }
}

#[test]
fn rejects_other_grant_other_signer_and_modified_scope_signature() {
    let f = fixture();
    let scope = SignedSensingScope::sign(&f.grant, &f.signer, f.request).unwrap();
    let other_signer = SigningKey::from_bytes(&[6; 32]);
    assert_eq!(
        SignedSensingScope::sign(&f.grant, &other_signer, f.request),
        Err(AuthorizationError::IdentityUnverified)
    );

    let other_grant = ConsentGrant::new_signed(
        [3; 32],
        &f.signer,
        [8; 32],
        ConsentScope(ConsentScope::ALL),
        77,
        10,
        500,
        4,
    );
    assert_eq!(
        scope.verify_scope(&other_grant, &other_grant.compute_digest(), &f.request),
        Err(AuthorizationError::InvalidGrant)
    );

    let mut tampered = scope;
    tampered.signature[0] ^= 1;
    assert_eq!(
        tampered.verify_scope(&f.grant, &f.grant.compute_digest(), &f.request),
        Err(AuthorizationError::IdentityUnverified)
    );
}

#[test]
fn rejects_tampered_grant_and_wrong_digest() {
    let f = fixture();
    let scope = SignedSensingScope::sign(&f.grant, &f.signer, f.request).unwrap();
    let mut tampered_grant = f.grant.clone();
    tampered_grant.purpose_hash ^= 1;
    assert_eq!(
        scope.verify_scope(
            &tampered_grant,
            &tampered_grant.compute_digest(),
            &f.request,
        ),
        Err(AuthorizationError::InvalidGrant)
    );
    assert_eq!(
        scope.verify_scope(&f.grant, &[0; 32], &f.request),
        Err(AuthorizationError::InvalidGrant)
    );
}

#[test]
fn nfc_modality_round_trips_as_its_own_signed_scope_tag() {
    let f = fixture();
    let request = SensingRequest::new(
        f.request.actor_id(),
        f.request.source_id(),
        SensingModality::Nfc,
        f.request.action(),
        f.request.purpose_hash(),
    )
    .unwrap();
    let scope = SignedSensingScope::sign(&f.grant, &f.signer, request).unwrap();
    let mut bytes = [0u8; SIGNED_SCOPE_ENCODED_LEN];
    assert_eq!(scope.encode_into(&mut bytes), Ok(SIGNED_SCOPE_ENCODED_LEN));
    assert_eq!(bytes[105], 11);
    let decoded = SignedSensingScope::decode(&bytes).unwrap();
    assert_eq!(decoded.modality(), SensingModality::Nfc);
    decoded
        .verify_scope(&f.grant, &f.grant.compute_digest(), &request)
        .unwrap();
}

#[test]
fn versioned_transport_round_trips_and_verifies_separately() {
    let f = fixture();
    let signed = SignedSensingScope::sign(&f.grant, &f.signer, f.request).unwrap();
    let mut encoded = [0u8; SIGNED_SCOPE_ENCODED_LEN + 3];
    encoded[SIGNED_SCOPE_ENCODED_LEN..].fill(0xA5);
    assert_eq!(
        signed.encode_into(&mut encoded),
        Ok(SIGNED_SCOPE_ENCODED_LEN)
    );
    assert_eq!(&encoded[SIGNED_SCOPE_ENCODED_LEN..], &[0xA5; 3]);

    let decoded = SignedSensingScope::decode(&encoded[..SIGNED_SCOPE_ENCODED_LEN]).unwrap();
    decoded
        .verify_scope(&f.grant, &f.grant.compute_digest(), &f.request)
        .unwrap();
    assert_eq!(decoded, signed);
}

#[test]
fn transport_rejects_short_trailing_unknown_version_and_tags() {
    let f = fixture();
    let signed = SignedSensingScope::sign(&f.grant, &f.signer, f.request).unwrap();
    let mut bytes = [0u8; SIGNED_SCOPE_ENCODED_LEN];
    signed.encode_into(&mut bytes).unwrap();

    assert_eq!(
        SignedSensingScope::decode(&bytes[..SIGNED_SCOPE_ENCODED_LEN - 1]),
        Err(SignedScopeEncodingError::WrongLength {
            expected: SIGNED_SCOPE_ENCODED_LEN,
            actual: SIGNED_SCOPE_ENCODED_LEN - 1,
        })
    );
    let mut trailing = [0u8; SIGNED_SCOPE_ENCODED_LEN + 1];
    trailing[..SIGNED_SCOPE_ENCODED_LEN].copy_from_slice(&bytes);
    assert!(matches!(
        SignedSensingScope::decode(&trailing),
        Err(SignedScopeEncodingError::WrongLength { .. })
    ));

    let mut bad_version = bytes;
    bad_version[0] = SIGNED_SCOPE_VERSION + 1;
    assert_eq!(
        SignedSensingScope::decode(&bad_version),
        Err(SignedScopeEncodingError::UnsupportedVersion)
    );
    let mut bad_modality = bytes;
    bad_modality[105] = 0xFF;
    assert_eq!(
        SignedSensingScope::decode(&bad_modality),
        Err(SignedScopeEncodingError::InvalidTag)
    );
    let mut bad_action = bytes;
    bad_action[106] = 0xFF;
    assert_eq!(
        SignedSensingScope::decode(&bad_action),
        Err(SignedScopeEncodingError::InvalidTag)
    );

    let mut zero_source = bytes;
    zero_source[97..105].fill(0);
    assert_eq!(
        SignedSensingScope::decode(&zero_source),
        Err(SignedScopeEncodingError::InvalidField)
    );
}

#[test]
fn short_encode_is_atomic_and_parsing_does_not_verify_signature() {
    let f = fixture();
    let signed = SignedSensingScope::sign(&f.grant, &f.signer, f.request).unwrap();
    let mut short = [0xCC; SIGNED_SCOPE_ENCODED_LEN - 1];
    assert_eq!(
        signed.encode_into(&mut short),
        Err(SignedScopeEncodingError::BufferTooSmall {
            required: SIGNED_SCOPE_ENCODED_LEN,
            available: SIGNED_SCOPE_ENCODED_LEN - 1,
        })
    );
    assert_eq!(short, [0xCC; SIGNED_SCOPE_ENCODED_LEN - 1]);

    let mut bytes = [0u8; SIGNED_SCOPE_ENCODED_LEN];
    signed.encode_into(&mut bytes).unwrap();
    bytes[147] ^= 1;
    let parsed = SignedSensingScope::decode(&bytes).unwrap();
    assert_eq!(
        parsed.verify_scope(&f.grant, &f.grant.compute_digest(), &f.request),
        Err(AuthorizationError::IdentityUnverified)
    );
}
