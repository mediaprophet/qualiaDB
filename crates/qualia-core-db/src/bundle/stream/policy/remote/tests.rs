use super::*;
use crate::bundle::stream::policy::PolicyError;
use crate::crypto::network::authority::{PinAuthority, PinConfig, PinHandle};
use crate::crypto::network::cose::PairedPublicKeys;
use crate::crypto::network::digest::sha384;
use crate::crypto::network::errors::CryptoError;
use crate::crypto::network::object_stream::{encode_recipient, RecipientFields};
use crate::crypto::network::types::{
    KeyEpoch, KeyPurpose, ML_DSA_65_PK_LEN, ML_DSA_65_SIG_LEN, ML_DSA_65_SK_LEN, ML_KEM_768_CT_LEN,
    ML_KEM_768_PK_LEN, ML_KEM_768_SK_LEN,
};
use crate::governance::consent_contract::ConsentLedger;
use crate::net::qdnf::authority::PolicyOutcome;
use crate::net::qdnf::policy_labels::{
    encode_label_into, verify_label, Confidentiality, LabelFields,
};
use crate::net::qdnf::types::{OperationId, StrongDigest};
use crate::semantic_instruments::resolve::{
    bounded_closure::{verify_closure, ClosureNode},
    DependencyKind,
};
use crate::sensing::signed_scope::SignedSensingScope;
use ed25519_dalek::SigningKey;
use fips204::{
    ml_dsa_65,
    traits::{KeyGen, SerDes},
};

fn digest(b: u8) -> StrongDigest {
    StrongDigest([b; 48])
}

fn op() -> OperationId {
    OperationId([7; 16])
}

struct Pins {
    owner: PinAuthority,
    admin: crate::crypto::network::authority::LocalBootstrap,
    handle: PinHandle,
    principal: StrongDigest,
}

impl Pins {
    fn new() -> Self {
        let principal = digest(31);
        let ml_pk = ml_dsa_65::KG::keygen_from_seed(&[7; 32]).0.into_bytes();
        let ed = SigningKey::from_bytes(&[8; 32]).verifying_key().to_bytes();
        let (mut owner, admin) = PinAuthority::new_local_bootstrap().unwrap();
        let handle = owner
            .install_pin(
                &admin,
                PinConfig {
                    principal,
                    provider_epoch: KeyEpoch {
                        purpose: KeyPurpose::ControllerSign,
                        epoch: 1,
                        controller: 9,
                    },
                    keys: PairedPublicKeys {
                        ml_dsa_65: &ml_pk,
                        ed25519: &ed,
                    },
                    purpose: RECIPIENT_PURPOSE,
                    audience: b"reader",
                    not_before_unix: 10,
                    expires_unix: 10_000,
                },
                10,
            )
            .unwrap();
        Self {
            owner,
            admin,
            handle,
            principal,
        }
    }

    fn rotate(&mut self) -> PinHandle {
        let ml_pk = ml_dsa_65::KG::keygen_from_seed(&[11; 32]).0.into_bytes();
        let ed = SigningKey::from_bytes(&[12; 32]).verifying_key().to_bytes();
        self.handle = self
            .owner
            .rotate_pin(
                &self.admin,
                self.handle,
                PinConfig {
                    principal: self.principal,
                    provider_epoch: KeyEpoch {
                        purpose: KeyPurpose::ControllerSign,
                        epoch: 2,
                        controller: 9,
                    },
                    keys: PairedPublicKeys {
                        ml_dsa_65: &ml_pk,
                        ed25519: &ed,
                    },
                    purpose: RECIPIENT_PURPOSE,
                    audience: b"reader",
                    not_before_unix: 10,
                    expires_unix: 10_000,
                },
                10,
            )
            .unwrap();
        self.handle
    }
}

fn enrollment(
    pin: PinHandle,
    principal: StrongDigest,
    source: StrongDigest,
) -> RecognitionEnrollment {
    RecognitionEnrollment {
        pin,
        principal,
        source,
        source_length: 64,
        policy: digest(42),
        publisher: digest(43),
        purpose: digest(44),
        allowed_ops: AllowedOps::BOTH,
        operation: op(),
        expires_unix: 1000,
        max_decision_lifetime: 30,
    }
}

