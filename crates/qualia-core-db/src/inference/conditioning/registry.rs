//! Profile registry with versioning, promotion/activation, and rollback (PP-081).
//!
//! Enforces reviewable lifecycle transitions, opaque version references,
//! and fail-closed state tracking.

use std::collections::BTreeMap;
use std::sync::{OnceLock, RwLock};

/// An immutable record of a registered profile version.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileVersionEntry {
    pub profile_id: String,
    pub version: u64,
    pub spec_identity: u64,
    pub approved_by: Option<String>,
}

/// Lifecycle state machine for a specific profile ID.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ProfileLifecycleState {
    pub active_version: Option<u64>,
    pub previous_version: Option<u64>,
    pub versions: BTreeMap<u64, ProfileVersionEntry>,
}

/// Profile registry tracking versions and active promotion pointers.
#[derive(Default, Debug)]
pub struct ConditioningRegistry {
    profiles: BTreeMap<String, ProfileLifecycleState>,
}

impl ConditioningRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a new profile version.
    pub fn register(
        &mut self,
        profile_id: &str,
        version: u64,
        spec_identity: u64,
        approved_by: Option<String>,
    ) -> Result<(), &'static str> {
        if profile_id.is_empty() || version == 0 {
            return Err("invalid profile_id or version");
        }
        let state = self.profiles.entry(profile_id.to_string()).or_default();
        if state.versions.contains_key(&version) {
            return Err("version already registered");
        }
        state.versions.insert(
            version,
            ProfileVersionEntry {
                profile_id: profile_id.to_string(),
                version,
                spec_identity,
                approved_by,
            },
        );
        Ok(())
    }

    /// Activate / promote a version to become the current active profile.
    pub fn activate(
        &mut self,
        profile_id: &str,
        version: u64,
    ) -> Result<(Option<u64>, u64), &'static str> {
        let state = self.profiles.get_mut(profile_id).ok_or("profile not found")?;
        if !state.versions.contains_key(&version) {
            return Err("version not found in registry");
        }
        let prev = state.active_version;
        state.previous_version = prev;
        state.active_version = Some(version);
        Ok((prev, version))
    }

    /// Rollback active version to a target version or previous active version.
    pub fn rollback(
        &mut self,
        profile_id: &str,
        target_version: Option<u64>,
    ) -> Result<(u64, u64), &'static str> {
        let state = self.profiles.get_mut(profile_id).ok_or("profile not found")?;
        let current = state.active_version.ok_or("no active version to rollback from")?;
        let target = match target_version {
            Some(t) => {
                if !state.versions.contains_key(&t) {
                    return Err("target rollback version not registered");
                }
                t
            }
            None => state
                .previous_version
                .ok_or("no previous active version available for rollback")?,
        };
        state.previous_version = Some(current);
        state.active_version = Some(target);
        Ok((current, target))
    }

    /// Query the currently active version for a profile.
    pub fn active_version(&self, profile_id: &str) -> Option<u64> {
        self.profiles.get(profile_id).and_then(|s| s.active_version)
    }

    /// Query the currently active version entry for a profile.
    pub fn get_active(&self, profile_id: &str) -> Option<&ProfileVersionEntry> {
        let v = self.active_version(profile_id)?;
        self.get_version(profile_id, v)
    }

    /// Query entry details for a registered version.
    pub fn get_version(&self, profile_id: &str, version: u64) -> Option<&ProfileVersionEntry> {
        self.profiles.get(profile_id).and_then(|s| s.versions.get(&version))
    }

    /// List all registered versions for a profile.
    pub fn list_versions(&self, profile_id: &str) -> Vec<u64> {
        self.profiles
            .get(profile_id)
            .map(|s| s.versions.keys().copied().collect())
            .unwrap_or_default()
    }
}

static GLOBAL_REGISTRY: OnceLock<RwLock<ConditioningRegistry>> = OnceLock::new();

pub fn global_registry() -> &'static RwLock<ConditioningRegistry> {
    GLOBAL_REGISTRY.get_or_init(|| RwLock::new(ConditioningRegistry::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_registry_lifecycle_flow() {
        let mut reg = ConditioningRegistry::new();
        let pid = "urn:qualia:profile:rust-review:v1";

        // Register versions 1 and 2
        assert!(reg.register(pid, 1, 0x1111, Some("timothy".into())).is_ok());
        assert!(reg.register(pid, 2, 0x2222, Some("timothy".into())).is_ok());

        // Duplicate registration fails
        assert!(reg.register(pid, 1, 0x1111, None).is_err());

        // Initial active is None
        assert_eq!(reg.active_version(pid), None);

        // Activate version 1
        let (prev, cur) = reg.activate(pid, 1).unwrap();
        assert_eq!(prev, None);
        assert_eq!(cur, 1);
        assert_eq!(reg.active_version(pid), Some(1));

        // Promote to version 2
        let (prev, cur) = reg.activate(pid, 2).unwrap();
        assert_eq!(prev, Some(1));
        assert_eq!(cur, 2);
        assert_eq!(reg.active_version(pid), Some(2));

        // Rollback without target -> reverts to previous active (1)
        let (from, to) = reg.rollback(pid, None).unwrap();
        assert_eq!(from, 2);
        assert_eq!(to, 1);
        assert_eq!(reg.active_version(pid), Some(1));

        // Rollback with explicit target 2
        let (from, to) = reg.rollback(pid, Some(2)).unwrap();
        assert_eq!(from, 1);
        assert_eq!(to, 2);
        assert_eq!(reg.active_version(pid), Some(2));
    }
}
