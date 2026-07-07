//! Social ↔ mesh bridge — bring the SocialWebNet mesh up from identity + the peer store.
//!
//! [`crate::node_identity::NodeIdentity`] holds this node's WireGuard secret; the
//! [`crate::social_peers`] store holds accepted peers (their WireGuard public key, overlay
//! address, and last-known endpoint). This module joins the two: it reports which peers are
//! *dialable* (have the material to form a tunnel) for the UI, and — on native targets — builds a
//! live [`SocialWebNet`](qualia_core_db::p2p::social_webnet::SocialWebNet) with a tunnel per
//! reachable peer.
//!
//! The split mirrors the crate boundary: `qualia-core-db` owns the mesh *mechanism* (sockets,
//! `boringtun`), this module owns the *social binding* (identity + peer records → mesh). The pure
//! [`dialability`] report needs neither sockets nor `boringtun`, so it compiles everywhere
//! (including the `wasm32` studio build); [`build_node_mesh`] is native-only.

use crate::social_peers::SocialPeer;

/// Whether a peer record carries a syntactically valid WireGuard public key (64 hex chars).
///
/// This is the minimum needed to address the peer on the mesh; it does not prove reachability.
fn valid_wg_pubkey(hex_key: &str) -> bool {
    hex_key.len() == 64 && hex_key.bytes().all(|b| b.is_ascii_hexdigit())
}

/// A per-peer report of whether the mesh can bring up a tunnel to them, and what (if anything) is
/// missing. Drives the Connect/Directory UI's "can I reach this peer?" affordance.
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct PeerMeshReport {
    /// The peer's DID.
    pub did: String,
    /// Friendly label.
    pub display_name: String,
    /// The peering is switched on ([`SocialPeer::active`]).
    pub active: bool,
    /// The peer has a usable WireGuard public key.
    pub has_wg_key: bool,
    /// A transport endpoint is known, so we can dial immediately (vs. waiting for the peer to
    /// reach us first, then learning their endpoint by roaming).
    pub has_endpoint: bool,
    /// Overall: can a tunnel be brought up *now* (active + key + endpoint)?
    pub dialable_now: bool,
    /// Can a tunnel form at all once the peer initiates (active + key, endpoint learned by roaming)?
    pub reachable: bool,
    /// Human-readable note on what is missing, or empty when `dialable_now`.
    pub note: String,
}

/// Compute the dialability report for a set of peers — a pure, side-effect-free view usable on any
/// target (native or wasm).
pub fn dialability(peers: &[SocialPeer]) -> Vec<PeerMeshReport> {
    peers
        .iter()
        .map(|p| {
            let has_wg_key = valid_wg_pubkey(&p.wireguard_pubkey_hex);
            let has_endpoint = p
                .endpoint
                .as_deref()
                .map(|e| e.parse::<std::net::SocketAddr>().is_ok())
                .unwrap_or(false);
            let reachable = p.active && has_wg_key;
            let dialable_now = reachable && has_endpoint;
            let note = if !p.active {
                "peering is switched off".to_string()
            } else if !has_wg_key {
                "no valid WireGuard public key".to_string()
            } else if !has_endpoint {
                "endpoint unknown — will connect when the peer reaches us (roaming)".to_string()
            } else {
                String::new()
            };
            PeerMeshReport {
                did: p.did.clone(),
                display_name: p.display_name.clone(),
                active: p.active,
                has_wg_key,
                has_endpoint,
                dialable_now,
                reachable,
                note,
            }
        })
        .collect()
}

// ===========================================================================
// Native mesh construction.
// ===========================================================================

#[cfg(not(target_arch = "wasm32"))]
mod native {
    use super::*;
    use std::net::{IpAddr, SocketAddr};
    use std::time::Duration;

    use qualia_core_db::p2p::mesh_service::MeshService;
    use qualia_core_db::p2p::social_webnet::SocialWebNet;
    use qualia_core_db::p2p::wireguard_userspace::WgKeypair;

    use crate::node_identity::NodeIdentity;

    /// The outcome of adding one peer to the mesh.
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    pub struct PeerAddOutcome {
        pub did: String,
        /// A tunnel was created for this peer.
        pub added: bool,
        /// The local socket the tunnel bound (advertise this back so the peer can reach us).
        pub local_addr: Option<String>,
        /// The endpoint we will dial, if known now.
        pub endpoint: Option<String>,
        /// Why the peer was skipped, or empty when `added`.
        pub note: String,
    }

