use super::{authorize_and_wrap, RecipientAuthorityProof, WrapError};
use crate::bundle::stream::policy::{
    adopt_local_whole_source, tests::Fixture, BoundClosure, LocalGrantRequest,
    LocalWholeSourceGrant, CLOSURE_PURPOSE,
};
use crate::crypto::network::{
    authority::{LocalBootstrap, PinAuthority, PinConfig, PinHandle},
    cose,
    key_provider::ProviderGrant,
    object_stream::*,
    types::{KeyEpoch, KeyPurpose},
};
use crate::net::qdnf::{authority::AuthorityOwner, types::StrongDigest as D};
use crate::specialized_libs::cryptographic_library::{
    AccessPolicy, AuthenticationMethod, KeyAlgorithm, KeyManager, KeyOperation, KeyType,
    ManagedObjectPublication, ManagedOperationControl, PublicationRequest, PublicationWorkspace,
    RecipientUnwrapRequest, SecurityLevel, SigningAccess, TimeRestrictions,
};
use ed25519_dalek::SigningKey;
use fips204::{
    ml_dsa_65,
    traits::{KeyGen, SerDes},
};
use std::cell::Cell;

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
fn provider(purpose: KeyPurpose) -> ProviderGrant {
    ProviderGrant {
        epoch: KeyEpoch {
            purpose,
            epoch: 1,
            controller: 7,
        },
        cell_epoch: 1,
        authority_generation: 1,
        deadline_unix: 2_000_000_100,
        revoked: false,
    }
}

struct Issuer {
    ml_pk: [u8; 1952],
    ml_sk: ml_dsa_65::PrivateKey,
    ed_pk: [u8; 32],
    ed_sk: SigningKey,
}
impl Issuer {
    fn new() -> Self {
        let (ml_pk, ml_sk) = ml_dsa_65::KG::keygen_from_seed(&[91; 32]);
        let ed_sk = SigningKey::from_bytes(&[92; 32]);
        Self {
            ml_pk: ml_pk.into_bytes(),
            ml_sk,
            ed_pk: ed_sk.verifying_key().to_bytes(),
            ed_sk,
        }
    }
    fn keys(&self) -> cose::PairedPublicKeys<'_> {
        cose::PairedPublicKeys {
            ml_dsa_65: &self.ml_pk,
            ed25519: &self.ed_pk,
        }
    }
    fn signing(&self) -> cose::SigningKeys<'_> {
        cose::SigningKeys {
            ml_dsa_65: &self.ml_sk,
            ed25519: &self.ed_sk,
        }
    }
}

fn install_recipient_pin(
    pins: &mut PinAuthority,
    admin: &LocalBootstrap,
    issuer: &Issuer,
    audience: &[u8; 48],
) -> PinHandle {
    pins.install_pin(
        admin,
        PinConfig {
            principal: D([31; 48]),
            provider_epoch: KeyEpoch {
                controller: 7,
                epoch: 1,
                purpose: KeyPurpose::ControllerSign,
            },
            keys: issuer.keys(),
            purpose: RECIPIENT_PURPOSE,
            audience,
            not_before_unix: 10,
            expires_unix: 100,
        },
        10,
    )
    .unwrap()
}

fn scope_fields<'a>(
    grant: &LocalWholeSourceGrant<'_, '_>,
    bound: &BoundClosure<'_>,
    context: D,
    ml: &'a [u8; 1184],
    dh: &'a [u8; 32],
) -> RecipientFields<'a> {
    let b = grant.binding();
    let m = bound.closure().manifest().fields();
    RecipientFields {
        descriptor: D([1; 48]),
        context,
        recipient: b.recipient,
        publisher: b.principal,
        service: D([4; 48]),
        purpose: b.purpose,
        instrument: b.instrument,
        scope: b.scope,
        operation: b.operation.0,
        closure: bound.closure().manifest_digest(),
        policy: m.policy,
        consumer: m.consumer,
        source: m.source,
        source_length: m.source_length,
        key_epoch: 1,
        not_before: 10,
        expires: 90,
        ml_public: ml,
        dh_public: dh,
    }
}

fn sign_record(record: RecipientRecord<'_>, issuer: &Issuer, out: &mut [u8]) -> usize {
    let auth = record.id();
    let binding = cose::Binding {
        descriptor_digest: &auth.0,
        key_ids: issuer.keys().key_ids(),
        controller_epoch: 1,
        purpose: RECIPIENT_PURPOSE,
        audience: &record.fields().publisher.0,
    };
    cose::sign_pair_raw(
        &binding,
        &issuer.signing(),
        &mut cose::Workspace::new(),
        out,
    )
    .unwrap()
}

