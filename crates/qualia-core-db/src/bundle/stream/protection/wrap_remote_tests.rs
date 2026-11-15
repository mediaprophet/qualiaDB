//! Remote grant wrap. Lives under protection/; does not rewrite policy/remote.
#![cfg(not(target_arch = "wasm32"))]

use super::{authorize_and_wrap_remote, RecipientAuthorityProof, WrapError};
use crate::bundle::stream::policy::remote::{
    install_remote_disclosure, issue_disclosure, AllowedOps, ControllerRecognition,
    IndependentAdmission, InstallRequest, IssueBuffers, IssueRequest, ManagedSigners,
    RecognitionAdmin, RecognitionEnrollment, RecognitionHandle, RemoteDisclosureGrant,
    VerifierChallengeSlot, DECISION_TICKET_BYTES, DISCLOSURE_DECISION_PURPOSE, RECIPIENT_BYTES,
    RECIPIENT_PURPOSE,
};
use crate::crypto::network::{
    authority::{LocalBootstrap, PinAuthority, PinConfig, PinHandle},
    cose,
    key_provider::ProviderGrant,
    object_stream::*,
    types::{KeyEpoch, KeyPurpose, ML_DSA_65_PK_LEN},
};
use crate::net::qdnf::{
    authority::{AuthorityOwner, CurrentAuthorityGuard},
    types::{OperationId, StrongDigest},
};
use crate::specialized_libs::cryptographic_library::{
    AccessPolicy, AuthenticationMethod, CryptographicLibrary, KeyAlgorithm, KeyManager,
    KeyOperation, KeyType, ManagedObjectPublication, ManagedOperationControl, PublicationRequest,
    PublicationWorkspace, RecipientUnwrapRequest, SecurityLevel, SigningAccess, SigningControl,
    TimeRestrictions,
};
use ed25519_dalek::SigningKey;
use fips204::{
    ml_dsa_65,
    traits::{KeyGen, SerDes},
};
use std::sync::Mutex;

const NOW: u64 = 2_000_000_000;
const TICKET_EXP: u64 = NOW + 30;
const PIN_EXP: u64 = NOW + 10_000;
const AUDIENCE: &[u8] = b"reader";
const CEILING: u64 = 128;

