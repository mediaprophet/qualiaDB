// Userspace WireGuard core — turnkey, no-admin, no `wg` CLI.
//
// This module replaces shelling out to the `wg`/`wg-quick` command-line tools with a
// fully in-process WireGuard implementation built on `boringtun` 0.7.1 (Cloudflare's
// portable, pure-Rust WireGuard). Nothing here spawns a subprocess, opens a kernel TUN
// device, or requires elevated privileges: a `boringtun::noise::Tunn` is a pure state
// machine that turns plaintext IP packets into encrypted WireGuard datagrams and back.
// The caller owns the UDP socket and the virtual interface; this core owns the crypto
// and the Noise_IKpsk2 handshake.
//
// The whole file is native-only: `boringtun` pulls in `ring` and OS entropy and does not
// build for `wasm32`. WASM peers reach the network through a relay, not this path.
#![cfg(not(target_arch = "wasm32"))]

use boringtun::noise::Tunn;
use boringtun::x25519::{PublicKey, StaticSecret};

/// A WireGuard static keypair (Curve25519).
///
/// `private` is the peer's long-term secret; `public` is what you publish so other peers
/// can address you. Both are the `x25519_dalek` types that `boringtun` itself consumes, so
/// they hand straight to [`new_tunnel`]/[`Tunn::new`] with no conversion.
pub struct WgKeypair {
    pub private: StaticSecret,
    pub public: PublicKey,
}

impl WgKeypair {
    /// Reconstruct a keypair from a 32-byte Curve25519 secret (e.g. loaded from the vault).
    /// The public key is derived deterministically from the secret.
    pub fn from_secret_bytes(secret: [u8; 32]) -> WgKeypair {
        let private = StaticSecret::from(secret);
        let public = PublicKey::from(&private);
        WgKeypair { private, public }
    }

    /// The 32 raw bytes of the secret key (for serialization into the key vault).
    pub fn private_bytes(&self) -> [u8; 32] {
        self.private.to_bytes()
    }

    /// The 32 raw bytes of the public key.
    pub fn public_bytes(&self) -> [u8; 32] {
        self.public.to_bytes()
    }

    /// This keypair's public key rendered as lowercase hex (see [`public_key_hex`]).
    pub fn public_hex(&self) -> String {
        public_key_hex(&self.public)
    }
}

/// Generate a fresh WireGuard keypair using the OS CSPRNG.
///
/// We fill 32 bytes via `rand` (the workspace's `rand = 0.10`, backed by the OS entropy
/// source) and build the `StaticSecret` from them, then derive the public key. This is
/// equivalent to `StaticSecret::random_from_rng(OsRng)` but avoids coupling to a specific
/// `rand_core` trait version — `x25519-dalek` 2 speaks `rand_core` 0.6 while the crate's
/// `rand` is 0.9-era, and the two RNG traits do not unify. Clamping is applied by
/// `x25519-dalek` at key-agreement time, exactly as in `random_from_rng`.
pub fn generate_keypair() -> WgKeypair {
    let mut secret = [0u8; 32];
    rand::fill(&mut secret[..]);
    WgKeypair::from_secret_bytes(secret)
}

/// Render a WireGuard public key as lowercase hex.
///
/// WireGuard's own config files use base64, but hex is what the rest of this codebase
/// uses for key identifiers, so we keep it consistent here. The 32-byte key becomes a
/// 64-character string.
pub fn public_key_hex(pk: &PublicKey) -> String {
    hex::encode(pk.as_bytes())
}

/// Parse a 32-byte public key from a 64-character hex string.
pub fn public_key_from_hex(s: &str) -> Result<PublicKey, String> {
    let bytes = hex::decode(s.trim()).map_err(|e| format!("invalid public-key hex: {e}"))?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| format!("public key must be 32 bytes, got {}", bytes.len()))?;
    Ok(PublicKey::from(arr))
}