fn wrap_proof<'a>(
    pin: PinHandle,
    issuer: &Issuer,
    envelope: &'a [u8],
) -> RecipientAuthorityProof<'a> {
    RecipientAuthorityProof {
        pin,
        key_ids: issuer.keys().key_ids(),
        epoch: 1,
        envelope,
    }
}

fn do_wrap(
    publication: &mut ManagedObjectPublication<'_>,
    grant: &mut LocalWholeSourceGrant<'_, '_>,
    pins: &PinAuthority,
    clock: impl Fn() -> u64,
    record: RecipientRecord<'_>,
    proof: RecipientAuthorityProof<'_>,
    out: &mut [u8],
) -> Result<usize, WrapError> {
    authorize_and_wrap(
        publication,
        grant,
        pins,
        clock,
        record,
        proof,
        &mut cose::Workspace::new(),
        out,
    )
}

fn assert_blank(out: &[u8]) {
    assert!(out.iter().all(|b| *b == 0));
}

struct Env<'p, 'd> {
    publication: ManagedObjectPublication<'p>,
    pins: PinAuthority,
    pin: PinHandle,
    issuer: Issuer,
    bound: BoundClosure<'d>,
    owner: AuthorityOwner,
    request: LocalGrantRequest,
    record_buf: [u8; RECIPIENT_BYTES],
    proof_buf: [u8; cose::MAX_ENVELOPE_BYTES],
    proof_len: usize,
    context: Context<'p>,
    recipient: KeyManager,
    cipher: [u8; 1],
    tag: [u8; 16],
    time: &'p Cell<u64>,
    ml: [u8; 1184],
    dh: [u8; 32],
}

fn with_env(work: impl FnOnce(&mut Env<'_, '_>)) {
    let fixture = Fixture::new();
    let (mut pins, admin, closure_pin, publisher_pin) = fixture.installed(CLOSURE_PURPOSE);
    let issuer = Issuer::new();
    let bound = fixture.bound(&pins, closure_pin, publisher_pin);
    let mut owner = AuthorityOwner::new();
    let request = fixture.request();
    let audience = request.principal.0;
    let pin = install_recipient_pin(&mut pins, &admin, &issuer, &audience);
    let mut probe = adopt_local_whole_source(&mut owner, &pins, &bound, fixture.request()).unwrap();
    let mut recipient = KeyManager::new();
    recipient.initialize().unwrap();
    let (ml, ml_public) = recipient
        .generate_key_pair(
            "ml".into(),
            KeyType::Private,
            KeyAlgorithm::Kyber,
            SecurityLevel::High,
        )
        .unwrap();
    let (dh, dh_public) = recipient
        .generate_key_pair(
            "dh".into(),
            KeyType::Private,
            KeyAlgorithm::X25519,
            SecurityLevel::High,
        )
        .unwrap();
    recipient.store_key(ml).unwrap();
    recipient.store_key(dh).unwrap();
    recipient.install_managed_signing_policy(policy("ml_private", vec![KeyOperation::Decrypt]));
    recipient.install_managed_signing_policy(policy("dh_private", vec![KeyOperation::Derive]));
    let ml: [u8; 1184] = ml_public.key_data.as_slice().try_into().unwrap();
    let dh: [u8; 32] = dh_public.key_data.as_slice().try_into().unwrap();
    let mut publisher = KeyManager::new();
    publisher.initialize().unwrap();
    publisher.install_managed_signing_policy(policy(
        "publish",
        vec![KeyOperation::Write, KeyOperation::Encrypt],
    ));
    let control = ManagedOperationControl::new(1).unwrap();
    let time = Cell::new(20u64);
    let clock = || time.get();
    let mut workspace = PublicationWorkspace::new();
    let publication_request = PublicationRequest {
        policy_id: "publish",
        policy_key_id: "publish",
        grant: provider(KeyPurpose::ObjectContent),
        access: access(),
        generation: 0,
        predecessor: D::ZERO,
        objects: 0,
        metadata_length: 1,
        workspace_budget: PublicationWorkspace::required_bytes(),
    };
    let mut publication = publisher
        .begin_object_publication(publication_request, &mut workspace, &control, &clock)
        .unwrap();
    let mut context_buf = [0; 164];
    encode_context(publication.context_fields(), &[], &mut context_buf).unwrap();
    let context = Context::parse(&context_buf).unwrap();
    publication.bind_context(context).unwrap();
    let mut sealer = publication.begin_metadata().unwrap();
    let mut cipher = [42];
    let mut tag = [0; 16];
    sealer.seal_next(&mut cipher, &mut tag).unwrap();
    sealer.finish().unwrap();
    let mut record_buf = [0; RECIPIENT_BYTES];
    encode_recipient(
        scope_fields(&probe, &bound, context.id(), &ml, &dh),
        &mut record_buf,
    )
    .unwrap();
    probe.close().unwrap();
    let record = RecipientRecord::parse(&record_buf).unwrap();
    let mut proof_buf = [0; cose::MAX_ENVELOPE_BYTES];
    let proof_len = sign_record(record, &issuer, &mut proof_buf);
    let mut env = Env {
        publication,
        pins,
        pin,
        issuer,
        bound,
        owner,
        request,
        record_buf,
        proof_buf,
        proof_len,
        context,
        recipient,
        cipher,
        tag,
        time: &time,
        ml,
        dh,
    };
    work(&mut env);
}

