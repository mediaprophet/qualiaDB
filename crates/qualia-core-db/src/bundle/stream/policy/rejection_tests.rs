use super::*;
#[test]
fn manifest_role_order_duplicates_and_exact_total_limit_refuse() {
    let f = Fixture::new();
    let header = super::super::types::MAGIC.len() + 48 * 5 + 8 + 4;
    let mut changed = f.manifest.clone();
    changed[header - 4] = 2;
    assert!(decode_manifest(&changed).is_err());
    changed = f.manifest.clone();
    let first = changed[header..header + 57].to_vec();
    changed[header + 57..header + 114].copy_from_slice(&first);
    assert!(decode_manifest(&changed).is_err());
    changed = f.manifest.clone();
    let end = changed.len();
    let pair = changed[end - 4..end - 2].to_vec();
    changed[end - 2..].copy_from_slice(&pair);
    assert!(decode_manifest(&changed).is_err());
    let overhead = header + 57;
    let original = vec![5; MAX_CLOSURE_ORIGINAL_BYTES - overhead];
    let nodes = [ClosureNode {
        digest: sha384(&original),
        kind: DependencyKind::Instrument,
        original_bytes: &original,
    }];
    let candidate = verify_closure(nodes[0].digest, &nodes, &[]).unwrap();
    let mut fields = f.fields;
    fields.semantic_root = nodes[0].digest;
    fields.policy = nodes[0].digest;
    fields.consumer = nodes[0].digest;
    let mut out = [0; MAX_MANIFEST_BYTES];
    let n = encode_manifest_into(fields, &candidate, &mut out).unwrap();
    assert_eq!(n + original.len(), MAX_CLOSURE_ORIGINAL_BYTES);
    decode_manifest(&out[..n])
        .unwrap()
        .matches(&candidate)
        .unwrap();
    out[n - 1] = out[n - 1].wrapping_add(1); // Original length is final u64 for the one-node graph.
    assert!(decode_manifest(&out[..n]).is_err());
}
#[test]
fn signed_unrecognized_issuer_and_wrong_audience_epoch_key_pair_refuse() {
    let f = Fixture::new();
    let (pins, _, pin, _publisher) = f.installed(CLOSURE_PURPOSE);
    let mut changed = f.manifest.clone();
    let issuer_start = super::super::types::MAGIC.len() + 48 * 4 + 8;
    changed[issuer_start] ^= 1;
    let signed = f.signed(&changed, CLOSURE_PURPOSE);
    let candidate = verify_closure(f.fields.semantic_root, &f.nodes, &f.edges).unwrap();
    assert!(matches!(
        authenticate_closure(
            candidate,
            &changed,
            &pins,
            pin,
            f.keys().key_ids(),
            1,
            b"reader",
            &signed,
            10,
            &mut cose::Workspace::new()
        ),
        Err(PolicyError::Reference)
    ));
    let signed = f.signed(&f.manifest, CLOSURE_PURPOSE);
    for i in 0..3 {
        let candidate = verify_closure(f.fields.semantic_root, &f.nodes, &f.edges).unwrap();
        let mut ids = f.keys().key_ids();
        if i == 0 {
            ids.ed25519[47] ^= 1;
        }
        let epoch = if i == 1 { 2 } else { 1 };
        let audience: &[u8] = if i == 2 { b"other" } else { b"reader" };
        assert!(
            authenticate_closure(
                candidate,
                &f.manifest,
                &pins,
                pin,
                ids,
                epoch,
                audience,
                &signed,
                10,
                &mut cose::Workspace::new()
            )
            .is_err()
        );
    }
}
#[test]
fn altered_descriptor_source_or_reference_and_partial_ceiling_refuse() {
    let f = Fixture::new();
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let mut changed = f.body.clone();
    // The source digest occurs within the embedded context; mutate its last byte.
    let pos = changed
        .windows(48)
        .position(|w| w == f.fields.source.0)
        .unwrap();
    changed[pos + 47] ^= 1;
    assert!(
        f.authenticated(&pins, pin, 10)
            .unwrap()
            .bind_descriptor(
                f.identity,
                &changed,
                &mut vec![0; HASH_SCRATCH_BYTES],
                &pins,
                f.publication(
                    publisher,
                    &f.signed_digest(f.identity.descriptor_id(), PUBLICATION_PURPOSE)
                ),
                10,
                &mut cose::Workspace::new()
            )
            .is_err()
    );
    changed = f.body.clone();
    let manifest_digest = sha384(&f.manifest);
    let pos = changed
        .windows(48)
        .position(|w| w == manifest_digest.0)
        .unwrap();
    changed[pos + 47] ^= 1;
    assert!(
        f.authenticated(&pins, pin, 10)
            .unwrap()
            .bind_descriptor(
                f.identity,
                &changed,
                &mut vec![0; HASH_SCRATCH_BYTES],
                &pins,
                f.publication(
                    publisher,
                    &f.signed_digest(f.identity.descriptor_id(), PUBLICATION_PURPOSE)
                ),
                10,
                &mut cose::Workspace::new()
            )
            .is_err()
    );
    let bound = f.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut request = f.request();
    request.resource_ceiling -= 1;
    assert!(matches!(
        adopt_local_whole_source(&mut owner, &pins, &bound, request),
        Err(PolicyError::Limit)
    ));
}
#[test]
fn close_drop_and_unwind_reclaim_bounded_parent_slots_without_eviction() {
    let f = Fixture::new();
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    for i in 0..24 {
        let mut request = f.request();
        request.operation = OperationId([i + 1; 16]);
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, request).unwrap();
        if i % 3 == 0 {
            grant.accept_local(&pins, || 20, |_| Ok(())).unwrap();
            grant.close().unwrap();
        } else if i % 3 == 1 {
            grant.cancel().unwrap();
        } else {
            let current_pins = &pins;
            let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
                grant
                    .accept_local::<()>(current_pins, || 20, |_| panic!("trusted callback failed"))
                    .unwrap();
            }));
            assert!(result.is_err());
        }
    }
}