fn digest(b: u8) -> StrongDigest {
    StrongDigest([b; 48])
}
fn op() -> OperationId {
    OperationId([7; 16])
}
fn access() -> SigningAccess<'static> {
    SigningAccess {
        authenticated_methods: &[AuthenticationMethod::HardwareToken],
        ip_address: "local",
    }
}
fn signing_policy(id: &str) -> AccessPolicy {
    AccessPolicy {
        policy_id: id.into(),
        key_id: id.into(),
        allowed_operations: vec![KeyOperation::Sign],
        required_auth: vec![AuthenticationMethod::HardwareToken],
        time_restrictions: TimeRestrictions {
            allowed_hours: vec![],
            allowed_days: vec![],
            start_date: None,
            end_date: None,
        },
        ip_restrictions: vec![],
    }
}
fn wrap_policy(id: &str, operations: Vec<KeyOperation>) -> AccessPolicy {
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
fn grant_epoch() -> ProviderGrant {
    ProviderGrant {
        epoch: KeyEpoch {
            purpose: KeyPurpose::ControllerSign,
            epoch: 1,
            controller: 9,
        },
        cell_epoch: 1,
        authority_generation: 1,
        deadline_unix: PIN_EXP,
        revoked: false,
    }
}
fn publish_grant() -> ProviderGrant {
    ProviderGrant {
        epoch: KeyEpoch {
            purpose: KeyPurpose::ObjectContent,
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

struct World {
    controller_lib: CryptographicLibrary,
    controller_pins: PinAuthority,
    controller_rec: ControllerRecognition,
    controller_rec_admin: RecognitionAdmin,
    rec_handle: RecognitionHandle,
    dec_pin: PinHandle,
    key_ids: crate::crypto::network::cose::KeyIds,
    control: SigningControl,
    verifier_pins: PinAuthority,
    verifier_pin_admin: LocalBootstrap,
    verifier_rec: ControllerRecognition,
    v_rec_pin: PinHandle,
    v_dec_pin: PinHandle,
    v_rec_handle: RecognitionHandle,
    owner: Mutex<AuthorityOwner>,
}

fn pin_keys<'a>(ml: &'a [u8; ML_DSA_65_PK_LEN], ed: &'a [u8; 32]) -> cose::PairedPublicKeys<'a> {
    cose::PairedPublicKeys {
        ml_dsa_65: ml,
        ed25519: ed,
    }
}

fn install_pair(
    pins: &mut PinAuthority,
    admin: &LocalBootstrap,
    principal: StrongDigest,
    ml: &[u8; ML_DSA_65_PK_LEN],
    ed: &[u8; 32],
) -> (PinHandle, PinHandle) {
    let epoch = KeyEpoch {
        purpose: KeyPurpose::ControllerSign,
        epoch: 1,
        controller: 9,
    };
    let rec = pins
        .install_pin(
            admin,
            PinConfig {
                principal,
                provider_epoch: epoch,
                keys: pin_keys(ml, ed),
                purpose: RECIPIENT_PURPOSE,
                audience: AUDIENCE,
                not_before_unix: NOW,
                expires_unix: PIN_EXP,
            },
            NOW,
        )
        .unwrap();
    let dec = pins
        .install_pin(
            admin,
            PinConfig {
                principal,
                provider_epoch: epoch,
                keys: pin_keys(ml, ed),
                purpose: DISCLOSURE_DECISION_PURPOSE,
                audience: AUDIENCE,
                not_before_unix: NOW,
                expires_unix: PIN_EXP,
            },
            NOW,
        )
        .unwrap();
    (rec, dec)
}

fn enrollment(pin: PinHandle, principal: StrongDigest) -> RecognitionEnrollment {
    RecognitionEnrollment {
        pin,
        principal,
        source: digest(41),
        source_length: 64,
        policy: digest(42),
        publisher: digest(43),
        purpose: digest(44),
        allowed_ops: AllowedOps::BOTH,
        operation: op(),
        expires_unix: PIN_EXP,
        max_decision_lifetime: 30,
    }
}

fn world() -> World {
    let mut controller_lib = CryptographicLibrary::new();
    controller_lib.initialize().unwrap();
    let (ml, ed) = controller_lib
        .generate_managed_signing_pair("ml", "ed", SecurityLevel::High)
        .unwrap();
    controller_lib.install_managed_signing_policy(signing_policy("ml_private"));
    controller_lib.install_managed_signing_policy(signing_policy("ed_private"));
    let mut ml_pk = [0u8; ML_DSA_65_PK_LEN];
    let mut ed_pk = [0u8; 32];
    ml_pk.copy_from_slice(&ml.key_data);
    ed_pk.copy_from_slice(&ed.key_data);
    let key_ids = pin_keys(&ml_pk, &ed_pk).key_ids();
    let principal = digest(31);
    let (mut controller_pins, controller_pin_admin) = PinAuthority::new_local_bootstrap().unwrap();
    let (rec_pin, dec_pin) = install_pair(
        &mut controller_pins,
        &controller_pin_admin,
        principal,
        &ml_pk,
        &ed_pk,
    );
    let (mut controller_rec, controller_rec_admin) =
        ControllerRecognition::new_local_admin().unwrap();
    let rec_handle = controller_rec
        .install(&controller_rec_admin, enrollment(rec_pin, principal), NOW)
        .unwrap();
    let (mut verifier_pins, verifier_pin_admin) = PinAuthority::new_local_bootstrap().unwrap();
    let (v_rec_pin, v_dec_pin) = install_pair(
        &mut verifier_pins,
        &verifier_pin_admin,
        principal,
        &ml_pk,
        &ed_pk,
    );
    let (mut verifier_rec, verifier_rec_admin) = ControllerRecognition::new_local_admin().unwrap();
    let v_rec_handle = verifier_rec
        .install(&verifier_rec_admin, enrollment(v_rec_pin, principal), NOW)
        .unwrap();
    World {
        controller_lib,
        controller_pins,
        controller_rec,
        controller_rec_admin,
        rec_handle,
        dec_pin,
        key_ids,
        control: SigningControl::new(1).unwrap(),
        verifier_pins,
        verifier_pin_admin,
        verifier_rec,
        v_rec_pin,
        v_dec_pin,
        v_rec_handle,
        owner: Mutex::new(AuthorityOwner::new()),
    }
}

fn independent(context: StrongDigest) -> IndependentAdmission {
    IndependentAdmission {
        descriptor: digest(1),
        context,
        recipient: digest(3),
        publisher: digest(43),
        service: digest(5),
        purpose: digest(44),
        instrument: digest(6),
        scope: digest(7),
        operation: op().0,
        closure: digest(8),
        policy: digest(42),
        consumer: digest(9),
        source: digest(41),
        source_length: 64,
    }
}

struct Issued {
    recipient: [u8; RECIPIENT_BYTES],
    rec_env: [u8; cose::MAX_ENVELOPE_BYTES],
    rec_n: usize,
    ticket: [u8; DECISION_TICKET_BYTES],
    tkt_env: [u8; cose::MAX_ENVELOPE_BYTES],
    tkt_n: usize,
}

fn issue_for(
    w: &mut World,
    fields: RecipientFields<'_>,
    independent: IndependentAdmission,
    challenge: [u8; 32],
    transcript: StrongDigest,
) -> Issued {
    let mut out = Issued {
        recipient: [0; RECIPIENT_BYTES],
        rec_env: [0; cose::MAX_ENVELOPE_BYTES],
        rec_n: 0,
        ticket: [0; DECISION_TICKET_BYTES],
        tkt_env: [0; cose::MAX_ENVELOPE_BYTES],
        tkt_n: 0,
    };
    let signers = ManagedSigners {
        library: &w.controller_lib,
        control: &w.control,
        grant: grant_epoch(),
        access: access(),
        key_ids: w.key_ids,
        controller_epoch: 1,
        ml_key_id: "ml_private",
        ed_key_id: "ed_private",
        ml_policy_id: "ml_private",
        ed_policy_id: "ed_private",
    };
    let lengths = issue_disclosure(
        &mut w.controller_rec,
        &w.controller_pins,
        IssueRequest {
            admin: &w.controller_rec_admin,
            handle: w.rec_handle,
            decision_pin: w.dec_pin,
            independent,
            recipient: fields,
            transcript,
            request_id: OperationId([9; 16]),
            challenge,
            action: crate::bundle::stream::policy::remote::DisclosureAction::MetadataRelease,
            not_before: NOW,
            expires: TICKET_EXP,
            audience: AUDIENCE,
            signers,
        },
        NOW,
        IssueBuffers {
            recipient: &mut out.recipient,
            recipient_envelope: &mut out.rec_env,
            ticket: &mut out.ticket,
            ticket_envelope: &mut out.tkt_env,
        },
    )
    .unwrap();
    out.rec_n = lengths.recipient_envelope;
    out.tkt_n = lengths.ticket_envelope;
    out
}

fn install_req<'a>(
    w: &'a World,
    issued: &'a Issued,
    independent: IndependentAdmission,
) -> InstallRequest<'a, 'a> {
    InstallRequest {
        owner: &w.owner,
        pins: &w.verifier_pins,
        recognition: &w.verifier_rec,
        recognized: w.verifier_rec.snapshot(w.v_rec_handle).unwrap(),
        decision_pin: w.v_dec_pin,
        independent,
        recipient_bytes: &issued.recipient,
        recipient_envelope: &issued.rec_env[..issued.rec_n],
        ticket_bytes: &issued.ticket,
        ticket_envelope: &issued.tkt_env[..issued.tkt_n],
        audience: AUDIENCE,
        now: NOW,
        ceiling: CEILING,
    }
}

fn begin_publication<'p>(
    publisher: &'p mut KeyManager,
    workspace: &'p mut PublicationWorkspace,
    control: &'p ManagedOperationControl,
    clock: &'p impl Fn() -> u64,
    context_buf: &'p mut [u8; 164],
) -> (ManagedObjectPublication<'p>, Context<'p>, [u8; 1], [u8; 16]) {
    let publication_request = PublicationRequest {
        policy_id: "publish",
        policy_key_id: "publish",
        grant: publish_grant(),
        access: access(),
        generation: 0,
        predecessor: StrongDigest::ZERO,
        objects: 0,
        metadata_length: 1,
        workspace_budget: PublicationWorkspace::required_bytes(),
    };
    let mut publication = publisher
        .begin_object_publication(publication_request, workspace, control, clock)
        .unwrap();
    encode_context(publication.context_fields(), &[], context_buf).unwrap();
    let context = Context::parse(context_buf).unwrap();
    publication.bind_context(context).unwrap();
    let mut sealer = publication.begin_metadata().unwrap();
    let mut cipher = [42];
    let mut tag = [0; 16];
    sealer.seal_next(&mut cipher, &mut tag).unwrap();
    sealer.finish().unwrap();
    let context = Context::parse(context_buf).unwrap();
    (publication, context, cipher, tag)
}

fn recipient_fields<'a>(
    independent: IndependentAdmission,
    context: StrongDigest,
    ml: &'a [u8; 1184],
    dh: &'a [u8; 32],
) -> RecipientFields<'a> {
    RecipientFields {
        descriptor: independent.descriptor,
        context,
        recipient: independent.recipient,
        publisher: independent.publisher,
        service: independent.service,
        purpose: independent.purpose,
        instrument: independent.instrument,
        scope: independent.scope,
        operation: independent.operation,
        closure: independent.closure,
        policy: independent.policy,
        consumer: independent.consumer,
        source: independent.source,
        source_length: independent.source_length,
        key_epoch: 1,
        not_before: 0,
        expires: 4_000_000_000,
        ml_public: ml,
        dh_public: dh,
    }
}

