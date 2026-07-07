//! SocialWebNet peer store.
//!
//! Where an accepted connection's peering material lands. Once a connection
//! offer has been mutually authenticated (see `handshake.rs`) and the peering
//! payload exchanged (see `connection_identifier.rs`), the resulting
//! [`SocialPeer`] — the counterpart's DID plus the WireGuard public key and
//! overlay address needed to reach them on the SocialWebNet mesh — is recorded
//! here.
//!
//! The store is deliberately thin and additive: pure list-manipulation helpers
//! ([`upsert`], [`find`]) carry the logic and are unit-tested in isolation,
//! while the `*_peer` functions layer a small pretty-JSON persistence step on
//! top (a `Vec<SocialPeer>` at `app_meta_dir()/social_peers.json`).

use std::fs;
use std::path::PathBuf;

use crate::state::app_meta_dir;

/// An accepted peer on the SocialWebNet mesh.
///
/// This is the material needed to reach and recognise a connection after the
/// handshake has completed — the counterpart's stable identifier, a friendly
/// label, and the WireGuard/overlay coordinates that route packets to them.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct SocialPeer {
    /// The peer's DID — the stable identifier that keys this record.
    pub did: String,
    /// Human-friendly label for the peer (from their profile / the invite).
    pub display_name: String,
    /// The peer's WireGuard public key, hex-encoded.
    pub wireguard_pubkey_hex: String,
    /// The peer's address on the SocialWebNet overlay.
    pub overlay_addr: String,
    /// Optional last-known transport endpoint (`host:port`) for the tunnel.
    pub endpoint: Option<String>,
    /// The relationship this peering was established under (free-form; aligns
    /// with `spc:relationType` in the directory ontology).
    pub relation_type: String,
    /// Unix seconds at which this peer was added.
    pub added_at: u64,
    /// Whether the peering is currently active (a soft on/off that leaves the
    /// record in place).
    pub active: bool,
    /// The peer's **envelope (X25519) public key**, hex — the key to *seal payloads to* this peer (distinct
    /// from the WireGuard key, which routes packets). Lets the accountability fabric resolve a
    /// worker/trustee's key from their peer record instead of pasting it. `None` until the peer publishes it.
    #[serde(default)]
    pub envelope_pubkey_hex: Option<String>,
}

// ---------------------------------------------------------------------------
// Pure helpers (unit-tested; no filesystem)
// ---------------------------------------------------------------------------

/// Insert or update `peer` in `peers`, keyed by [`SocialPeer::did`].
///
/// If a peer with the same `did` already exists it is replaced in place
/// (preserving its position); otherwise `peer` is appended.
pub fn upsert(peers: &mut Vec<SocialPeer>, peer: SocialPeer) {
    if let Some(slot) = peers.iter_mut().find(|p| p.did == peer.did) {
        *slot = peer;
    } else {
        peers.push(peer);
    }
}

/// Find the peer with the given `did`, if present.
pub fn find<'a>(peers: &'a [SocialPeer], did: &str) -> Option<&'a SocialPeer> {
    peers.iter().find(|p| p.did == did)
}

// ---------------------------------------------------------------------------
// Persistence (filesystem)
// ---------------------------------------------------------------------------

fn peers_path() -> PathBuf {
    app_meta_dir().join("social_peers.json")
}

fn save_peers(peers: &[SocialPeer]) -> Result<(), String> {
    let path = peers_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let text = serde_json::to_string_pretty(peers).map_err(|e| e.to_string())?;
    fs::write(path, text).map_err(|e| e.to_string())
}

/// Load every stored peer. Returns `vec![]` if the store file is absent or
/// unreadable.
pub fn list_peers() -> Vec<SocialPeer> {
    fs::read_to_string(peers_path())
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_default()
}

/// Register (insert-or-update) a peer, then persist the store.
pub fn register_peer(peer: SocialPeer) -> Result<(), String> {
    let mut peers = list_peers();
    upsert(&mut peers, peer);
    save_peers(&peers)
}

/// Set the `active` flag on the peer with the given `did`, then persist.
///
/// Returns an error if no peer with that `did` is stored.
pub fn set_peer_active(did: &str, active: bool) -> Result<(), String> {
    let mut peers = list_peers();
    match peers.iter_mut().find(|p| p.did == did) {
        Some(p) => p.active = active,
        None => return Err(format!("no peer with did {did}")),
    }
    save_peers(&peers)
}

