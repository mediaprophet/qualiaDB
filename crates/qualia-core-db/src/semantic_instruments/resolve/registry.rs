//! Installed-release index (SI-05).
//!
//! Local catalog only. Collecting a release is distinct from activating it:
//! `closed == false` may be stored; activation requires verified closure.
//! Duplicate `release_id` is rejected unless `content_digest` is identical
//! (idempotent). No network fetch.

use std::collections::BTreeMap;

use super::dependency::DependencyLock;
use crate::semantic_instruments::errors::InstrumentError;

/// A collected release. `closed` means required deps were verified, not merely present.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstalledRelease {
    pub release_id: String,
    pub content_digest: String,
    pub lock: DependencyLock,
    pub closed: bool,
}

/// Deterministic in-memory index keyed by `release_id`.
#[derive(Debug, Clone, Default)]
pub struct LocalRegistry {
    by_id: BTreeMap<String, InstalledRelease>,
}

impl LocalRegistry {
    pub fn new() -> Self {
        Self {
            by_id: BTreeMap::new(),
        }
    }

    /// Store a collected release. Open records (`closed == false`) are allowed.
    ///
    /// Duplicate `release_id` is `Canonical("already installed")` unless the
    /// digest matches (idempotent no-op). Empty digest is `MissingField`.
    pub fn install(&mut self, release: InstalledRelease) -> Result<(), InstrumentError> {
        if release.release_id.is_empty() {
            return Err(InstrumentError::MissingField("release_id"));
        }
        if release.content_digest.is_empty() {
            return Err(InstrumentError::MissingField("content_digest"));
        }
        if let Some(existing) = self.by_id.get(&release.release_id) {
            if existing.content_digest == release.content_digest {
                return Ok(());
            }
            return Err(InstrumentError::Canonical("already installed".into()));
        }
        self.by_id.insert(release.release_id.clone(), release);
        Ok(())
    }

    pub fn lookup(&self, release_id: &str) -> Option<&InstalledRelease> {
        self.by_id.get(release_id)
    }

    /// Drop a collected release. Receipt history is not this index.
    pub fn uninstall(&mut self, release_id: &str) -> Option<InstalledRelease> {
        self.by_id.remove(release_id)
    }

    /// Fail closed: activation requires `closed == true` (all required deps verified).
    pub fn activate(&self, release_id: &str) -> Result<&InstalledRelease, InstrumentError> {
        let release = self
            .by_id
            .get(release_id)
            .ok_or(InstrumentError::RequiredMissing)?;
        if !release.closed {
            return Err(InstrumentError::NotClosed);
        }
        Ok(release)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const UNIT_CONVERT: &str = "https://ns.webizen.org/demo/unit-convert/releases/1.0.0";
    const DIGEST: &str = "sha256:aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn empty_lock(release_id: &str) -> DependencyLock {
        DependencyLock {
            release_id: release_id.into(),
            entries: Vec::new(),
        }
    }

    fn closed_unit_convert() -> InstalledRelease {
        InstalledRelease {
            release_id: UNIT_CONVERT.into(),
            content_digest: DIGEST.into(),
            lock: empty_lock(UNIT_CONVERT),
            closed: true,
        }
    }

    #[test]
    fn install_then_lookup() {
        let mut reg = LocalRegistry::new();
        reg.install(closed_unit_convert()).unwrap();
        let found = reg.lookup(UNIT_CONVERT).expect("stored");
        assert_eq!(found.release_id, UNIT_CONVERT);
        assert_eq!(found.content_digest, DIGEST);
        assert!(found.lock.is_empty());
        assert!(found.closed);
    }

    #[test]
    fn activate_closed_release_ok() {
        let mut reg = LocalRegistry::new();
        // Empty lock + closed is valid: unit-convert has no required deps.
        reg.install(closed_unit_convert()).unwrap();
        let active = reg.activate(UNIT_CONVERT).unwrap();
        assert_eq!(active.release_id, UNIT_CONVERT);
        assert!(active.closed);
    }

    #[test]
    fn activate_open_release_is_not_closed() {
        let mut reg = LocalRegistry::new();
        let mut open = closed_unit_convert();
        open.closed = false;
        reg.install(open).unwrap();
        assert!(reg.lookup(UNIT_CONVERT).is_some());
        assert_eq!(reg.activate(UNIT_CONVERT), Err(InstrumentError::NotClosed));
    }

    #[test]
    fn activate_missing_is_required_missing() {
        let reg = LocalRegistry::new();
        assert_eq!(
            reg.activate(UNIT_CONVERT),
            Err(InstrumentError::RequiredMissing)
        );
    }

    #[test]
    fn empty_digest_is_missing_field() {
        let mut reg = LocalRegistry::new();
        let mut release = closed_unit_convert();
        release.content_digest.clear();
        assert_eq!(
            reg.install(release),
            Err(InstrumentError::MissingField("content_digest"))
        );
        assert!(reg.lookup(UNIT_CONVERT).is_none());
    }

    #[test]
    fn uninstall_drops_lookup_not_history() {
        let mut reg = LocalRegistry::new();
        reg.install(closed_unit_convert()).unwrap();
        assert!(reg.uninstall(UNIT_CONVERT).is_some());
        assert!(reg.lookup(UNIT_CONVERT).is_none());
        assert!(reg.uninstall(UNIT_CONVERT).is_none());
    }

    #[test]
    fn duplicate_id_rejects_unless_digest_identical() {
        let mut reg = LocalRegistry::new();
        reg.install(closed_unit_convert()).unwrap();
        // Idempotent: same id + same digest is a no-op.
        reg.install(closed_unit_convert()).unwrap();
        let mut other = closed_unit_convert();
        other.content_digest =
            "sha256:bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into();
        match reg.install(other) {
            Err(InstrumentError::Canonical(msg)) => assert_eq!(msg, "already installed"),
            other => panic!("expected Canonical already installed, got {other:?}"),
        }
        assert_eq!(reg.lookup(UNIT_CONVERT).unwrap().content_digest, DIGEST);
    }
}
