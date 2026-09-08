//! Two-peer native IPC example. No IP, DNS, or libp2p.
//!
//! This is the Qualia replacement for a libp2p Swarm listen/dial round trip.

use qualia_peer::PeerHost;

fn main() {
    let host = PeerHost::new(64 * 1024 * 1024).expect("cell budget");
    let n = host.exchange(b"qdnf-native").expect("native exchange");
    println!("native ipc exchanged {n} bytes (libp2p not used)");
}
