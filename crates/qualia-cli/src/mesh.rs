//! `qualia-cli mesh-probe` — a two-machine SocialWebNet reachability probe.
//!
//! This is the manual, cross-host counterpart to the in-process loopback tests: it stands up a real
//! userspace-WireGuard tunnel between two *separate machines* over a real network, so a human can
//! confirm handshake + data (and NAT traversal) end-to-end. Only one AI instrument writes this; a
//! person runs the two halves.
//!
//! ## Zero key-copying: keys derive from a shared passphrase + role
//!
//! WireGuard is mutually authenticated — each side must know the other's static public key. To keep
//! the manual procedure to *one* shared secret, both halves derive **both** keypairs deterministically
//! from a passphrase and a role tag:
//!
//! ```text
//!   secret(role) = SHA-256("qualia-mesh-probe:v1:" || role || ":" || passphrase)
//! ```
//!
//! The `listen` side is role `a`; the `connect` side is role `b`. Each computes its own secret and the
//! peer's public key from the same passphrase, so nothing but the passphrase (and the listener's
//! address) needs to be shared.
//!
//! > The passphrase mode is for **testing reachability**, not production peering — production keys come
//! > from `NodeIdentity` / the connection-identifier exchange, never a shared phrase.
//!
//! ## Procedure
//!
//! On the machine that will listen (say its public IP is `A_IP`):
//! ```text
//!   qualia-cli mesh-probe listen --pass "our-test-2026" --port 51820
//! ```
//! On the other machine:
//! ```text
//!   qualia-cli mesh-probe connect --pass "our-test-2026" --peer A_IP:51820 --message "hello"
//! ```
//! The listener prints each decrypted inner packet; the connector reports handshake + send. For a
//! machine behind NAT, forward/allow UDP `51820` to the listener (or run the listener on the
//! public-IP side).

use std::net::{SocketAddr, ToSocketAddrs};
use std::time::{Duration, Instant};

use clap::Subcommand;
use sha2::{Digest, Sha256};

use qualia_core_db::p2p::mesh_datagram::{self, ports};
use qualia_core_db::p2p::wireguard_runtime::{TunnelEvent, WgTunnel};
use qualia_core_db::p2p::wireguard_userspace::WgKeypair;

#[derive(Subcommand, Debug)]
pub enum MeshAction {
    /// Print a fresh random WireGuard keypair (secret + public), for explicit-key setups.
    Keygen,
    /// Listen for a probe connection (role A). Prints decrypted inner packets until Ctrl-C.
    Listen {
        /// Shared passphrase both sides agree on (derives both keypairs).
        #[arg(long)]
        pass: String,
        /// UDP port to bind (default 51820, WireGuard's conventional port).
        #[arg(long, default_value_t = 51820)]
        port: u16,
        /// Seconds to run before exiting (0 = run until Ctrl-C).
        #[arg(long, default_value_t = 0)]
        seconds: u64,
    },
    /// Connect to a listening probe (role B), complete the handshake, and send a message.
    Connect {
        /// Shared passphrase both sides agree on (must match the listener's).
        #[arg(long)]
        pass: String,
        /// The listener's address, `host:port` (e.g. `203.0.113.5:51820`).
        #[arg(long)]
        peer: String,
        /// Message to send once the tunnel is up.
        #[arg(long, default_value = "hello from the SocialWebNet probe")]
        message: String,
        /// How many times to send the message (1s apart).
        #[arg(long, default_value_t = 1)]
        count: u32,
        /// Seconds to wait for the handshake before giving up.
        #[arg(long, default_value_t = 15)]
        timeout: u64,
    },
}

