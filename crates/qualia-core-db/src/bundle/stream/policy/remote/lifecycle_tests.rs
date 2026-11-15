//! Stage R2 issuance / install / recheck lifecycle. Two independently owned parties.
#![cfg(not(target_arch = "wasm32"))]

use super::grant::{install_remote_disclosure, InstallRequest, VerifierChallengeSlot};
use super::issuer::{
    fresh_challenge, issue_disclosure, sign_with_managed, IssueBuffers, IssueRequest,
    ManagedSigners,
};
use super::*;
use crate::crypto::network::authority::{
    AuthoritySource, LocalBootstrap, PinAuthority, PinConfig, PinHandle,
};
use crate::crypto::network::cose::{self, Binding, KeyIds, PairedPublicKeys};
use crate::crypto::network::digest::sha384;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::key_provider::ProviderGrant;
use crate::crypto::network::object_stream::{encode_recipient, RecipientFields};
use crate::crypto::network::types::{KeyEpoch, KeyPurpose, ML_DSA_65_PK_LEN};
use crate::net::qdnf::authority::{
    AuthorityBinding, AuthorityOwner, ContactState, CurrentAuthorityGuard, PolicyOutcome,
};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{OperationId, StrongDigest};
use crate::specialized_libs::cryptographic_library::{
    AccessPolicy, AuthenticationMethod, CryptographicLibrary, KeyOperation, SecurityLevel,
    SigningAccess, SigningControl, TimeRestrictions,
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
fn policy(id: &str) -> AccessPolicy {
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
fn grant_epoch(epoch: u64) -> ProviderGrant {
    ProviderGrant {
        epoch: KeyEpoch {
            purpose: KeyPurpose::ControllerSign,
            epoch,
            controller: 9,
        },
        cell_epoch: 1,
        authority_generation: 1,
        deadline_unix: PIN_EXP,
        revoked: false,
    }
}

#[allow(dead_code)]
struct World {
    controller_lib: CryptographicLibrary,
    controller_pins: PinAuthority,
    controller_pin_admin: LocalBootstrap,
    controller_rec: ControllerRecognition,
    controller_rec_admin: RecognitionAdmin,
    rec_pin: PinHandle,
    dec_pin: PinHandle,
    rec_handle: RecognitionHandle,
    key_ids: KeyIds,
    ml_pk: [u8; ML_DSA_65_PK_LEN],
    ed_pk: [u8; 32],
    control: SigningControl,
    principal: StrongDigest,
    verifier_lib: CryptographicLibrary,
    verifier_pins: PinAuthority,
    verifier_pin_admin: LocalBootstrap,
    verifier_rec: ControllerRecognition,
    verifier_rec_admin: RecognitionAdmin,
    v_rec_pin: PinHandle,
    v_dec_pin: PinHandle,
    v_rec_handle: RecognitionHandle,
    owner: Mutex<AuthorityOwner>,
    controller_owner: Mutex<AuthorityOwner>,
}

fn pin_keys<'a>(ml: &'a [u8; ML_DSA_65_PK_LEN], ed: &'a [u8; 32]) -> PairedPublicKeys<'a> {
    PairedPublicKeys {
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
    controller_lib.install_managed_signing_policy(policy("ml_private"));
    controller_lib.install_managed_signing_policy(policy("ed_private"));
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

    let mut verifier_lib = CryptographicLibrary::new();
    verifier_lib.initialize().unwrap();
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
        controller_pin_admin,
        controller_rec,
        controller_rec_admin,
        rec_pin,
        dec_pin,
        rec_handle,
        key_ids,
        ml_pk,
        ed_pk,
        control: SigningControl::new(1).unwrap(),
        principal,
        verifier_lib,
        verifier_pins,
        verifier_pin_admin,
        verifier_rec,
        verifier_rec_admin,
        v_rec_pin,
        v_dec_pin,
        v_rec_handle,
        owner: Mutex::new(AuthorityOwner::new()),
        controller_owner: Mutex::new(AuthorityOwner::new()),
    }
}

fn independent() -> IndependentAdmission {
    IndependentAdmission {
        descriptor: digest(1),
        context: digest(2),
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

fn recipient_fields<'a>(ml: &'a [u8; 1184], dh: &'a [u8; 32]) -> RecipientFields<'a> {
    let i = independent();
    RecipientFields {
        descriptor: i.descriptor,
        context: i.context,
        recipient: i.recipient,
        publisher: i.publisher,
        service: i.service,
        purpose: i.purpose,
        instrument: i.instrument,
        scope: i.scope,
        operation: i.operation,
        closure: i.closure,
        policy: i.policy,
        consumer: i.consumer,
        source: i.source,
        source_length: i.source_length,
        key_epoch: 1,
        not_before: NOW,
        expires: NOW + 100,
        ml_public: ml,
        dh_public: dh,
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

fn signers<'a>(w: &'a World) -> ManagedSigners<'a> {
    ManagedSigners {
        library: &w.controller_lib,
        control: &w.control,
        grant: grant_epoch(1),
        access: access(),
        key_ids: w.key_ids,
        controller_epoch: 1,
        ml_key_id: "ml_private",
        ed_key_id: "ed_private",
        ml_policy_id: "ml_private",
        ed_policy_id: "ed_private",
    }
}

fn issue_ok(w: &mut World, challenge: [u8; 32], transcript: StrongDigest) -> Issued {
    let mut out = Issued {
        recipient: [0; RECIPIENT_BYTES],
        rec_env: [0; cose::MAX_ENVELOPE_BYTES],
        rec_n: 0,
        ticket: [0; DECISION_TICKET_BYTES],
        tkt_env: [0; cose::MAX_ENVELOPE_BYTES],
        tkt_n: 0,
    };
    let ml = [3u8; 1184];
    let dh = [4u8; 32];
    let fields = recipient_fields(&ml, &dh);
    let signers = ManagedSigners {
        library: &w.controller_lib,
        control: &w.control,
        grant: grant_epoch(1),
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
            independent: independent(),
            recipient: fields,
            transcript,
            request_id: OperationId([9; 16]),
            challenge,
            action: DisclosureAction::MetadataRelease,
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

fn install_req<'a>(w: &'a World, issued: &'a Issued) -> InstallRequest<'a, 'a> {
    InstallRequest {
        owner: &w.owner,
        pins: &w.verifier_pins,
        recognition: &w.verifier_rec,
        recognized: w.verifier_rec.snapshot(w.v_rec_handle).unwrap(),
        decision_pin: w.v_dec_pin,
        independent: independent(),
        recipient_bytes: &issued.recipient,
        recipient_envelope: &issued.rec_env[..issued.rec_n],
        ticket_bytes: &issued.ticket,
        ticket_envelope: &issued.tkt_env[..issued.tkt_n],
        audience: AUDIENCE,
        now: NOW,
        ceiling: CEILING,
    }
}

fn dummy_binding(n: u8) -> AuthorityBinding {
    AuthorityBinding {
        principal: digest(n),
        recipient: digest(n.wrapping_add(80)),
        instrument: digest(4),
        scope: digest(5),
        purpose: digest(6),
        operation: OperationId([n; 16]),
    }
}

#[test]
fn issue_and_verify_installs_grant_on_independent_verifier() {
    let mut w = world();
    assert!(!core::ptr::eq(&w.owner, &w.controller_owner));
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let mut workspace = cose::Workspace::new();
    let grant =
        install_remote_disclosure(install_req(&w, &issued), &mut slot, &mut workspace).unwrap();
    let desc = [7u8; 48];
    assert!(matches!(
        w.verifier_pins.verify_source(
            AuthoritySource::SignedControllerChain(&[]),
            &issued.rec_env[..issued.rec_n],
            &Binding {
                descriptor_digest: &desc,
                key_ids: w.key_ids,
                controller_epoch: 1,
                purpose: RECIPIENT_PURPOSE,
                audience: AUDIENCE,
            },
            NOW,
            &mut workspace,
        ),
        Err(CryptoError::Unsupported)
    ));
    let guard = CurrentAuthorityGuard::try_lock(&w.owner).unwrap();
    grant
        .recheck(
            &guard,
            &w.owner,
            &w.verifier_pins,
            &w.verifier_rec,
            grant.receipt_a(),
            NOW + 1,
            DisclosureAction::MetadataRelease,
        )
        .unwrap();
    drop(guard);
    let mut grant = grant;
    grant.close_now(&w.owner).unwrap();
}

#[test]
fn arbitrary_self_signer_denied() {
    let mut w = world();
    let mut other = CryptographicLibrary::new();
    other.initialize().unwrap();
    let (ml, ed) = other
        .generate_managed_signing_pair("ml", "ed", SecurityLevel::High)
        .unwrap();
    other.install_managed_signing_policy(policy("ml_private"));
    other.install_managed_signing_policy(policy("ed_private"));
    let mut ml_pk = [0u8; ML_DSA_65_PK_LEN];
    let mut ed_pk = [0u8; 32];
    ml_pk.copy_from_slice(&ml.key_data);
    ed_pk.copy_from_slice(&ed.key_data);
    let key_ids = PairedPublicKeys {
        ml_dsa_65: &ml_pk,
        ed25519: &ed_pk,
    }
    .key_ids();
    let control = SigningControl::new(1).unwrap();
    let mut issued = issue_ok(&mut w, fresh_challenge().unwrap(), digest(50));
    let rec_id = sha384(&issued.recipient);
    let signers = ManagedSigners {
        library: &other,
        control: &control,
        grant: grant_epoch(1),
        access: access(),
        key_ids,
        controller_epoch: 1,
        ml_key_id: "ml_private",
        ed_key_id: "ed_private",
        ml_policy_id: "ml_private",
        ed_policy_id: "ed_private",
    };
    let mut workspace = cose::Workspace::new();
    issued.rec_n = sign_with_managed(
        &signers,
        Binding {
            descriptor_digest: &rec_id.0,
            key_ids,
            controller_epoch: 1,
            purpose: RECIPIENT_PURPOSE,
            audience: AUDIENCE,
        },
        NOW,
        &mut issued.rec_env,
        &mut workspace,
    )
    .unwrap();
    let mut slot = VerifierChallengeSlot::new();
    let _ = slot
        .issue(&w.owner, digest(50), OperationId([9; 16]))
        .unwrap();
    assert!(
        install_remote_disclosure(install_req(&w, &issued), &mut slot, &mut workspace,).is_err()
    );
}

#[test]
fn right_key_wrong_role_denied() {
    let mut w = world();
    let mut issued = issue_ok(&mut w, fresh_challenge().unwrap(), digest(50));
    let rec_id = sha384(&issued.recipient);
    let mut workspace = cose::Workspace::new();
    issued.rec_n = sign_with_managed(
        &signers(&w),
        Binding {
            descriptor_digest: &rec_id.0,
            key_ids: w.key_ids,
            controller_epoch: 1,
            purpose: DISCLOSURE_DECISION_PURPOSE,
            audience: AUDIENCE,
        },
        NOW,
        &mut issued.rec_env,
        &mut workspace,
    )
    .unwrap();
    let mut slot = VerifierChallengeSlot::new();
    let _ = slot
        .issue(&w.owner, digest(50), OperationId([9; 16]))
        .unwrap();
    assert!(matches!(
        install_remote_disclosure(install_req(&w, &issued), &mut slot, &mut workspace),
        Err(PolicyError::Crypto(CryptoError::Unauthorized))
    ));
}

#[test]
fn a_field_substitution_denied() {
    let mut w = world();
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let ml = [3u8; 1184];
    let dh = [4u8; 32];
    let mut workspace = cose::Workspace::new();
    for subst in [
        |f: &mut RecipientFields| f.descriptor = digest(99),
        |f: &mut RecipientFields| f.context = digest(99),
        |f: &mut RecipientFields| f.source = digest(99),
        |f: &mut RecipientFields| f.source_length = 128,
        |f: &mut RecipientFields| f.operation = [9; 16],
        |f: &mut RecipientFields| f.recipient = digest(99),
    ] {
        let mut fields = recipient_fields(&ml, &dh);
        subst(&mut fields);
        let mut buf = [0u8; RECIPIENT_BYTES];
        encode_recipient(fields, &mut buf).unwrap();
        let mut tampered = issued.recipient;
        tampered.copy_from_slice(&buf);
        let mut slot = VerifierChallengeSlot::new();
        let _ = slot
            .issue(&w.owner, transcript, OperationId([9; 16]))
            .unwrap();
        let mut req = install_req(&w, &issued);
        req.recipient_bytes = &tampered;
        assert!(install_remote_disclosure(req, &mut slot, &mut workspace).is_err());
    }
}

#[test]
fn nonce_replay_denied() {
    let mut w = world();
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let mut workspace = cose::Workspace::new();
    let grant =
        install_remote_disclosure(install_req(&w, &issued), &mut slot, &mut workspace).unwrap();
    assert!(matches!(
        install_remote_disclosure(install_req(&w, &issued), &mut slot, &mut workspace),
        Err(PolicyError::Authority(QdnfError::Replay))
    ));
    drop(grant);
}

#[test]
fn cross_session_ticket_denied() {
    let mut w = world();
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, digest(51), OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, digest(50));
    assert!(install_remote_disclosure(
        install_req(&w, &issued),
        &mut slot,
        &mut cose::Workspace::new(),
    )
    .is_err());
}

#[test]
fn expired_ticket_denied() {
    let mut w = world();
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let mut req = install_req(&w, &issued);
    req.now = TICKET_EXP;
    assert!(matches!(
        install_remote_disclosure(req, &mut slot, &mut cose::Workspace::new()),
        Err(PolicyError::Crypto(CryptoError::Expired))
    ));
}

#[test]
fn enrolled_controller_cannot_issue_for_unenrolled_source() {
    let mut w = world();
    let mut independent = independent();
    independent.source = digest(99);
    let ml = [3u8; 1184];
    let dh = [4u8; 32];
    let mut fields = recipient_fields(&ml, &dh);
    fields.source = digest(99);
    let mut recipient = [0u8; RECIPIENT_BYTES];
    let mut rec_env = [0u8; cose::MAX_ENVELOPE_BYTES];
    let mut ticket = [0u8; DECISION_TICKET_BYTES];
    let mut tkt_env = [0u8; cose::MAX_ENVELOPE_BYTES];
    let signers = ManagedSigners {
        library: &w.controller_lib,
        control: &w.control,
        grant: grant_epoch(1),
        access: access(),
        key_ids: w.key_ids,
        controller_epoch: 1,
        ml_key_id: "ml_private",
        ed_key_id: "ed_private",
        ml_policy_id: "ml_private",
        ed_policy_id: "ed_private",
    };
    assert!(matches!(
        issue_disclosure(
            &mut w.controller_rec,
            &w.controller_pins,
            IssueRequest {
                admin: &w.controller_rec_admin,
                handle: w.rec_handle,
                decision_pin: w.dec_pin,
                independent,
                recipient: fields,
                transcript: digest(50),
                request_id: OperationId([9; 16]),
                challenge: fresh_challenge().unwrap(),
                action: DisclosureAction::MetadataRelease,
                not_before: NOW,
                expires: TICKET_EXP,
                audience: AUDIENCE,
                signers,
            },
            NOW,
            IssueBuffers {
                recipient: &mut recipient,
                recipient_envelope: &mut rec_env,
                ticket: &mut ticket,
                ticket_envelope: &mut tkt_env,
            },
        ),
        Err(PolicyError::Reference)
    ));
}

#[test]
fn install_independent_source_not_enrolled_denied() {
    let mut w = world();
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let mut independent = independent();
    independent.source = digest(99);
    let ml = [3u8; 1184];
    let dh = [4u8; 32];
    let mut fields = recipient_fields(&ml, &dh);
    fields.source = digest(99);
    let mut tampered = [0u8; RECIPIENT_BYTES];
    encode_recipient(fields, &mut tampered).unwrap();
    let mut req = install_req(&w, &issued);
    req.independent = independent;
    req.recipient_bytes = &tampered;
    assert!(matches!(
        install_remote_disclosure(req, &mut slot, &mut cose::Workspace::new()),
        Err(PolicyError::Reference)
    ));
}

#[test]
fn recognition_rotate_before_recheck_denied() {
    let mut w = world();
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let (ml, ed) = w
        .verifier_lib
        .generate_managed_signing_pair("ml-rot", "ed-rot", SecurityLevel::High)
        .unwrap();
    let mut ml_pk = [0u8; ML_DSA_65_PK_LEN];
    let mut ed_pk = [0u8; 32];
    ml_pk.copy_from_slice(&ml.key_data);
    ed_pk.copy_from_slice(&ed.key_data);
    let grant = install_remote_disclosure(
        InstallRequest {
            owner: &w.owner,
            pins: &w.verifier_pins,
            recognition: &w.verifier_rec,
            recognized: w.verifier_rec.snapshot(w.v_rec_handle).unwrap(),
            decision_pin: w.v_dec_pin,
            independent: independent(),
            recipient_bytes: &issued.recipient,
            recipient_envelope: &issued.rec_env[..issued.rec_n],
            ticket_bytes: &issued.ticket,
            ticket_envelope: &issued.tkt_env[..issued.tkt_n],
            audience: AUDIENCE,
            now: NOW,
            ceiling: CEILING,
        },
        &mut slot,
        &mut cose::Workspace::new(),
    )
    .unwrap();
    let extra = w
        .verifier_pins
        .install_pin(
            &w.verifier_pin_admin,
            PinConfig {
                principal: w.principal,
                provider_epoch: KeyEpoch {
                    purpose: KeyPurpose::ControllerSign,
                    epoch: 2,
                    controller: 9,
                },
                keys: PairedPublicKeys {
                    ml_dsa_65: &ml_pk,
                    ed25519: &ed_pk,
                },
                purpose: RECIPIENT_PURPOSE,
                audience: b"reader-rotated",
                not_before_unix: NOW,
                expires_unix: PIN_EXP,
            },
            NOW,
        )
        .unwrap();
    w.verifier_rec
        .rotate(
            &w.verifier_rec_admin,
            w.v_rec_handle,
            enrollment(extra, w.principal),
            NOW,
        )
        .unwrap();
    let guard = CurrentAuthorityGuard::try_lock(&w.owner).unwrap();
    assert!(matches!(
        grant.recheck(
            &guard,
            &w.owner,
            &w.verifier_pins,
            &w.verifier_rec,
            grant.receipt_a(),
            NOW + 1,
            DisclosureAction::MetadataRelease,
        ),
        Err(PolicyError::Authority(QdnfError::StaleGeneration))
    ));
}

#[test]
fn rotate_pin_before_install_denied() {
    let mut w = world();
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let (ml, ed) = w
        .verifier_lib
        .generate_managed_signing_pair("ml2", "ed2", SecurityLevel::High)
        .unwrap();
    let mut ml_pk = [0u8; ML_DSA_65_PK_LEN];
    let mut ed_pk = [0u8; 32];
    ml_pk.copy_from_slice(&ml.key_data);
    ed_pk.copy_from_slice(&ed.key_data);
    w.v_rec_pin = w
        .verifier_pins
        .rotate_pin(
            &w.verifier_pin_admin,
            w.v_rec_pin,
            PinConfig {
                principal: w.principal,
                provider_epoch: KeyEpoch {
                    purpose: KeyPurpose::ControllerSign,
                    epoch: 2,
                    controller: 9,
                },
                keys: PairedPublicKeys {
                    ml_dsa_65: &ml_pk,
                    ed25519: &ed_pk,
                },
                purpose: RECIPIENT_PURPOSE,
                audience: AUDIENCE,
                not_before_unix: NOW,
                expires_unix: PIN_EXP,
            },
            NOW,
        )
        .unwrap();
    assert!(install_remote_disclosure(
        install_req(&w, &issued),
        &mut slot,
        &mut cose::Workspace::new(),
    )
    .is_err());
}

#[test]
fn revoke_before_recheck_denied() {
    let mut w = world();
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let mut grant = install_remote_disclosure(
        install_req(&w, &issued),
        &mut slot,
        &mut cose::Workspace::new(),
    )
    .unwrap();
    let mut guard = CurrentAuthorityGuard::try_lock(&w.owner).unwrap();
    grant.revoke_current(&mut guard, &w.owner).unwrap();
    assert_eq!(
        grant
            .recheck(
                &guard,
                &w.owner,
                &w.verifier_pins,
                &w.verifier_rec,
                grant.receipt_a(),
                NOW + 1,
                DisclosureAction::MetadataRelease,
            )
            .unwrap_err(),
        PolicyError::Authority(QdnfError::Revoked)
    );
    drop(guard);
    grant.retire_current(&w.owner).unwrap();
}

#[test]
fn shared_mutex_guard_no_deadlock() {
    let mut w = world();
    let session = {
        let mut owner = w.owner.lock().unwrap();
        owner
            .install_grant_with_ceiling(
                dummy_binding(11),
                NOW,
                TICKET_EXP,
                ContactState::Active,
                CEILING,
            )
            .unwrap()
            .2
    };
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let grant = install_remote_disclosure(
        install_req(&w, &issued),
        &mut slot,
        &mut cose::Workspace::new(),
    )
    .unwrap();
    let guard = CurrentAuthorityGuard::try_lock(&w.owner).unwrap();
    guard
        .check_handle(&w.owner, session, dummy_binding(11), NOW + 1, CEILING)
        .unwrap();
    grant
        .recheck(
            &guard,
            &w.owner,
            &w.verifier_pins,
            &w.verifier_rec,
            grant.receipt_a(),
            NOW + 1,
            DisclosureAction::MetadataRelease,
        )
        .unwrap();
    assert!(matches!(
        CurrentAuthorityGuard::try_lock(&w.owner),
        Err(QdnfError::WouldBlock)
    ));
    drop(guard);
    drop(grant);
}

#[test]
fn eight_slot_exhaustion_and_revoke_retire_reuse() {
    let mut w = world();
    {
        let mut owner = w.owner.lock().unwrap();
        for i in 1..=7 {
            owner
                .install_grant_with_ceiling(
                    dummy_binding(i),
                    NOW,
                    TICKET_EXP,
                    ContactState::Active,
                    CEILING,
                )
                .unwrap();
        }
    }
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let mut grant = install_remote_disclosure(
        install_req(&w, &issued),
        &mut slot,
        &mut cose::Workspace::new(),
    )
    .unwrap();
    {
        let mut owner = w.owner.lock().unwrap();
        assert_eq!(
            owner
                .install_grant_with_ceiling(
                    dummy_binding(20),
                    NOW,
                    TICKET_EXP,
                    ContactState::Active,
                    CEILING,
                )
                .unwrap_err(),
            QdnfError::Capacity
        );
    }
    grant.close_now(&w.owner).unwrap();
    w.owner
        .lock()
        .unwrap()
        .install_grant_with_ceiling(
            dummy_binding(20),
            NOW,
            TICKET_EXP,
            ContactState::Active,
            CEILING,
        )
        .unwrap();
}

#[test]
fn drop_without_lock_does_not_leak_by_mutating() {
    let mut w = world();
    let transcript = digest(50);
    let mut slot = VerifierChallengeSlot::new();
    let challenge = slot
        .issue(&w.owner, transcript, OperationId([9; 16]))
        .unwrap();
    let issued = issue_ok(&mut w, challenge, transcript);
    let grant = install_remote_disclosure(
        install_req(&w, &issued),
        &mut slot,
        &mut cose::Workspace::new(),
    )
    .unwrap();
    {
        let _hold = CurrentAuthorityGuard::try_lock(&w.owner).unwrap();
        drop(grant);
    }
    let mut owner = w.owner.lock().unwrap();
    for i in 1..=7 {
        owner
            .install_grant_with_ceiling(
                dummy_binding(i),
                NOW,
                TICKET_EXP,
                ContactState::Active,
                CEILING,
            )
            .unwrap();
    }
    assert_eq!(
        owner
            .install_grant_with_ceiling(
                dummy_binding(20),
                NOW,
                TICKET_EXP,
                ContactState::Active,
                CEILING,
            )
            .unwrap_err(),
        QdnfError::Capacity
    );
}

#[test]
fn policy_outcome_allow_is_not_a_grant() {
    assert_eq!(
        refuse_policy_outcome(PolicyOutcome::Allow),
        PolicyError::Reference
    );
    assert_eq!(
        DisclosureDecision::refuse_policy_outcome(PolicyOutcome::Allow),
        PolicyError::Reference
    );
}

#[test]
fn bool_is_not_a_grant() {
    assert_eq!(refuse_bool_authority(true), PolicyError::Reference);
    assert_eq!(
        DisclosureDecision::refuse_bool(false),
        PolicyError::Reference
    );
    assert_eq!(
        refuse_wire_role_install(1),
        PolicyError::Crypto(CryptoError::Unauthorized)
    );
    assert_eq!(
        refuse_delegation_chain(b"chain"),
        PolicyError::Crypto(CryptoError::Unsupported)
    );
}