    /// Build a live [`SocialWebNet`] for this node, adding a tunnel for every *reachable* peer
    /// (active + valid WireGuard key). Inactive or keyless peers are skipped with a note. Peers
    /// whose endpoint is known are pre-pointed; the rest wait to learn it by roaming.
    ///
    /// Returns the mesh and a per-peer outcome list. Bringing up the handshake and pumping is the
    /// caller's job (the mesh is a passive, caller-driven state machine — see
    /// [`SocialWebNet::pump`]).
    pub fn build_node_mesh(
        identity: &NodeIdentity,
        peers: &[SocialPeer],
        bind_ip: IpAddr,
        read_timeout: Option<Duration>,
    ) -> Result<(SocialWebNet, Vec<PeerAddOutcome>), String> {
        let keys = WgKeypair::from_secret_bytes(identity.wg_secret);
        let mut mesh = SocialWebNet::new(keys, bind_ip, read_timeout);
        let mut outcomes = Vec::with_capacity(peers.len());

        for p in peers {
            if !p.active {
                outcomes.push(PeerAddOutcome {
                    did: p.did.clone(),
                    added: false,
                    local_addr: None,
                    endpoint: None,
                    note: "peering is switched off".into(),
                });
                continue;
            }
            let endpoint: Option<SocketAddr> =
                p.endpoint.as_deref().and_then(|e| e.parse().ok());
            match mesh.add_peer(&p.did, &p.wireguard_pubkey_hex, endpoint) {
                Ok(local) => outcomes.push(PeerAddOutcome {
                    did: p.did.clone(),
                    added: true,
                    local_addr: Some(local.to_string()),
                    endpoint: endpoint.map(|e| e.to_string()),
                    note: String::new(),
                }),
                Err(e) => outcomes.push(PeerAddOutcome {
                    did: p.did.clone(),
                    added: false,
                    local_addr: None,
                    endpoint: None,
                    note: e,
                }),
            }
        }
        Ok((mesh, outcomes))
    }

