//! Length-delimited transcript construction (cryptographic profile §5).
//!
//! Each item: u16be(label_len) || label || u32be(value_len) || value
//! Digest: SHA-384("QPR-TRANSCRIPT-PQ-1" || ordered_items) for qpr-pq-1.

use sha2::{Digest, Sha384};

use super::errors::CryptoError;
use crate::net::qdnf::types::StrongDigest;

pub const MAX_TRANSCRIPT: usize = 4096;
const DOMAIN: &[u8] = b"QPR-TRANSCRIPT-PQ-1";

#[derive(Clone)]
pub struct Transcript {
    buf: [u8; MAX_TRANSCRIPT],
    len: usize,
}

impl Transcript {
    pub const fn new() -> Self {
        Self {
            buf: [0u8; MAX_TRANSCRIPT],
            len: 0,
        }
    }

    pub fn append(&mut self, label: &[u8], value: &[u8]) -> Result<(), CryptoError> {
        let need = 2usize
            .checked_add(label.len())
            .and_then(|n| n.checked_add(4))
            .and_then(|n| n.checked_add(value.len()))
            .ok_or(CryptoError::Range)?;
        if self.len.checked_add(need).ok_or(CryptoError::Range)? > MAX_TRANSCRIPT {
            return Err(CryptoError::Capacity);
        }
        let label_len: u16 = label.len().try_into().map_err(|_| CryptoError::Range)?;
        let value_len: u32 = value.len().try_into().map_err(|_| CryptoError::Range)?;
        let mut i = self.len;
        self.buf[i..i + 2].copy_from_slice(&label_len.to_be_bytes());
        i += 2;
        self.buf[i..i + label.len()].copy_from_slice(label);
        i += label.len();
        self.buf[i..i + 4].copy_from_slice(&value_len.to_be_bytes());
        i += 4;
        self.buf[i..i + value.len()].copy_from_slice(value);
        i += value.len();
        self.len = i;
        Ok(())
    }

    pub fn digest(&self) -> StrongDigest {
        let mut hasher = Sha384::new();
        hasher.update(DOMAIN);
        hasher.update(&self.buf[..self.len]);
        let out = hasher.finalize();
        let mut d = StrongDigest::ZERO;
        d.0.copy_from_slice(&out);
        d
    }

    pub fn as_bytes(&self) -> &[u8] {
        &self.buf[..self.len]
    }
}

impl Default for Transcript {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn length_delimited_prevents_ambiguity() {
        let mut a = Transcript::new();
        a.append(b"ab", b"cd").unwrap();
        let mut b = Transcript::new();
        b.append(b"a", b"bcd").unwrap();
        assert_ne!(a.digest(), b.digest());
    }
}