fn grant_request(e: &Env<'_, '_>) -> LocalGrantRequest {
    LocalGrantRequest {
        principal: e.request.principal,
        recipient: e.request.recipient,
        purpose: e.request.purpose,
        operation: e.request.operation,
        not_before: e.request.not_before,
        expires: e.request.expires,
        resource_ceiling: e.request.resource_ceiling,
    }
}

#[test]
fn grant_missing_spent_expired_leave_no_master_bytes() {
    with_env(|e| {
        let request = grant_request(e);
        let mut grant = adopt_local_whole_source(&mut e.owner, &e.pins, &e.bound, request).unwrap();
        grant.accept_local(&e.pins, || 20, |_| Ok(())).unwrap();
        let record = RecipientRecord::parse(&e.record_buf).unwrap();
        let proof = wrap_proof(e.pin, &e.issuer, &e.proof_buf[..e.proof_len]);
        let mut out = [0xA5; WRAP_BYTES];
        assert_eq!(
            do_wrap(
                &mut e.publication,
                &mut grant,
                &e.pins,
                || 20,
                record,
                proof,
                &mut out,
            ),
            Err(WrapError::Spent)
        );
        assert_blank(&out);
    });
    with_env(|e| {
        e.time.set(90);
        let request = grant_request(e);
        let clock = e.time;
        let mut grant = adopt_local_whole_source(&mut e.owner, &e.pins, &e.bound, request).unwrap();
        let record = RecipientRecord::parse(&e.record_buf).unwrap();
        let proof = wrap_proof(e.pin, &e.issuer, &e.proof_buf[..e.proof_len]);
        let mut out = [0xA5; WRAP_BYTES];
        assert_eq!(
            do_wrap(
                &mut e.publication,
                &mut grant,
                &e.pins,
                || clock.get(),
                record,
                proof,
                &mut out,
            ),
            Err(WrapError::Unauthorized)
        );
        assert_blank(&out);
    });
}

#[test]
fn wrong_recipient_or_context_refuses_and_erases() {
    with_env(|e| {
        let request = grant_request(e);
        let mut grant = adopt_local_whole_source(&mut e.owner, &e.pins, &e.bound, request).unwrap();
        let mut fields = scope_fields(&grant, &e.bound, e.context.id(), &e.ml, &e.dh);
        fields.recipient = D([43; 48]);
        encode_recipient(fields, &mut e.record_buf).unwrap();
        let record = RecipientRecord::parse(&e.record_buf).unwrap();
        e.proof_len = sign_record(record, &e.issuer, &mut e.proof_buf);
        let proof = wrap_proof(e.pin, &e.issuer, &e.proof_buf[..e.proof_len]);
        let mut out = [0xA5; WRAP_BYTES];
        assert_eq!(
            do_wrap(
                &mut e.publication,
                &mut grant,
                &e.pins,
                || 20,
                record,
                proof,
                &mut out,
            ),
            Err(WrapError::Unauthorized)
        );
        assert_blank(&out);
    });
    with_env(|e| {
        let request = grant_request(e);
        let mut grant = adopt_local_whole_source(&mut e.owner, &e.pins, &e.bound, request).unwrap();
        let mut fields = scope_fields(&grant, &e.bound, e.context.id(), &e.ml, &e.dh);
        fields.context = D([6; 48]);
        encode_recipient(fields, &mut e.record_buf).unwrap();
        let record = RecipientRecord::parse(&e.record_buf).unwrap();
        e.proof_len = sign_record(record, &e.issuer, &mut e.proof_buf);
        let proof = wrap_proof(e.pin, &e.issuer, &e.proof_buf[..e.proof_len]);
        let mut out = [0xA5; WRAP_BYTES];
        assert_eq!(
            do_wrap(
                &mut e.publication,
                &mut grant,
                &e.pins,
                || 20,
                record,
                proof,
                &mut out,
            ),
            Err(WrapError::Unauthorized)
        );
        assert_blank(&out);
    });
    with_env(|e| {
        let request = grant_request(e);
        let mut grant = adopt_local_whole_source(&mut e.owner, &e.pins, &e.bound, request).unwrap();
        let record = RecipientRecord::parse(&e.record_buf).unwrap();
        let auth = record.id();
        let binding = cose::Binding {
            descriptor_digest: &auth.0,
            key_ids: e.issuer.keys().key_ids(),
            controller_epoch: 1,
            purpose: b"other",
            audience: &record.fields().publisher.0,
        };
        e.proof_len = cose::sign_pair_raw(
            &binding,
            &e.issuer.signing(),
            &mut cose::Workspace::new(),
            &mut e.proof_buf,
        )
        .unwrap();
        let proof = wrap_proof(e.pin, &e.issuer, &e.proof_buf[..e.proof_len]);
        let mut out = [0xA5; WRAP_BYTES];
        assert_eq!(
            do_wrap(
                &mut e.publication,
                &mut grant,
                &e.pins,
                || 20,
                record,
                proof,
                &mut out,
            ),
            Err(WrapError::Unauthorized)
        );
        assert_blank(&out);
    });
}

