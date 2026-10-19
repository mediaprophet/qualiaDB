//! QDNF & Native SDN DNS Bridge for POET OS-Shell.
//!
//! Integrates QDNF Nym bearer, post-quantum IPC exchange, QDP front-door SDN DNS,
//! protected contact state transitions, and identifier fabric plane separation.
//! Adheres to AGENTS.md §0-B (< 320 lines).

use crate::q_hash;
use crate::NQuin;
use crate::PermissiveRoutingLane;
use crate::net::dns::quin_records::{encode_mx_record, encode_ns_record, OP_DNS_QDP_FRONT_DOOR};
use crate::net::dns::sdn::{parse_front_door_txt, parse_ns_encoded_did};
use crate::net::qdnf::authority::{Plane, TemporalGrant};
use crate::net::qdnf::bearer::nym::{decapsulate_nym, encapsulate_nym, NYM_MAGIC, NYM_SPHINX_MTU};
use crate::net::qdnf::contracts::identity::{
    consume_as_person, consume_authority, infer_natural_person_from_handle,
    inverse_functional_transfers_authority, same_as_transfers_authority, ReferentKind,
};
use crate::net::qdnf::fabric::{attempt_join, JoinAttempt, JoinKey};
use crate::net::qdnf::registries::BearerProfile;
use crate::net::qdnf::session::contact::{
    admit_contact, admit_help, group_membership_is_private_consent, payment_bypasses_block,
    revoke_mandate, ContactTable,
};
use crate::net::qdnf::types::{ObservedLocator, ProfileId, StrongDigest};
use crate::net::qdnf::vertical::two_peer_ipc_exchange;
use vibe::Value;

/// Evaluates the QDNF Nym mixnet transition bearer and post-quantum vertical slice.
pub fn evaluate_qdnf_nym_bearer(network: &str, peer_id: &str) -> Vec<(&'static str, Value)> {
    let peer_bytes = peer_id.as_bytes();
    let trunc_len = peer_bytes.len().min(32);
    let locator = ObservedLocator::from_slice(&peer_bytes[..trunc_len])
        .unwrap_or(ObservedLocator::EMPTY);

    let mut env_buf = [0u8; 1024];
    let enc_len = encapsulate_nym(&locator, b"QNYM-PROBE", &mut env_buf).unwrap_or(0);

    let mut out_sender = ObservedLocator::EMPTY;
    let mut dec_buf = [0u8; 1024];
    let _ = decapsulate_nym(&env_buf[..enc_len], &mut out_sender, &mut dec_buf);

    let pq_bytes = two_peer_ipc_exchange(b"qdnf-pq-check").unwrap_or(0);
    let magic_str = std::str::from_utf8(&NYM_MAGIC).unwrap_or("QNYM");

    vec![
        ("qdnf_profile", Value::String("NymMixnetTransitionV1".into())),
        ("qdnf_profile_code", Value::U64(BearerProfile::NymMixnetTransitionV1 as u64)),
        ("nym_magic", Value::String(magic_str.into())),
        ("nym_sphinx_mtu", Value::U64(NYM_SPHINX_MTU as u64)),
        ("nym_envelope_bytes", Value::U64(enc_len as u64)),
        ("qpr_suite", Value::String("qpr-pq-1 (ML-KEM-768 + X25519 + ChaCha20Poly1305)".into())),
        ("qpr_vertical_slice_bytes", Value::U64(pq_bytes as u64)),
        ("nym_network", Value::String(network.into())),
    ]
}

