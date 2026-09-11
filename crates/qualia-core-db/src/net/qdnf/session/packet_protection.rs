//! Packet protection: directional keys, AEAD, packet numbers and replay (E02.3).
//!
//! Authenticate before updating the receive window or exposing plaintext.
//! `install_update` retains the immediately previous generation (keys +
//! packet space) so in-flight packets can open during overlap (E02.4 / E09.6).

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
/// Bounded stack scratch for `open`. Caller still supplies `out`.
pub const MAX_PROTECTED_BODY: usize = 4096;

struct KeySpace {
    send_key: [u8; AEAD_KEY_LEN],
    recv_key: [u8; AEAD_KEY_LEN],
    space: PacketSpace,
    generation: Generation,
}

impl KeySpace {
    fn empty() -> Self {
        Self {
            send_key: [0u8; AEAD_KEY_LEN],
            recv_key: [0u8; AEAD_KEY_LEN],
            space: PacketSpace::new(),
            generation: Generation::ZERO,
        }
    }

    fn occupied(&self) -> bool {
        self.generation != Generation::ZERO
    }

    fn erase(&mut self) {
        self.send_key.zeroize();
        self.recv_key.zeroize();
        self.space = PacketSpace::new();
        self.generation = Generation::ZERO;
    }
}

pub struct PacketProtection {
    current: KeySpace,
    previous: KeySpace,
}

impl PacketProtection {
    pub fn from_keys(
        send_key: [u8; AEAD_KEY_LEN],
        recv_key: [u8; AEAD_KEY_LEN],
        generation: Generation,
    ) -> Result<Self, QdnfError> {
        reject_key_pair(&send_key, &recv_key)?;
        if generation == Generation::ZERO {
            return Err(QdnfError::Unauthorized);
        }
        Ok(Self {
            current: KeySpace {
                send_key,
                recv_key,
                space: PacketSpace::new(),
                generation,
            },
            previous: KeySpace::empty(),
        })
    }

    #[inline]
    pub const fn generation(&self) -> Generation {
        self.current.generation
    }

    /// Encrypt `plaintext` in place. `out` receives pn || ciphertext || tag.
    pub fn seal(
        &mut self,
        aad: &[u8],
        plaintext: &mut [u8],
        out: &mut [u8],
    ) -> Result<usize, QdnfError> {
        if plaintext.len() > MAX_PROTECTED_BODY {
            return Err(QdnfError::Capacity);
        }
        let need = plaintext
            .len()
            .checked_add(OVERHEAD)
            .ok_or(QdnfError::Range)?;
        if out.len() < need {
            return Err(QdnfError::Capacity);
        }
        let pn = self.current.space.allocate_send()?;
        let mut nonce = [0u8; AEAD_NONCE_LEN];
        nonce_from_packet_number(pn, &mut nonce);
        let mut tag = [0u8; AEAD_TAG_LEN];
        encrypt_in_place(&self.current.send_key, &nonce, aad, plaintext, &mut tag)?;
        out[..PN_LEN].copy_from_slice(&pn.0.to_be_bytes());
        out[PN_LEN..PN_LEN + plaintext.len()].copy_from_slice(plaintext);
        let tag_off = PN_LEN + plaintext.len();
        out[tag_off..tag_off + AEAD_TAG_LEN].copy_from_slice(&tag);
        Ok(need)
    }

    /// Decrypt into `out`. Replay/window updates happen only after AEAD success.
    /// Tries current recv key first; on AEAD failure only, tries previous.
    pub fn open(&mut self, aad: &[u8], sealed: &[u8], out: &mut [u8]) -> Result<usize, QdnfError> {
        if sealed.len() < OVERHEAD {
            return Err(QdnfError::Truncated);
        }
        let body_len = sealed.len() - OVERHEAD;
        if body_len > MAX_PROTECTED_BODY {
            return Err(QdnfError::Capacity);
        }
        if out.len() < body_len {
            return Err(QdnfError::Capacity);
        }
        let pn_bytes: [u8; 8] = sealed[..PN_LEN].try_into().map_err(|_| QdnfError::Range)?;
        let pn = PacketNumber(u64::from_be_bytes(pn_bytes));
        let mut body = [0u8; MAX_PROTECTED_BODY];
        body[..body_len].copy_from_slice(&sealed[PN_LEN..PN_LEN + body_len]);
        let mut tag = [0u8; AEAD_TAG_LEN];
        tag.copy_from_slice(&sealed[PN_LEN + body_len..]);
        match open_one(
            &self.current.recv_key,
            &mut self.current.space,
            aad,
            pn,
            &mut body[..body_len],
            &tag,
            out,
        ) {
            Ok(n) => Ok(n),
            Err(QdnfError::CryptoFailure) if self.previous.occupied() => {
                body[..body_len].copy_from_slice(&sealed[PN_LEN..PN_LEN + body_len]);
                open_one(
                    &self.previous.recv_key,
                    &mut self.previous.space,
                    aad,
                    pn,
                    &mut body[..body_len],
                    &tag,
                    out,
                )
            }
            Err(e) => Err(e),
        }
    }

