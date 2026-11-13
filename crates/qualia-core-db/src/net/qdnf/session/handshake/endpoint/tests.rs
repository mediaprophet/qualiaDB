use super::*;
use crate::crypto::network::{
    authority::{LocalBootstrap, PinAuthority, PinConfig},
    cose,
    digest::sha384,
    key_provider::ProviderGrant,
    types::{KeyEpoch, KeyPurpose},
};
use crate::net::{
    peer::runtime::{ReservationLedger, ResourceBudget},
    qdnf::{
        authority::AuthorityOwner,
        bearer::{contract::Bearer, ipc_pair},
        types::{OperationId, ScopeEpoch},
    },
};
use crate::specialized_libs::cryptographic_library::{
    AccessPolicy, AuthenticationMethod, CryptographicLibrary, KeyOperation, ManagedPairedSigning,
    PairedSigningRequest, SecurityLevel, SigningAccess, SigningControl, TimeRestrictions,
};
use std::sync::Mutex;
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
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
struct Signer {
    library: CryptographicLibrary,
    control: SigningControl,
    ml: [u8; 1952],
    ed: [u8; 32],
}
impl Signer {
    fn new() -> Self {
        let mut library = CryptographicLibrary::new();
        library.initialize().unwrap();
        let (ml, ed) = library
            .generate_managed_signing_pair("ml", "ed", SecurityLevel::High)
            .unwrap();
        library.install_managed_signing_policy(policy("ml_private"));
        library.install_managed_signing_policy(policy("ed_private"));
        Self {
            library,
            control: SigningControl::new(1).unwrap(),
            ml: ml.key_data.as_slice().try_into().unwrap(),
            ed: ed.key_data.as_slice().try_into().unwrap(),
        }
    }
    fn keys(&self) -> cose::PairedPublicKeys<'_> {
        cose::PairedPublicKeys {
            ml_dsa_65: &self.ml,
            ed25519: &self.ed,
        }
    }
    fn capability<'a>(
        &'a self,
        c: &'a SigningCommitment,
        clock: &'a dyn Fn() -> u64,
    ) -> ManagedPairedSigning<'a> {
        self.library
            .begin_paired_signing(
                PairedSigningRequest {
                    ml_key_id: "ml_private",
                    ed_key_id: "ed_private",
                    ml_policy_id: "ml_private",
                    ed_policy_id: "ed_private",
                    binding: c.binding(),
                    grant: ProviderGrant {
                        epoch: KeyEpoch {
                            purpose: KeyPurpose::ControllerSign,
                            epoch: 1,
                            controller: 7,
                        },
                        cell_epoch: 1,
                        authority_generation: 1,
                        deadline_unix: u64::MAX,
                        revoked: false,
                    },
                    access: SigningAccess {
                        authenticated_methods: &[AuthenticationMethod::HardwareToken],
                        ip_address: "local",
                    },
                },
                &self.control,
                clock,
            )
            .unwrap()
    }
}
struct Fixture {
    initiator: Signer,
    responder: Signer,
    intent: Intent,
    pins_i: PinAuthority,
    pins_r: PinAuthority,
    admin_i: LocalBootstrap,
    admin_r: LocalBootstrap,
    peer_i: PeerTrust,
    peer_r: PeerTrust,
}
impl Fixture {
    fn new() -> Self {
        let initiator = Signer::new();
        let responder = Signer::new();
        let time = now();
        let intent = Intent {
            initiator: sha384(b"initiator"),
            responder: sha384(b"responder"),
            service: sha384(b"qbdl"),
            purpose: sha384(b"stream"),
            instrument: sha384(b"grant"),
            scope: sha384(b"source"),
            operation: OperationId([1; 16]),
            initiator_keys: initiator.keys().key_ids(),
            responder_keys: responder.keys().key_ids(),
            initiator_epoch: 1,
            responder_epoch: 1,
            expires_unix: time + 300,
        };
        let (mut pins_i, admin_i) = PinAuthority::new_local_bootstrap().unwrap();
        let (mut pins_r, admin_r) = PinAuthority::new_local_bootstrap().unwrap();
        let epoch = KeyEpoch {
            purpose: KeyPurpose::ControllerSign,
            epoch: 1,
            controller: 7,
        };
        let peer_i = PeerTrust {
            pin: pins_i
                .install_pin(
                    &admin_i,
                    PinConfig {
                        principal: intent.responder,
                        provider_epoch: epoch,
                        keys: responder.keys(),
                        purpose: RESPONDER_PURPOSE,
                        audience: &intent.initiator.0,
                        not_before_unix: time,
                        expires_unix: time + 300,
                    },
                    time,
                )
                .unwrap(),
        };
        let peer_r = PeerTrust {
            pin: pins_r
                .install_pin(
                    &admin_r,
                    PinConfig {
                        principal: intent.initiator,
                        provider_epoch: epoch,
                        keys: initiator.keys(),
                        purpose: INITIATOR_PURPOSE,
                        audience: &intent.responder.0,
                        not_before_unix: time,
                        expires_unix: time + 300,
                    },
                    time,
                )
                .unwrap(),
        };
        Self {
            initiator,
            responder,
            intent,
            pins_i,
            pins_r,
            admin_i,
            admin_r,
            peer_i,
            peer_r,
        }
    }
}
fn ledger() -> Mutex<ReservationLedger> {
    let cap = ResourceBudget {
        bytes: 2 * ENDPOINT_RESERVATION_BYTES,
        work: 64,
        io: 64,
    };
    Mutex::new(ReservationLedger::new(cap, cap, cap, cap, cap))
}
fn owner() -> Mutex<AuthorityOwner> {
    Mutex::new(AuthorityOwner::new())
}
#[test]
fn independently_driven_actual_bearer_managed_keys_and_received_finished() {
    let f = Fixture::new();
    let oi = owner();
    let or = owner();
    let li = ledger();
    let lr = ledger();
    let ai = EndpointAuthority::adopt_local(
        &oi,
        f.intent,
        Role::Initiator,
        now(),
        ENDPOINT_RESERVATION_BYTES,
    )
    .unwrap();
    let ar = EndpointAuthority::adopt_local(
        &or,
        f.intent,
        Role::Responder,
        now(),
        ENDPOINT_RESERVATION_BYTES,
    )
    .unwrap();
    let (mut bi, mut br) = ipc_pair(ScopeEpoch { scope: 1, epoch: 1 }, 1500).unwrap();
    let ri = Route::observed(&bi, br.locator()).unwrap();
    let rr = Route::observed(&br, bi.locator()).unwrap();
    let mut wi = EndpointWorkspace::new();
    let mut wr = EndpointWorkspace::new();
    let mut i = Initiator::new(
        &ai,
        EndpointReservation::admit(&li).unwrap(),
        &mut wi,
        ri,
        f.peer_i,
        &now,
    )
    .unwrap();
    let mut r = Responder::new(
        &ar,
        EndpointReservation::admit(&lr).unwrap(),
        &mut wr,
        rr,
        f.peer_r,
        &now,
    )
    .unwrap();
    let commitment = i.signing_commitment().unwrap();
    i.sign_client(f.initiator.capability(&commitment, &now), &f.pins_i, &now)
        .unwrap();
    // Each tick operates only one local endpoint and one bearer; the receiver can wait independently.
    assert_eq!(r.receive_step(&mut br, &f.pins_r, &now), Err(E::WouldBlock));
    for _ in 0..8 {
        let done = i.send_step(&mut bi, &f.pins_i, &now).unwrap();
        let received = r.receive_step(&mut br, &f.pins_r, &now).unwrap();
        if done {
            assert!(received);
            break;
        }
    }
    assert_eq!(i.phase(), Phase::AwaitingServer);
    assert_eq!(r.phase(), Phase::ServerProof);
    let commitment = r.signing_commitment().unwrap();
    r.sign_server(f.responder.capability(&commitment, &now), &f.pins_r, &now)
        .unwrap();
    for _ in 0..8 {
        let done = r.send_step(&mut br, &f.pins_r, &now).unwrap();
        let received = i.receive_step(&mut bi, &f.pins_i, &now).unwrap();
        if done {
            assert!(received);
            break;
        }
    }
    assert_eq!(i.phase(), Phase::InitiatorFinishedSending);
    assert_eq!(r.phase(), Phase::AwaitingFinished);
    // Neither installed state existed before the received peer confirmation.
    assert!(i.send_step(&mut bi, &f.pins_i, &now).unwrap());
    assert_eq!(i.phase(), Phase::Ready);
    assert_eq!(i.common.transcript.as_bytes().len(), 3389);
    let mut installed_i = i.install(&f.pins_i, &now).unwrap();
    assert_eq!(r.phase(), Phase::AwaitingFinished);
    assert!(r.receive_step(&mut br, &f.pins_r, &now).unwrap());
    assert_eq!(r.common.transcript.as_bytes().len(), 3389);
    let mut installed_r = r.install(&f.pins_r, &now).unwrap();
    assert_eq!(installed_i.transcript(), installed_r.transcript());
    installed_i
        .with_current(&f.pins_i, &now, |ib, ik| {
            installed_r.with_current(&f.pins_r, &now, |rb, rk| {
                assert_eq!(ik.send_key(), rk.recv_key());
                assert_eq!(ik.recv_key(), rk.send_key());
                assert_ne!(ik.send_key(), ik.recv_key());
                assert_eq!(ib.operation(), rb.operation());
                Ok(())
            })
        })
        .unwrap();
    ar.revoke().unwrap();
    assert_eq!(
        installed_r.with_current(&f.pins_r, &now, |_, _| Ok(())),
        Err(E::Revoked)
    );
    drop(installed_i);
    drop(installed_r);
    drop(i);
    drop(r);
    assert_eq!(li.lock().unwrap().used().host.bytes, 0);
    assert_eq!(lr.lock().unwrap().used().host.bytes, 0);
}

#[path = "negative_tests.rs"]
mod negative_tests;

#[path = "matrix_tests.rs"]
mod matrix_tests;

#[path = "fixture_tests.rs"]
mod fixture_tests;

#[path = "established/protected_channel/endpoint_tests.rs"]
mod channel_tests;
