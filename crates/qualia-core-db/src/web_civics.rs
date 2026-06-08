use ed25519_dalek::VerifyingKey;
use sha2::{Digest, Sha256};
use std::net::Ipv6Addr;

/// Derives a Unique Local IPv6 Address (ULA) in the fd00::/8 block
/// from an Ed25519 Webizen Public Key.
/// This enforces Cryptokey Routing where the DID is mathematically
/// synonymous with the IPv6 address.
pub fn derive_webizen_ipv6(public_key: &VerifyingKey) -> Ipv6Addr {
    let mut hasher = Sha256::new();
    hasher.update(public_key.as_bytes());
    let result = hasher.finalize();

    // ULA addresses start with fd (1111 1101)
    let mut ipv6_bytes = [0u8; 16];
    ipv6_bytes[0] = 0xfd;

    // Copy 15 bytes of the hash into the remaining 15 bytes of the IPv6 address
    ipv6_bytes[1..16].copy_from_slice(&result[0..15]);

    Ipv6Addr::from(ipv6_bytes)
}