#[test]
fn revocation_rotation_and_foreign_owner_after_staging_refuse_current_use() {
    let f = Fixture::new();
    let (mut pins, admin, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    pins.revoke_pin(&admin, pin).unwrap();
    let mut owner = AuthorityOwner::new();
    assert!(matches!(
        adopt_local_whole_source(&mut owner, &pins, &bound, f.request()),
        Err(PolicyError::Crypto(QdnfError::Revoked))
    ));
    for rotate in [false, true] {
        let (mut pins, admin, pin, publisher) = f.installed(CLOSURE_PURPOSE);
        let bound = f.bound(&pins, pin, publisher);
        let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
        if rotate {
            let (ml, _) = ml_dsa_65::KG::keygen_from_seed(&[81; 32]);
            let ml = ml.into_bytes();
            let ed = SigningKey::from_bytes(&[82; 32]).verifying_key().to_bytes();
            pins.rotate_pin(
                &admin,
                pin,
                PinConfig {
                    principal: f.fields.issuer,
                    provider_epoch: KeyEpoch {
                        controller: 7,
                        epoch: 2,
                        purpose: KeyPurpose::ControllerSign,
                    },
                    keys: cose::PairedPublicKeys {
                        ml_dsa_65: &ml,
                        ed25519: &ed,
                    },
                    purpose: CLOSURE_PURPOSE,
                    audience: b"reader",
                    not_before_unix: 10,
                    expires_unix: 100,
                },
                20,
            )
            .unwrap();
        } else {
            pins.revoke_pin(&admin, pin).unwrap();
        }
        let mut effects = 0;
        assert!(
            grant
                .accept_local(
                    &pins,
                    || 20,
                    |_| {
                        effects += 1;
                        Ok(())
                    }
                )
                .is_err()
        );
        assert_eq!(effects, 0);
    }
    let (pins, _, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let bound = f.bound(&pins, pin, publisher);
    let (other, _, _, _) = f.installed(CLOSURE_PURPOSE);
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
    assert!(matches!(
        grant.accept_local(&other, || 20, |_| Ok(())),
        Err(PolicyError::Crypto(QdnfError::Unauthorized))
    ));
}

#[test]
fn publisher_role_signature_and_current_receipt_are_required_for_entry_roots() {
    let f = Fixture::new();
    let (mut pins, admin, pin, publisher) = f.installed(CLOSURE_PURPOSE);
    let signed = f.signed_digest(f.identity.descriptor_id(), PUBLICATION_PURPOSE);
    assert!(
        f.authenticated(&pins, pin, 10)
            .unwrap()
            .bind_descriptor(
                f.identity,
                &f.body,
                &mut vec![0; HASH_SCRATCH_BYTES],
                &pins,
                f.publication(pin, &signed),
                10,
                &mut cose::Workspace::new()
            )
            .is_err()
    );
    let bad_signature = f.signed_digest(sha384(b"another-descriptor"), PUBLICATION_PURPOSE);
    assert!(
        f.authenticated(&pins, pin, 10)
            .unwrap()
            .bind_descriptor(
                f.identity,
                &f.body,
                &mut vec![0; HASH_SCRATCH_BYTES],
                &pins,
                f.publication(publisher, &bad_signature),
                10,
                &mut cose::Workspace::new()
            )
            .is_err()
    );
    // Simulate structurally admitted attacker-selected entry-page roots. Even an
    // internal candidate with the recomputed body ID cannot bypass publisher proof.
    let body = decode_body(&f.body).unwrap();
    let page = body.page_refs().get(0).unwrap();
    let mut changed = f.body.clone();
    let pos = changed
        .windows(48)
        .rposition(|w| w == page.digest.0)
        .unwrap();
    changed[pos + 47] ^= 1;
    let mut identity = f.identity;
    identity.descriptor = domain_digest(
        DigestDomain::Body,
        &changed,
        &mut vec![0; HASH_SCRATCH_BYTES],
    )
    .unwrap();
    assert!(
        f.authenticated(&pins, pin, 10)
            .unwrap()
            .bind_descriptor(
                identity,
                &changed,
                &mut vec![0; HASH_SCRATCH_BYTES],
                &pins,
                f.publication(publisher, &signed),
                10,
                &mut cose::Workspace::new()
            )
            .is_err()
    );
    let bound = f.bound(&pins, pin, publisher);
    let mut owner = AuthorityOwner::new();
    let mut grant = adopt_local_whole_source(&mut owner, &pins, &bound, f.request()).unwrap();
    pins.revoke_pin(&admin, publisher).unwrap();
    let mut effects = 0;
    assert!(matches!(
        grant.accept_local(
            &pins,
            || 20,
            |_| {
                effects += 1;
                Ok(())
            }
        ),
        Err(PolicyError::Crypto(QdnfError::Revoked))
    ));
    assert_eq!(effects, 0);
}
