//! Future seam: `qualia-crypto` (`crypto/` today). QPU stays fail-closed.

mod sha256;

pub use sha256::blake3;
pub use sha256::digest as sha256;
pub use sha256::sha512;
