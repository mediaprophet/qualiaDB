//! Deterministic local dependency resolution (SI-05).
//!
//! Catalog records are the only source. No network is consulted.

use crate::semantic_instruments::errors::InstrumentError;
use crate::semantic_instruments::resolve::dependency::*;

/// Plan a lock for `release_id` from `requirements` against a local `catalog`.
///
/// Requirements are cloned and sorted by `id`. Optional misses are omitted;
/// required misses fail closed. Entries are sorted by `id` before return.
pub fn plan_resolution(
    release_id: &str,
    requirements: &[DependencyReq],
    catalog: &[CatalogRecord],
) -> Result<DependencyLock, InstrumentError> {
    let prepared = prepare_requirements(release_id, requirements)?;
    let mut entries = Vec::with_capacity(prepared.len());
    for req in &prepared {
        if let Some(entry) = resolve_one(req, catalog)? {
            entries.push(entry);
        }
    }
    entries.sort_by(|a, b| a.id.cmp(&b.id));
    Ok(DependencyLock {
        release_id: release_id.to_string(),
        entries,
    })
}

fn prepare_requirements(
    release_id: &str,
    requirements: &[DependencyReq],
) -> Result<Vec<DependencyReq>, InstrumentError> {
    let mut working = requirements.to_vec();
    working.sort_by(|a, b| a.id.cmp(&b.id));
    let mut out: Vec<DependencyReq> = Vec::with_capacity(working.len());
    for req in working {
        if req.id == release_id {
            return Err(InstrumentError::DependencyCycle);
        }
        if let Some(prev) = out.last() {
            if prev.id == req.id {
                if prev != &req {
                    return Err(InstrumentError::DependencyCycle);
                }
                continue;
            }
        }
        out.push(req);
    }
    Ok(out)
}

fn resolve_one(
    req: &DependencyReq,
    catalog: &[CatalogRecord],
) -> Result<Option<LockEntry>, InstrumentError> {
    let mut best: Option<&CatalogRecord> = None;
    for rec in catalog {
        if rec.id != req.id {
            continue;
        }
        if !version_matches(&req.version_constraint, &rec.version) {
            continue;
        }
        if !req.expected_digest.is_empty() && rec.digest != req.expected_digest {
            continue;
        }
        if !rec.available_offline || rec.revoked {
            continue;
        }
        match best {
            None => best = Some(rec),
            Some(cur) => {
                if (&rec.version, &rec.locator) < (&cur.version, &cur.locator) {
                    best = Some(rec);
                }
            }
        }
    }
    if let Some(rec) = best {
        require_digest(req, rec)?;
        return Ok(Some(LockEntry {
            id: rec.id.clone(),
            version: rec.version.clone(),
            digest: rec.digest.clone(),
            locator: rec.locator.clone(),
        }));
    }
    if !req.required {
        return Ok(None);
    }
    Err(diagnose_required(req, catalog))
}

fn diagnose_required(req: &DependencyReq, catalog: &[CatalogRecord]) -> InstrumentError {
    let mut any_id = false;
    let mut any_online = false;
    let mut all_revoked = true;
    let mut version_hit = false;
    let mut digest_hit = false;
    let mut online_revoked = false;
    let mut offline_live = false;

    for rec in catalog {
        if rec.id != req.id {
            continue;
        }
        any_id = true;
        if rec.available_offline {
            any_online = true;
        }
        if !rec.revoked {
            all_revoked = false;
        }
        if !version_matches(&req.version_constraint, &rec.version) {
            continue;
        }
        version_hit = true;
        if !req.expected_digest.is_empty() && rec.digest != req.expected_digest {
            continue;
        }
        digest_hit = true;
        match (rec.available_offline, rec.revoked) {
            (true, true) => online_revoked = true,
            (false, false) => offline_live = true,
            _ => {}
        }
    }

    if version_hit && !digest_hit && !req.expected_digest.is_empty() {
        return InstrumentError::DigestMismatchDep;
    }
    if online_revoked {
        return InstrumentError::RevokedDependency;
    }
    if digest_hit && offline_live {
        return InstrumentError::OfflineHeld;
    }
    if digest_hit && all_revoked {
        return InstrumentError::RevokedDependency;
    }
    if !any_id {
        return InstrumentError::RequiredMissing;
    }
    if all_revoked {
        return InstrumentError::RevokedDependency;
    }
    if !any_online {
        return InstrumentError::OfflineHeld;
    }
    InstrumentError::RequiredMissing
}