/// Remove the peer with the given `did` from the store, then persist.
///
/// Removing an absent `did` is a no-op success (the store already lacks it).
pub fn remove_peer(did: &str) -> Result<(), String> {
    let mut peers = list_peers();
    peers.retain(|p| p.did != did);
    save_peers(&peers)
}

/// Set the peer's **envelope (X25519) public key** (hex), then persist. Errors if no such peer.
pub fn set_peer_envelope_key(did: &str, pubkey_hex: &str) -> Result<(), String> {
    let mut peers = list_peers();
    match peers.iter_mut().find(|p| p.did == did) {
        Some(p) => p.envelope_pubkey_hex = Some(pubkey_hex.to_string()),
        None => return Err(format!("no peer with did {did}")),
    }
    save_peers(&peers)
}

/// Resolve `dids` to `(did, envelope_pubkey_hex)` pairs from `peers` — the parties whose envelope key is
/// known. Parties without a published key (or not peered) are simply omitted (the caller learns which keys
/// are still missing by comparing lengths). Pure; unit-tested.
pub fn resolve_envelope_keys(peers: &[SocialPeer], dids: &[String]) -> Vec<(String, String)> {
    dids.iter()
        .filter_map(|did| {
            find(peers, did)
                .and_then(|p| p.envelope_pubkey_hex.clone())
                .map(|pk| (did.clone(), pk))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Tests — PURE ONLY. These operate on a local `Vec<SocialPeer>` via the
// pure helpers and never touch the real filesystem / app dir.
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(did: &str, name: &str) -> SocialPeer {
        SocialPeer {
            did: did.to_string(),
            display_name: name.to_string(),
            wireguard_pubkey_hex: "aa".repeat(32),
            overlay_addr: "10.44.0.2".to_string(),
            endpoint: Some("203.0.113.5:51820".to_string()),
            relation_type: "collaboration".to_string(),
            added_at: 1_700_000_000,
            active: true,
            envelope_pubkey_hex: None,
        }
    }

    #[test]
    fn upsert_new_did_appends() {
        let mut peers = vec![peer("did:key:alice", "Alice")];
        upsert(&mut peers, peer("did:key:bob", "Bob"));

        assert_eq!(peers.len(), 2);
        assert_eq!(peers[0].did, "did:key:alice");
        assert_eq!(peers[1].did, "did:key:bob");
    }

    #[test]
    fn resolve_envelope_keys_returns_only_peers_with_a_published_key() {
        let mut alice = peer("did:key:alice", "Alice");
        alice.envelope_pubkey_hex = Some("ab".repeat(32));
        let bob = peer("did:key:bob", "Bob"); // no envelope key yet
        let peers = vec![alice, bob];
        let resolved = resolve_envelope_keys(
            &peers,
            &["did:key:alice".to_string(), "did:key:bob".to_string(), "did:key:carol".to_string()],
        );
        // Only Alice has a key; Bob (no key) and Carol (not a peer) are omitted.
        assert_eq!(resolved, vec![("did:key:alice".to_string(), "ab".repeat(32))]);
    }

    #[test]
    fn upsert_existing_did_replaces_in_place() {
        let mut peers = vec![peer("did:key:alice", "Alice"), peer("did:key:bob", "Bob")];

        let mut updated = peer("did:key:alice", "Alice (renamed)");
        updated.overlay_addr = "10.44.0.9".to_string();
        updated.active = false;
        upsert(&mut peers, updated);

        // Length unchanged: replacement, not append.
        assert_eq!(peers.len(), 2);
        // Position preserved.
        assert_eq!(peers[0].did, "did:key:alice");
        assert_eq!(peers[1].did, "did:key:bob");
        // Fields updated.
        assert_eq!(peers[0].display_name, "Alice (renamed)");
        assert_eq!(peers[0].overlay_addr, "10.44.0.9");
        assert!(!peers[0].active);
    }

    #[test]
    fn find_returns_peer_or_none() {
        let peers = vec![peer("did:key:alice", "Alice"), peer("did:key:bob", "Bob")];

        let found = find(&peers, "did:key:bob");
        assert!(found.is_some());
        assert_eq!(found.unwrap().display_name, "Bob");

        assert!(find(&peers, "did:key:carol").is_none());
    }
}
