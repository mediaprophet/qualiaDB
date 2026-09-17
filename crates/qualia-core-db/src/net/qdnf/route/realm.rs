//! E08.5 — inter-realm policy authentication and default-route leak prevention.

use crate::net::qdnf::errors::QdnfError;

/// Authenticate an advertised realm against local membership.
///
/// Unsigned inter-realm adverts are [`QdnfError::Unauthorized`]. Intra-realm
/// adverts do not require `signed_ok`. `advert_realm` is a bit index 0..63.
pub fn authenticate_realm_policy(
    advert_realm: u8,
    local_realms: u64,
    signed_ok: bool,
) -> Result<(), QdnfError> {
    if advert_realm >= 64 {
        return Err(QdnfError::Range);
    }
    let bit = 1u64 << advert_realm;
    if local_realms & bit != 0 {
        return Ok(());
    }
    if !signed_ok {
        return Err(QdnfError::Unauthorized);
    }
    Ok(())
}

/// Per-realm default-route install table. Foreign-realm defaults are refused.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RealmRouteTable {
    defaults: [bool; 64],
}

impl RealmRouteTable {
    pub const fn new() -> Self {
        Self {
            defaults: [false; 64],
        }
    }

    /// True if any realm currently has an installed default.
    pub fn has_default_route(&self) -> bool {
        let mut i = 0usize;
        while i < 64 {
            if self.defaults[i] {
                return true;
            }
            i += 1;
        }
        false
    }

    pub fn has_default_in(&self, realm: u8) -> bool {
        if realm >= 64 {
            return false;
        }
        self.defaults[realm as usize]
    }

    /// Install a default only when `advert_realm == into_realm`.
    ///
    /// A route originated in realm A is never a default in realm B; `has_default_route`
    /// stays false after that attempt.
    pub fn install_default(
        &mut self,
        advert_realm: u8,
        into_realm: u8,
        signed_ok: bool,
    ) -> Result<(), QdnfError> {
        if into_realm >= 64 {
            return Err(QdnfError::Range);
        }
        let local = 1u64 << into_realm;
        authenticate_realm_policy(advert_realm, local, signed_ok)?;
        if advert_realm != into_realm {
            return Err(QdnfError::Denied);
        }
        self.defaults[into_realm as usize] = true;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsigned_inter_realm_advert_unauthorized() {
        let local_b = 1u64 << 1;
        assert_eq!(
            authenticate_realm_policy(0, local_b, false),
            Err(QdnfError::Unauthorized)
        );
        assert!(authenticate_realm_policy(0, local_b, true).is_ok());
        assert!(authenticate_realm_policy(1, local_b, false).is_ok());
    }

    #[test]
    fn signed_inter_realm_does_not_install_default_into_other_realm() {
        let mut table = RealmRouteTable::new();
        assert!(authenticate_realm_policy(0, 1u64 << 1, true).is_ok());
        assert_eq!(table.install_default(0, 1, true), Err(QdnfError::Denied));
        assert!(!table.has_default_route());
        assert!(!table.has_default_in(1));
        table.install_default(1, 1, false).unwrap();
        assert!(table.has_default_route());
        assert!(table.has_default_in(1));
        assert!(!table.has_default_in(0));
    }

    #[test]
    fn advert_realm_out_of_range() {
        assert_eq!(
            authenticate_realm_policy(64, u64::MAX, true),
            Err(QdnfError::Range)
        );
    }
}