    /// Move current keys+space into previous (erasing any older previous) and
    /// install new keys with a fresh packet-number space. Generations must
    /// strictly increase. Nonce reuse is forbidden: spaces are not shared.
    pub fn install_update(
        &mut self,
        send_key: [u8; AEAD_KEY_LEN],
        recv_key: [u8; AEAD_KEY_LEN],
        generation: Generation,
    ) -> Result<(), QdnfError> {
        reject_key_pair(&send_key, &recv_key)?;
        if generation == Generation::ZERO || generation.0 <= self.current.generation.0 {
            return Err(QdnfError::StaleGeneration);
        }
        self.previous.erase();
        self.previous.send_key = self.current.send_key;
        self.previous.recv_key = self.current.recv_key;
        self.previous.space = self.current.space;
        self.previous.generation = self.current.generation;
        self.current.send_key = send_key;
        self.current.recv_key = recv_key;
        self.current.space = PacketSpace::new();
        self.current.generation = generation;
        Ok(())
    }
}

fn reject_key_pair(
    send_key: &[u8; AEAD_KEY_LEN],
    recv_key: &[u8; AEAD_KEY_LEN],
) -> Result<(), QdnfError> {
    if *send_key == [0u8; AEAD_KEY_LEN]
        || *recv_key == [0u8; AEAD_KEY_LEN]
        || send_key == recv_key
    {
        return Err(QdnfError::CryptoFailure);
    }
    Ok(())
}

fn open_one(
    recv_key: &[u8; AEAD_KEY_LEN],
    space: &mut PacketSpace,
    aad: &[u8],
    pn: PacketNumber,
    body: &mut [u8],
    tag: &[u8; AEAD_TAG_LEN],
    out: &mut [u8],
) -> Result<usize, QdnfError> {
    let mut nonce = [0u8; AEAD_NONCE_LEN];
    nonce_from_packet_number(pn, &mut nonce);
    decrypt_in_place(recv_key, &nonce, aad, body, tag)?;
    space.accept_receive(pn)?;
    out[..body.len()].copy_from_slice(body);
    Ok(body.len())
}

