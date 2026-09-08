//! Reachability cookies for QLink anti-amplification.
//!
//! A cookie proves **reachability of an observed locator only**. It is not a
//! handshake share, not a membership grant, and not application admission.

use crate::crypto::network::kdf::hmac_sha384;
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::ObservedLocator;

pub const COOKIE_LEN: usize = 16;
pub const MAX_PENDING: usize = 32;
pub const MAX_PER_LOCATOR: usize = 2;
/// Replies per cookie before [`QdnfError::Denied`].
pub const REPLY_CAP: u8 = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ReachabilityCookie {
    pub bytes: [u8; COOKIE_LEN],
    pub locator: ObservedLocator,
    pub expiry_unix: u64,
    replies: u8,
}

pub struct CookieJar {
    slots: [Option<ReachabilityCookie>; MAX_PENDING],
}

impl CookieJar {
    pub const fn new() -> Self {
        Self {
            slots: [None; MAX_PENDING],
        }
    }

    /// Issue a 16-byte HMAC cookie bound to `locator`. Does not open a session.
    pub fn issue(
        &mut self,
        locator: ObservedLocator,
        now_unix: u64,
        ttl_secs: u64,
        secret: &[u8],
    ) -> Result<ReachabilityCookie, QdnfError> {
        let expiry_unix = now_unix.checked_add(ttl_secs).ok_or(QdnfError::Range)?;
        if self.live_for_locator(&locator, now_unix) >= MAX_PER_LOCATOR {
            return Err(QdnfError::Capacity);
        }
        let slot = self.free_slot(now_unix).ok_or(QdnfError::Capacity)?;
        let bytes = mint_cookie(&locator, expiry_unix, secret)?;
        let cookie = ReachabilityCookie {
            bytes,
            locator,
            expiry_unix,
            replies: 0,
        };
        self.slots[slot] = Some(cookie);
        Ok(cookie)
    }

    /// Record a reply from the same observed locator. Still not membership.
    pub fn accept_reply(
        &mut self,
        cookie: &[u8; COOKIE_LEN],
        locator: &ObservedLocator,
        now_unix: u64,
    ) -> Result<(), QdnfError> {
        let stored = match self.find_mut(cookie) {
            Some(slot) => slot,
            None => return Err(QdnfError::Unauthorized),
        };
        if now_unix >= stored.expiry_unix {
            return Err(QdnfError::Expired);
        }
        if stored.locator != *locator {
            return Err(QdnfError::Unauthorized);
        }
        if stored.replies >= REPLY_CAP {
            return Err(QdnfError::Denied);
        }
        stored.replies = stored.replies.saturating_add(1);
        Ok(())
    }

    pub fn grants_membership() -> bool {
        false
    }

    pub fn grants_application() -> bool {
        false
    }

    fn live_for_locator(&self, locator: &ObservedLocator, now_unix: u64) -> usize {
        let mut n = 0usize;
        let mut i = 0usize;
        while i < MAX_PENDING {
            if let Some(c) = self.slots[i] {
                if c.locator == *locator && now_unix < c.expiry_unix {
                    n += 1;
                }
            }
            i += 1;
        }
        n
    }

    fn free_slot(&self, now_unix: u64) -> Option<usize> {
        let mut expired = None;
        let mut i = 0usize;
        while i < MAX_PENDING {
            match self.slots[i] {
                None => return Some(i),
                Some(c) if now_unix >= c.expiry_unix => {
                    if expired.is_none() {
                        expired = Some(i);
                    }
                }
                Some(_) => {}
            }
            i += 1;
        }
        expired
    }

    fn find_mut(&mut self, cookie: &[u8; COOKIE_LEN]) -> Option<&mut ReachabilityCookie> {
        self.slots
            .iter_mut()
            .find_map(|slot| slot.as_mut().filter(|c| cookie_eq(&c.bytes, cookie)))
    }
}

impl Default for CookieJar {
    fn default() -> Self {
        Self::new()
    }
}