fn recognize_ok(
    table: &ControllerRecognition,
    principal: StrongDigest,
    source: StrongDigest,
) -> RecognizedController {
    table
        .recognize(
            principal,
            source,
            64,
            digest(42),
            digest(43),
            digest(44),
            op(),
            DisclosureAction::MetadataRelease,
            10,
        )
        .unwrap()
}

fn policy_fields() -> DisclosurePolicyFields {
    let mut closure_labels = [StrongDigest::ZERO; 16];
    closure_labels[0] = digest(20);
    DisclosurePolicyFields {
        coverage: SourceCoverage::WHOLE,
        source: digest(41),
        source_length: 64,
        controller: digest(31),
        publisher: digest(43),
        source_label: digest(21),
        closure_labels,
        closure_label_count: 1,
        recipient: digest(3),
        purpose: digest(44),
        operation: op(),
        consumer: digest(9),
        rule: digest(22),
        graph: digest(23),
    }
}

fn empty_budget() -> ClosureBudget {
    ClosureBudget {
        original_bytes: 0,
        nodes: 0,
        edges: 0,
    }
}

fn encode_a(buf: &mut [u8; RECIPIENT_BYTES]) -> RecipientRecord<'_> {
    let ml = [3u8; 1184];
    let dh = [4u8; 32];
    let fields = RecipientFields {
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
        key_epoch: 1,
        not_before: 10,
        expires: 100,
        ml_public: &ml,
        dh_public: &dh,
    };
    encode_recipient(fields, buf).unwrap();
    RecipientRecord::parse(buf).unwrap()
}

#[test]
fn unknown_principal_is_denied() {
    let pins = Pins::new();
    let (mut table, admin) = ControllerRecognition::new_local_admin().unwrap();
    table
        .install(
            &admin,
            enrollment(pins.handle, pins.principal, digest(41)),
            10,
        )
        .unwrap();
    assert_eq!(
        table
            .recognize(
                digest(99),
                digest(41),
                64,
                digest(42),
                digest(43),
                digest(44),
                op(),
                DisclosureAction::MetadataRelease,
                10,
            )
            .unwrap_err(),
        PolicyError::Reference
    );
    assert_eq!(
        recognize_ok(&table, pins.principal, digest(41)).principal(),
        pins.principal
    );
}

#[test]
fn source_wildcard_and_empty_source_are_denied() {
    let pins = Pins::new();
    let (mut table, admin) = ControllerRecognition::new_local_admin().unwrap();
    let mut empty = enrollment(pins.handle, pins.principal, StrongDigest::ZERO);
    assert_eq!(
        table.install(&admin, empty, 10).unwrap_err(),
        PolicyError::Reference
    );
    empty.source = digest(41);
    empty.source_length = 0;
    assert_eq!(
        table.install(&admin, empty, 10).unwrap_err(),
        PolicyError::Reference
    );
    table
        .install(
            &admin,
            enrollment(pins.handle, pins.principal, digest(41)),
            10,
        )
        .unwrap();
    assert_eq!(
        table
            .recognize(
                pins.principal,
                StrongDigest::ZERO,
                64,
                digest(42),
                digest(43),
                digest(44),
                op(),
                DisclosureAction::MetadataRelease,
                10,
            )
            .unwrap_err(),
        PolicyError::Reference
    );
}

#[test]
fn eighth_enrollment_ok_ninth_is_capacity() {
    let pins = Pins::new();
    let (mut table, admin) = ControllerRecognition::new_local_admin().unwrap();
    for i in 0..MAX_RECOGNIZED {
        table
            .install(
                &admin,
                enrollment(pins.handle, pins.principal, digest(61 + i as u8)),
                10,
            )
            .unwrap();
    }
    assert_eq!(
        table
            .install(
                &admin,
                enrollment(
                    pins.handle,
                    pins.principal,
                    digest(61 + MAX_RECOGNIZED as u8)
                ),
                10,
            )
            .unwrap_err(),
        PolicyError::Capacity
    );
}