fn do_remote_wrap(
    publication: &mut ManagedObjectPublication<'_>,
    grant: &RemoteDisclosureGrant<'_>,
    w: &World,
    record: RecipientRecord<'_>,
    proof: RecipientAuthorityProof<'_>,
    out: &mut [u8],
) -> Result<usize, WrapError> {
    let guard = CurrentAuthorityGuard::try_lock(&w.owner).unwrap();
    let n = authorize_and_wrap_remote(
        publication,
        grant,
        &guard,
        &w.owner,
        &w.verifier_pins,
        &w.verifier_rec,
        grant.receipt_a(),
        || NOW,
        record,
        proof,
        AUDIENCE,
        &mut cose::Workspace::new(),
        out,
    );
    drop(guard);
    n
}

#[test]
fn remote_grant_wrap_is_exact_envelope_then_private_handle() {
    let mut w = world();
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
    recipient.install_managed_signing_policy(wrap_policy(
        "ml_private",
        vec![KeyOperation::Decrypt],
    ));
    recipient.install_managed_signing_policy(wrap_policy(
        "dh_private",
        vec![KeyOperation::Derive],
    ));
    let ml: [u8; 1184] = ml_public.key_data.as_slice().try_into().unwrap();
    let dh: [u8; 32] = dh_public.key_data.as_slice().try_into().unwrap();
    let mut publisher = KeyManager::new();
    publisher.initialize().unwrap();
    publisher.install_managed_signing_policy(wrap_policy(
        "publish",
        vec![KeyOperation::Write, KeyOperation::Encrypt],
    ));
    let control = ManagedOperationControl::new(1).unwrap();
    let clock = || NOW;
    let mut workspace = PublicationWorkspace::new();
    let mut context_buf = [0u8; 164];
    let (mut publication, context, cipher, tag) = begin_publication(
        &mut publisher,
        &mut workspace,
        &control,
        &clock,
        &mut context_buf,
    );
    let facts = independent(context.id());
    let fields = recipient_fields(facts, context.id(), &ml, &dh);
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_for(&mut w, fields, facts, challenge, transcript);
    let mut cose_ws = cose::Workspace::new();
    let grant =
        install_remote_disclosure(install_req(&w, &issued, facts), &mut slot, &mut cose_ws).unwrap();
    let record = RecipientRecord::parse(&issued.recipient).unwrap();
    let proof = RecipientAuthorityProof {
        pin: w.v_rec_pin,
        key_ids: w.key_ids,
        epoch: 1,
        envelope: &issued.rec_env[..issued.rec_n],
    };
    let mut out = [0xA5; WRAP_BYTES];
    let n = do_remote_wrap(&mut publication, &grant, &w, record, proof, &mut out).unwrap();
    assert_eq!(n, WRAP_BYTES);
    assert_eq!(&out[..12], b"QBDL-PWR-V1\0");
    let wrap = Wrap::parse(&out).unwrap();
    assert_eq!(wrap.auth, record.id());
    let unwrap_now = || {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(20)
    };
    let unwrap_control = ManagedOperationControl::new(1).unwrap();
    let mut opened = recipient
        .begin_recipient_unwrap(
            RecipientUnwrapRequest {
                ml_key_id: "ml_private",
                dh_key_id: "dh_private",
                ml_policy_id: "ml_private",
                dh_policy_id: "dh_private",
                record,
                context,
                grant: ProviderGrant {
                    epoch: KeyEpoch {
                        purpose: KeyPurpose::ObjectRecipient,
                        epoch: 1,
                        controller: 7,
                    },
                    cell_epoch: 1,
                    authority_generation: 1,
                    deadline_unix: 2_000_000_100,
                    revoked: false,
                },
                access: access(),
            },
            &unwrap_control,
            &unwrap_now,
        )
        .unwrap()
        .unwrap(wrap)
        .unwrap();
    let mut plain = cipher;
    opened.decrypt_next(&mut plain, &tag).unwrap();
    assert_eq!(plain, [42]);
    let mut grant = grant;
    grant.close_now(&w.owner).unwrap();
}

