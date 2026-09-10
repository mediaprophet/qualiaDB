//! Internet mesh-probe connect path as a library (no `qualia-cli` / OpenSSL).
//!
//! Cursor cloud images may lack `libssl-dev`, so the connect half of the
//! two-host test must run from `qualia-core-db`. Listen still uses `qualia-cli`
//! on a reachable host. See `qdnf-imp/internet-two-host.md`.

#![cfg(not(target_arch = "wasm32"))]

use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use sha2::{Digest, Sha256};

use crate::net::qdnf::frame::{encode_frame, FrameHeader};
use crate::net::qdnf::registries::{FrameType, NextProtocol};
use crate::p2p::mesh_datagram::{self, ports};
use crate::p2p::wireguard_runtime::WgTunnel;
use crate::p2p::wireguard_userspace::WgKeypair;

/// Same domain as `qualia-cli mesh-probe` (`v1` role + passphrase).
pub fn derive_probe_secret(role: &str, pass: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"qualia-mesh-probe:v1:");
    hasher.update(role.as_bytes());
    hasher.update(b":");
    hasher.update(pass.as_bytes());
    let digest = hasher.finalize();
    let mut out = [0u8; 32];
    out.copy_from_slice(&digest);
    out
}

/// Result of one connect attempt. `handshake` is the only success bit that matters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectReport {
    pub handshake: bool,
    pub sent_qdnf: bool,
}

/// Connector role `b` toward a listener that used role `a` and the same `pass`.
pub fn connect_probe(
    pass: &str,
    peer: &str,
    qdnf: bool,
    timeout: Duration,
) -> Result<ConnectReport, String> {
    let my_keys = WgKeypair::from_secret_bytes(derive_probe_secret("b", pass));
    let peer_keys = WgKeypair::from_secret_bytes(derive_probe_secret("a", pass));
    let endpoint: SocketAddr = peer
        .to_socket_addrs()
        .map_err(|e| e.to_string())?
        .next()
        .ok_or_else(|| format!("could not resolve '{peer}'"))?;

    let bind: SocketAddr = "0.0.0.0:0"
        .parse::<SocketAddr>()
        .map_err(|e| e.to_string())?;
    let mut tunnel = WgTunnel::bind(&my_keys, peer_keys.public, bind, 2)?;
    tunnel.set_read_timeout(Some(Duration::from_millis(500)))?;
    tunnel.set_peer_endpoint(endpoint);
    tunnel.initiate_handshake()?;

    let deadline = Instant::now() + timeout;
    while !tunnel.has_session() {
        if Instant::now() >= deadline {
            return Ok(ConnectReport {
                handshake: false,
                sent_qdnf: false,
            });
        }
        let _ = tunnel.pump()?;
        let _ = tunnel.tick()?;
    }

    let mut sent_qdnf = false;
    if qdnf {
        let header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
        let mut wire = [0u8; 128];
        let n = encode_frame(&header, &[], &mut wire).map_err(|e| e.to_string())?;
        let packet = mesh_datagram::encode_datagram(ports::QDNF, ports::QDNF, &wire[..n]);
        sent_qdnf = tunnel.send_packet(&packet)?;
        let until = Instant::now() + Duration::from_millis(400);
        while Instant::now() < until {
            let _ = tunnel.pump()?;
        }
    }
    Ok(ConnectReport {
        handshake: true,
        sent_qdnf,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_secrets_differ_by_role() {
        let a = derive_probe_secret("a", "phrase");
        let b = derive_probe_secret("b", "phrase");
        assert_ne!(a, b);
        assert_eq!(a, derive_probe_secret("a", "phrase"));
    }

    /// Set `QDNF_LISTEN_ADDR` and `QDNF_MESH_PASS` to execute the internet connect half.
    /// Absent env: skip. Present env: handshake must succeed.
    #[test]
    fn internet_connect_if_env_set() {
        let Ok(peer) = std::env::var("QDNF_LISTEN_ADDR") else {
            return;
        };
        if peer.is_empty() {
            return;
        }
        let pass = std::env::var("QDNF_MESH_PASS").unwrap_or_default();
        assert!(
            !pass.is_empty(),
            "QDNF_LISTEN_ADDR is set; also set QDNF_MESH_PASS"
        );
        let report = connect_probe(&pass, &peer, true, Duration::from_secs(20))
            .expect("connect_probe");
        assert!(
            report.handshake,
            "internet WireGuard handshake failed toward {peer}"
        );
        assert!(report.sent_qdnf, "QDNF QFrame was not transmitted");
    }
}