#[test]
fn authorized_wrap_is_exact_envelope_then_private_handle() {
    with_env(|e| {
        let request = grant_request(e);
        let mut grant = adopt_local_whole_source(&mut e.owner, &e.pins, &e.bound, request).unwrap();
        let mut fields = scope_fields(&grant, &e.bound, e.context.id(), &e.ml, &e.dh);
        fields.not_before = 0;
        fields.expires = 4_000_000_000;
        encode_recipient(fields, &mut e.record_buf).unwrap();
        let record = RecipientRecord::parse(&e.record_buf).unwrap();
        e.proof_len = sign_record(record, &e.issuer, &mut e.proof_buf);
        let proof = wrap_proof(e.pin, &e.issuer, &e.proof_buf[..e.proof_len]);
        let mut out = [0xA5; WRAP_BYTES];
        let n = do_wrap(
            &mut e.publication,
            &mut grant,
            &e.pins,
            || 20,
            record,
            proof,
            &mut out,
        )
        .unwrap();
        assert_eq!(n, WRAP_BYTES);
        assert_eq!(&out[..12], b"QBDL-PWR-V1\0");
        let wrap = Wrap::parse(&out).unwrap();
        assert_eq!(wrap.auth, record.id());
        assert_eq!(wrap.key_ids, record.key_ids());
        let control = ManagedOperationControl::new(1).unwrap();
        let now = || {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(20)
        };
        let mut opened = e
            .recipient
            .begin_recipient_unwrap(
                RecipientUnwrapRequest {
                    ml_key_id: "ml_private",
                    dh_key_id: "dh_private",
                    ml_policy_id: "ml_private",
                    dh_policy_id: "dh_private",
                    record,
                    context: e.context,
                    grant: provider(KeyPurpose::ObjectRecipient),
                    access: access(),
                },
                &control,
                &now,
            )
            .unwrap()
            .unwrap(wrap)
            .unwrap();
        let mut plain = e.cipher;
        opened.decrypt_next(&mut plain, &e.tag).unwrap();
        assert_eq!(plain, [42]);
    });
}

#[test]
fn sixteenth_wrap_succeeds_seventeenth_is_closed() {
    with_env(|e| {
        let mut last = [0u8; WRAP_BYTES];
        for _ in 0..16 {
            let request = grant_request(e);
            let mut grant =
                adopt_local_whole_source(&mut e.owner, &e.pins, &e.bound, request).unwrap();
            let record = RecipientRecord::parse(&e.record_buf).unwrap();
            let proof = wrap_proof(e.pin, &e.issuer, &e.proof_buf[..e.proof_len]);
            let mut out = [0xA5; WRAP_BYTES];
            assert_eq!(
                do_wrap(
                    &mut e.publication,
                    &mut grant,
                    &e.pins,
                    || 20,
                    record,
                    proof,
                    &mut out,
                )
                .unwrap(),
                WRAP_BYTES
            );
            assert_eq!(&out[..12], b"QBDL-PWR-V1\0");
            last = out;
            grant.close().unwrap();
        }
        let request = grant_request(e);
        let mut grant = adopt_local_whole_source(&mut e.owner, &e.pins, &e.bound, request).unwrap();
        let record = RecipientRecord::parse(&e.record_buf).unwrap();
        let proof = wrap_proof(e.pin, &e.issuer, &e.proof_buf[..e.proof_len]);
        let mut out = [0xA5; WRAP_BYTES];
        assert_eq!(
            do_wrap(
                &mut e.publication,
                &mut grant,
                &e.pins,
                || 20,
                record,
                proof,
                &mut out,
            ),
            Err(WrapError::Closed)
        );
        assert_blank(&out);
        assert_eq!(&last[..12], b"QBDL-PWR-V1\0");
    });
}
