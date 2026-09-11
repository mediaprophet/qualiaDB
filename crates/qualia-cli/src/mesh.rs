//! `qualia-cli mesh-probe` — a two-machine SocialWebNet reachability probe.
//!
//! Cross-host counterpart to the in-process loopback tests: userspace WireGuard over
//! real UDP. This is the labelled **internet transition** path, not Native Independent
//! Ethernet. Barriers, NAT class, and the grok-bot listen job are in
//! `docs/manuals/standards/qualia-decentralized-network-fabric/qdnf-imp/internet-two-host.md`.
//!
//! ## Zero key-copying: keys derive from a shared passphrase + role
//!
//! ```text
//!   secret(role) = SHA-256("qualia-mesh-probe:v1:" || role || ":" || passphrase)
//! ```
//!
//! Listen is role `a`; connect is role `b`.
//!
//! ## Internet procedure (one side must be a reachable UDP listener)
//!
//! Listener (public IP or port-forward — typically grok-bot / desktop / VPS):
//! ```text
//!   qualia-cli mesh-probe observe --port 51820
//!   qualia-cli mesh-probe listen --pass PHRASE --port 51820 --seconds 300
//! ```
//! Connector (Cursor cloud agent on address-dependent SNAT — connect-only):
//! ```text
//!   qualia-cli mesh-probe connect --pass PHRASE --peer LISTENER_IP:51820 --qdnf
//! ```
//!
//! STUN-learned addresses from an address-dependent NAT must not be given to the peer
//! as a listen locator. Cursor cloud `ports` forwards TCP to the operator desktop; it
//! is not a public WireGuard UDP endpoint.

use std::net::{SocketAddr, ToSocketAddrs, UdpSocket};
use std::time::{Duration, Instant};

use clap::Subcommand;
use sha2::{Digest, Sha256};

use qualia_core_db::net::qdnf::frame::{encode_frame, FrameHeader, MAGIC};
use qualia_core_db::net::qdnf::registries::{FrameType, NextProtocol};
use qualia_core_db::p2p::mesh_datagram::{self, ports};
use qualia_core_db::p2p::stun_observe::{
    observe_mapping, recommend_role, ProbeRole, MappingClass,
};
use qualia_core_db::p2p::wireguard_runtime::{TunnelEvent, WgTunnel};
use qualia_core_db::p2p::wireguard_userspace::WgKeypair;

const STUN_A: (&str, u16) = ("stun.l.google.com", 3478);
const STUN_B: (&str, u16) = ("stun.cloudflare.com", 3478);