/// Build a `boringtun` tunnel state machine for one peer.
///
/// A `Tunn` is a point-to-point WireGuard connection: `mine` is our static keypair,
/// `peer_public` is the remote peer's static public key, and `index` is a local session
/// index (any `u32`; `boringtun` shifts it into the WireGuard sender-index space). This
/// core deliberately keeps the tunnel plain — no preshared key, no persistent keepalive,
/// and no shared rate limiter — so callers get a minimal, predictable state machine:
///
/// * `preshared_key = None`  — optional psk2 layer left off; add later if a peer requires it.
/// * `persistent_keepalive = None` — the caller drives keepalives/timers explicitly.
/// * `rate_limiter = None` — `Tunn` builds its own default under-load limiter.
///
/// `Tunn::new` in 0.7.1 is infallible (returns `Self`), but we keep a `Result` signature so
/// the public surface stays stable if a future boringtun revision makes construction fallible
/// or we add validation (e.g. rejecting an all-zero peer key) here.
pub fn new_tunnel(
    mine: &WgKeypair,
    peer_public: PublicKey,
    index: u32,
) -> Result<Tunn, String> {
    // `StaticSecret` is not `Copy` and `Tunn::new` takes it by value, so hand it a clone
    // built from our stored secret bytes; `mine` keeps ownership of its own key.
    let static_private = StaticSecret::from(mine.private.to_bytes());
    let tunn = Tunn::new(
        static_private,
        peer_public,
        None, // preshared_key
        None, // persistent_keepalive (seconds)
        index,
        None, // rate_limiter — Tunn constructs a default
    );
    Ok(tunn)
}

#[cfg(test)]
mod tests {
    use super::*;
    use boringtun::noise::TunnResult;

    /// Scratch buffer large enough for any WireGuard control/data frame.
    const BUF: usize = 2048;

    /// Build a minimal *valid* IPv4 packet carrying `payload`.
    ///
    /// This matters: `Tunn::decapsulate` runs `validate_decapsulated_packet`, which checks
    /// the IP version nibble and that the header's total-length field does not exceed the
    /// buffer. A raw `b"hello"` is **not** a valid IP packet and would decapsulate to
    /// `TunnResult::Err(InvalidPacket)`, so the end-to-end data assertion must use a real
    /// IPv4 frame. We hand-roll a 20-byte header (version 4, IHL 5) + payload and set the
    /// total-length field so validation passes and the returned slice equals the input.
    fn make_ipv4_packet(payload: &[u8]) -> Vec<u8> {
        let total_len = 20 + payload.len();
        let mut pkt = vec![0u8; total_len];
        pkt[0] = 0x45; // IPv4, IHL = 5 (20-byte header)
        pkt[2..4].copy_from_slice(&(total_len as u16).to_be_bytes()); // total length (big-endian)
        pkt[8] = 64; // TTL
        pkt[9] = 17; // protocol = UDP (arbitrary; not inspected)
        pkt[12..16].copy_from_slice(&[192, 168, 0, 1]); // src IP
        pkt[16..20].copy_from_slice(&[192, 168, 0, 2]); // dst IP
        pkt[20..].copy_from_slice(payload);
        pkt
    }