impl Drop for PacketProtection {
    fn drop(&mut self) {
        self.current.erase();
        self.previous.erase();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::qdnf::session::rekey::MAX_OLD_KEYS;

    fn pair() -> (PacketProtection, PacketProtection) {
        let a = PacketProtection::from_keys([1u8; 32], [2u8; 32], Generation(1)).unwrap();
        let b = PacketProtection::from_keys([2u8; 32], [1u8; 32], Generation(1)).unwrap();
        (a, b)
    }

    fn pn_of(sealed: &[u8]) -> u64 {
        u64::from_be_bytes(sealed[..PN_LEN].try_into().unwrap())
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

    #[test]
    fn install_update_uses_new_keys_and_fresh_packet_space() {
        let (mut a, mut b) = pair();
        let mut pt = *b"hello-qpr";
        let mut sealed_old = [0u8; 64];
        let n_old = a.seal(b"aad", &mut pt, &mut sealed_old).unwrap();
        a.install_update([3u8; 32], [4u8; 32], Generation(2))
            .unwrap();
        b.install_update([4u8; 32], [3u8; 32], Generation(2))
            .unwrap();
        let mut out = [0u8; 16];
        let got_old = b.open(b"aad", &sealed_old[..n_old], &mut out).unwrap();
        assert_eq!(&out[..got_old], b"hello-qpr");
        let mut pt2 = *b"hello-qpr";
        let mut sealed = [0u8; 64];
        let n2 = a.seal(b"aad", &mut pt2, &mut sealed).unwrap();
        assert_eq!(&sealed[..PN_LEN], &[0, 0, 0, 0, 0, 0, 0, 0]);
        let got = b.open(b"aad", &sealed[..n2], &mut out).unwrap();
        assert_eq!(&out[..got], b"hello-qpr");
        assert_eq!(a.generation(), Generation(2));
        assert_eq!(
            a.install_update([5u8; 32], [6u8; 32], Generation(2)),
            Err(QdnfError::StaleGeneration)
        );
    }

    #[test]
    fn open_rejects_body_over_max_protected_body() {
        let (_a, mut b) = pair();
        let sealed = [0u8; MAX_PROTECTED_BODY + OVERHEAD + 1];
        let mut out = [0u8; MAX_PROTECTED_BODY + 1];
        assert_eq!(
            b.open(b"aad", &sealed, &mut out),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn round_trip_4096_body() {
        let (mut a, mut b) = pair();
        let mut original = [0u8; MAX_PROTECTED_BODY];
        let mut i = 0usize;
        while i < MAX_PROTECTED_BODY {
            original[i] = (i % 251) as u8;
            i += 1;
        }
        let mut pt = original;
        let mut sealed = [0u8; MAX_PROTECTED_BODY + OVERHEAD];
        let n = a.seal(b"aad", &mut pt, &mut sealed).unwrap();
        assert_eq!(n, MAX_PROTECTED_BODY + OVERHEAD);
        assert_ne!(
            &sealed[PN_LEN..PN_LEN + MAX_PROTECTED_BODY],
            original.as_slice()
        );
        let mut out = [0u8; MAX_PROTECTED_BODY];
        let got = b.open(b"aad", &sealed[..n], &mut out).unwrap();
        assert_eq!(got, MAX_PROTECTED_BODY);
        assert_eq!(&out[..got], original.as_slice());
    }

    #[test]
    fn previous_generation_in_flight_opens_during_overlap() {
        let (mut a, mut b) = pair();
        let mut pt = *b"hello-qpr";
        let mut sealed_old = [0u8; 64];
        let n_old = a.seal(b"aad", &mut pt, &mut sealed_old).unwrap();
        a.install_update([3u8; 32], [4u8; 32], Generation(2))
            .unwrap();
        b.install_update([4u8; 32], [3u8; 32], Generation(2))
            .unwrap();
        let mut out = [0u8; 16];
        let got = b.open(b"aad", &sealed_old[..n_old], &mut out).unwrap();
        assert_eq!(&out[..got], b"hello-qpr");
        let mut gen = 2u64;
        let mut k = 5u8;
        while gen < 2 + MAX_OLD_KEYS as u64 {
            gen += 1;
            a.install_update([k; 32], [k.wrapping_add(1); 32], Generation(gen))
                .unwrap();
            b.install_update([k.wrapping_add(1); 32], [k; 32], Generation(gen))
                .unwrap();
            k = k.wrapping_add(2);
        }
        assert_eq!(
            b.open(b"aad", &sealed_old[..n_old], &mut out),
            Err(QdnfError::CryptoFailure)
        );
    }

    #[test]
    fn install_update_starts_new_pn_space() {
        let (mut a, mut b) = pair();
        let mut first = *b"hello-qpr";
        let mut sealed1 = [0u8; 64];
        let n1 = a.seal(b"aad", &mut first, &mut sealed1).unwrap();
        let pn_gen1 = pn_of(&sealed1);
        assert_eq!(pn_gen1, 0);
        a.install_update([3u8; 32], [4u8; 32], Generation(2))
            .unwrap();
        b.install_update([4u8; 32], [3u8; 32], Generation(2))
            .unwrap();
        let mut second = *b"hello-qpr";
        let mut sealed2 = [0u8; 64];
        let n2 = a.seal(b"aad", &mut second, &mut sealed2).unwrap();
        assert_eq!(pn_of(&sealed2), pn_gen1);
        assert_ne!(&sealed1[..n1], &sealed2[..n2]);
        let mut out = [0u8; 16];
        let got2 = b.open(b"aad", &sealed2[..n2], &mut out).unwrap();
        assert_eq!(&out[..got2], b"hello-qpr");
        let got1 = b.open(b"aad", &sealed1[..n1], &mut out).unwrap();
        assert_eq!(&out[..got1], b"hello-qpr");
    }
}