/// Derive a deterministic 32-byte WireGuard secret from `(role, passphrase)`.
fn derive_secret(role: &str, pass: &str) -> [u8; 32] {
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

pub fn run(action: &MeshAction) -> Result<(), Box<dyn std::error::Error>> {
    match action {
        MeshAction::Keygen => {
            let kp = qualia_core_db::p2p::wireguard_userspace::generate_keypair();
            println!("WireGuard keypair (random):");
            println!("  secret : {}", hex_lower(&kp.private_bytes()));
            println!("  public : {}", kp.public_hex());
            Ok(())
        }
        MeshAction::Listen { pass, port, seconds } => run_listen(pass, *port, *seconds),
        MeshAction::Connect { pass, peer, message, count, timeout } => {
            run_connect(pass, peer, message, *count, *timeout)
        }
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

fn run_listen(pass: &str, port: u16, seconds: u64) -> Result<(), Box<dyn std::error::Error>> {
    // Role A: my key = derive("a"); peer (the connector) = derive("b").
    let my_keys = WgKeypair::from_secret_bytes(derive_secret("a", pass));
    let peer_keys = WgKeypair::from_secret_bytes(derive_secret("b", pass));

    let bind: SocketAddr = format!("0.0.0.0:{port}").parse()?;
    let mut tunnel = WgTunnel::bind(&my_keys, peer_keys.public_bytes().into(), bind, 1)?;
    tunnel.set_read_timeout(Some(Duration::from_millis(500)))?;

    println!("SocialWebNet probe — LISTEN (role A)");
    println!("  my WG public : {}", my_keys.public_hex());
    println!("  bound        : {}", tunnel.local_addr()?);
    println!("  expecting peer WG public : {}", peer_keys.public_hex());
    if seconds == 0 {
        println!("  waiting for a connection… (Ctrl-C to stop)");
    } else {
        println!("  waiting up to {seconds}s for a connection…");
    }

    let deadline = if seconds == 0 {
        None
    } else {
        Some(Instant::now() + Duration::from_secs(seconds))
    };
    let mut announced_session = false;

    loop {
        if let Some(d) = deadline {
            if Instant::now() >= d {
                println!("  (timeout reached; exiting)");
                return Ok(());
            }
        }
        match tunnel.pump()? {
            TunnelEvent::InnerPacket(inner) => match mesh_datagram::decode_datagram(&inner) {
                Some(d) => println!(
                    "  ← received {} bytes on port {}: \"{}\"",
                    d.payload.len(),
                    d.dst_port,
                    String::from_utf8_lossy(&d.payload)
                ),
                None => println!(
                    "  ← received a {}-byte inner packet (not a UDP datagram)",
                    inner.len()
                ),
            },
            TunnelEvent::Progressed => {
                if !announced_session && tunnel.has_session() {
                    announced_session = true;
                    println!(
                        "  ✓ handshake complete with {:?} — tunnel is up",
                        tunnel.peer_endpoint().map(|e| e.to_string()).unwrap_or_default()
                    );
                }
            }
            TunnelEvent::Idle => {
                // Periodically drive timers (keepalives/rekey) while idle.
                let _ = tunnel.tick();
            }
        }
    }
}

fn run_connect(
    pass: &str,
    peer: &str,
    message: &str,
    count: u32,
    timeout: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // Role B: my key = derive("b"); peer (the listener) = derive("a").
    let my_keys = WgKeypair::from_secret_bytes(derive_secret("b", pass));
    let peer_keys = WgKeypair::from_secret_bytes(derive_secret("a", pass));

    let endpoint = peer
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| format!("could not resolve peer address '{peer}'"))?;

    let bind: SocketAddr = "0.0.0.0:0".parse()?;
    let mut tunnel = WgTunnel::bind(&my_keys, peer_keys.public_bytes().into(), bind, 2)?;
    tunnel.set_read_timeout(Some(Duration::from_millis(500)))?;
    tunnel.set_peer_endpoint(endpoint);

    println!("SocialWebNet probe — CONNECT (role B)");
    println!("  my WG public : {}", my_keys.public_hex());
    println!("  peer         : {endpoint} (WG public {})", peer_keys.public_hex());
    println!("  initiating handshake…");
    tunnel.initiate_handshake()?;

    // Drive the handshake to completion. `pump` processes the peer's response; `tick` drives
    // WireGuard's timers so a *lost* initiation is retransmitted (e.g. the listener wasn't ready
    // when our first init went out) — without it a single dropped init would hang until timeout.
    let deadline = Instant::now() + Duration::from_secs(timeout);
    while !tunnel.has_session() {
        if Instant::now() >= deadline {
            return Err(format!(
                "handshake did not complete within {timeout}s — check the peer address, that the \
                 listener is running with the same --pass, and that UDP is reachable (NAT/firewall)"
            )
            .into());
        }
        let _ = tunnel.pump()?;
        let _ = tunnel.tick()?;
    }
    println!("  ✓ handshake complete — tunnel is up");

    // Send the message `count` times. `send_packet` returns whether ciphertext actually went on the
    // wire (vs. being held pending a (re)handshake), so report honestly rather than assuming.
    for i in 1..=count {
        let packet = mesh_datagram::encode_datagram(ports::CHAT, ports::CHAT, message.as_bytes());
        let transmitted = tunnel.send_packet(&packet)?;
        if transmitted {
            println!("  → sent [{i}/{count}]: \"{message}\"");
        } else {
            println!("  → queued [{i}/{count}] (awaiting session): \"{message}\"");
        }
        // Pump briefly so any keepalive/response is processed before the next send.
        let until = Instant::now() + Duration::from_millis(900);
        while Instant::now() < until {
            let _ = tunnel.pump()?;
        }
    }
    println!(
        "  done. Delivery is best-effort UDP; the listener prints each datagram it decrypts."
    );
    Ok(())
}