    /// Two peers, entirely in memory, no sockets: prove a full WireGuard handshake and one
    /// data packet complete. This is the acceptance test for "userspace WireGuard works with
    /// zero external systems".
    #[test]
    fn two_peer_handshake_and_data() {
        // --- key setup -------------------------------------------------------------------
        let a_keys = generate_keypair();
        let b_keys = generate_keypair();

        // Hex round-trips and public keys are distinct.
        let a_pub_hex = public_key_hex(&a_keys.public);
        assert_eq!(a_pub_hex.len(), 64, "public key hex must be 64 chars");
        let a_pub_back = public_key_from_hex(&a_pub_hex).expect("hex round-trip");
        assert_eq!(a_pub_back.as_bytes(), a_keys.public.as_bytes());
        assert_ne!(
            a_keys.public.as_bytes(),
            b_keys.public.as_bytes(),
            "two fresh keypairs must differ"
        );

        // A knows B's public key; B knows A's. Session indices are arbitrary but distinct.
        let mut a = new_tunnel(&a_keys, b_keys.public, 1).expect("build A tunnel");
        let mut b = new_tunnel(&b_keys, a_keys.public, 2).expect("build B tunnel");

        // --- drive the handshake by hand -------------------------------------------------
        // A initiates. Then we shuttle each side's WriteToNetwork output to the other side's
        // decapsulate, bounded to a handful of iterations. In practice WireGuard needs:
        //   A --init-->  B
        //   A <--resp--  B
        //   A --keepalive(data)--> B
        // i.e. two shuttled packets after the init. The loop is defensive: it stops as soon
        // as neither side has more handshake traffic to send.
        let mut a_buf = [0u8; BUF];
        let mut in_flight: Vec<u8> = match a.encapsulate(&[], &mut a_buf) {
            TunnResult::WriteToNetwork(pkt) => pkt.to_vec(),
            other => panic!("A did not produce a handshake init: {other:?}"),
        };

        // `send_to_a` flips which tunnel receives the next in-flight packet each round.
        let mut send_to_a = false; // next packet goes to B first
        let mut handshake_done = false;

        for _round in 0..10 {
            let mut out = [0u8; BUF];
            let (recv, _peer_label) = if send_to_a {
                (&mut a, "A")
            } else {
                (&mut b, "B")
            };

            let result = recv.decapsulate(None, &in_flight, &mut out);
            match result {
                TunnResult::WriteToNetwork(pkt) => {
                    // The receiver produced a reply (handshake response or keepalive) that
                    // must be delivered to the other side on the next round.
                    in_flight = pkt.to_vec();
                    send_to_a = !send_to_a;
                }
                TunnResult::Done => {
                    // No more handshake traffic to shuttle. If both sides now hold a live
                    // session, the handshake has completed.
                    handshake_done = true;
                    break;
                }
                TunnResult::WriteToTunnelV4(_, _) | TunnResult::WriteToTunnelV6(_, _) => {
                    // Unexpected during the handshake phase (no data sent yet), but harmless.
                    handshake_done = true;
                    break;
                }
                TunnResult::Err(e) => panic!("handshake decapsulate error: {e:?}"),
            }
        }

        assert!(
            handshake_done,
            "handshake did not converge within the bounded loop"
        );

        // --- prove a data packet flows A -> B --------------------------------------------
        // After the handshake, A should have an established session and be able to send real
        // data. Give the encapsulate a couple of tries in case a queued keepalive comes out
        // first (WireGuard may emit control traffic before the first data frame).
        let plaintext = make_ipv4_packet(b"hello");
        let mut sent: Option<Vec<u8>> = None;
        for _ in 0..4 {
            let mut enc = [0u8; BUF];
            match a.encapsulate(&plaintext, &mut enc) {
                TunnResult::WriteToNetwork(pkt) => {
                    // Could be the data packet, or a fresh handshake init if no session yet.
                    // Try to decapsulate on B; if it yields our plaintext we're done.
                    let candidate = pkt.to_vec();
                    let mut dec = [0u8; BUF];
                    match b.decapsulate(None, &candidate, &mut dec) {
                        TunnResult::WriteToTunnelV4(data, _addr) => {
                            sent = Some(data.to_vec());
                            break;
                        }
                        TunnResult::WriteToNetwork(reply) => {
                            // B answered with a handshake response/keepalive; feed it back to
                            // A so the session establishes, then retry the data send.
                            let reply = reply.to_vec();
                            let mut back = [0u8; BUF];
                            let _ = a.decapsulate(None, &reply, &mut back);
                        }
                        TunnResult::Done => {}
                        other => panic!("unexpected B decapsulate during data phase: {other:?}"),
                    }
                }
                TunnResult::Done => {}
                other => panic!("unexpected A encapsulate during data phase: {other:?}"),
            }
        }

        let received = sent.expect("B never received the data packet from A");
        assert_eq!(
            received, plaintext,
            "decapsulated packet must equal the plaintext A sent"
        );
    }

    #[test]
    fn keypair_serialization_round_trip() {
        let kp = generate_keypair();
        let sk = kp.private_bytes();
        let rebuilt = WgKeypair::from_secret_bytes(sk);
        assert_eq!(rebuilt.public_bytes(), kp.public_bytes());
        assert_eq!(rebuilt.public_hex(), kp.public_hex());
    }
}