fn mint_cookie(
    locator: &ObservedLocator,
    expiry_unix: u64,
    secret: &[u8],
) -> Result<[u8; COOKIE_LEN], QdnfError> {
    let loc = locator.as_slice();
    let mut info = [0u8; 40];
    let n = loc.len();
    info[..n].copy_from_slice(loc);
    info[n..n + 8].copy_from_slice(&expiry_unix.to_be_bytes());
    let mut mac = [0u8; 48];
    hmac_sha384(secret, &info[..n + 8], &mut mac).map_err(|_| QdnfError::CryptoFailure)?;
    let mut out = [0u8; COOKIE_LEN];
    out.copy_from_slice(&mac[..COOKIE_LEN]);
    Ok(out)
}

fn cookie_eq(a: &[u8; COOKIE_LEN], b: &[u8; COOKIE_LEN]) -> bool {
    let mut diff = 0u8;
    let mut i = 0usize;
    while i < COOKIE_LEN {
        diff |= a[i] ^ b[i];
        i += 1;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECRET: &[u8] = b"qdnf-reachability-cookie-secret";

    fn loc(byte: u8) -> ObservedLocator {
        ObservedLocator::from_slice(&[byte]).unwrap()
    }

    #[test]
    fn issue_then_accept_same_locator() {
        let mut jar = CookieJar::new();
        let locator = loc(1);
        let cookie = jar.issue(locator, 1_000, 30, SECRET).unwrap();
        assert_eq!(jar.accept_reply(&cookie.bytes, &locator, 1_000), Ok(()));
    }

    #[test]
    fn wrong_locator_is_unauthorized() {
        let mut jar = CookieJar::new();
        let cookie = jar.issue(loc(1), 1_000, 30, SECRET).unwrap();
        assert_eq!(
            jar.accept_reply(&cookie.bytes, &loc(2), 1_000),
            Err(QdnfError::Unauthorized)
        );
    }

    #[test]
    fn expired_cookie_is_expired() {
        let mut jar = CookieJar::new();
        let locator = loc(1);
        let cookie = jar.issue(locator, 1_000, 10, SECRET).unwrap();
        assert_eq!(
            jar.accept_reply(&cookie.bytes, &locator, cookie.expiry_unix),
            Err(QdnfError::Expired)
        );
    }

    #[test]
    fn fourth_reply_is_denied() {
        let mut jar = CookieJar::new();
        let locator = loc(1);
        let cookie = jar.issue(locator, 1_000, 30, SECRET).unwrap();
        for _ in 0..REPLY_CAP {
            assert_eq!(jar.accept_reply(&cookie.bytes, &locator, 1_000), Ok(()));
        }
        assert_eq!(
            jar.accept_reply(&cookie.bytes, &locator, 1_000),
            Err(QdnfError::Denied)
        );
    }

    #[test]
    fn third_pending_same_locator_is_capacity() {
        let mut jar = CookieJar::new();
        let locator = loc(7);
        jar.issue(locator, 1_000, 30, SECRET).unwrap();
        jar.issue(locator, 1_000, 31, SECRET).unwrap();
        assert_eq!(
            jar.issue(locator, 1_000, 32, SECRET),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn cookie_does_not_grant_membership_or_application() {
        assert!(!CookieJar::grants_membership());
        assert!(!CookieJar::grants_application());
    }

    #[test]
    fn thirty_third_distinct_locator_is_capacity() {
        let mut jar = CookieJar::new();
        for i in 0..MAX_PENDING {
            let locator = loc(i as u8 + 1);
            jar.issue(locator, 1_000, 30, SECRET).unwrap();
        }
        assert_eq!(
            jar.issue(loc(0xFF), 1_000, 30, SECRET),
            Err(QdnfError::Capacity)
        );
    }

    #[test]
    fn unknown_cookie_is_unauthorized() {
        let mut jar = CookieJar::new();
        assert_eq!(
            jar.accept_reply(&[0u8; COOKIE_LEN], &loc(1), 1_000),
            Err(QdnfError::Unauthorized)
        );
    }
}