#[derive(Subcommand, Debug)]
pub enum MeshAction {
    /// Print a fresh random WireGuard keypair (secret + public), for explicit-key setups.
    Keygen,
    /// Classify NAT mapping with two STUN servers on one UDP socket. Does not handshake.
    Observe {
        /// UDP port to bind (0 = OS-chosen).
        #[arg(long, default_value_t = 51820)]
        port: u16,
    },
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
        /// Message to send once the tunnel is up (chat port; ignored when `--qdnf`).
        #[arg(long, default_value = "hello from the SocialWebNet probe")]
        message: String,
        /// How many times to send the message (1s apart).
        #[arg(long, default_value_t = 1)]
        count: u32,
        /// Seconds to wait for the handshake before giving up.
        #[arg(long, default_value_t = 15)]
        timeout: u64,
        /// Send a QDNF QFrame on overlay port 6423 instead of a chat datagram.
        #[arg(long, default_value_t = false)]
        qdnf: bool,
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
        MeshAction::Observe { port } => run_observe(*port),
        MeshAction::Listen {
            pass,
            port,
            seconds,
        } => run_listen(pass, *port, *seconds),
        MeshAction::Connect {
            pass,
            peer,
            message,
            count,
            timeout,
            qdnf,
        } => run_connect(pass, peer, message, *count, *timeout, *qdnf),
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
    print_stun_on_socket(tunnel.udp_socket());
    tunnel.set_read_timeout(Some(Duration::from_millis(500)))?;
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
                Some(d) if d.dst_port == ports::QDNF => {
                    let magic_ok = d.payload.starts_with(&MAGIC);
                    println!(
                        "  ← QDNF overlay {} bytes magic_ok={magic_ok}",
                        d.payload.len()
                    );
                }
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
                        tunnel
                            .peer_endpoint()
                            .map(|e| e.to_string())
                            .unwrap_or_default()
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
    qdnf: bool,
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
    println!(
        "  peer         : {endpoint} (WG public {})",
        peer_keys.public_hex()
    );
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
        let packet = if qdnf {
            let header = FrameHeader::new(FrameType::DiscoveryBeacon, NextProtocol::QLink);
            let mut wire = [0u8; 128];
            let n = encode_frame(&header, &[], &mut wire)?;
            mesh_datagram::encode_datagram(ports::QDNF, ports::QDNF, &wire[..n])
        } else {
            mesh_datagram::encode_datagram(ports::CHAT, ports::CHAT, message.as_bytes())
        };
        let transmitted = tunnel.send_packet(&packet)?;
        if transmitted {
            if qdnf {
                println!("  → sent [{i}/{count}] QDNF DiscoveryBeacon on overlay {}", ports::QDNF);
            } else {
                println!("  → sent [{i}/{count}]: \"{message}\"");
            }
        } else {
            println!("  → queued [{i}/{count}] (awaiting session)");
        }
        // Pump briefly so any keepalive/response is processed before the next send.
        let until = Instant::now() + Duration::from_millis(900);
        while Instant::now() < until {
            let _ = tunnel.pump()?;
        }
    }
    println!("  done. Delivery is best-effort UDP; the listener prints each datagram it decrypts.");
    Ok(())
}

fn resolve_ipv4(host: &str, port: u16) -> Result<SocketAddr, Box<dyn std::error::Error>> {
    let addr = (host, port)
        .to_socket_addrs()?
        .find(|a| a.is_ipv4())
        .ok_or_else(|| format!("no IPv4 for {host}:{port}"))?;
    Ok(addr)
}

fn print_report(
    report: &qualia_core_db::p2p::stun_observe::ObserveReport,
) {
    let role = recommend_role(report.class);
    println!("SOCIALWEBNET-INTERNET-OFFER v1");
    println!("  local_bind     : {}", report.local);
    println!("  srflx_stun_a   : {}", report.sample_a);
    println!("  srflx_stun_b   : {}", report.sample_b);
    println!(
        "  mapping        : {}",
        match report.class {
            MappingClass::EndpointIndependent => "endpoint-independent",
            MappingClass::AddressDependent => "address-dependent",
        }
    );
    println!(
        "  role           : {}",
        match role {
            ProbeRole::ConnectOnly => "connect-only (do not publish STUN as a listen locator)",
        }
    );
    println!(
        "  listener_viable: not proven — only claim listen if you have a public IP or UDP port-forward"
    );
    println!(
        "  handshake      : internet_two_host_handshake_executed=false until a remote peer answers"
    );
}

fn print_stun_on_socket(sock: &UdpSocket) {
    match (resolve_ipv4(STUN_A.0, STUN_A.1), resolve_ipv4(STUN_B.0, STUN_B.1)) {
        (Ok(a), Ok(b)) => match observe_mapping(sock, a, b, Duration::from_secs(3)) {
            Ok(report) => print_report(&report),
            Err(e) => println!("  stun observe failed: {e}"),
        },
        (a, b) => println!("  stun resolve failed: {a:?} {b:?}"),
    }
}

fn run_observe(port: u16) -> Result<(), Box<dyn std::error::Error>> {
    let bind = SocketAddr::from(([0, 0, 0, 0], port));
    let sock = UdpSocket::bind(bind)?;
    println!("SocialWebNet probe — OBSERVE");
    println!("  bound : {}", sock.local_addr()?);
    print_stun_on_socket(&sock);
    Ok(())
}