#[test]
fn rotation_replaces_recognition_and_invalidates_old_decision_ids() {
    let mut pins = Pins::new();
    let (mut table, admin) = ControllerRecognition::new_local_admin().unwrap();
    let handle = table
        .install(
            &admin,
            enrollment(pins.handle, pins.principal, digest(41)),
            10,
        )
        .unwrap();
    let old = table.snapshot(handle).unwrap();
    let consumed = table.consume_decision_id(&admin, handle).unwrap();
    assert_eq!(consumed, 1);
    let mut a_buf = [0u8; RECIPIENT_BYTES];
    let a = encode_a(&mut a_buf);
    let old_ticket = DecisionTicketFields {
        recipient_record_id: a.id(),
        transcript: digest(50),
        request_id: OperationId([9; 16]),
        challenge: [5; 32],
        controller_generation: old.controller_generation(),
        policy: old.policy(),
        action: DisclosureAction::MetadataRelease,
        not_before: 10,
        expires: 40,
        decision_id: consumed,
    };
    DisclosureDecision::bind(&old, &old_ticket, 10).unwrap();
    let old_pin = pins.handle;
    let new_pin = pins.rotate();
    assert_ne!(new_pin, old_pin);
    let new_handle = table
        .rotate(
            &admin,
            handle,
            enrollment(new_pin, pins.principal, digest(41)),
            10,
        )
        .unwrap();
    assert_ne!(new_handle, handle);
    assert!(matches!(
        table.snapshot(handle),
        Err(PolicyError::Crypto(CryptoError::StaleGeneration))
    ));
    let new = table.snapshot(new_handle).unwrap();
    assert_eq!(
        recognize_ok(&table, pins.principal, digest(41)).controller_generation(),
        new.controller_generation()
    );
    assert_ne!(new.pin(), old.pin());
    assert_ne!(new.controller_generation(), old.controller_generation());
    assert!(new.decision_id_floor() > consumed);
    assert_eq!(
        DisclosureDecision::bind(&new, &old_ticket, 10).unwrap_err(),
        PolicyError::Reference
    );
    let mut reused = old_ticket;
    reused.controller_generation = new.controller_generation();
    reused.decision_id = consumed;
    assert_eq!(
        DisclosureDecision::bind(&new, &reused, 10).unwrap_err(),
        PolicyError::Reference
    );
    reused.decision_id = new.decision_id_floor();
    DisclosureDecision::bind(&new, &reused, 10).unwrap();
}

#[test]
fn wrong_purpose_publisher_or_expired_recognition_is_denied() {
    let pins = Pins::new();
    let (mut table, admin) = ControllerRecognition::new_local_admin().unwrap();
    table
        .install(
            &admin,
            enrollment(pins.handle, pins.principal, digest(41)),
            10,
        )
        .unwrap();
    assert_eq!(
        table
            .recognize(
                pins.principal,
                digest(41),
                64,
                digest(42),
                digest(43),
                digest(45),
                op(),
                DisclosureAction::MetadataRelease,
                10,
            )
            .unwrap_err(),
        PolicyError::Reference
    );
    assert_eq!(
        table
            .recognize(
                pins.principal,
                digest(41),
                64,
                digest(42),
                digest(46),
                digest(44),
                op(),
                DisclosureAction::MetadataRelease,
                10,
            )
            .unwrap_err(),
        PolicyError::Reference
    );
    assert_eq!(
        table
            .recognize(
                pins.principal,
                digest(41),
                64,
                digest(42),
                digest(43),
                digest(44),
                op(),
                DisclosureAction::MetadataRelease,
                1000,
            )
            .unwrap_err(),
        PolicyError::Crypto(CryptoError::Expired)
    );
}