#[test]
fn foreign_source_pin_is_unauthorized_and_erases() {
    let mut w = world();
    let mut publisher = KeyManager::new();
    publisher.initialize().unwrap();
    publisher.install_managed_signing_policy(wrap_policy(
        "publish",
        vec![KeyOperation::Write, KeyOperation::Encrypt],
    ));
    let control = ManagedOperationControl::new(1).unwrap();
    let clock = || NOW;
    let mut workspace = PublicationWorkspace::new();
    let mut context_buf = [0u8; 164];
    let (mut publication, context, _cipher, _tag) = begin_publication(
        &mut publisher,
        &mut workspace,
        &control,
        &clock,
        &mut context_buf,
    );
    let ml = [3u8; 1184];
    let dh = [4u8; 32];
    let facts = independent(context.id());
    let fields = recipient_fields(facts, context.id(), &ml, &dh);
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_for(&mut w, fields, facts, challenge, transcript);
    let mut cose_ws = cose::Workspace::new();
    let grant =
        install_remote_disclosure(install_req(&w, &issued, facts), &mut slot, &mut cose_ws).unwrap();
    let issuer = Issuer::new();
    let foreign_pin = w
        .verifier_pins
        .install_pin(
            &w.verifier_pin_admin,
            PinConfig {
                principal: digest(31),
                provider_epoch: KeyEpoch {
                    controller: 9,
                    epoch: 1,
                    purpose: KeyPurpose::ControllerSign,
                },
                keys: issuer.keys(),
                purpose: RECIPIENT_PURPOSE,
                audience: AUDIENCE,
                not_before_unix: NOW,
                expires_unix: PIN_EXP,
            },
            NOW,
        )
        .unwrap();
    let mut foreign = facts;
    foreign.source = digest(99);
    let fields = recipient_fields(foreign, context.id(), &ml, &dh);
    let mut record_buf = [0u8; RECIPIENT_BYTES];
    encode_recipient(fields, &mut record_buf).unwrap();
    let record = RecipientRecord::parse(&record_buf).unwrap();
    let auth = record.id();
    let mut envelope = [0u8; cose::MAX_ENVELOPE_BYTES];
    let n = cose::sign_pair_raw(
        &cose::Binding {
            descriptor_digest: &auth.0,
            key_ids: issuer.keys().key_ids(),
            controller_epoch: 1,
            purpose: RECIPIENT_PURPOSE,
            audience: AUDIENCE,
        },
        &issuer.signing(),
        &mut cose::Workspace::new(),
        &mut envelope,
    )
    .unwrap();
    let proof = RecipientAuthorityProof {
        pin: foreign_pin,
        key_ids: issuer.keys().key_ids(),
        epoch: 1,
        envelope: &envelope[..n],
    };
    let mut out = [0xA5; WRAP_BYTES];
    match do_remote_wrap(&mut publication, &grant, &w, record, proof, &mut out) {
        Err(WrapError::Unauthorized) => {}
        Err(_) => panic!("foreign source must be unauthorized"),
        Ok(_) => panic!("foreign source must refuse"),
    }
    assert!(out.iter().all(|b| *b == 0));
    let mut grant = grant;
    grant.close_now(&w.owner).unwrap();
}
