//! Packet protection: directional keys, AEAD, packet numbers and replay (E02.3).
//!
//! Authenticate before updating the receive window or exposing plaintext.

use crate::crypto::network::aead::{decrypt_in_place, encrypt_in_place};
use crate::crypto::network::types::{AEAD_KEY_LEN, AEAD_TAG_LEN};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::session::packet::{
    nonce_from_packet_number, PacketNumber, PacketSpace, AEAD_NONCE_LEN,
};
use crate::net::qdnf::types::Generation;
use zeroize::Zeroize;

pub const PN_LEN: usize = 8;
pub const OVERHEAD: usize = PN_LEN + AEAD_TAG_LEN;

pub struct PacketProtection {
    send_key: [u8; AEAD_KEY_LEN],
    recv_key: [u8; AEAD_KEY_LEN],
    space: PacketSpace,
    generation: Generation,
}

impl PacketProtection {
    pub fn from_keys(
        send_key: [u8; AEAD_KEY_LEN],
        recv_key: [u8; AEAD_KEY_LEN],
        generation: Generation,
    ) -> Result<Self, QdnfError> {
        if send_key == [0u8; AEAD_KEY_LEN]
            || recv_key == [0u8; AEAD_KEY_LEN]
            || send_key == recv_key
        {
            return Err(QdnfError::CryptoFailure);
        }
        if generation == Generation::ZERO {
            return Err(QdnfError::Unauthorized);
        }
        Ok(Self {
            send_key,
            recv_key,
            space: PacketSpace::new(),
            generation,
        })
    }

    #[inline]
    pub const fn generation(&self) -> Generation {
        self.generation
    }

    /// Encrypt `plaintext` in place. `out` receives pn || ciphertext || tag.
    pub fn seal(
        &mut self,
        aad: &[u8],
        plaintext: &mut [u8],
        out: &mut [u8],
    ) -> Result<usize, QdnfError> {
        let need = plaintext
            .len()
            .checked_add(OVERHEAD)
            .ok_or(QdnfError::Range)?;
        if out.len() < need {
            return Err(QdnfError::Capacity);
        }
        let pn = self.space.allocate_send()?;
        let mut nonce = [0u8; AEAD_NONCE_LEN];
        nonce_from_packet_number(pn, &mut nonce);
        let mut tag = [0u8; AEAD_TAG_LEN];
        encrypt_in_place(&self.send_key, &nonce, aad, plaintext, &mut tag)?;
        out[..PN_LEN].copy_from_slice(&pn.0.to_be_bytes());
        out[PN_LEN..PN_LEN + plaintext.len()].copy_from_slice(plaintext);
        let tag_off = PN_LEN + plaintext.len();
        out[tag_off..tag_off + AEAD_TAG_LEN].copy_from_slice(&tag);
        Ok(need)
    }

    /// Decrypt into `out`. Replay/window updates happen only after AEAD success.
    pub fn open(&mut self, aad: &[u8], sealed: &[u8], out: &mut [u8]) -> Result<usize, QdnfError> {
        if sealed.len() < OVERHEAD {
            return Err(QdnfError::Truncated);
        }
        let body_len = sealed.len() - OVERHEAD;
        if out.len() < body_len {
            return Err(QdnfError::Capacity);
        }
        let pn_bytes: [u8; 8] = sealed[..PN_LEN].try_into().map_err(|_| QdnfError::Range)?;
        let pn = PacketNumber(u64::from_be_bytes(pn_bytes));
        let mut body = [0u8; 256];
        if body_len > body.len() {
            return Err(QdnfError::Capacity);
        }
        body[..body_len].copy_from_slice(&sealed[PN_LEN..PN_LEN + body_len]);
        let mut tag = [0u8; AEAD_TAG_LEN];
        tag.copy_from_slice(&sealed[PN_LEN + body_len..]);
        let mut nonce = [0u8; AEAD_NONCE_LEN];
        nonce_from_packet_number(pn, &mut nonce);
        decrypt_in_place(&self.recv_key, &nonce, aad, &mut body[..body_len], &tag)?;
        self.space.accept_receive(pn)?;
        out[..body_len].copy_from_slice(&body[..body_len]);
        Ok(body_len)
    }
}

impl Drop for PacketProtection {
    fn drop(&mut self) {
        self.send_key.zeroize();
        self.recv_key.zeroize();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pair() -> (PacketProtection, PacketProtection) {
        let a = PacketProtection::from_keys([1u8; 32], [2u8; 32], Generation(1)).unwrap();
        let b = PacketProtection::from_keys([2u8; 32], [1u8; 32], Generation(1)).unwrap();
        (a, b)
    }

    #[test]
    fn round_trip_hides_plaintext() {
        let (mut a, mut b) = pair();
        let app = b"hello-qpr";
        let mut pt = *app;
        let mut sealed = [0u8; 64];
        let n = a.seal(b"aad", &mut pt, &mut sealed).unwrap();
        assert_ne!(&sealed[PN_LEN..PN_LEN + app.len()], app.as_slice());
        let mut out = [0u8; 16];
        let got = b.open(b"aad", &sealed[..n], &mut out).unwrap();
        assert_eq!(&out[..got], app);
    }

    #[test]
    fn tamper_does_not_advance_replay_or_plaintext() {
        let (mut a, mut b) = pair();
        let mut pt = *b"hello-qpr";
        let mut sealed = [0u8; 64];
        let n = a.seal(b"aad", &mut pt, &mut sealed).unwrap();
        sealed[PN_LEN] ^= 0xff;
        let mut out = [0u8; 16];
        assert!(b.open(b"aad", &sealed[..n], &mut out).is_err());
        assert_ne!(&out[..9], b"hello-qpr");
        sealed[PN_LEN] ^= 0xff;
        let got = b.open(b"aad", &sealed[..n], &mut out).unwrap();
        assert_eq!(&out[..got], b"hello-qpr");
    }

    #[test]
    fn replay_is_rejected() {
        let (mut a, mut b) = pair();
        let mut pt = *b"hello-qpr";
        let mut sealed = [0u8; 64];
        let n = a.seal(b"aad", &mut pt, &mut sealed).unwrap();
        let mut out = [0u8; 16];
        assert!(b.open(b"aad", &sealed[..n], &mut out).is_ok());
        assert_eq!(
            b.open(b"aad", &sealed[..n], &mut out),
            Err(QdnfError::Replay)
        );
    }

    #[test]
    fn wrong_aad_yields_no_plaintext() {
        let (mut a, mut b) = pair();
        let mut pt = *b"hello-qpr";
        let mut sealed = [0u8; 64];
        let n = a.seal(b"aad-1", &mut pt, &mut sealed).unwrap();
        let mut out = [0u8; 16];
        assert!(b.open(b"aad-2", &sealed[..n], &mut out).is_err());
    }
}
