//! Access modality and substrate-tier gate logic (from `0.0.19-g1-access-tier`).
//!
//! Gates data by tier, fail-closed:
//! - non-permissive (open commons) is served to all;
//! - permissive (`wf:` / credential-gated / protected) is served only to a *verified*
//!   HumanCentric system, never to the traditional web.
//!
//! Pairs with [`super::credentials`] — the credential a request carries is what verifies a
//! HumanCentric system for the permissive tier.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessModality {
    HumanCentric,
    TraditionalWeb,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataTier {
    /// Open commons, served to all.
    NonPermissive,
    /// `wf:` / credential-gated / protected, served only to verified HumanCentric systems.
    Permissive,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessError {
    /// The HumanCentric system is unverified; access fails closed.
    UnverifiedHumanCentric,
    /// The access modality (e.g. TraditionalWeb) lacks clearance for this tier.
    InsufficientTier,
}

impl AccessModality {
    /// Whether this modality may access `tier`. Fails closed when requirements aren't met.
    pub fn can_access(&self, tier: DataTier, is_verified: bool) -> Result<(), AccessError> {
        match (self, tier) {
            // Open commons: served to all.
            (_, DataTier::NonPermissive) => Ok(()),
            // Permissive is never served to the traditional web.
            (AccessModality::TraditionalWeb, DataTier::Permissive) => {
                Err(AccessError::InsufficientTier)
            }
            // Permissive is served to HumanCentric only when verified.
            (AccessModality::HumanCentric, DataTier::Permissive) => {
                if is_verified {
                    Ok(())
                } else {
                    Err(AccessError::UnverifiedHumanCentric)
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn traditional_web_access() {
        let m = AccessModality::TraditionalWeb;
        assert_eq!(m.can_access(DataTier::NonPermissive, false), Ok(()));
        assert_eq!(m.can_access(DataTier::NonPermissive, true), Ok(()));
        // Permissive is failed closed for the traditional web, verified or not.
        assert_eq!(
            m.can_access(DataTier::Permissive, false),
            Err(AccessError::InsufficientTier)
        );
        assert_eq!(
            m.can_access(DataTier::Permissive, true),
            Err(AccessError::InsufficientTier)
        );
    }

    #[test]
    fn human_centric_access() {
        let m = AccessModality::HumanCentric;
        assert_eq!(m.can_access(DataTier::NonPermissive, false), Ok(()));
        // Permissive requires verification; fails closed when unverified.
        assert_eq!(m.can_access(DataTier::Permissive, true), Ok(()));
        assert_eq!(
            m.can_access(DataTier::Permissive, false),
            Err(AccessError::UnverifiedHumanCentric)
        );
    }
}