    /// Build the mesh from identity + peers and start it running on its own thread.
    ///
    /// Convenience over [`build_node_mesh`] + [`MeshService::spawn`]: returns the running service
    /// (drive it with `send`/`try_recv`/`initiate_handshake`) alongside the per-peer add outcomes.
    /// This is the top of the stack the desktop process holds to run the SocialWebNet.
    pub fn start_node_mesh_service(
        identity: &NodeIdentity,
        peers: &[SocialPeer],
        bind_ip: IpAddr,
        read_timeout: Option<Duration>,
    ) -> Result<(MeshService, Vec<PeerAddOutcome>), String> {
        let (mesh, outcomes) = build_node_mesh(identity, peers, bind_ip, read_timeout)?;
        Ok((MeshService::spawn(mesh), outcomes))
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub use native::{build_node_mesh, start_node_mesh_service, PeerAddOutcome};

#[cfg(test)]
mod tests {
    use super::*;

    fn peer(did: &str, wg: &str, endpoint: Option<&str>, active: bool) -> SocialPeer {
        SocialPeer {
            did: did.into(),
            display_name: did.into(),
            wireguard_pubkey_hex: wg.into(),
            overlay_addr: "fd00::1".into(),
            endpoint: endpoint.map(|e| e.into()),
            relation_type: "spc:Collaboration".into(),
            added_at: 0,
            active,
            envelope_pubkey_hex: None,
        }
    }

    const GOOD_KEY: &str = "aa11bb22cc33dd44ee55ff6677889900aa11bb22cc33dd44ee55ff6677889900";

    #[test]
    fn dialability_classifies_peers() {
        let peers = vec![
            peer("did:wf:now", GOOD_KEY, Some("203.0.113.5:51820"), true), // dialable now
            peer("did:wf:roam", GOOD_KEY, None, true),                     // reachable, wait for roaming
            peer("did:wf:off", GOOD_KEY, Some("203.0.113.6:51820"), false), // inactive
            peer("did:wf:nokey", "zz", Some("203.0.113.7:51820"), true),    // bad key
        ];
        let r = dialability(&peers);

        let now = &r[0];
        assert!(now.dialable_now && now.reachable && now.has_endpoint && now.note.is_empty());

        let roam = &r[1];
        assert!(roam.reachable && !roam.dialable_now && !roam.has_endpoint);
        assert!(roam.note.contains("roaming"));

        let off = &r[2];
        assert!(!off.reachable && !off.dialable_now);
        assert!(off.note.contains("switched off"));

        let nokey = &r[3];
        assert!(!nokey.has_wg_key && !nokey.reachable);
        assert!(nokey.note.contains("WireGuard"));
    }

    /// End-to-end social binding: two nodes' identities + peer records (pointing at each other)
    /// build two live meshes that complete a handshake and carry an inner IPv6 packet over
    /// loopback. Proves an *accepted peer record* is sufficient to form a real tunnel.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn accepted_peer_records_form_a_real_tunnel() {
        use crate::node_identity::NodeIdentity;
        use qualia_core_db::p2p::social_webnet::MeshPacket;
        use qualia_core_db::p2p::wireguard_userspace::WgKeypair;
        use std::time::Duration;

        // Two node identities (in-memory; distinct wg secrets).
        let a_id = NodeIdentity { ed25519_secret: [1u8; 32], wg_secret: [2u8; 32] };
        let b_id = NodeIdentity { ed25519_secret: [3u8; 32], wg_secret: [4u8; 32] };
        let a_wg = WgKeypair::from_secret_bytes(a_id.wg_secret).public_hex();
        let b_wg = WgKeypair::from_secret_bytes(b_id.wg_secret).public_hex();

        // Each node's peer store holds the other, keyed by DID, with the other's real WG pubkey.
        let a_peers = vec![peer("did:wf:bob", &b_wg, None, true)];
        let b_peers = vec![peer("did:wf:alice", &a_wg, None, true)];

        let to = Some(Duration::from_millis(300));
        let ip = "127.0.0.1".parse().unwrap();
        let (mut a, a_out) = build_node_mesh(&a_id, &a_peers, ip, to).unwrap();
        let (mut b, b_out) = build_node_mesh(&b_id, &b_peers, ip, to).unwrap();
        assert!(a_out[0].added && b_out[0].added, "both peers added to their meshes");

        // Exchange the bound endpoints (coordination-plane step) and connect.
        let a_local: std::net::SocketAddr = a_out[0].local_addr.clone().unwrap().parse().unwrap();
        let b_local: std::net::SocketAddr = b_out[0].local_addr.clone().unwrap().parse().unwrap();
        a.set_peer_endpoint("did:wf:bob", b_local).unwrap();
        b.set_peer_endpoint("did:wf:alice", a_local).unwrap();

        a.initiate_handshake("did:wf:bob").unwrap();
        for _ in 0..20 {
            if a.has_session("did:wf:bob") {
                break;
            }
            let _ = b.pump_all();
            let _ = a.pump_all();
        }
        assert!(a.has_session("did:wf:bob"), "handshake completed from the peer records");
        let _ = b.pump_all();

        // Carry a packet A→B, addressed by the peer's DID.
        let payload = {
            let body = b"from an accepted peer record";
            let mut p = vec![0u8; 40 + body.len()];
            p[0] = 0x60;
            p[4..6].copy_from_slice(&(body.len() as u16).to_be_bytes());
            p[6] = 17;
            p[7] = 64;
            p[8] = 0xfd;
            p[23] = 0x01;
            p[24] = 0xfd;
            p[39] = 0x02;
            p[40..].copy_from_slice(body);
            p
        };
        assert!(a.send_to("did:wf:bob", &payload).unwrap());

        let mut got: Option<MeshPacket> = None;
        for _ in 0..10 {
            for evt in b.pump_all() {
                if let Ok(pkt) = evt {
                    got = Some(pkt);
                }
            }
            if got.is_some() {
                break;
            }
        }
        let pkt = got.expect("B received the packet");
        assert_eq!(pkt.peer_id, "did:wf:alice");
        assert_eq!(pkt.inner, payload);
    }

    /// The running-service convenience: build from identity + peers and get a live `MeshService`
    /// with the peer already added.
    #[cfg(not(target_arch = "wasm32"))]
    #[test]
    fn start_node_mesh_service_runs_with_the_peer() {
        use crate::node_identity::NodeIdentity;
        use crate::social_mesh::start_node_mesh_service;
        use std::time::Duration;

        let id = NodeIdentity { ed25519_secret: [5u8; 32], wg_secret: [6u8; 32] };
        let peers = vec![peer("did:wf:peer", GOOD_KEY, None, true)];
        let (svc, outcomes) = start_node_mesh_service(
            &id,
            &peers,
            "127.0.0.1".parse().unwrap(),
            Some(Duration::from_millis(50)),
        )
        .unwrap();

        assert!(outcomes[0].added, "peer added to the running mesh");
        assert_eq!(svc.peers().unwrap(), vec!["did:wf:peer".to_string()]);
        // Clean shutdown (also exercised by Drop).
        drop(svc);
    }
}
