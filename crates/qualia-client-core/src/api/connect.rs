//! Connection flow: magic link â†’ verify â†’ SocialWebNet peer

#![allow(non_snake_case)]

use super::*;


#[cfg(not(target_arch = "wasm32"))]
fn resolve_front_door_did(front_door_did: &str) -> Result<String, String> {
    if !front_door_did.is_empty() {
        return Ok(front_door_did.to_string());
    }
    crate::domains::list_domains()
        .first()
        .map(|d| d.front_door_did.clone())
        .ok_or_else(|| "no domain yet â€” create one in Domains & Mail first".to_string())
}

#[cfg(not(target_arch = "wasm32"))]
fn build_signed_identifier(
    front_door_did: String,
    relation_type: String,
    domain: &str,
) -> Result<crate::connection_identifier::ConnectionIdentifier, String> {
    let id = crate::node_identity::NodeIdentity::load_or_create()?;
    let fdd = resolve_front_door_did(&front_door_did)?;
    let rendezvous = if domain.is_empty() {
        vec![]
    } else {
        vec![crate::connection_identifier::RendezvousHint { kind: "domain".into(), value: domain.to_string() }]
    };
    let now = mail_now_unix();
    let mut ci = crate::connection_identifier::ConnectionIdentifier {
        version: crate::connection_identifier::CI_VERSION,
        front_door_did: fdd,
        identity_pubkey_hex: String::new(),
        wireguard_pubkey_hex: id.wireguard_pubkey_hex(),
        overlay_addr: id.overlay_addr(),
        rendezvous,
        relation_type,
        display_name: crate::user_profile::load_profile().display_name,
        created_at: now,
        expires_at: now + 7 * 24 * 3600,
        nonce: uuid::Uuid::new_v4().to_string(),
        signature_hex: String::new(),
    };
    ci.sign(&id.signing_key());
    Ok(ci)
}

/// A signed connection identifier for this node (self-certifying front-door DID + WireGuard peering).
#[cfg(not(target_arch = "wasm32"))]
pub fn generate_connection_identifier(
    front_door_did: String,
    relation_type: String,
) -> Result<serde_json::Value, String> {
    let ci = build_signed_identifier(front_door_did, relation_type, "")?;
    serde_json::to_value(ci).map_err(|e| e.to_string())
}

/// A magic link (deep link + https + mailto) carrying this node's connection identifier.
#[cfg(not(target_arch = "wasm32"))]
pub fn generate_magic_link(
    front_door_did: String,
    relation_type: String,
    domain: String,
) -> Result<serde_json::Value, String> {
    let ci = build_signed_identifier(front_door_did, relation_type, &domain)?;
    let deep = crate::magic_link::to_deep_link(&ci)?;
    let https = if domain.is_empty() {
        String::new()
    } else {
        crate::magic_link::to_https_link(&ci, &domain)?
    };
    let mailto = crate::magic_link::to_mailto(&ci, "Connect with me on Webizen")?;
    Ok(serde_json::json!({ "deep_link": deep, "https_link": https, "mailto": mailto }))
}

/// Accept a magic link: parse + **verify** the identifier (self-certifying), then register the sender as a
/// SocialWebNet peer (their WireGuard peering material). Half of the mutual peering; the return handshake
/// completes it.
#[cfg(not(target_arch = "wasm32"))]
pub fn accept_connection(link: String) -> Result<serde_json::Value, String> {
    let ci = crate::magic_link::from_link(&link)?;
    ci.verify()?;
    if ci.is_expired(mail_now_unix()) {
        return Err("this connection link has expired".into());
    }
    let peer = crate::social_peers::SocialPeer {
        did: ci.front_door_did.clone(),
        display_name: ci.display_name.clone(),
        wireguard_pubkey_hex: ci.wireguard_pubkey_hex.clone(),
        overlay_addr: ci.overlay_addr.clone(),
        endpoint: ci
            .rendezvous
            .iter()
            .find(|r| r.kind == "domain" || r.kind == "edge")
            .map(|r| r.value.clone()),
        relation_type: ci.relation_type.clone(),
        added_at: mail_now_unix(),
        active: true,
        // Set separately once the peer publishes their envelope key (or via the handshake, later).
        envelope_pubkey_hex: None,
    };
    crate::social_peers::register_peer(peer.clone())?;
    serde_json::to_value(peer).map_err(|e| e.to_string())
}

/// The SocialWebNet peers (accepted connections).
pub fn list_social_peers() -> Result<serde_json::Value, String> {
    serde_json::to_value(crate::social_peers::list_peers()).map_err(|e| e.to_string())
}

/// Enable/disable a peer (the socially-defined revoke).
pub fn set_social_peer_active(did: String, active: bool) -> Result<serde_json::Value, String> {
    crate::social_peers::set_peer_active(&did, active)?;
    list_social_peers()
}

/// Set peer transport endpoint for mesh dial (`host:port`), or clear with empty/None.
pub fn set_social_peer_endpoint(
    did: String,
    endpoint: Option<String>,
) -> Result<serde_json::Value, String> {
    crate::social_peers::set_peer_endpoint(&did, endpoint.as_deref())?;
    list_social_peers()
}

/// Per-peer mesh dialability â€” which accepted peers can form a SocialWebNet tunnel now, which must
/// wait for the peer to reach us (roaming), and which are missing key material. Pure/read-only.
pub fn mesh_dialability() -> Result<serde_json::Value, String> {
    let peers = crate::social_peers::list_peers();
    serde_json::to_value(crate::social_mesh::dialability(&peers)).map_err(|e| e.to_string())
}