#[cfg(test)]
mod tests {
    use super::*;

    fn req(id: &str, constraint: &str, digest: &str, required: bool) -> DependencyReq {
        DependencyReq {
            id: id.into(),
            kind: DependencyKind::Ontology,
            version_constraint: constraint.into(),
            expected_digest: digest.into(),
            required,
            purpose: "test".into(),
        }
    }

    fn rec(
        id: &str,
        version: &str,
        digest: &str,
        locator: &str,
        revoked: bool,
        available_offline: bool,
    ) -> CatalogRecord {
        CatalogRecord {
            id: id.into(),
            version: version.into(),
            digest: digest.into(),
            locator: locator.into(),
            revoked,
            available_offline,
        }
    }

    #[test]
    fn exact_pin_resolves() {
        let pin = req("ont", "1.0.0", "", true);
        let catalog = [rec("ont", "1.0.0", "sha256:aa", "local:ont", false, true)];
        let lock = plan_resolution("rel", &[pin.clone(), pin], &catalog).unwrap();
        assert_eq!(lock.release_id, "rel");
        assert_eq!(lock.entries.len(), 1);
        assert_eq!(lock.entries[0].version, "1.0.0");
        assert_eq!(lock.entries[0].digest, "sha256:aa");
        assert_eq!(lock.entries[0].locator, "local:ont");
    }

    #[test]
    fn compatible_caret_matches_patch() {
        let requirements = [req("ont", "^1.0", "", true)];
        let catalog = [rec("ont", "1.0.2", "sha256:aa", "local:ont", false, true)];
        let lock = plan_resolution("rel", &requirements, &catalog).unwrap();
        assert_eq!(lock.entries[0].version, "1.0.2");
    }

    #[test]
    fn optional_missing_omitted_required_missing_errors() {
        let optional = [req("ont", "1.0.0", "", false)];
        let lock = plan_resolution("rel", &optional, &[]).unwrap();
        assert!(lock.entries.is_empty());

        let required = [req("ont", "1.0.0", "", true)];
        let err = plan_resolution("rel", &required, &[]).unwrap_err();
        assert_eq!(err, InstrumentError::RequiredMissing);
    }

    #[test]
    fn digest_mismatch_is_digest_mismatch_dep() {
        let requirements = [req("ont", "1.0.0", "sha256:aa", true)];
        let catalog = [rec("ont", "1.0.0", "sha256:bb", "local:ont", false, true)];
        let err = plan_resolution("rel", &requirements, &catalog).unwrap_err();
        assert_eq!(err, InstrumentError::DigestMismatchDep);
    }

    #[test]
    fn cycle_when_req_id_equals_release() {
        let requirements = [req("rel", "1.0.0", "", true)];
        let err = plan_resolution("rel", &requirements, &[]).unwrap_err();
        assert_eq!(err, InstrumentError::DependencyCycle);
    }

    #[test]
    fn revoked_required_is_revoked_dependency() {
        let requirements = [req("ont", "1.0.0", "", true)];
        let catalog = [rec("ont", "1.0.0", "sha256:aa", "local:ont", true, true)];
        let err = plan_resolution("rel", &requirements, &catalog).unwrap_err();
        assert_eq!(err, InstrumentError::RevokedDependency);
    }

    #[test]
    fn offline_required_is_offline_held() {
        let requirements = [req("ont", "1.0.0", "", true)];
        let catalog = [rec("ont", "1.0.0", "sha256:aa", "local:ont", false, false)];
        let err = plan_resolution("rel", &requirements, &catalog).unwrap_err();
        assert_eq!(err, InstrumentError::OfflineHeld);
    }

    #[test]
    fn lock_digest_stable_for_same_inputs() {
        let requirements = [req("lex", "2.0.0", "", true), req("ont", "1.0.0", "", true)];
        let catalog = [
            rec("ont", "1.0.0", "sha256:aa", "local:ont", false, true),
            rec("lex", "2.0.0", "sha256:bb", "local:lex", false, true),
        ];
        let a = plan_resolution("rel", &requirements, &catalog).unwrap();
        let reversed = [requirements[1].clone(), requirements[0].clone()];
        let b = plan_resolution("rel", &reversed, &catalog).unwrap();
        assert_eq!(a.entries[0].id, "lex");
        assert_eq!(a.entries[1].id, "ont");
        assert_eq!(a.digest(), b.digest());
    }
}
