//! Future seam: `qualia-crypto` (`crypto/` today). QPU stays fail-closed.

mod privacy;
mod sha256;

pub use privacy::gaussian_sigma;
pub use sha256::blake3;
pub use sha256::digest as sha256;
pub use sha256::sha512;
