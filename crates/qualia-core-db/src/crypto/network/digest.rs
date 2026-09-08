//! SHA-384 typed digest. Compact q_hash is never produced here.

use sha2::{Digest, Sha384};

use super::errors::CryptoError;
use super::types::SHA384_LEN;
use crate::net::qdnf::types::StrongDigest;

pub fn sha384(data: &[u8]) -> StrongDigest {
    let mut hasher = Sha384::new();
    hasher.update(data);
    let out = hasher.finalize();
    let mut d = StrongDigest::ZERO;
    d.0.copy_from_slice(&out);
    d
}

pub fn sha384_into(data: &[u8], out: &mut [u8]) -> Result<usize, CryptoError> {
    if out.len() < SHA384_LEN {
        return Err(CryptoError::Capacity);
    }
    let digest = sha384(data);
    out[..SHA384_LEN].copy_from_slice(&digest.0);
    Ok(SHA384_LEN)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_input_is_stable() {
        let a = sha384(b"");
        let b = sha384(b"");
        assert_eq!(a, b);
        assert_ne!(a, StrongDigest::ZERO);
    }

    #[test]
    fn short_output_is_capacity() {
        let mut buf = [0u8; 8];
        assert_eq!(sha384_into(b"x", &mut buf), Err(CryptoError::Capacity));
    }
}
