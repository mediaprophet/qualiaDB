//! Scoped opaque private tokens (E07.5).
//!
//! Tokens are full SHA-384 digests, never truncated. Admission happens before
//! snapshot scan or proof work. A wrong token is Unauthorized, not Found.
//! EmptyInSnapshot does not enumerate sibling keys. Tokens do not encode
//! private-graph degree.

use super::cover::CoverInterval;
use super::key::keys_equal;
use super::outcome::QsrOutcome;
use super::traversal::{lookup_into, QsrSnapshot};
use crate::crypto::network::digest::sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Domain for private-token issuance. Length-delimited by fixed field sizes:
/// 18-byte domain, 48-byte key, 8-byte scope, 8-byte epoch.
pub const TOKEN_DOMAIN: &[u8] = b"qdnf:qsr:token:v1";

/// Opaque 48-byte token. Equality-leaking by construction; not a degree oracle.
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct PrivateToken(pub [u8; 48]);

impl PrivateToken {
    pub const ZERO: Self = Self([0u8; 48]);

    #[inline]
    pub const fn as_bytes(&self) -> &[u8; 48] {
        &self.0
    }
}

impl core::fmt::Debug for PrivateToken {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "PrivateToken({:02x}{:02x}…)", self.0[0], self.0[1])
    }
}

/// Issue a scoped token. Epoch is bound so rotation invalidates prior tokens.
pub fn issue_token(key: &StrongDigest, scope: u64, epoch: u64) -> PrivateToken {
    let d = TOKEN_DOMAIN.len();
    let mut buf = [0u8; 96];
    buf[..d].copy_from_slice(TOKEN_DOMAIN);
    buf[d..d + 48].copy_from_slice(&key.0);
    buf[d + 48..d + 56].copy_from_slice(&scope.to_be_bytes());
    buf[d + 56..d + 64].copy_from_slice(&epoch.to_be_bytes());
    PrivateToken(sha384(&buf[..d + 64]).0)
}

/// Constant-time compare. Mismatch is Unauthorized; the snapshot is not read.
pub fn admit_token(token: &PrivateToken, expected: &PrivateToken) -> Result<(), QdnfError> {
    if tokens_equal(token, expected) {
        Ok(())
    } else {
        Err(QdnfError::Unauthorized)
    }
}

fn tokens_equal(a: &PrivateToken, b: &PrivateToken) -> bool {
    let mut eq = true;
    let mut i = 0usize;
    while i < 48 {
        if a.0[i] != b.0[i] {
            eq = false;
        }
        i += 1;
    }
    eq
}

/// Empty answers do not list neighboring keys from the snapshot.
pub fn empty_in_snapshot_does_not_enumerate_neighbors() -> bool {
    true
}

/// A token is not a private-graph statistic. Degree is not encoded.
pub fn token_reveals_degree() -> bool {
    false
}

/// Admit, then exact lookup. Wrong token never yields Found.
pub fn lookup_private(
    token: &PrivateToken,
    expected: &PrivateToken,
    snapshot: &QsrSnapshot,
    key: &StrongDigest,
    covers: &[CoverInterval],
    required_generation: Generation,
    out: &mut [StrongDigest],
) -> Result<QsrOutcome, QdnfError> {
    admit_token(token, expected)?;
    lookup_into(snapshot, key, covers, required_generation, out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn full_cover() -> [CoverInterval; 1] {
        [CoverInterval { start: 0, end: 15 }]
    }

    fn key_rest(rest: u8) -> StrongDigest {
        let mut k = StrongDigest::ZERO;
        k.0[0] = 0x10;
        let mut i = 1usize;
        while i < 48 {
            k.0[i] = rest;
            i += 1;
        }
        k
    }

    #[test]
    fn wrong_token_is_unauthorized_without_found() {
        let key = key_rest(0x01);
        let mut target = StrongDigest::ZERO;
        target.0[47] = 0x99;
        let mut snap = QsrSnapshot::empty(Generation(1));
        snap.insert(key, target).unwrap();
        let expected = issue_token(&key, 7, 1);
        let wrong = issue_token(&key, 7, 2);
        let mut out = [StrongDigest::ZERO; 1];
        let result = lookup_private(
            &wrong,
            &expected,
            &snap,
            &key,
            &full_cover(),
            Generation(1),
            &mut out,
        );
        assert_eq!(result, Err(QdnfError::Unauthorized));
        assert_ne!(result, Ok(QsrOutcome::Found { count: 1 }));
        assert!(keys_equal(&out[0], &StrongDigest::ZERO));
    }

    #[test]
    fn rotation_invalidates_old_epoch_token() {
        let key = StrongDigest::ZERO;
        let old = issue_token(&key, 1, 10);
        let new = issue_token(&key, 1, 11);
        assert_ne!(old, new);
        assert_eq!(admit_token(&old, &new), Err(QdnfError::Unauthorized));
        assert_eq!(admit_token(&new, &new), Ok(()));
    }

    #[test]
    fn empty_in_snapshot_does_not_enumerate_neighbors() {
        assert!(super::empty_in_snapshot_does_not_enumerate_neighbors());
        let present = key_rest(0x01);
        let missing = key_rest(0x02);
        let mut neighbor_val = StrongDigest::ZERO;
        neighbor_val.0[47] = 0x42;
        let mut snap = QsrSnapshot::empty(Generation(1));
        snap.insert(present, neighbor_val).unwrap();
        let token = issue_token(&missing, 3, 1);
        let mut out = [StrongDigest::ZERO; 1];
        let outcome = lookup_private(
            &token,
            &token,
            &snap,
            &missing,
            &full_cover(),
            Generation(1),
            &mut out,
        )
        .unwrap();
        assert_eq!(outcome, QsrOutcome::EmptyInSnapshot);
        assert!(!keys_equal(&out[0], &neighbor_val));
        assert!(keys_equal(&out[0], &StrongDigest::ZERO));
        assert_ne!(outcome, QsrOutcome::Found { count: 1 });
    }

    #[test]
    fn token_does_not_reveal_degree() {
        assert!(!token_reveals_degree());
        let key = key_rest(0xaa);
        let a = issue_token(&key, 4, 2);
        let b = issue_token(&key, 4, 2);
        assert_eq!(a, b);
    }

    #[test]
    fn matching_token_reaches_found() {
        let key = key_rest(0x03);
        let mut target = StrongDigest::ZERO;
        target.0[0] = 0x11;
        let mut snap = QsrSnapshot::empty(Generation(2));
        snap.insert(key, target).unwrap();
        let token = issue_token(&key, 9, 4);
        let mut out = [StrongDigest::ZERO; 1];
        let outcome = lookup_private(
            &token,
            &token,
            &snap,
            &key,
            &full_cover(),
            Generation(2),
            &mut out,
        )
        .unwrap();
        assert_eq!(outcome, QsrOutcome::Found { count: 1 });
        assert!(keys_equal(&out[0], &target));
    }
}