#[test]
fn policy_artifact_missing_gap_header_or_index_coverage_is_denied() {
    let mut buf = [0u8; MAX_DISCLOSURE_POLICY_BYTES];
    let n = encode_disclosure_policy_into(policy_fields(), empty_budget(), &mut buf).unwrap();
    decode_disclosure_policy(&buf[..n]).unwrap();
    let nodes = [ClosureNode {
        digest: sha384(&buf[..n]),
        kind: DependencyKind::LawPackage,
        original_bytes: &buf[..n],
    }];
    let candidate = verify_closure(nodes[0].digest, &nodes, &[]).unwrap();
    bind_policy_to_closure(&buf[..n], &candidate).unwrap();

    for bits in [
        SourceCoverage::INDEX | SourceCoverage::GAPS | SourceCoverage::ENTRIES,
        SourceCoverage::HEADER | SourceCoverage::GAPS | SourceCoverage::ENTRIES,
        SourceCoverage::HEADER | SourceCoverage::INDEX | SourceCoverage::ENTRIES,
        SourceCoverage::HEADER | SourceCoverage::INDEX | SourceCoverage::GAPS,
        0,
    ] {
        let mut fields = policy_fields();
        fields.coverage = SourceCoverage::from_bits(bits);
        assert_eq!(
            encode_disclosure_policy_into(
                fields,
                empty_budget(),
                &mut [0; MAX_DISCLOSURE_POLICY_BYTES]
            )
            .unwrap_err(),
            PolicyError::Graph
        );
        let mut damaged = buf;
        damaged[17] = bits;
        assert_eq!(
            decode_disclosure_policy(&damaged[..n]).unwrap_err(),
            PolicyError::Graph
        );
    }
    let mut unknown = buf;
    unknown[16] = 2;
    assert_eq!(
        decode_disclosure_policy(&unknown[..n]).unwrap_err(),
        PolicyError::Encoding
    );
}

#[test]
fn decision_ticket_codec_roundtrip_and_truncated_trailing_wrong_domain_refuse() {
    let mut a_buf = [0u8; RECIPIENT_BYTES];
    let a = encode_a(&mut a_buf);
    assert_eq!(a.encoded().len(), 1856);
    assert_eq!(RECIPIENT_BYTES, REMOTE_AUTHORIZATION_BYTES);
    let expected = IndependentAdmission {
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
    };
    check_recipient_against_independent(&a, &expected).unwrap();
    let mut wrong = expected;
    wrong.publisher = digest(1);
    assert_eq!(
        check_recipient_against_independent(&a, &wrong).unwrap_err(),
        PolicyError::Reference
    );

    let fields = DecisionTicketFields {
        recipient_record_id: a.id(),
        transcript: digest(50),
        request_id: OperationId([9; 16]),
        challenge: [5; 32],
        controller_generation: crate::net::qdnf::types::Generation(1),
        policy: digest(42),
        action: DisclosureAction::PayloadUse,
        not_before: 10,
        expires: 40,
        decision_id: 1,
    };
    let mut ticket = [0u8; DECISION_TICKET_BYTES];
    assert_eq!(
        encode_decision_ticket_into(fields, &mut ticket).unwrap(),
        DECISION_TICKET_BYTES
    );
    assert_eq!(decode_decision_ticket(&ticket).unwrap(), fields);
    assert_eq!(
        ticket_package_len(DECISION_TICKET_BYTES, 0).unwrap(),
        DECISION_TICKET_BYTES
    );
    assert_eq!(
        ticket_package_len(
            DECISION_TICKET_BYTES,
            MAX_TICKET_PACKAGE_BYTES - DECISION_TICKET_BYTES
        )
        .unwrap(),
        MAX_TICKET_PACKAGE_BYTES
    );
    assert_eq!(
        ticket_package_len(
            DECISION_TICKET_BYTES,
            MAX_TICKET_PACKAGE_BYTES - DECISION_TICKET_BYTES + 1
        )
        .unwrap_err(),
        PolicyError::Limit
    );
    assert_eq!(
        decode_decision_ticket(&ticket[..DECISION_TICKET_BYTES - 1]).unwrap_err(),
        PolicyError::Encoding
    );
    let mut trailing = [0u8; DECISION_TICKET_BYTES + 1];
    trailing[..DECISION_TICKET_BYTES].copy_from_slice(&ticket);
    assert_eq!(
        decode_decision_ticket(&trailing).unwrap_err(),
        PolicyError::Encoding
    );
    let mut domain = ticket;
    domain[0] ^= 1;
    assert_eq!(
        decode_decision_ticket(&domain).unwrap_err(),
        PolicyError::Encoding
    );
    assert_eq!(
        DISCLOSURE_DECISION_PURPOSE,
        b"qbdl-stream-protected-v1/disclosure-decision"
    );
}

