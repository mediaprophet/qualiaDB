//! Example 64-byte vertical slice. Not the application API.
//!
//! Applications should use `PeerHost::pair` / `PeerHost::exchange_protected`.

use qualia_peer::two_peer_ipc_exchange;

fn main() {
    let n = two_peer_ipc_exchange(b"qsync-hello").expect("64-byte demo");
    println!("vertical demo exchanged {n} bytes (not the application API)");
}
