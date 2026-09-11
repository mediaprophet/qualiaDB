//! Application QPR example: two separately constructed peers over local IPC.
//!
//! This is the Qualia replacement for a libp2p Swarm listen/dial round trip.
//! The 64-byte vertical slice (`two_peer_ipc_exchange`) is a separate example,
//! not this application API.

use qualia_peer::{PeerHost, ScopeEpoch, AUTHORISED_ROUNDTRIP_CAP};

fn main() {
    let host = PeerHost::new(64 * 1024 * 1024).expect("cell budget");
    let n = host.exchange(b"hello-qpr").expect("thin wrapper");
    println!("exchange wrapper {n} bytes (libp2p not used)");

    let scope = ScopeEpoch { scope: 1, epoch: 1 };
    let (_a, _b) = host
        .pair(b"did:q42:a", b"did:q42:b", scope, 1280)
        .expect("pair with host cell_bytes");
    let mut app = [0x5Au8; AUTHORISED_ROUNDTRIP_CAP];
    app[0] = 0x51;
    let p = host
        .exchange_protected(&app)
        .expect("authorised protected exchange");
    println!("exchange_protected {p} bytes through public QPR");
}
