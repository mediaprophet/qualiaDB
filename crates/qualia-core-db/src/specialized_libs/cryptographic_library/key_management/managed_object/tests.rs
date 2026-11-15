use super::super::managed_control::ManagedOperationControl;
use super::super::{
    AccessPolicy, AuthenticationMethod, KeyAlgorithm, KeyManager, KeyOperation, KeyType,
    SecurityLevel, SigningAccess, TimeRestrictions,
};
use super::*;
use crate::crypto::network::{
    digest::sha384,
    key_provider::ProviderGrant,
    object_stream::*,
    types::{KeyEpoch, KeyPurpose},
};
use crate::net::qdnf::types::StrongDigest as D;
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}
fn access() -> SigningAccess<'static> {
    SigningAccess {
        authenticated_methods: &[AuthenticationMethod::HardwareToken],
        ip_address: "local",
    }
}
fn policy(id: &str, operations: Vec<KeyOperation>) -> AccessPolicy {
    AccessPolicy {
        policy_id: id.into(),
        key_id: id.into(),
        allowed_operations: operations,
        required_auth: vec![AuthenticationMethod::HardwareToken],
        time_restrictions: TimeRestrictions {
            allowed_hours: vec![],
            allowed_days: vec![],
            start_date: None,
            end_date: None,
        },
        ip_restrictions: vec!["local".into()],
    }
}
fn grant(purpose: KeyPurpose) -> ProviderGrant {
    ProviderGrant {
        epoch: KeyEpoch {
            purpose,
            epoch: 1,
            controller: 7,
        },
        cell_epoch: 1,
        authority_generation: 1,
        deadline_unix: now() + 300,
        revoked: false,
    }
}
fn request(objects: u32) -> PublicationRequest<'static> {
    PublicationRequest {
        policy_id: "publish",
        policy_key_id: "publish",
        grant: grant(KeyPurpose::ObjectContent),
        access: access(),
        generation: 0,
        predecessor: D::ZERO,
        objects,
        metadata_length: 1,
        workspace_budget: PublicationWorkspace::required_bytes(),
    }
}
fn manager() -> KeyManager {
    let mut m = KeyManager::new();
    m.initialize().unwrap();
    m.install_managed_signing_policy(policy(
        "publish",
        vec![KeyOperation::Write, KeyOperation::Encrypt],
    ));
    m
}
#[test]
fn publication_policy_budget_empty_and_drop_cleanup() {
    let mut m = KeyManager::new();
    m.initialize().unwrap();
    let control = ManagedOperationControl::new(1).unwrap();
    let mut workspace = PublicationWorkspace::new();
    let mut context = [0; 220];
    let mut geometry = [0; 36];
    assert!(
        m.begin_object_publication(request(1), &mut workspace, &control, &now)
            .is_err()
    );
    m.install_managed_signing_policy(policy("publish", vec![KeyOperation::Write]));
    assert!(
        m.begin_object_publication(request(1), &mut workspace, &control, &now)
            .is_err()
    );
    m.install_managed_signing_policy(policy(
        "publish",
        vec![KeyOperation::Write, KeyOperation::Encrypt],
    ));
    let mut short = request(1);
    short.workspace_budget -= 1;
    assert!(
        m.begin_object_publication(short, &mut workspace, &control, &now)
            .is_err()
    );
    encode_geometry(0, &[GeometryRow { id: 7, length: 0 }], &mut geometry).unwrap();
    let reference = PageRef {
        page: 0,
        rows: 1,
        length: 36,
        digest: sha384(&geometry),
    };
    let mut p = m
        .begin_object_publication(request(1), &mut workspace, &control, &now)
        .unwrap();
    encode_context(p.context_fields(), &[reference], &mut context).unwrap();
    p.bind_context(Context::parse(&context).unwrap()).unwrap();
    let mut empty = p
        .begin_object(GeometryPage::parse(&geometry).unwrap(), 0)
        .unwrap();
    assert!(empty.seal_next(&mut [], &mut [0; 16]).is_err());
    empty.finish().unwrap();
    let mut master = [1; 32];
    p.metadata_master_row(0, &mut master).unwrap();
    assert_eq!(master, [0; 32]);
    let mut metadata = p.begin_metadata().unwrap();
    metadata.seal_next(&mut [42], &mut [0; 16]).unwrap();
    metadata.finish().unwrap();
    drop(p);
    assert!(workspace.masters.iter().all(|key| *key == [0; 32]));
}
#[test]
fn seal_cancel_generation_expiry_and_unwind_are_terminal() {
    use std::cell::Cell;
    for mode in 0..4 {
        let mut m = manager();
        let control = ManagedOperationControl::new(1).unwrap();
        let mut workspace = PublicationWorkspace::new();
        let mut context = [0; 164];
        let late = Cell::new(false);
        let clock = || {
            if late.get() {
                if mode == 3 {
                    panic!("clock panic")
                }
                if mode == 2 {
                    return u64::MAX;
                }
            }
            now()
        };
        let mut p = m
            .begin_object_publication(request(0), &mut workspace, &control, &clock)
            .unwrap();
        encode_context(p.context_fields(), &[], &mut context).unwrap();
        p.bind_context(Context::parse(&context).unwrap()).unwrap();
        let mut sealer = p.begin_metadata().unwrap();
        if mode == 0 {
            control.cancel().unwrap();
        }
        if mode == 1 {
            control.advance_generation(2).unwrap();
        }
        late.set(true);
        let mut buffer = [55];
        let mut tag = [3; 16];
        if mode == 3 {
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(
                    || sealer.seal_next(&mut buffer, &mut tag)
                ))
                .is_err()
            );
        } else {
            assert!(sealer.seal_next(&mut buffer, &mut tag).is_err());
        }
        assert_eq!(buffer, [0]);
        assert_eq!(tag, [0; 16]);
        assert!(sealer.publication.is_poisoned());
        assert!(
            sealer
                .publication
                .workspace
                .masters
                .iter()
                .all(|key| *key == [0; 32])
        );
        drop(sealer);
        drop(p);
    }
}
#[test]
fn new_dh_and_kem_algorithms_are_not_signing_aliases() {
    use crate::specialized_libs::cryptographic_library::SignatureEngine;
    let mut m = manager();
    let mut engine = SignatureEngine::new();
    engine.initialize().unwrap();
    for algorithm in [
        KeyAlgorithm::X25519,
        KeyAlgorithm::Kyber,
        KeyAlgorithm::NTRU,
    ] {
        let (private, public) = m
            .generate_key_pair(
                "agreement".into(),
                KeyType::Private,
                algorithm,
                SecurityLevel::High,
            )
            .unwrap();
        assert!(engine.sign_data(&private, b"not signing").is_err());
        let sig = crate::specialized_libs::cryptographic_library::Signature {
            signature_id: "x".into(),
            key_id: public.key_id.clone(),
            algorithm,
            data: b"not signing".to_vec(),
            signature: vec![0; 64],
            timestamp: 0,
        };
        assert!(
            engine
                .verify_signature(&public, &sig, b"not signing")
                .is_err()
        );
    }
}
#[path = "recipient_tests.rs"]
mod recipient_tests;
