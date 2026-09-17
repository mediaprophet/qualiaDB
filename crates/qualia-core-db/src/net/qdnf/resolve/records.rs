//! DNI / RAR / alias records. Compact hashes are lookup aids only.

use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{DniCoordinate, StrongDigest};

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResolveOutcome {
    Routes = 1,
    NoRoute = 2,
    Denied = 3,
    Ambiguous = 4,
    Conflict = 5,
    Unsupported = 6,
    BudgetExhausted = 7,
    Incomplete = 8,
}

#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RouteAdvert {
    pub target: StrongDigest,
    pub dni: DniCoordinate,
    pub expires_unix: u64,
    pub sequence: u64,
}

pub const MAX_CANDIDATES: usize = 64;
pub const MAX_VERIFICATIONS: usize = 16;
pub const MAX_RETURNED_ROUTES: usize = 8;
pub const MAX_DIAL_RACES: usize = 3;
pub const MAX_ALIASES: usize = 16;

pub fn select_routes(
    candidates: &[RouteAdvert],
    now_unix: u64,
    out: &mut [RouteAdvert],
) -> Result<(usize, ResolveOutcome), QdnfError> {
    if out.len() < 1 {
        return Err(QdnfError::Capacity);
    }
    if candidates.len() > MAX_CANDIDATES {
        return Err(QdnfError::Capacity);
    }
    let mut n = 0usize;
    let mut verified = 0usize;
    for c in candidates {
        if verified >= MAX_VERIFICATIONS {
            return Ok((n, ResolveOutcome::Incomplete));
        }
        verified += 1;
        if now_unix >= c.expires_unix {
            continue;
        }
        if n >= MAX_RETURNED_ROUTES || n >= out.len() {
            break;
        }
        out[n] = *c;
        n += 1;
    }
    if n == 0 {
        Ok((0, ResolveOutcome::NoRoute))
    } else {
        Ok((n, ResolveOutcome::Routes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expired_routes_are_not_returned() {
        let c = [RouteAdvert {
            target: StrongDigest::ZERO,
            dni: DniCoordinate::ZERO,
            expires_unix: 1,
            sequence: 1,
        }];
        let mut out = [c[0]; 8];
        let (n, outcome) = select_routes(&c, 10, &mut out).unwrap();
        assert_eq!(n, 0);
        assert_eq!(outcome, ResolveOutcome::NoRoute);
    }
}