/// Evaluates QDP Front-Door SDN DNS, bare-registrar NS records, and Super-Quin encoding.
pub fn evaluate_qdnf_dns_front_door(
    domain: &str,
    front_door_did: &str,
    agent_type: &str,
    nym_locator: &str,
) -> (Vec<(&'static str, Value)>, Option<NQuin>) {
    let canonical_agent = match agent_type.to_ascii_lowercase().as_str() {
        "naturalperson" | "person" => "person",
        "organization" | "org" => "organization",
        "device" | "agent" => "agent",
        _ => "person",
    };

    let qdp_txt_clauses = format!(
        "qdp:signer <{front_door_did}> ; qdp:agentType \"{canonical_agent}\" ; qdp:wireguard \"a1b2c3d4e5f60718\" ; qdp:overlay \"10.42.0.1\" ; qdp:nym \"{nym_locator}\" ; qdp:profile <https://{domain}/.well-known/QDP>"
    );

    let view = parse_front_door_txt(domain, &qdp_txt_clauses).unwrap_or_else(|_| {
        let mut v = crate::net::dns::sdn::SdnFrontDoorView::empty(domain);
        v.front_door_did = front_door_did;
        v
    });

    let quin_opt = view.to_quin(3600, true);
    let quin_parity_ok = quin_opt.as_ref().map(|q| q.verify_ecc_parity()).unwrap_or(false);

    let did_suffix = front_door_did.strip_prefix("did:q42:").unwrap_or(front_door_did);
    let ns_host = format!("ns1.{did_suffix}.webizen.network.");
    let ns2_host = format!("ns2.{did_suffix}.webizen.network.");

    let ns_ptr = parse_ns_encoded_did(&ns_host)
        .and_then(|(_, ptr)| ptr)
        .unwrap_or(0);

    let _ns_quin = encode_ns_record(domain, &ns_host, 3600, PermissiveRoutingLane::PassthroughStandard);
    let _mx_quin = encode_mx_record(domain, 10, &format!("mail.{domain}"), 3600, PermissiveRoutingLane::PassthroughStandard);

    let pairs = vec![
        ("dns_qdp_txt", Value::String(format!("_qdp.{domain}. 3600 IN TXT \"{qdp_txt_clauses}\""))),
        ("qdp_txt_clauses", Value::String(qdp_txt_clauses)),
        ("dns_ns1_record", Value::String(format!("{domain}. 3600 IN NS {ns_host}"))),
        ("dns_ns2_record", Value::String(format!("{domain}. 3600 IN NS {ns2_host}"))),
        ("ns_topological_pointer", Value::U64(ns_ptr)),
        ("front_door_quin_opcode", Value::U64(OP_DNS_QDP_FRONT_DOOR as u64)),
        ("front_door_parity_valid", Value::Bool(quin_parity_ok)),
    ];

    (pairs, quin_opt)
}

/// Evaluates QDNF contact gating, consent lifecycle, and reserved help slots.
pub fn evaluate_qdnf_contact_gate(
    peer_did: &str,
    purpose_rel: &str,
    action: &str,
) -> Vec<(&'static str, Value)> {
    let mut peer_d = StrongDigest::ZERO;
    let ph = q_hash(peer_did);
    peer_d.0[0..8].copy_from_slice(&ph.to_be_bytes());
    peer_d.0[8] = 0x42;

    let mut purpose_d = StrongDigest::ZERO;
    let puh = q_hash(purpose_rel);
    purpose_d.0[0..8].copy_from_slice(&puh.to_be_bytes());
    purpose_d.0[8] = 0x42;

    let mut table = ContactTable::new();
    let grant = TemporalGrant {
        purpose_digest: purpose_d,
        audience_digest: StrongDigest::ZERO,
        authority_generation: 1,
        not_before_unix: 0,
        expires_unix: u64::MAX,
        profile: ProfileId::QDNF_CRYPTO_1,
    };

    let (state_str, admitted) = match action {
        "send_request" | "request" => {
            let _ = table.insert_request(peer_d, purpose_d, false, 1);
            ("Request", false)
        }
        "consent" => {
            let _ = table.insert_request(peer_d, purpose_d, false, 1);
            let _ = table.consent(peer_d);
            ("Consent", false)
        }
        "accept_connect" | "activate" => {
            let _ = table.insert_request(peer_d, purpose_d, false, 1);
            let _ = table.consent(peer_d);
            let _ = table.activate(peer_d);
            let ok = admit_contact(&table, peer_d, purpose_d, false, 100, &grant).is_ok();
            ("Active", ok)
        }
        "suspend" => {
            let _ = table.insert_request(peer_d, purpose_d, false, 1);
            let _ = table.consent(peer_d);
            let _ = table.activate(peer_d);
            let _ = table.suspend(peer_d);
            ("Suspended", false)
        }
        "block" => {
            let _ = table.insert_request(peer_d, purpose_d, false, 1);
            let _ = table.block(peer_d);
            ("Blocked", false)
        }
        "revoke_mandate" => {
            let _ = table.insert_request(peer_d, purpose_d, false, 1);
            let _ = revoke_mandate(&mut table, peer_d, 1);
            ("Blocked", false)
        }
        "admit_help" => {
            let ok = admit_help(&mut table, peer_d).is_ok();
            ("HelpAdmitted", ok)
        }
        _ => {
            let _ = table.insert_request(peer_d, purpose_d, false, 1);
            ("Request", false)
        }
    };

    vec![
        ("qdnf_contact_state", Value::String(state_str.into())),
        ("qdnf_contact_admitted", Value::Bool(admitted)),
        ("qdnf_contact_slots_max", Value::U64(16)),
        ("qdnf_help_slots_reserved", Value::U64(2)),
        ("payment_bypasses_block", Value::Bool(payment_bypasses_block())),
        ("group_membership_is_private_consent", Value::Bool(group_membership_is_private_consent())),
    ]
}

/// Evaluates identifier fabric planes and verifies strict personhood separation.
pub fn evaluate_qdnf_identifier_fabric(_identifier_did: &str) -> Vec<(&'static str, Value)> {
    let auth_handle = consume_authority(ReferentKind::Handle).is_err();
    let auth_inst = consume_authority(ReferentKind::Instrument).is_err();
    let person_handle = consume_as_person(ReferentKind::Handle).is_err();
    let person_inst = consume_as_person(ReferentKind::Instrument).is_err();

    let join_did = attempt_join(JoinAttempt {
        left: Plane::Entity,
        right: Plane::Instrument,
        key: JoinKey::Did,
    }).is_err();

    let join_wallet = attempt_join(JoinAttempt {
        left: Plane::Entity,
        right: Plane::Instrument,
        key: JoinKey::Wallet,
    }).is_err();

    let did_wallet_refused = join_did && join_wallet;
    let handle_not_person = person_handle && person_inst && auth_handle && auth_inst;

    vec![
        ("fabric_plane_separation", Value::String("FND-02.01: Entity | Claim | Handle | Instrument | NaturalPerson".into())),
        ("handle_is_person", Value::Bool(!handle_not_person)),
        ("same_as_transfers_authority", Value::Bool(same_as_transfers_authority())),
        ("infer_natural_person_from_handle", Value::Bool(infer_natural_person_from_handle())),
        ("inverse_functional_transfers_authority", Value::Bool(inverse_functional_transfers_authority())),
        ("did_wallet_join_refused", Value::Bool(did_wallet_refused)),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_qdnf_nym_bearer() {
        let pairs = evaluate_qdnf_nym_bearer("sandbox", "peer_node_42");
        assert!(pairs.iter().any(|(k, v)| *k == "nym_magic" && v == &Value::String("QNYM".into())));
        assert!(pairs.iter().any(|(k, v)| *k == "nym_sphinx_mtu" && v == &Value::U64(1024)));
    }

    #[test]
    fn test_qdnf_dns_front_door() {
        let (pairs, quin) = evaluate_qdnf_dns_front_door(
            "personal.webizen.example",
            "did:q42:testfront",
            "NaturalPerson",
            "nym:sandbox:1234@gw",
        );
        assert!(quin.is_some());
        assert!(pairs.iter().any(|(k, v)| *k == "front_door_parity_valid" && v == &Value::Bool(true)));
        assert!(pairs.iter().any(|(k, _)| *k == "dns_ns1_record"));
    }

    #[test]
    fn test_qdnf_contact_gate() {
        let pairs = evaluate_qdnf_contact_gate("did:q42:peer_bob", "spc:Collaboration", "accept_connect");
        assert!(pairs.iter().any(|(k, v)| *k == "qdnf_contact_state" && v == &Value::String("Active".into())));
        assert!(pairs.iter().any(|(k, v)| *k == "qdnf_contact_admitted" && v == &Value::Bool(true)));
    }

    #[test]
    fn test_qdnf_identifier_fabric() {
        let pairs = evaluate_qdnf_identifier_fabric("did:q42:alice");
        assert!(pairs.iter().any(|(k, v)| *k == "handle_is_person" && v == &Value::Bool(false)));
        assert!(pairs.iter().any(|(k, v)| *k == "did_wallet_join_refused" && v == &Value::Bool(true)));
    }
}
