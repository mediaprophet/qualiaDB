//! Derived projections from authorised views only (E06.4, E11.3).
//!
//! A projection digest does not replace the original. Deltas are planned
//! only when `view_gen == live_gen`. Private cross-tenant deduplication is
//! [`QdnfError::Denied`]. Dedup is not a membership oracle.

use crate::crypto::network::digest::sha384;
use crate::net::peer::replication::originals::{projection_replaces_original, OriginalObject};
use crate::net::qdnf::errors::QdnfError;
use crate::net::qdnf::types::{Generation, StrongDigest};

/// Derived view of an original. `digest` is not the original identity.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Projection {
    pub from: StrongDigest,
    pub digest: StrongDigest,
}

/// Hash `proj_bytes` as a projection of `orig`. Does not overwrite `orig`.
///
/// [`QdnfError::Malformed`] if `orig.digest` is zero or `proj_bytes` is empty.
pub fn derive_projection(
    orig: &OriginalObject,
    proj_bytes: &[u8],
) -> Result<Projection, QdnfError> {
    if orig.digest == StrongDigest::ZERO || orig.len == 0 {
        return Err(QdnfError::Malformed);
    }
    if proj_bytes.is_empty() {
        return Err(QdnfError::Malformed);
    }
    let digest = sha384(proj_bytes);
    let _ = projection_replaces_original();
    Ok(Projection {
        from: orig.digest,
        digest,
    })
}

/// Plan a semantic delta. Cross-tenant is Denied. Unauthorised view is
/// Unauthorized. Prefer [`plan_delta_at`] with generations.
pub fn plan_delta(view_authorised: bool, cross_tenant: bool) -> Result<(), QdnfError> {
    let view_gen = if view_authorised {
        Generation(1)
    } else {
        Generation(0)
    };
    plan_delta_at(view_gen, Generation(1), cross_tenant)
}

/// Generation form: `view_gen == live_gen` else [`QdnfError::Unauthorized`].
/// `cross_tenant` true → [`QdnfError::Denied`] (no private cross-tenant dedup).
pub fn plan_delta_at(
    view_gen: Generation,
    live_gen: Generation,
    cross_tenant: bool,
) -> Result<(), QdnfError> {
    if cross_tenant {
        return Err(QdnfError::Denied);
    }
    if view_gen != live_gen {
        return Err(QdnfError::Unauthorized);
    }
    Ok(())
}

/// Deduplication of identical bytes is not a membership / completeness oracle.
#[inline]
pub fn membership_oracle_from_dedup() -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::peer::replication::originals::store_original;

    #[test]
    fn original_is_not_projection() {
        let orig_bytes = b"original-object-bytes";
        let proj_bytes = b"derived-projection";
        let orig = store_original(orig_bytes, 64).unwrap();
        let proj = derive_projection(&orig, proj_bytes).unwrap();
        assert_eq!(proj.from, orig.digest);
        assert_ne!(proj.digest, orig.digest);
        assert!(!projection_replaces_original());
        assert_ne!(proj.digest, proj.from);
    }

    #[test]
    fn unauthorized_view_is_unauthorized() {
        assert_eq!(plan_delta(false, false), Err(QdnfError::Unauthorized));
        assert_eq!(
            plan_delta_at(Generation(1), Generation(2), false),
            Err(QdnfError::Unauthorized)
        );
        plan_delta(true, false).unwrap();
        plan_delta_at(Generation(4), Generation(4), false).unwrap();
    }

    #[test]
    fn cross_tenant_is_denied() {
        assert_eq!(plan_delta(true, true), Err(QdnfError::Denied));
        assert_eq!(
            plan_delta_at(Generation(1), Generation(1), true),
            Err(QdnfError::Denied)
        );
        assert!(!membership_oracle_from_dedup());
    }

    #[test]
    fn empty_projection_is_malformed() {
        let orig = store_original(b"abc", 8).unwrap();
        assert_eq!(derive_projection(&orig, b""), Err(QdnfError::Malformed));
    }
}