#[test]
fn disclosure_decision_has_no_public_constructor() {
    assert_eq!(
        DisclosureDecision::refuse_policy_outcome(PolicyOutcome::Allow),
        PolicyError::Reference
    );
    assert_eq!(
        refuse_policy_outcome(PolicyOutcome::Allow),
        PolicyError::Reference
    );
    assert_eq!(
        DisclosureDecision::refuse_bool(true),
        PolicyError::Reference
    );
    assert_eq!(refuse_bool_authority(false), PolicyError::Reference);
    let pins = Pins::new();
    let (mut table, admin) = ControllerRecognition::new_local_admin().unwrap();
    let handle = table
        .install(
            &admin,
            enrollment(pins.handle, pins.principal, digest(41)),
            10,
        )
        .unwrap();
    let recognized = table.snapshot(handle).unwrap();
    let mut a_buf = [0u8; RECIPIENT_BYTES];
    let a = encode_a(&mut a_buf);
    let ticket = DecisionTicketFields {
        recipient_record_id: a.id(),
        transcript: digest(50),
        request_id: OperationId([9; 16]),
        challenge: [5; 32],
        controller_generation: recognized.controller_generation(),
        policy: recognized.policy(),
        action: DisclosureAction::MetadataRelease,
        not_before: 10,
        expires: 40,
        decision_id: recognized.decision_id_floor(),
    };
    let decision = DisclosureDecision::bind(&recognized, &ticket, 10).unwrap();
    assert_eq!(decision.action(), DisclosureAction::MetadataRelease);
    assert_eq!(decision.policy(), digest(42));
    assert_eq!(decision.decision_id(), 1);
}

#[test]
fn recognition_table_holds_no_key_material() {
    use core::mem::size_of;
    assert_eq!(RECIPIENT_BYTES, 1856);
    assert_eq!(REMOTE_AUTHORIZATION_BYTES, 1856);
    assert!(size_of::<ControllerRecognition>() < 8 * ML_DSA_65_PK_LEN);
    assert!(size_of::<ControllerRecognition>() < ML_DSA_65_SK_LEN);
    assert!(size_of::<RecognitionEnrollment>() < ML_DSA_65_PK_LEN);
    assert!(size_of::<RecognitionEnrollment>() < ML_KEM_768_PK_LEN);
    assert!(size_of::<RecognitionEnrollment>() < ML_DSA_65_SIG_LEN);
    assert!(size_of::<RecognitionHandle>() < 64);
    assert_ne!(size_of::<RecognitionEnrollment>(), ML_DSA_65_PK_LEN);
    assert_ne!(size_of::<RecognitionEnrollment>(), ML_KEM_768_PK_LEN);
    assert_ne!(size_of::<RecognitionEnrollment>(), ML_KEM_768_SK_LEN);
    assert_ne!(size_of::<ControllerRecognition>(), 8 * ML_DSA_65_PK_LEN);
    let _ = (ML_KEM_768_CT_LEN, ML_DSA_65_SK_LEN);
    assert_eq!(size_of::<AllowedOps>(), size_of::<u16>());
}

#[test]
fn wire_role_flag_cannot_install() {
    assert_eq!(
        refuse_wire_role_install(1),
        PolicyError::Crypto(CryptoError::Unauthorized)
    );
    assert_eq!(
        refuse_delegation_chain(b"chain"),
        PolicyError::Crypto(CryptoError::Unsupported)
    );
    assert_eq!(
        refuse_all_subjects_consented_flag(true),
        PolicyError::Reference
    );
}

#[test]
fn verified_label_consent_ledger_and_sensing_scope_are_not_remote_authority() {
    let fields = LabelFields::request(Confidentiality::C1Private, digest(9));
    let mut buf = [0u8; 256];
    let n = encode_label_into(&fields, &mut buf).unwrap();
    let label = verify_label(fields, &buf[..n]).unwrap();
    assert_eq!(
        refuse_verified_label_as_issuer(&label),
        PolicyError::Reference
    );
    assert_eq!(
        refuse_consent_ledger_as_pq_source_auth(&ConsentLedger::new()),
        PolicyError::Crypto(CryptoError::Unsupported)
    );
    let _reject: fn(&SignedSensingScope) -> PolicyError =
        refuse_signed_sensing_scope_as_whole_source;
}
